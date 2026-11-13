//! Compilatore: dall'AST a una [`ScenaCompilata`] pronta per il generatore.
//!
//! Qui avvengono tutte le decisioni "intelligenti": validazione semantica, risoluzione
//! delle posizioni relative, ancoraggio dei blocchi temporali relativi e pre-calcolo
//! dei keyframe per ogni canale di animazione. Il generatore si limita a stampare.
//!
//! Oltre agli errori (bloccanti), il compilatore raccoglie gli [`Avviso`] non bloccanti
//! di SPECIFICA B.6 nel campo [`ScenaCompilata::avvisi`].

pub mod easing;

use std::collections::HashMap;

use crate::ast::{Ancora, Attore, Colore, Easing, Forma, Posizione, Programma, Stile, Tempo, Verbo};
use crate::errore::{Avviso, Errore, Risultato};
use crate::parser::suggerisci;
use crate::security::{limiti, valida_nome_font};
use easing::{Curva, curva};

/// Tolleranza per i confronti tra istanti (in secondi).
const EPSILON: f64 = 1e-9;

/// Margine delle ancore del palco (`in alto` = 10% dal bordo superiore).
const MARGINE_ANCORE: f64 = 0.1;

/// Scena pronta per la generazione dell'HTML.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenaCompilata {
    /// Larghezza del palco in pixel.
    pub larghezza: f64,
    /// Altezza del palco in pixel.
    pub altezza: f64,
    /// Colore di sfondo.
    pub sfondo: Colore,
    /// FPS dichiarati (informativi).
    pub fps: Option<u32>,
    /// Durata totale della timeline in secondi (0 se non ci sono blocchi).
    pub durata: f64,
    /// Attori nell'ordine di creazione.
    pub attori: Vec<AttoreCompilato>,
    /// Avvisi non bloccanti (SPECIFICA B.6), ordinati per riga crescente e senza
    /// duplicati (stessa riga e stesso messaggio compaiono una volta sola).
    pub avvisi: Vec<Avviso>,
}

/// Attore con posizione assoluta risolta e animazioni pre-calcolate.
#[derive(Debug, Clone, PartialEq)]
pub struct AttoreCompilato {
    /// Identificatore HTML/CSS generato (`a0`, `a1`, ...): mai derivato dall'input utente.
    pub id: String,
    /// Nome originale (da sanificare prima di qualsiasi output).
    pub nome: String,
    /// Forma validata.
    pub forma: Forma,
    /// Stile validato.
    pub stile: Stile,
    /// Ascissa del centro, in pixel.
    pub x: f64,
    /// Ordinata del centro, in pixel.
    pub y: f64,
    /// Opacità a inizio scena.
    pub opacita_iniziale: f64,
    /// Scala a inizio scena (`s0`, SPECIFICA B.1). Se ≠ 1 il generatore la emette come
    /// `scale` nella regola statica del wrapper `#aN`.
    pub scala_iniziale: f64,
    /// Rotazione a inizio scena in gradi (`r0`, SPECIFICA B.1). Se ≠ 0 il generatore la
    /// emette come `rotate` nella regola statica del wrapper `#aN`.
    pub rotazione_iniziale: f64,
    /// Animazioni, al massimo una per canale.
    pub animazioni: Vec<Animazione>,
}

/// Canali di animazione indipendenti (proprietà CSS individuali componibili).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Canale {
    /// `opacity`
    Opacita,
    /// `translate`
    Traslazione,
    /// `rotate`
    Rotazione,
    /// `scale`
    Scala,
}

impl Canale {
    /// Suffisso usato nei nomi `@keyframes` generati.
    pub fn suffisso(self) -> &'static str {
        match self {
            Canale::Opacita => "opacita",
            Canale::Traslazione => "traslazione",
            Canale::Rotazione => "rotazione",
            Canale::Scala => "scala",
        }
    }

    /// Proprietà CSS animata dal canale.
    pub fn proprieta_css(self) -> &'static str {
        match self {
            Canale::Opacita => "opacity",
            Canale::Traslazione => "translate",
            Canale::Rotazione => "rotate",
            Canale::Scala => "scale",
        }
    }

    /// Nome leggibile, per i messaggi d'errore.
    fn descrizione(self) -> &'static str {
        match self {
            Canale::Opacita => "l'opacità",
            Canale::Traslazione => "la posizione",
            Canale::Rotazione => "la rotazione",
            Canale::Scala => "la scala",
        }
    }

    fn valore(self, v: [f64; 2]) -> Valore {
        match self {
            Canale::Opacita => Valore::Opacita(v[0]),
            Canale::Traslazione => Valore::Traslazione(v[0], v[1]),
            Canale::Rotazione => Valore::Rotazione(v[0]),
            Canale::Scala => Valore::Scala(v[0]),
        }
    }
}

/// Animazione di un singolo canale lungo tutta la durata della scena.
#[derive(Debug, Clone, PartialEq)]
pub struct Animazione {
    /// Canale animato.
    pub canale: Canale,
    /// Keyframe ordinati per `offset` crescente (il primo a 0, l'ultimo a 1).
    pub keyframe: Vec<Keyframe>,
}

/// Un fotogramma chiave.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe {
    /// Posizione nella timeline, da 0.0 (inizio) a 1.0 (fine scena).
    pub offset: f64,
    /// Valore del canale in questo istante.
    pub valore: Valore,
    /// Curva da applicare fino al keyframe successivo (`None` = lineare).
    pub curva: Option<Curva>,
}

/// Valore tipizzato di un canale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Valore {
    /// Opacità 0.0-1.0.
    Opacita(f64),
    /// Spostamento (dx, dy) in pixel rispetto alla posizione iniziale.
    Traslazione(f64, f64),
    /// Rotazione in gradi.
    Rotazione(f64),
    /// Fattore di scala.
    Scala(f64),
}

/// Effetto di un segmento sul proprio canale.
#[derive(Debug, Clone, Copy)]
enum Effetto {
    /// Raggiunge un valore assoluto.
    Verso([f64; 2]),
    /// Somma un delta al valore corrente.
    Delta([f64; 2]),
    /// Oscilla tra il valore corrente e `valore × (1 + ampiezza)`.
    Pulsazione { ampiezza: f64, bpm: f64 },
}

/// Un'azione collocata nel tempo su un canale.
#[derive(Debug, Clone, Copy)]
struct Segmento {
    inizio: f64,
    fine: f64,
    effetto: Effetto,
    easing: Easing,
    /// `true` per `appare sfumando`: se è il primo segmento di opacità, l'attore parte invisibile.
    appare: bool,
    riga: usize,
}

/// Messaggio dell'avviso «rotazione invisibile su un cerchio» (SPECIFICA B.6).
fn messaggio_cerchio_ruotato(nome: &str) -> String {
    format!("\"{nome}\" è un cerchio: la rotazione non cambia il suo aspetto")
}

/// Compila un [`Programma`] in una [`ScenaCompilata`].
///
/// Gli avvisi non bloccanti (SPECIFICA B.6) sono restituiti in [`ScenaCompilata::avvisi`].
///
/// # Errori
/// Errori semantici (attori sconosciuti o duplicati, posizioni circolari, azioni
/// sovrapposte, valori fuori dai limiti) con riga e suggerimento.
pub fn compila(programma: &Programma) -> Risultato<ScenaCompilata> {
    let palco = &programma.palco;
    for (nome, valore) in [("larghezza", palco.larghezza), ("altezza", palco.altezza)] {
        if !(1.0..=limiti::MAX_LATO_PALCO).contains(&valore) {
            return Err(Errore::alla_riga(
                palco.riga,
                format!("la {nome} del palco deve essere tra 1 e {} pixel", limiti::MAX_LATO_PALCO),
            ));
        }
    }
    if programma.attori.len() > limiti::MAX_ATTORI {
        return Err(Errore::globale(format!(
            "troppi attori ({}): il massimo è {}",
            programma.attori.len(),
            limiti::MAX_ATTORI
        )));
    }

    let mut avvisi = Vec::new();
    let mut indici: HashMap<&str, usize> = HashMap::new();
    for (i, attore) in programma.attori.iter().enumerate() {
        valida_attore(attore)?;
        if matches!(attore.forma, Forma::Cerchio { .. }) && attore.stile.rotazione != 0.0 {
            avvisi.push(
                Avviso::alla_riga(attore.riga, messaggio_cerchio_ruotato(&attore.nome))
                    .con_suggerimento("togli «rotazione …» dalla creazione del cerchio"),
            );
        }
        if let Some(&precedente) = indici.get(attore.nome.as_str()) {
            return Err(Errore::alla_riga(
                attore.riga,
                format!(
                    "esiste già un attore chiamato \"{}\" (creato alla riga {})",
                    attore.nome, programma.attori[precedente].riga
                ),
            )
            .con_suggerimento("dai a ogni attore un nome diverso"));
        }
        indici.insert(attore.nome.as_str(), i);
    }

    let posizioni = risolvi_posizioni(programma, &indici)?;
    let (segmenti, durata) = costruisci_segmenti(programma, &indici, &posizioni, &mut avvisi)?;

    let mut attori = Vec::with_capacity(programma.attori.len());
    for (i, ((attore, (x, y)), canali)) in programma
        .attori
        .iter()
        .zip(posizioni)
        .zip(segmenti)
        .enumerate()
    {
        let mut opacita_iniziale = attore.stile.opacita;
        let mut animazioni = Vec::new();
        for (canale, mut segmenti) in canali {
            if segmenti.is_empty() {
                continue;
            }
            segmenti.sort_by(|a, b| a.inizio.total_cmp(&b.inizio));
            controlla_sovrapposizioni(&attore.nome, canale, &segmenti)?;
            let iniziale = match canale {
                Canale::Opacita => {
                    if segmenti.first().is_some_and(|s| s.appare) {
                        opacita_iniziale = 0.0;
                    }
                    [opacita_iniziale, 0.0]
                }
                Canale::Traslazione => [0.0, 0.0],
                Canale::Rotazione => [attore.stile.rotazione, 0.0],
                Canale::Scala => [attore.stile.scala, 0.0],
            };
            animazioni.push(Animazione {
                canale,
                keyframe: genera_keyframe(canale, iniziale, &segmenti, durata)?,
            });
        }
        animazioni.sort_by_key(|a| a.canale);
        attori.push(AttoreCompilato {
            id: format!("a{i}"),
            nome: attore.nome.clone(),
            forma: attore.forma.clone(),
            stile: attore.stile.clone(),
            x,
            y,
            opacita_iniziale,
            scala_iniziale: attore.stile.scala,
            rotazione_iniziale: attore.stile.rotazione,
            animazioni,
        });
    }

    // Ordinamento stabile: a parità di riga resta l'ordine di rilevazione.
    avvisi.sort_by_key(|a| a.riga);
    avvisi.dedup_by(|b, a| a.riga == b.riga && a.messaggio == b.messaggio);

    Ok(ScenaCompilata {
        larghezza: palco.larghezza,
        altezza: palco.altezza,
        sfondo: palco.sfondo,
        fps: palco.fps,
        durata,
        attori,
        avvisi,
    })
}

/// Verifica i valori di un attore rispetto ai limiti di sicurezza.
fn valida_attore(attore: &Attore) -> Risultato<()> {
    let riga = attore.riga;
    if attore.nome.chars().count() > limiti::MAX_LUNGHEZZA_NOME {
        return Err(Errore::alla_riga(
            riga,
            format!("il nome dell'attore è troppo lungo (massimo {} caratteri)", limiti::MAX_LUNGHEZZA_NOME),
        ));
    }
    let dimensione = |nome: &str, v: f64| -> Risultato<()> {
        if (0.0..=limiti::MAX_DIMENSIONE).contains(&v) {
            Ok(())
        } else {
            Err(Errore::alla_riga(
                riga,
                format!("{nome} deve essere tra 0 e {} pixel (trovato {v})", limiti::MAX_DIMENSIONE),
            ))
        }
    };
    match &attore.forma {
        Forma::Cerchio { raggio } => dimensione("il raggio", *raggio)?,
        Forma::Rettangolo { larghezza, altezza, arrotondamento } => {
            dimensione("la larghezza", *larghezza)?;
            dimensione("l'altezza", *altezza)?;
            dimensione("l'arrotondamento", *arrotondamento)?;
        }
        Forma::Testo { contenuto, grandezza, font, .. } => {
            dimensione("la grandezza del testo", *grandezza)?;
            if contenuto.chars().count() > limiti::MAX_LUNGHEZZA_TESTO {
                return Err(Errore::alla_riga(
                    riga,
                    format!("il testo è troppo lungo (massimo {} caratteri)", limiti::MAX_LUNGHEZZA_TESTO),
                ));
            }
            if let Some(f) = font {
                valida_nome_font(f).map_err(|m| Errore::alla_riga(riga, m))?;
            }
        }
    }
    if !(0.0..=1.0).contains(&attore.stile.opacita) {
        return Err(Errore::alla_riga(riga, "l'opacità deve essere tra 0.0 e 1.0"));
    }
    if let Some((spessore, _)) = attore.stile.bordo {
        dimensione("lo spessore del bordo", spessore)?;
    }
    let (scala, rotazione) = (attore.stile.scala, attore.stile.rotazione);
    if scala < 0.0 {
        return Err(Errore::alla_riga(riga, "la scala iniziale non può essere negativa")
            .con_suggerimento("usa un valore da 0 in su, es. «scala 0.5»"));
    }
    // `contains` è falso anche per NaN e infiniti: anche quelli sono rifiutati.
    if !(0.0..=limiti::MAX_SCALA).contains(&scala) {
        return Err(Errore::alla_riga(
            riga,
            format!("la scala iniziale è troppo grande (massimo {})", limiti::MAX_SCALA),
        ));
    }
    if !(-limiti::MAX_GRADI..=limiti::MAX_GRADI).contains(&rotazione) {
        return Err(Errore::alla_riga(
            riga,
            format!("la rotazione iniziale è troppo grande (massimo {} gradi)", limiti::MAX_GRADI),
        ));
    }
    Ok(())
}

/// Cerca l'indice dell'attore `nome`; se non esiste, l'errore suggerisce il nome più simile.
///
/// `escludi` è il nome dell'attore che si sta posizionando: non viene mai proposto nel
/// suggerimento «forse intendevi …?», perché un attore non può riferirsi a sé stesso.
fn indice_attore(
    nome: &str,
    indici: &HashMap<&str, usize>,
    riga: usize,
    escludi: Option<&str>,
) -> Risultato<usize> {
    indici.get(nome).copied().ok_or_else(|| {
        // Ordinati: il suggerimento non deve dipendere dall'ordine della HashMap.
        let mut nomi: Vec<&str> =
            indici.keys().copied().filter(|n| Some(*n) != escludi).collect();
        nomi.sort_unstable();
        let e = Errore::alla_riga(riga, format!("non esiste nessun attore chiamato \"{nome}\""));
        match suggerisci(nome, &nomi) {
            Some(s) => e.con_suggerimento(format!("forse intendevi \"{s}\"?")),
            None => e.con_suggerimento("crea prima l'attore con «Crea ... chiamato \"nome\" ...»"),
        }
    })
}

fn controlla_coordinate(x: f64, y: f64, riga: usize) -> Risultato<(f64, f64)> {
    if x.abs() > limiti::MAX_COORDINATA || y.abs() > limiti::MAX_COORDINATA {
        return Err(Errore::alla_riga(
            riga,
            format!("coordinate fuori dai limiti (massimo ±{} pixel)", limiti::MAX_COORDINATA),
        ));
    }
    Ok((x, y))
}

/// Calcola il centro assoluto di ogni attore, risolvendo le catene di posizioni
/// relative in modo iterativo (nessuna ricorsione: niente stack overflow) e
/// rilevando i riferimenti circolari.
fn risolvi_posizioni(
    programma: &Programma,
    indici: &HashMap<&str, usize>,
) -> Risultato<Vec<(f64, f64)>> {
    let attori = &programma.attori;
    let (l, h) = (programma.palco.larghezza, programma.palco.altezza);
    let mut risolte: Vec<Option<(f64, f64)>> = vec![None; attori.len()];
    let mut in_catena = vec![false; attori.len()];

    for i in 0..attori.len() {
        let mut catena = Vec::new();
        let mut corrente = i;
        while risolte[corrente].is_none() {
            if in_catena[corrente] {
                let nomi: Vec<String> = catena
                    .iter()
                    .map(|&j: &usize| format!("\"{}\"", attori[j].nome))
                    .collect();
                return Err(Errore::alla_riga(
                    attori[corrente].riga,
                    format!("posizioni circolari: {} dipendono l'uno dall'altro", nomi.join(" → ")),
                )
                .con_suggerimento("almeno uno degli attori deve avere una posizione assoluta o ancorata al palco"));
            }
            in_catena[corrente] = true;
            catena.push(corrente);
            match &attori[corrente].posizione {
                Posizione::Relativa { riferimento, .. } => {
                    let attore = &attori[corrente];
                    corrente = indice_attore(riferimento, indici, attore.riga, Some(&attore.nome))?;
                }
                _ => break,
            }
        }
        for &j in catena.iter().rev() {
            let attore = &attori[j];
            let punto = match &attore.posizione {
                Posizione::Assoluta { x, y } => (*x, *y),
                Posizione::Palco(ancora) => {
                    let (fx, fy) = match ancora {
                        Ancora::Centro => (0.5, 0.5),
                        Ancora::Alto => (0.5, MARGINE_ANCORE),
                        Ancora::Basso => (0.5, 1.0 - MARGINE_ANCORE),
                        Ancora::Sinistra => (MARGINE_ANCORE, 0.5),
                        Ancora::Destra => (1.0 - MARGINE_ANCORE, 0.5),
                        Ancora::AltoSinistra => (MARGINE_ANCORE, MARGINE_ANCORE),
                        Ancora::AltoDestra => (1.0 - MARGINE_ANCORE, MARGINE_ANCORE),
                        Ancora::BassoSinistra => (MARGINE_ANCORE, 1.0 - MARGINE_ANCORE),
                        Ancora::BassoDestra => (1.0 - MARGINE_ANCORE, 1.0 - MARGINE_ANCORE),
                    };
                    (l * fx, h * fy)
                }
                Posizione::Relativa { direzione, riferimento, distanza } => {
                    let k = indice_attore(riferimento, indici, attore.riga, Some(&attore.nome))?;
                    let (rx, ry) = risolte[k].ok_or_else(|| {
                        Errore::alla_riga(attore.riga, "posizione relativa non risolvibile")
                    })?;
                    let (dx, dy) = direzione.vettore();
                    (rx + dx * distanza, ry + dy * distanza)
                }
            };
            risolte[j] = Some(controlla_coordinate(punto.0, punto.1, attore.riga)?);
            in_catena[j] = false;
        }
    }

    risolte
        .into_iter()
        .map(|p| p.ok_or_else(|| Errore::globale("errore interno: posizione non risolta")))
        .collect()
}

type SegmentiPerCanale = Vec<(Canale, Vec<Segmento>)>;

/// Colloca i blocchi nel tempo e converte ogni azione in un segmento su un canale.
/// Restituisce i segmenti per attore e la durata totale della scena.
fn costruisci_segmenti(
    programma: &Programma,
    indici: &HashMap<&str, usize>,
    posizioni: &[(f64, f64)],
    avvisi: &mut Vec<Avviso>,
) -> Risultato<(Vec<SegmentiPerCanale>, f64)> {
    let canali = [Canale::Opacita, Canale::Traslazione, Canale::Rotazione, Canale::Scala];
    let mut per_attore: Vec<SegmentiPerCanale> = programma
        .attori
        .iter()
        .map(|_| canali.iter().map(|c| (*c, Vec::new())).collect())
        .collect();

    let mut fine_precedente = 0.0_f64;
    let mut durata = 0.0_f64;
    for blocco in &programma.timeline.blocchi {
        let (inizio, fine) = match blocco.tempo {
            Tempo::Assoluto { inizio, fine } => (inizio, fine),
            Tempo::Relativo { attesa, durata } => {
                let inizio = fine_precedente + attesa;
                (inizio, inizio + durata)
            }
        };
        if fine > limiti::MAX_DURATA_SCENA {
            return Err(Errore::alla_riga(
                blocco.riga,
                format!("la scena supererebbe la durata massima di {} secondi", limiti::MAX_DURATA_SCENA),
            ));
        }
        fine_precedente = fine;
        durata = durata.max(fine);

        for azione in &blocco.azioni {
            let i = indice_attore(&azione.attore, indici, azione.riga, None)?;
            let attore = &programma.attori[i];
            let (x0, y0) = posizioni[i];
            let errore = |m: String| Errore::alla_riga(azione.riga, m);
            let (canale, effetto, appare) = match azione.verbo {
                Verbo::AppareSfumando => {
                    (Canale::Opacita, Effetto::Verso([attore.stile.opacita, 0.0]), true)
                }
                Verbo::ScompareSfumando => (Canale::Opacita, Effetto::Verso([0.0, 0.0]), false),
                Verbo::CambiaOpacita(v) => {
                    if !(0.0..=1.0).contains(&v) {
                        return Err(errore("l'opacità deve essere tra 0.0 e 1.0".into()));
                    }
                    (Canale::Opacita, Effetto::Verso([v, 0.0]), false)
                }
                Verbo::SiSpostaA { x, y } => {
                    controlla_coordinate(x, y, azione.riga)?;
                    (Canale::Traslazione, Effetto::Verso([x - x0, y - y0]), false)
                }
                Verbo::Scivola { direzione, distanza } => {
                    if distanza.abs() > limiti::MAX_COORDINATA {
                        return Err(errore(format!(
                            "spostamento troppo grande (massimo {} pixel)",
                            limiti::MAX_COORDINATA
                        )));
                    }
                    let (dx, dy) = direzione.vettore();
                    (Canale::Traslazione, Effetto::Delta([dx * distanza, dy * distanza]), false)
                }
                Verbo::Ruota(gradi) => {
                    if gradi.abs() > limiti::MAX_GRADI {
                        return Err(errore(format!("rotazione troppo grande (massimo {} gradi)", limiti::MAX_GRADI)));
                    }
                    if matches!(attore.forma, Forma::Cerchio { .. }) {
                        avvisi.push(
                            Avviso::alla_riga(azione.riga, messaggio_cerchio_ruotato(&attore.nome))
                                .con_suggerimento(
                                    "togli «ruota di …», oppure usa un quadrato o un rettangolo se vuoi vederlo girare",
                                ),
                        );
                    }
                    (Canale::Rotazione, Effetto::Delta([gradi, 0.0]), false)
                }
                Verbo::ScalaFinoA(f) => {
                    if f > limiti::MAX_SCALA {
                        return Err(errore(format!("fattore di scala troppo grande (massimo {})", limiti::MAX_SCALA)));
                    }
                    (Canale::Scala, Effetto::Verso([f, 0.0]), false)
                }
                Verbo::Pulsa { intensita, bpm } => {
                    if !(bpm > 0.0 && bpm <= limiti::MAX_BPM) {
                        return Err(errore(format!("il ritmo deve essere tra 1 e {} BPM", limiti::MAX_BPM)));
                    }
                    if intensita > limiti::MAX_INTENSITA {
                        return Err(errore(format!("intensità troppo alta (massimo {})", limiti::MAX_INTENSITA)));
                    }
                    (Canale::Scala, Effetto::Pulsazione { ampiezza: 0.25 * intensita, bpm }, false)
                }
            };
            let segmento = Segmento { inizio, fine, effetto, easing: azione.easing, appare, riga: azione.riga };
            if let Some((_, lista)) = per_attore[i].iter_mut().find(|(c, _)| *c == canale) {
                lista.push(segmento);
            }
        }
    }
    Ok((per_attore, durata))
}

fn controlla_sovrapposizioni(nome: &str, canale: Canale, segmenti: &[Segmento]) -> Risultato<()> {
    for coppia in segmenti.windows(2) {
        if let [a, b] = coppia
            && b.inizio < a.fine - EPSILON
        {
            return Err(Errore::alla_riga(
                b.riga,
                format!(
                    "\"{nome}\" ha due azioni che cambiano {} nello stesso momento (righe {} e {})",
                    canale.descrizione(),
                    a.riga,
                    b.riga
                ),
            )
            .con_suggerimento("sposta una delle due azioni in un blocco successivo, es. «Poi per 1 secondo:»"));
        }
    }
    Ok(())
}

/// Accumula keyframe fondendo quelli che cadono nello stesso istante.
struct Accumulatore {
    canale: Canale,
    durata: f64,
    keyframe: Vec<Keyframe>,
}

impl Accumulatore {
    fn aggiungi(&mut self, tempo: f64, valore: [f64; 2], curva: Option<Curva>) -> Risultato<()> {
        let offset = if self.durata > 0.0 { (tempo / self.durata).clamp(0.0, 1.0) } else { 0.0 };
        let valore = self.canale.valore(valore);
        if let Some(ultimo) = self.keyframe.last_mut()
            && (ultimo.offset - offset).abs() < EPSILON
        {
            ultimo.valore = valore;
            if curva.is_some() {
                ultimo.curva = curva;
            }
            return Ok(());
        }
        if self.keyframe.len() >= limiti::MAX_KEYFRAME {
            return Err(Errore::globale(format!(
                "animazione troppo complessa: più di {} fotogrammi chiave",
                limiti::MAX_KEYFRAME
            ))
            .con_suggerimento("riduci il ritmo delle pulsazioni o la durata dei blocchi"));
        }
        self.keyframe.push(Keyframe { offset, valore, curva });
        Ok(())
    }
}

/// Converte i segmenti (ordinati, non sovrapposti) di un canale in keyframe sull'intera scena.
fn genera_keyframe(
    canale: Canale,
    iniziale: [f64; 2],
    segmenti: &[Segmento],
    durata: f64,
) -> Risultato<Vec<Keyframe>> {
    let mut acc = Accumulatore { canale, durata, keyframe: Vec::new() };
    let mut corrente = iniziale;
    acc.aggiungi(0.0, corrente, None)?;

    for s in segmenti {
        let c = curva(s.easing);
        acc.aggiungi(s.inizio, corrente, Some(c.clone()))?;
        match s.effetto {
            Effetto::Verso(v) => corrente = v,
            Effetto::Delta(d) => corrente = [corrente[0] + d[0], corrente[1] + d[1]],
            Effetto::Pulsazione { ampiezza, bpm } => {
                let durata_seg = s.fine - s.inizio;
                let periodo = 60.0 / bpm;
                let battiti = (durata_seg / periodo).round().max(1.0);
                if battiti * 2.0 > limiti::MAX_KEYFRAME as f64 {
                    return Err(Errore::alla_riga(
                        s.riga,
                        format!("troppi battiti ({battiti}) per questa pulsazione"),
                    )
                    .con_suggerimento("riduci il ritmo o la durata del blocco"));
                }
                let mezzi = battiti as usize * 2;
                let passo = durata_seg / mezzi as f64;
                let base = corrente[0];
                let picco = base * (1.0 + ampiezza);
                for k in 1..mezzi {
                    let v = if k % 2 == 1 { picco } else { base };
                    acc.aggiungi(s.inizio + passo * k as f64, [v, 0.0], Some(c.clone()))?;
                }
            }
        }
        acc.aggiungi(s.fine, corrente, None)?;
    }
    acc.aggiungi(durata, corrente, None)?;
    Ok(acc.keyframe)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::parser::analizza;

    const PALCO: &str = "Prepara un palco grande 400 per 300.\n";

    /// Analizza il sorgente e applica `modifica` all'AST: il parser non conosce
    /// ancora le proprietà `scala` e `rotazione` di B.1.
    fn programma(corpo: &str, modifica: impl FnOnce(&mut Programma)) -> Programma {
        let mut p = analizza(&format!("{PALCO}{corpo}")).unwrap();
        modifica(&mut p);
        p
    }

    fn keyframe(scena: &ScenaCompilata, attore: usize, canale: Canale) -> Vec<Valore> {
        scena.attori[attore]
            .animazioni
            .iter()
            .find(|a| a.canale == canale)
            .unwrap()
            .keyframe
            .iter()
            .map(|k| k.valore)
            .collect()
    }

    #[test]
    fn stato_iniziale_predefinito() {
        let s = compila(&programma("Crea un quadrato chiamato \"q\" al centro.\n", |_| {})).unwrap();
        assert_eq!(s.attori[0].scala_iniziale, 1.0);
        assert_eq!(s.attori[0].rotazione_iniziale, 0.0);
        assert!(s.avvisi.is_empty());
    }

    #[test]
    fn stato_iniziale_dichiarato_entra_nei_canali() {
        let p = programma(
            "Crea un quadrato chiamato \"q\" al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"q\" ruota di 90 gradi.\n\
             \x20 \"q\" si ingrandisce fino a 2.\n",
            |p| {
                p.attori[0].stile.scala = 0.0;
                p.attori[0].stile.rotazione = 45.0;
            },
        );
        let s = compila(&p).unwrap();
        assert_eq!(s.attori[0].scala_iniziale, 0.0);
        assert_eq!(s.attori[0].rotazione_iniziale, 45.0);
        // `ruota di` è relativo, `si ingrandisce` è assoluto.
        let r = keyframe(&s, 0, Canale::Rotazione);
        assert_eq!(r.first(), Some(&Valore::Rotazione(45.0)));
        assert_eq!(r.last(), Some(&Valore::Rotazione(135.0)));
        let k = keyframe(&s, 0, Canale::Scala);
        assert_eq!(k.first(), Some(&Valore::Scala(0.0)));
        assert_eq!(k.last(), Some(&Valore::Scala(2.0)));
    }

    #[test]
    fn pulsa_attorno_alla_scala_iniziale() {
        let corpo = "Crea un quadrato chiamato \"q\" al centro.\n\
                     Dal secondo 0 al secondo 1:\n\
                     \x20 \"q\" pulsa con intensità 2 a ritmo 60.\n";
        let s = compila(&programma(corpo, |p| p.attori[0].stile.scala = 2.0)).unwrap();
        let k = keyframe(&s, 0, Canale::Scala);
        assert_eq!(k.first(), Some(&Valore::Scala(2.0)));
        assert!(k.contains(&Valore::Scala(3.0)));
        // Con base 0 la pulsazione resta a 0, senza errore.
        let s = compila(&programma(corpo, |p| p.attori[0].stile.scala = 0.0)).unwrap();
        assert!(keyframe(&s, 0, Canale::Scala).iter().all(|v| *v == Valore::Scala(0.0)));
    }

    #[test]
    fn errori_stato_iniziale() {
        let crea = "Crea un quadrato chiamato \"q\" al centro.\n";
        let e = compila(&programma(crea, |p| p.attori[0].stile.scala = -0.5)).unwrap_err();
        assert_eq!(
            e.to_string(),
            "Errore alla riga 2: la scala iniziale non può essere negativa\n  \
             Suggerimento: usa un valore da 0 in su, es. «scala 0.5»"
        );
        let e = compila(&programma(crea, |p| p.attori[0].stile.scala = 1000.5)).unwrap_err();
        assert_eq!(e.messaggio, "la scala iniziale è troppo grande (massimo 1000)");
        let e = compila(&programma(crea, |p| p.attori[0].stile.rotazione = -360_001.0)).unwrap_err();
        assert_eq!(e.messaggio, "la rotazione iniziale è troppo grande (massimo 360000 gradi)");
        assert_eq!(e.riga, Some(2));
        assert!(compila(&programma(crea, |p| p.attori[0].stile.scala = f64::NAN)).is_err());
    }

    #[test]
    fn avvisi_rotazione_su_cerchio() {
        let p = programma(
            "Crea un cerchio chiamato \"c\" al centro.\n\
             Crea un quadrato chiamato \"q\" al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"c\" ruota di 90 gradi.\n\
             \x20 \"q\" ruota di 90 gradi.\n",
            |p| {
                p.attori[0].stile.rotazione = 10.0;
                p.attori[1].stile.rotazione = 10.0;
            },
        );
        let s = compila(&p).unwrap();
        let testi: Vec<String> = s.avvisi.iter().map(ToString::to_string).collect();
        assert_eq!(
            testi,
            [
                "Avviso alla riga 2: \"c\" è un cerchio: la rotazione non cambia il suo aspetto\n  \
                 Suggerimento: togli «rotazione …» dalla creazione del cerchio",
                "Avviso alla riga 5: \"c\" è un cerchio: la rotazione non cambia il suo aspetto\n  \
                 Suggerimento: togli «ruota di …», oppure usa un quadrato o un rettangolo se vuoi vederlo girare",
            ]
        );
    }

    #[test]
    fn avvisi_ordinati_e_deduplicati() {
        let p = programma(
            "Crea un cerchio chiamato \"c\" al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"c\" ruota di 90 gradi.\n\
             Dal secondo 1 al secondo 2:\n\
             \x20 \"c\" ruota di 90 gradi.\n\
             Dal secondo 2 al secondo 3:\n\
             \x20 \"c\" ruota di 90 gradi.\n",
            |p| {
                // Righe forzate: 9, 4, 4 → attesi due avvisi, alle righe 4 e 9.
                for (blocco, riga) in p.timeline.blocchi.iter_mut().zip([9, 4, 4]) {
                    blocco.azioni[0].riga = riga;
                }
            },
        );
        let s = compila(&p).unwrap();
        let righe: Vec<usize> = s.avvisi.iter().map(|a| a.riga).collect();
        assert_eq!(righe, [4, 9]);
    }

    #[test]
    fn suggerimento_non_propone_l_attore_stesso() {
        let p = programma("Crea un cerchio chiamato \"a\" a destra di \"b\" di 10.\n", |_| {});
        let e = compila(&p).unwrap_err();
        assert_eq!(e.messaggio, "non esiste nessun attore chiamato \"b\"");
        assert_ne!(e.suggerimento.as_deref(), Some("forse intendevi \"a\"?"));
    }
}
