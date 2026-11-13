//! Compilatore: dall'AST a una [`ScenaCompilata`] pronta per il generatore.
//!
//! Qui avvengono tutte le decisioni "intelligenti": validazione semantica, risoluzione
//! delle posizioni relative, ancoraggio dei blocchi temporali relativi e pre-calcolo
//! dei keyframe per ogni canale di animazione. Il generatore si limita a stampare.
//!
//! Oltre agli errori (bloccanti), il compilatore raccoglie gli [`Avviso`] non bloccanti
//! di SPECIFICA B.6 nel campo [`ScenaCompilata::avvisi`].

pub mod easing;
mod gruppi;
mod ingombro;
mod keyframe;
mod orbita;
mod posizioni;
mod traslazione;

use std::collections::HashMap;

use crate::ast::{Attore, Azione, Colore, Direzione, Easing, Forma, Programma, Stile, Tempo, Verbo};
use crate::errore::{Avviso, Errore, Risultato};
use crate::parser::suggerisci;
use crate::security::{limiti, valida_nome_font};
use easing::{Curva, curva};
use gruppi::Albero;
use keyframe::{Accumulatore, Vett};

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
    /// FPS dichiarati con `a N fotogrammi` (informativi nell'HTML; predefinito di `--fps`
    /// per `joypp render`, SPECIFICA B.9.1).
    pub fps: Option<u32>,
    /// Durata totale della timeline in secondi (0 se non ci sono blocchi).
    pub durata: f64,
    /// Attori e gruppi in **pre-ordine** (visita in profondità) dell'albero dei gruppi
    /// (SPECIFICA B.3.5, B.3.7). Invarianti garantiti dal compilatore:
    ///
    /// 1. **Genitore prima dei figli:** se `attori[i].genitore == Some(g)`, allora `g < i` e
    ///    `attori[g].forma` è [`Forma::Gruppo`].
    /// 2. **Sottoalberi contigui:** i discendenti di un gruppo `g` occupano un intervallo
    ///    contiguo subito dopo `g`; il sottoalbero finisce al primo indice `j > g` il cui
    ///    genitore non è `g` né un discendente di `g`.
    /// 3. **Ordine dei fratelli = ordine nel DOM:** i figli di uno stesso genitore (il palco,
    ///    `None`, oppure un gruppo) compaiono nell'ordine di disegno di B.3.5: per riga di
    ///    creazione, dove un gruppo prende il posto del suo discendente creato per primo
    ///    (riga minima). Il generatore deve emettere gli elementi **esattamente in
    ///    quest'ordine**, annidando ogni figlio nel wrapper del suo genitore, senza riordinare.
    /// 4. **Compatibilità v0.1:** senza gruppi l'ordine coincide con quello di creazione e
    ///    gli `id` sono identici a quelli della v0.1.
    ///
    /// Per emettere l'albero basta una pila dei gruppi aperti: prima di emettere `attori[i]`
    /// si chiudono i wrapper finché la cima della pila non è `attori[i].genitore` (o la pila
    /// è vuota se il genitore è `None`). In alternativa si può ricorrere con
    /// [`ScenaCompilata::figli`].
    pub attori: Vec<AttoreCompilato>,
    /// Avvisi non bloccanti (SPECIFICA B.6), ordinati per riga crescente e senza
    /// duplicati (stessa riga e stesso messaggio compaiono una volta sola).
    pub avvisi: Vec<Avviso>,
}

impl ScenaCompilata {
    /// Indici (in [`ScenaCompilata::attori`]) dei figli diretti di `genitore`, nell'ordine
    /// in cui vanno emessi nel DOM. `None` indica il palco (attori e gruppi di primo livello).
    ///
    /// Ogni chiamata scorre tutti gli attori: per emettere l'intero albero in tempo lineare
    /// conviene la pila descritta in [`ScenaCompilata::attori`].
    pub fn figli(&self, genitore: Option<usize>) -> impl Iterator<Item = usize> + '_ {
        self.attori
            .iter()
            .enumerate()
            .filter(move |(_, a)| a.genitore == genitore)
            .map(|(i, _)| i)
    }
}

/// Attore (o gruppo) con posizione risolta e animazioni pre-calcolate.
#[derive(Debug, Clone, PartialEq)]
pub struct AttoreCompilato {
    /// Identificatore HTML/CSS generato (`aK`, dove `K` è l'indice in
    /// [`ScenaCompilata::attori`]): mai derivato dall'input utente.
    pub id: String,
    /// Nome originale (da sanificare prima di qualsiasi output).
    pub nome: String,
    /// Forma validata. Per un [`Forma::Gruppo`] il generatore emette solo il wrapper
    /// `<div class="attore gruppo" id="aK" data-nome="…">` con i figli dentro e **nessuna**
    /// `.forma` (B.3.5); l'albero va ricostruito da [`AttoreCompilato::genitore`], non dal
    /// campo `membri` (che conserva i nomi solo a scopo informativo).
    pub forma: Forma,
    /// Stile validato. Per un gruppo contano solo `livello` (lo `z-index` del wrapper, che
    /// ordina il gruppo tra i suoi fratelli) e i valori iniziali qui sotto.
    pub stile: Stile,
    /// Indice in [`ScenaCompilata::attori`] del gruppo genitore diretto, oppure `None` per gli
    /// attori e i gruppi di primo livello (figli del palco). Vedi gli invarianti di
    /// [`ScenaCompilata::attori`].
    pub genitore: Option<usize>,
    /// Ascissa del centro in pixel, **relativa al genitore** (B.3.5): diventa `left` del
    /// wrapper `#aK`.
    ///
    /// * Primo livello (`genitore == None`): relativa al palco, come nella v0.1.
    /// * Membro di un gruppo: `posizione_dichiarata − P_genitore`, dove `P_genitore` è il
    ///   pivot del gruppo genitore (B.3.3).
    /// * Gruppo: il suo pivot `P` meno il pivot del genitore (o meno `(0, 0)` se è di primo
    ///   livello).
    pub x: f64,
    /// Ordinata del centro in pixel, relativa al genitore con le stesse regole di
    /// [`AttoreCompilato::x`]: diventa `top` del wrapper `#aK`.
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
///
/// L'ordine delle varianti (`Ord`) è l'ordine in cui le [`Animazione`] di un attore
/// compaiono in [`AttoreCompilato::animazioni`].
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
    /// `background-color` (cerchi, rettangoli) o `color` (testi), sulla `.forma`
    /// (SPECIFICA B.4.2). Mai presente su un gruppo.
    Colore,
}

/// Elemento del DOM su cui il generatore applica l'animazione di un canale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bersaglio {
    /// Il wrapper `#aK` (classe `.attore`): proprietà composite (`opacity`, `translate`,
    /// `rotate`, `scale`), accelerate dalla GPU. Vanno in `animation` e in `will-change`
    /// della regola `#aK{…}`.
    Wrapper,
    /// L'elemento visibile `#aK>.forma` (canale `colore`): l'animazione va in una regola
    /// `#aK>.forma{animation:…}` con la stessa durata e `linear both`, e **non** va in
    /// `will-change` (B.4.2: provoca solo un ridisegno, non è una proprietà composita).
    Forma,
}

impl Canale {
    /// Suffisso usato nei nomi `@keyframes` generati (`@keyframes aK-<suffisso>`).
    pub fn suffisso(self) -> &'static str {
        match self {
            Canale::Opacita => "opacita",
            Canale::Traslazione => "traslazione",
            Canale::Rotazione => "rotazione",
            Canale::Scala => "scala",
            Canale::Colore => "colore",
        }
    }

    /// Elemento del DOM che riceve l'animazione del canale.
    pub fn bersaglio(self) -> Bersaglio {
        match self {
            Canale::Colore => Bersaglio::Forma,
            _ => Bersaglio::Wrapper,
        }
    }

    /// Proprietà CSS animata dal canale per un attore di forma `forma`.
    ///
    /// Solo [`Canale::Colore`] dipende dalla forma: `color` per [`Forma::Testo`],
    /// `background-color` per cerchi e rettangoli. Il compilatore non produce mai il canale
    /// `colore` per un [`Forma::Gruppo`] (B.3.8); in quel caso il valore restituito
    /// (`background-color`) non va usato.
    pub fn proprieta_css(self, forma: &Forma) -> &'static str {
        match self {
            Canale::Opacita => "opacity",
            Canale::Traslazione => "translate",
            Canale::Rotazione => "rotate",
            Canale::Scala => "scale",
            Canale::Colore => match forma {
                Forma::Testo { .. } => "color",
                _ => "background-color",
            },
        }
    }

    /// Nome leggibile, per i messaggi d'errore.
    fn descrizione(self) -> &'static str {
        match self {
            Canale::Opacita => "l'opacità",
            Canale::Traslazione => "la posizione",
            Canale::Rotazione => "la rotazione",
            Canale::Scala => "la scala",
            Canale::Colore => "il colore",
        }
    }

    /// Tutti i canali, nell'ordine di `Ord`.
    const TUTTI: [Canale; 5] = [Canale::Opacita, Canale::Traslazione, Canale::Rotazione, Canale::Scala, Canale::Colore];

    /// Posizione del canale in [`Canale::TUTTI`] (indice degli array per canale).
    fn indice(self) -> usize {
        match self {
            Canale::Opacita => 0,
            Canale::Traslazione => 1,
            Canale::Rotazione => 2,
            Canale::Scala => 3,
            Canale::Colore => 4,
        }
    }

    /// Converte il valore interno nel [`Valore`] pubblico.
    fn valore(self, v: Vett) -> Valore {
        match self {
            Canale::Opacita => Valore::Opacita(v[0]),
            Canale::Traslazione => Valore::Traslazione(v[0], v[1]),
            Canale::Rotazione => Valore::Rotazione(v[0]),
            Canale::Scala => Valore::Scala(v[0]),
            Canale::Colore => {
                // I valori vengono sempre da un `Colore` (nessuna interpolazione in Rust):
                // la conversione è esatta; `clamp` la rende comunque sicura.
                let componente = |c: f64| c.round().clamp(0.0, 255.0) as u8;
                Valore::Colore(Colore {
                    r: componente(v[0]),
                    g: componente(v[1]),
                    b: componente(v[2]),
                    a: v[3].clamp(0.0, 1.0),
                })
            }
        }
    }
}

/// Valore interno del canale `colore` per un [`Colore`].
fn vettore_colore(c: Colore) -> Vett {
    [f64::from(c.r), f64::from(c.g), f64::from(c.b), c.a]
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
    /// Curva da applicare fino al keyframe successivo (`None` = lineare), emessa come
    /// `animation-timing-function` del keyframe. [`Curva::Gradino`] (`steps(1,end)`) segnala
    /// un salto istantaneo (B.0.3).
    pub curva: Option<Curva>,
}

/// Valore tipizzato di un canale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Valore {
    /// Opacità 0.0-1.0.
    Opacita(f64),
    /// Spostamento (dx, dy) in pixel rispetto alla posizione iniziale, nel sistema del
    /// genitore (B.0.2).
    Traslazione(f64, f64),
    /// Rotazione in gradi.
    Rotazione(f64),
    /// Fattore di scala.
    Scala(f64),
    /// Colore del canale `colore` (B.4.2). Il generatore lo riemette dai valori numerici con
    /// la stessa regola dei colori statici (`#rrggbb` se opaco, altrimenti `rgba(…)`), mai
    /// dal testo dell'utente.
    Colore(Colore),
}

/// Effetto di un segmento sul proprio canale.
#[derive(Debug, Clone, Copy)]
enum Effetto {
    /// Raggiunge un valore assoluto.
    Verso(Vett),
    /// Somma un delta al valore corrente.
    Delta(Vett),
    /// Oscilla tra il valore corrente e `valore × (1 + ampiezza)`.
    Pulsazione { ampiezza: f64, bpm: f64 },
    /// `entra da` (B.2.3): dal fuori campo del lato alla posizione dichiarata.
    EntraDa(Direzione),
    /// `esce verso` (B.2.2): dalla posizione corrente al fuori campo del lato.
    EsceVerso(Direzione),
    /// `orbita` (B.4.1) attorno all'attore di indice `centro` (indice di `Programma::attori`).
    Orbita { centro: usize, raggio: f64, giri: f64, orario: bool },
}

/// Un'azione collocata nel tempo su un canale.
#[derive(Debug, Clone, Copy)]
struct Segmento {
    inizio: f64,
    fine: f64,
    effetto: Effetto,
    easing: Easing,
    /// Verbo di opacità che ha prodotto il segmento (`None` sugli altri canali). Serve alla
    /// partenza invisibile di `appare sfumando` (A.3) e al testo dell'avviso «opacità
    /// invariata» (B.6).
    opacita: Option<AzioneOpacita>,
    riga: usize,
}

/// Verbo che agisce sul canale `opacita`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AzioneOpacita {
    /// `appare sfumando`: se è il primo segmento di opacità, l'attore parte invisibile (A.3).
    Appare,
    /// `scompare sfumando`.
    Scompare,
    /// `cambia opacità a X`.
    Cambia,
}

/// Tolleranza dell'avviso «opacità invariata» (SPECIFICA B.6): `|arrivo − corrente| < 1e-9`.
const TOLLERANZA_OPACITA: f64 = 1e-9;

/// Messaggio dell'avviso «rotazione invisibile su un cerchio» (SPECIFICA B.6).
fn messaggio_cerchio_ruotato(nome: &str) -> String {
    format!("\"{nome}\" è un cerchio: la rotazione non cambia il suo aspetto")
}

/// Opacità di arrivo di `appare sfumando` (SPECIFICA A.3, v0.2.1): l'opacità dichiarata se è
/// maggiore di 0, altrimenti 1.
fn arrivo_appare(dichiarata: f64) -> f64 {
    if dichiarata > 0.0 { dichiarata } else { 1.0 }
}

/// Avviso «opacità invariata» (SPECIFICA B.6) per un'azione di opacità alla riga `riga` che
/// porta l'attore `nome` al valore `arrivo`, già uguale a quello corrente.
fn avviso_opacita_invariata(nome: &str, azione: AzioneOpacita, arrivo: f64, riga: usize) -> Avviso {
    let (messaggio, suggerimento) = match azione {
        AzioneOpacita::Appare => (
            format!("\"{nome}\" appare sfumando ma la sua opacità è già {arrivo}: l'azione non cambia nulla"),
            "togli «appare sfumando», oppure fallo prima scomparire con «scompare sfumando»",
        ),
        AzioneOpacita::Scompare => (
            format!("\"{nome}\" scompare sfumando ma la sua opacità è già {arrivo}: l'azione non cambia nulla"),
            "togli «scompare sfumando», oppure fallo prima apparire con «appare sfumando»",
        ),
        AzioneOpacita::Cambia => (
            format!(
                "\"{nome}\" cambia opacità a {arrivo} ma la sua opacità è già {arrivo}: l'azione non cambia nulla"
            ),
            "togli «cambia opacità a …», oppure scegli un valore diverso da quello attuale",
        ),
    };
    Avviso::alla_riga(riga, messaggio).con_suggerimento(suggerimento)
}

/// Controlla i segmenti (ordinati, non sovrapposti) del canale `opacita` dell'attore `nome`,
/// partendo dall'opacità `iniziale` del canale, e aggiunge ad `avvisi` un avviso «opacità
/// invariata» (SPECIFICA B.6) per ogni azione il cui arrivo è uguale al valore corrente
/// all'inizio del segmento.
fn controlla_opacita_invariata(nome: &str, iniziale: f64, segmenti: &[Segmento], avvisi: &mut Vec<Avviso>) {
    let mut corrente = iniziale;
    for s in segmenti {
        // Tutti i verbi di opacità producono segmenti `Verso`.
        let Effetto::Verso(v) = s.effetto else {
            continue;
        };
        let arrivo = v[0];
        if let Some(azione) = s.opacita
            && (arrivo - corrente).abs() < TOLLERANZA_OPACITA
        {
            avvisi.push(avviso_opacita_invariata(nome, azione, arrivo, s.riga));
        }
        corrente = arrivo;
    }
}

/// Compila un [`Programma`] in una [`ScenaCompilata`].
///
/// Gli avvisi non bloccanti (SPECIFICA B.6) sono restituiti in [`ScenaCompilata::avvisi`].
///
/// # Errori
/// Errori semantici (attori sconosciuti o duplicati, posizioni circolari, azioni
/// sovrapposte, valori fuori dai limiti) con riga e suggerimento.
pub fn compila(programma: &Programma) -> Risultato<ScenaCompilata> {
    compila_con_tetto(programma, limiti::MAX_KEYFRAME_TOTALI)
}

/// Come [`compila`], con un tetto globale dei keyframe esplicito (i test lo abbassano per
/// non dover generare davvero `MAX_KEYFRAME_TOTALI` keyframe).
fn compila_con_tetto(programma: &Programma, massimo_keyframe: usize) -> Risultato<ScenaCompilata> {
    let tetto = keyframe::Tetto::nuovo(massimo_keyframe);
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

    // B.0.5 passo 1: albero dei gruppi e posizioni dichiarate (pivot compresi).
    let albero = gruppi::costruisci(&programma.attori, &indici)?;
    let posizioni = posizioni::risolvi(programma, &indici, &albero)?;
    let (mut segmenti, durata) = costruisci_segmenti(programma, &indici, &albero, &posizioni.centri, &mut avvisi)?;
    for (attore, canali) in programma.attori.iter().zip(segmenti.iter_mut()) {
        for canale in Canale::TUTTI {
            let lista = &mut canali[canale.indice()];
            lista.sort_by(|a, b| a.inizio.total_cmp(&b.inizio));
            controlla_sovrapposizioni(&attore.nome, canale, lista)?;
        }
    }

    // B.0.5 passo 2: canali che non dipendono da altri attori.
    let n = programma.attori.len();
    let mut accumulati: Vec<[Option<Accumulatore>; 5]> = (0..n).map(|_| Default::default()).collect();
    let mut opacita_iniziale: Vec<f64> = programma.attori.iter().map(|a| a.stile.opacita).collect();
    for (i, attore) in programma.attori.iter().enumerate() {
        for canale in [Canale::Opacita, Canale::Rotazione, Canale::Scala, Canale::Colore] {
            let lista = &segmenti[i][canale.indice()];
            let Some(primo) = lista.first() else {
                continue;
            };
            let iniziale = match canale {
                Canale::Opacita => {
                    // A.3: se la prima azione di opacità è `appare sfumando`, si parte invisibili.
                    if primo.opacita == Some(AzioneOpacita::Appare) {
                        opacita_iniziale[i] = 0.0;
                    }
                    controlla_opacita_invariata(&attore.nome, opacita_iniziale[i], lista, &mut avvisi);
                    [opacita_iniziale[i], 0.0, 0.0, 0.0]
                }
                Canale::Rotazione => [attore.stile.rotazione, 0.0, 0.0, 0.0],
                Canale::Scala => [attore.stile.scala, 0.0, 0.0, 0.0],
                Canale::Colore => vettore_colore(attore.stile.colore),
                Canale::Traslazione => continue,
            };
            accumulati[i][canale.indice()] = Some(genera_canale(iniziale, lista, durata, &tetto)?);
        }
    }

    // B.0.5 passo 3: traslazione, dal livello più interno al più esterno.
    let scala_rif: Vec<f64> = programma
        .attori
        .iter()
        .zip(&accumulati)
        .map(|(attore, canali)| {
            canali[Canale::Scala.indice()]
                .as_ref()
                .and_then(Accumulatore::estensione)
                .map_or(attore.stile.scala.abs(), |(min, max)| min[0].abs().max(max[0].abs()))
        })
        .collect();
    let ruota: Vec<bool> = segmenti.iter().map(|c| !c[Canale::Rotazione.indice()].is_empty()).collect();
    let segmenti_traslazione: Vec<Vec<Segmento>> =
        segmenti.iter_mut().map(|c| std::mem::take(&mut c[Canale::Traslazione.indice()])).collect();
    let contesto = traslazione::Contesto {
        programma,
        albero: &albero,
        posizioni: &posizioni,
        durata,
        scala_rif: &scala_rif,
        ruota: &ruota,
        tetto: &tetto,
    };
    let traslazioni = traslazione::calcola(&contesto, &segmenti_traslazione)?;
    for (canali, acc) in accumulati.iter_mut().zip(traslazioni) {
        canali[Canale::Traslazione.indice()] = acc;
    }

    // Uscita in pre-ordine (B.3.5), con indici e coordinate relativi al genitore.
    let mut nuovo_indice = vec![0usize; n];
    for (k, &i) in albero.preordine.iter().enumerate() {
        nuovo_indice[i] = k;
    }
    let mut attori = Vec::with_capacity(n);
    for (k, &i) in albero.preordine.iter().enumerate() {
        let attore = &programma.attori[i];
        let (x, y) = posizioni.centri[i];
        let (px, py) = albero.genitore[i].map_or((0.0, 0.0), |g| posizioni.centri[g]);
        let animazioni: Vec<Animazione> = Canale::TUTTI
            .into_iter()
            .filter_map(|canale| {
                accumulati[i][canale.indice()]
                    .take()
                    .map(|acc| Animazione { canale, keyframe: acc.in_keyframe(canale) })
            })
            .collect();
        attori.push(AttoreCompilato {
            id: format!("a{k}"),
            nome: attore.nome.clone(),
            forma: attore.forma.clone(),
            stile: attore.stile.clone(),
            genitore: albero.genitore[i].map(|g| nuovo_indice[g]),
            x: x - px,
            y: y - py,
            opacita_iniziale: opacita_iniziale[i],
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
        // Membri e profondità sono controllati da `gruppi::costruisci` (B.3.8).
        Forma::Gruppo { .. } => {}
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

/// Segmenti di un attore, uno per canale (indice [`Canale::indice`]).
type SegmentiPerCanale = [Vec<Segmento>; 5];

/// Colloca i blocchi nel tempo e converte ogni azione in un segmento su un canale.
/// Restituisce i segmenti per attore (indici di `Programma::attori`) e la durata totale
/// della scena. `posizioni` sono i centri dichiarati.
fn costruisci_segmenti(
    programma: &Programma,
    indici: &HashMap<&str, usize>,
    albero: &Albero,
    posizioni: &[(f64, f64)],
    avvisi: &mut Vec<Avviso>,
) -> Risultato<(Vec<SegmentiPerCanale>, f64)> {
    let mut per_attore: Vec<SegmentiPerCanale> = programma.attori.iter().map(|_| Default::default()).collect();

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
            let (canale, effetto, opacita) = match &azione.verbo {
                Verbo::EntraDa(lato) => (Canale::Traslazione, Effetto::EntraDa(*lato), None),
                Verbo::EsceVerso(lato) => (Canale::Traslazione, Effetto::EsceVerso(*lato), None),
                Verbo::Orbita { .. } => {
                    (Canale::Traslazione, valida_orbita(programma, indici, albero, i, azione)?, None)
                }
                Verbo::CambiaColore(colore) => {
                    if matches!(attore.forma, Forma::Gruppo { .. }) {
                        return Err(errore(format!("\"{}\" è un gruppo e non ha un colore da cambiare", attore.nome))
                            .con_suggerimento("cambia il colore dei singoli membri"));
                    }
                    (Canale::Colore, Effetto::Verso(vettore_colore(*colore)), None)
                }
                Verbo::AppareSfumando => (
                    Canale::Opacita,
                    Effetto::Verso([arrivo_appare(attore.stile.opacita), 0.0, 0.0, 0.0]),
                    Some(AzioneOpacita::Appare),
                ),
                Verbo::ScompareSfumando => {
                    (Canale::Opacita, Effetto::Verso([0.0; 4]), Some(AzioneOpacita::Scompare))
                }
                Verbo::CambiaOpacita(v) => {
                    let v = *v;
                    if !(0.0..=1.0).contains(&v) {
                        return Err(errore("l'opacità deve essere tra 0.0 e 1.0".into()));
                    }
                    (Canale::Opacita, Effetto::Verso([v, 0.0, 0.0, 0.0]), Some(AzioneOpacita::Cambia))
                }
                Verbo::SiSpostaA { x, y } => {
                    let (x, y) = controlla_coordinate(*x, *y, azione.riga)?;
                    (Canale::Traslazione, Effetto::Verso([x - x0, y - y0, 0.0, 0.0]), None)
                }
                Verbo::Scivola { direzione, distanza } => {
                    let (direzione, distanza) = (*direzione, *distanza);
                    if distanza.abs() > limiti::MAX_COORDINATA {
                        return Err(errore(format!(
                            "spostamento troppo grande (massimo {} pixel)",
                            limiti::MAX_COORDINATA
                        )));
                    }
                    let (dx, dy) = direzione.vettore();
                    (Canale::Traslazione, Effetto::Delta([dx * distanza, dy * distanza, 0.0, 0.0]), None)
                }
                Verbo::Ruota(gradi) => {
                    let gradi = *gradi;
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
                    (Canale::Rotazione, Effetto::Delta([gradi, 0.0, 0.0, 0.0]), None)
                }
                Verbo::ScalaFinoA(f) => {
                    let f = *f;
                    if f > limiti::MAX_SCALA {
                        return Err(errore(format!("fattore di scala troppo grande (massimo {})", limiti::MAX_SCALA)));
                    }
                    (Canale::Scala, Effetto::Verso([f, 0.0, 0.0, 0.0]), None)
                }
                Verbo::Pulsa { intensita, bpm } => {
                    let (intensita, bpm) = (*intensita, *bpm);
                    if !(bpm > 0.0 && bpm <= limiti::MAX_BPM) {
                        return Err(errore(format!("il ritmo deve essere tra 1 e {} BPM", limiti::MAX_BPM)));
                    }
                    if intensita > limiti::MAX_INTENSITA {
                        return Err(errore(format!("intensità troppo alta (massimo {})", limiti::MAX_INTENSITA)));
                    }
                    (Canale::Scala, Effetto::Pulsazione { ampiezza: 0.25 * intensita, bpm }, None)
                }
            };
            let segmento = Segmento { inizio, fine, effetto, easing: azione.easing, opacita, riga: azione.riga };
            per_attore[i][canale.indice()].push(segmento);
        }
    }
    Ok((per_attore, durata))
}

/// Controlla un'azione `orbita` dell'attore `i` (SPECIFICA B.4.1) e ne costruisce l'effetto.
///
/// # Errori
/// Centro inesistente o uguale all'attore, raggio o giri non validi, centro che non è un
/// fratello né il gruppo genitore diretto.
fn valida_orbita(
    programma: &Programma,
    indici: &HashMap<&str, usize>,
    albero: &Albero,
    i: usize,
    azione: &Azione,
) -> Risultato<Effetto> {
    let riga = azione.riga;
    let Verbo::Orbita { centro, raggio, giri, orario } = &azione.verbo else {
        return Err(Errore::globale("errore interno: l'azione non è un'orbita"));
    };
    let (raggio, giri, orario) = (*raggio, *giri, *orario);
    let nome = &programma.attori[i].nome;
    let k = indice_attore(centro, indici, riga, Some(nome))?;
    if k == i {
        return Err(Errore::alla_riga(riga, format!("\"{nome}\" non può orbitare attorno a sé stesso"))
            .con_suggerimento("scegli un altro attore come centro dell'orbita"));
    }
    if raggio.is_nan() || raggio <= 0.0 {
        return Err(Errore::alla_riga(riga, "il raggio dell'orbita deve essere maggiore di 0"));
    }
    if raggio > limiti::MAX_COORDINATA {
        return Err(Errore::alla_riga(
            riga,
            format!("il raggio dell'orbita è troppo grande (massimo {} pixel)", limiti::MAX_COORDINATA),
        ));
    }
    if giri.is_nan() || giri <= 0.0 {
        return Err(Errore::alla_riga(riga, "il numero di giri deve essere maggiore di 0")
            .con_suggerimento("per invertire il verso usa «in senso orario» o «in senso antiorario»"));
    }
    let fratelli = albero.genitore[k] == albero.genitore[i];
    if !fratelli && albero.genitore[i] != Some(k) {
        return Err(Errore::alla_riga(
            riga,
            format!("\"{nome}\" e \"{centro}\" appartengono a gruppi diversi: l'orbita richiede lo stesso gruppo"),
        )
        .con_suggerimento("metti entrambi nello stesso gruppo, oppure fai orbitare il gruppo"));
    }
    Ok(Effetto::Orbita { centro: k, raggio, giri, orario })
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

/// Converte i segmenti (ordinati, non sovrapposti) di un canale che non dipende da altri
/// attori in keyframe sull'intera scena.
fn genera_canale(iniziale: Vett, segmenti: &[Segmento], durata: f64, tetto: &keyframe::Tetto) -> Risultato<Accumulatore> {
    let mut acc = Accumulatore::nuovo(durata, tetto);
    let mut corrente = iniziale;
    acc.aggiungi(0.0, corrente, None)?;
    for s in segmenti {
        applica_base(&mut acc, &mut corrente, s)?;
    }
    acc.aggiungi(durata, corrente, None)?;
    Ok(acc)
}

/// Applica un segmento `Verso`, `Delta` o `Pulsazione` a partire dal valore `corrente`, che
/// viene aggiornato al valore finale del segmento.
///
/// # Errori
/// Troppi battiti o fotogrammi chiave; un effetto di traslazione speciale (`entra da`,
/// `esce verso`, `orbita`) è un errore interno: lo gestisce [`traslazione`].
fn applica_base(acc: &mut Accumulatore, corrente: &mut Vett, s: &Segmento) -> Risultato<()> {
    let c = curva(s.easing);
    acc.aggiungi(s.inizio, *corrente, Some(c.clone()))?;
    match s.effetto {
        Effetto::Verso(v) => *corrente = v,
        Effetto::Delta(d) => *corrente = [corrente[0] + d[0], corrente[1] + d[1], corrente[2] + d[2], corrente[3] + d[3]],
        Effetto::EntraDa(_) | Effetto::EsceVerso(_) | Effetto::Orbita { .. } => {
            return Err(Errore::globale("errore interno: effetto di traslazione fuori dal suo canale"));
        }
        Effetto::Pulsazione { ampiezza, bpm } => {
            let durata_seg = s.fine - s.inizio;
            let periodo = 60.0 / bpm;
            let battiti = (durata_seg / periodo).round().max(1.0);
            if battiti * 2.0 > limiti::MAX_KEYFRAME as f64 {
                return Err(Errore::alla_riga(s.riga, format!("troppi battiti ({battiti}) per questa pulsazione"))
                    .con_suggerimento("riduci il ritmo o la durata del blocco"));
            }
            let mezzi = battiti as usize * 2;
            let passo = durata_seg / mezzi as f64;
            let base = corrente[0];
            let picco = base * (1.0 + ampiezza);
            for k in 1..mezzi {
                let v = if k % 2 == 1 { picco } else { base };
                acc.aggiungi(s.inizio + passo * k as f64, [v, 0.0, 0.0, 0.0], Some(c.clone()))?;
            }
        }
    }
    acc.aggiungi(s.fine, *corrente, None)
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
    fn contratto_canale_colore() {
        let testo = Forma::Testo { contenuto: "x".into(), grandezza: 10.0, font: None, grassetto: false };
        let cerchio = Forma::Cerchio { raggio: 1.0 };
        assert_eq!(Canale::Colore.suffisso(), "colore");
        assert_eq!(Canale::Colore.proprieta_css(&testo), "color");
        assert_eq!(Canale::Colore.proprieta_css(&cerchio), "background-color");
        assert_eq!(Canale::Colore.bersaglio(), Bersaglio::Forma);
        assert_eq!(Canale::Scala.proprieta_css(&testo), "scale");
        assert_eq!(Canale::Traslazione.bersaglio(), Bersaglio::Wrapper);
        assert!(Canale::Scala < Canale::Colore);
    }

    #[test]
    fn senza_gruppi_tutti_di_primo_livello() {
        let s = compila(&programma(
            "Crea un quadrato chiamato \"a\" al centro.\nCrea un cerchio chiamato \"b\" al centro.\n",
            |_| {},
        ))
        .unwrap();
        assert!(s.attori.iter().all(|a| a.genitore.is_none()));
        assert_eq!(s.figli(None).collect::<Vec<_>>(), [0, 1]);
        assert_eq!(s.figli(Some(0)).count(), 0);
    }

    // ---- Blocco 2B (B.2, B.3, B.4): AST costruito a mano ----

    use crate::ast::{Ancora, Attore, Blocco, Palco, Posizione, Timeline};

    fn crea(nome: &str, forma: Forma, posizione: Posizione) -> Attore {
        Attore { nome: nome.into(), forma, stile: Stile::default(), posizione, riga: 0 }
    }

    fn cerchio(nome: &str, raggio: f64, x: f64, y: f64) -> Attore {
        crea(nome, Forma::Cerchio { raggio }, Posizione::Assoluta { x, y })
    }

    fn gruppo(nome: &str, membri: &[&str]) -> Attore {
        let membri = membri.iter().map(|m| m.to_string()).collect();
        crea(nome, Forma::Gruppo { membri }, Posizione::Palco(Ancora::Centro))
    }

    type Riga<'a> = (&'a str, Verbo, Easing);

    /// Palco `l × h`; gli attori ricevono le righe 2, 3, ...; ogni blocco e ogni azione la
    /// riga successiva.
    fn scena(l: f64, h: f64, mut attori: Vec<Attore>, blocchi: Vec<(f64, f64, Vec<Riga>)>) -> Programma {
        let mut riga = 1;
        for a in &mut attori {
            riga += 1;
            a.riga = riga;
        }
        let blocchi = blocchi
            .into_iter()
            .map(|(inizio, fine, azioni)| {
                riga += 1;
                let riga_blocco = riga;
                let azioni = azioni
                    .into_iter()
                    .map(|(attore, verbo, easing)| {
                        riga += 1;
                        crate::ast::Azione { attore: attore.into(), verbo, easing, riga }
                    })
                    .collect();
                Blocco { tempo: Tempo::Assoluto { inizio, fine }, azioni, riga: riga_blocco }
            })
            .collect();
        Programma {
            palco: Palco { larghezza: l, altezza: h, sfondo: Colore::rgb(0, 0, 0), fps: None, riga: 1 },
            attori,
            timeline: Timeline { blocchi },
        }
    }

    fn per_nome<'a>(s: &'a ScenaCompilata, nome: &str) -> (usize, &'a AttoreCompilato) {
        s.attori.iter().enumerate().find(|(_, a)| a.nome == nome).unwrap()
    }

    fn animazione<'a>(s: &'a ScenaCompilata, nome: &str, canale: Canale) -> &'a Animazione {
        per_nome(s, nome).1.animazioni.iter().find(|a| a.canale == canale).unwrap()
    }

    fn traslazioni(s: &ScenaCompilata, nome: &str) -> Vec<(f64, f64)> {
        animazione(s, nome, Canale::Traslazione)
            .keyframe
            .iter()
            .map(|k| match k.valore {
                Valore::Traslazione(x, y) => (x, y),
                altro => panic!("valore inatteso {altro:?}"),
            })
            .collect()
    }

    fn vicino(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6
    }

    fn orbita(centro: &str, raggio: f64, giri: f64, orario: bool) -> Verbo {
        Verbo::Orbita { centro: centro.into(), raggio, giri, orario }
    }

    /// Esempio completo di SPECIFICA B.8.
    fn esempio_b8() -> Programma {
        let colore = |r, g, b| Colore::rgb(r, g, b);
        let mut sole = cerchio("sole", 60.0, 0.0, 0.0);
        sole.posizione = Posizione::Palco(Ancora::Centro);
        sole.stile.colore = colore(0xF5, 0x9E, 0x0B);
        let mut terra = cerchio("terra", 18.0, 0.0, 0.0);
        terra.posizione =
            Posizione::Relativa { direzione: Direzione::Destra, riferimento: "sole".into(), distanza: 200.0 };
        let mut sistema = gruppo("sistema", &["sole", "terra"]);
        sistema.stile.scala = 0.8;
        sistema.stile.livello = 1;
        let titolo = crea(
            "titolo",
            Forma::Testo { contenuto: "SISTEMA ORBITALE".into(), grandezza: 36.0, font: None, grassetto: true },
            Posizione::Palco(Ancora::Alto),
        );
        let mut stella = crea(
            "stella",
            Forma::Rettangolo { larghezza: 24.0, altezza: 24.0, arrotondamento: 0.0 },
            Posizione::Palco(Ancora::BassoDestra),
        );
        stella.stile.scala = 0.0;
        stella.stile.rotazione = 45.0;
        scena(
            1280.0,
            720.0,
            vec![sole, terra, sistema, titolo, stella],
            vec![
                (
                    0.0,
                    1.5,
                    vec![
                        ("sistema", Verbo::EntraDa(Direzione::Sinistra), Easing::Scatto),
                        ("titolo", Verbo::EntraDa(Direzione::Alto), Easing::Morbido),
                    ],
                ),
                (
                    1.5,
                    7.5,
                    vec![
                        ("terra", orbita("sole", 200.0, 2.0, true), Easing::Lineare),
                        ("sole", Verbo::Pulsa { intensita: 0.2, bpm: 30.0 }, Easing::Morbido),
                        ("sole", Verbo::CambiaColore(colore(0xFD, 0xE6, 0x8A)), Easing::Morbido),
                        ("stella", Verbo::ScalaFinoA(1.5), Easing::Elastico),
                        ("stella", Verbo::Ruota(180.0), Easing::Elastico),
                    ],
                ),
                (
                    7.5,
                    9.0,
                    vec![
                        ("sistema", Verbo::EsceVerso(Direzione::Destra), Easing::Morbido),
                        ("titolo", Verbo::EsceVerso(Direzione::Alto), Easing::Morbido),
                        ("stella", Verbo::ScompareSfumando, Easing::Morbido),
                    ],
                ),
            ],
        )
    }

    #[test]
    fn esempio_b8_pivot_albero_e_fuori_campo() {
        let s = compila(&esempio_b8()).unwrap();
        assert!(s.avvisi.is_empty());
        // Pre-ordine: "sistema" prende il posto di "sole" (creato per primo).
        let nomi: Vec<&str> = s.attori.iter().map(|a| a.nome.as_str()).collect();
        assert_eq!(nomi, ["sistema", "sole", "terra", "titolo", "stella"]);
        let id: Vec<&str> = s.attori.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(id, ["a0", "a1", "a2", "a3", "a4"]);
        let genitori: Vec<Option<usize>> = s.attori.iter().map(|a| a.genitore).collect();
        assert_eq!(genitori, [None, Some(0), Some(0), None, None]);
        // Pivot P = (719, 360); i membri sono relativi a P.
        let (_, sistema) = per_nome(&s, "sistema");
        assert_eq!((sistema.x, sistema.y), (719.0, 360.0));
        assert_eq!((s.attori[1].x, s.attori[1].y), (-79.0, 0.0));
        assert_eq!((s.attori[2].x, s.attori[2].y), (121.0, 0.0));
        // "sistema" parte fuori campo: pivot a x = −113.2.
        let t = traslazioni(&s, "sistema");
        assert!(vicino(t[0], (-113.2 - 719.0, 0.0)), "{t:?}");
        // "titolo": stima 576 × 54, centro fuori campo a y = −29 (dichiarato a y = 72).
        let t = traslazioni(&s, "titolo");
        assert!(vicino(t[0], (0.0, -29.0 - 72.0)), "{t:?}");
        assert!(vicino(*t.last().unwrap(), (0.0, -29.0 - 72.0)), "{t:?}");
        // "terra": distanza = raggio, nessun salto; 2 giri da 45 campioni.
        let terra = animazione(&s, "terra", Canale::Traslazione);
        assert!(terra.keyframe.iter().all(|k| k.curva != Some(Curva::Gradino)));
        // Stato iniziale + 91 campioni + fine scena.
        assert_eq!(terra.keyframe.len(), 1 + 91 + 1);
        // Colore di "sole": dal dichiarato a quello finale.
        let colori: Vec<Valore> = animazione(&s, "sole", Canale::Colore).keyframe.iter().map(|k| k.valore).collect();
        assert_eq!(colori.first(), Some(&Valore::Colore(Colore::rgb(0xF5, 0x9E, 0x0B))));
        assert_eq!(colori.last(), Some(&Valore::Colore(Colore::rgb(0xFD, 0xE6, 0x8A))));
        // Il gruppo non ha mai il canale colore.
        assert!(sistema.animazioni.iter().all(|a| a.canale != Canale::Colore));
    }

    #[test]
    fn entra_ed_esce_con_salto() {
        let q = crea(
            "q",
            Forma::Rettangolo { larghezza: 100.0, altezza: 100.0, arrotondamento: 0.0 },
            Posizione::Palco(Ancora::Centro),
        );
        let p = scena(
            400.0,
            300.0,
            vec![q],
            vec![
                (0.0, 1.0, vec![("q", Verbo::EntraDa(Direzione::Sinistra), Easing::Lineare)]),
                (1.0, 2.0, vec![("q", Verbo::EsceVerso(Direzione::Destra), Easing::Lineare)]),
                (2.0, 3.0, vec![("q", Verbo::EntraDa(Direzione::Sinistra), Easing::Lineare)]),
            ],
        );
        let s = compila(&p).unwrap();
        // Fuori campo: sinistra x = −50 − 2, destra x = 400 + 50 + 2 (centro dichiarato x = 200).
        assert_eq!(traslazioni(&s, "q"), [(-252.0, 0.0), (0.0, 0.0), (252.0, 0.0), (-252.0, 0.0), (0.0, 0.0)]);
        let k = &animazione(&s, "q", Canale::Traslazione).keyframe;
        assert_eq!(k[2].curva, Some(Curva::Gradino));
        assert!((k[3].offset - 2.001 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn esce_verso_mantiene_l_altro_asse() {
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0)],
            vec![
                (0.0, 1.0, vec![("c", Verbo::Scivola { direzione: Direzione::Destra, distanza: 30.0 }, Easing::Lineare)]),
                (1.0, 2.0, vec![("c", Verbo::EsceVerso(Direzione::Basso), Easing::Lineare)]),
            ],
        );
        let s = compila(&p).unwrap();
        // y fuori campo = 300 + 10 + 2 = 312, cioè dy = 162; dx resta 30.
        assert_eq!(traslazioni(&s, "c").last(), Some(&(30.0, 162.0)));
    }

    #[test]
    fn ingombro_prudente_usa_scala_e_rotazione_animate() {
        let mut q = crea(
            "q",
            Forma::Rettangolo { larghezza: 60.0, altezza: 80.0, arrotondamento: 0.0 },
            Posizione::Assoluta { x: 200.0, y: 150.0 },
        );
        q.stile.scala = 2.0;
        let p = scena(
            400.0,
            300.0,
            vec![q],
            vec![
                (0.0, 1.0, vec![("q", Verbo::Ruota(90.0), Easing::Lineare)]),
                (1.0, 2.0, vec![("q", Verbo::EsceVerso(Direzione::Sinistra), Easing::Lineare)]),
            ],
        );
        let s = compila(&p).unwrap();
        // Rotazione animata → cerchio circoscritto: 2 × √(30² + 40²) = 100.
        let ultimo = *traslazioni(&s, "q").last().unwrap();
        assert!(vicino(ultimo, (-102.0 - 200.0, 0.0)), "{ultimo:?}");
    }

    fn orbita_semplice(raggio: f64, orario: bool) -> ScenaCompilata {
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0), cerchio("p", 5.0, 300.0, 150.0)],
            vec![(0.0, 1.0, vec![("p", orbita("c", raggio, 0.25, orario), Easing::Lineare)])],
        );
        compila(&p).unwrap()
    }

    #[test]
    fn orbita_verso_e_campioni() {
        // Raggio 100 → 36 campioni per giro → 9 per un quarto di giro (10 keyframe).
        let s = orbita_semplice(100.0, true);
        let t = traslazioni(&s, "p");
        assert_eq!(t.len(), 10);
        assert!(vicino(t[0], (0.0, 0.0)));
        // Orario sullo schermo (y verso il basso): da destra del centro a sotto.
        assert!(vicino(*t.last().unwrap(), (-100.0, 100.0)), "{t:?}");
        let s = orbita_semplice(100.0, false);
        assert!(vicino(*traslazioni(&s, "p").last().unwrap(), (-100.0, -100.0)));
    }

    #[test]
    fn orbita_con_salto_sul_cerchio() {
        let s = orbita_semplice(50.0, true);
        let k = &animazione(&s, "p", Canale::Traslazione).keyframe;
        assert_eq!(k.len(), 11);
        assert_eq!(k[0].curva, Some(Curva::Gradino));
        assert_eq!(k[1].valore, Valore::Traslazione(-50.0, 0.0));
        assert!((k[1].offset - 0.001).abs() < 1e-12);
    }

    #[test]
    fn orbita_attorno_a_un_fratello_in_movimento() {
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0), cerchio("p", 5.0, 300.0, 150.0)],
            vec![
                (0.0, 1.0, vec![("c", Verbo::SiSpostaA { x: 100.0, y: 150.0 }, Easing::Lineare)]),
                (1.0, 2.0, vec![("p", orbita("c", 200.0, 0.5, true), Easing::Morbido)]),
            ],
        );
        let s = compila(&p).unwrap();
        let k = &animazione(&s, "p", Canale::Traslazione).keyframe;
        // Centro statico in (100, 150): distanza 200 = raggio, nessun salto.
        assert!(k.iter().all(|k| k.curva != Some(Curva::Gradino)));
        assert!(vicino(*traslazioni(&s, "p").last().unwrap(), (-400.0, 0.0)));
    }

    #[test]
    fn orbita_attorno_al_gruppo_genitore() {
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 100.0, 150.0), cerchio("p", 10.0, 300.0, 150.0), gruppo("g", &["c", "p"])],
            vec![(0.0, 1.0, vec![("p", orbita("g", 100.0, 1.0, false), Easing::Lineare)])],
        );
        let s = compila(&p).unwrap();
        // P = (200, 150): "p" è già sul cerchio di raggio 100 e torna al punto di partenza.
        assert!(vicino(*traslazioni(&s, "p").last().unwrap(), (0.0, 0.0)));
    }

    fn errore_orbita(verbo: Verbo, attori: Vec<Attore>) -> String {
        let p = scena(400.0, 300.0, attori, vec![(0.0, 1.0, vec![("p", verbo, Easing::Lineare)])]);
        compila(&p).unwrap_err().messaggio
    }

    #[test]
    fn errori_dell_orbita() {
        let due = || vec![cerchio("c", 10.0, 200.0, 150.0), cerchio("p", 5.0, 300.0, 150.0)];
        assert_eq!(errore_orbita(orbita("p", 10.0, 1.0, false), due()), "\"p\" non può orbitare attorno a sé stesso");
        assert_eq!(
            errore_orbita(orbita("c", 0.0, 1.0, false), due()),
            "il raggio dell'orbita deve essere maggiore di 0"
        );
        assert_eq!(errore_orbita(orbita("c", 10.0, 0.0, false), due()), "il numero di giri deve essere maggiore di 0");
        assert_eq!(
            errore_orbita(orbita("c", 10.0, 1e6, false), due()),
            "orbita troppo lunga: 36000001 fotogrammi chiave (massimo 10000)"
        );
        assert_eq!(errore_orbita(orbita("x", 10.0, 1.0, false), due()), "non esiste nessun attore chiamato \"x\"");
        let mut attori = due();
        attori.push(gruppo("g", &["c"]));
        assert_eq!(
            errore_orbita(orbita("c", 10.0, 1.0, false), attori),
            "\"p\" e \"c\" appartengono a gruppi diversi: l'orbita richiede lo stesso gruppo"
        );
    }

    #[test]
    fn posizioni_relative_a_un_gruppo_e_cicli() {
        let mut x = cerchio("x", 5.0, 0.0, 0.0);
        x.posizione = Posizione::Relativa { direzione: Direzione::Destra, riferimento: "g".into(), distanza: 100.0 };
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("a", 10.0, 100.0, 100.0), cerchio("b", 10.0, 200.0, 200.0), gruppo("g", &["a", "b"]), x],
            vec![],
        );
        let s = compila(&p).unwrap();
        // P = centro di [90, 210] × [90, 210] = (150, 150).
        assert_eq!((per_nome(&s, "x").1.x, per_nome(&s, "x").1.y), (250.0, 150.0));
        // Un membro posizionato rispetto al proprio gruppo è un ciclo.
        let mut a = cerchio("a", 10.0, 0.0, 0.0);
        a.posizione = Posizione::Relativa { direzione: Direzione::Destra, riferimento: "g".into(), distanza: 10.0 };
        let p = scena(400.0, 300.0, vec![a, cerchio("b", 10.0, 1.0, 1.0), gruppo("g", &["a", "b"])], vec![]);
        let e = compila(&p).unwrap_err();
        assert!(e.messaggio.starts_with("posizioni circolari: \"a\" → \"g\""), "{e}");
    }

    #[test]
    fn gruppo_annidato_ruotato_e_scalato() {
        let mut g = gruppo("g", &["a", "b"]);
        g.stile.rotazione = 90.0;
        let mut esterno = gruppo("e", &["g", "c"]);
        esterno.stile.scala = 2.0;
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("a", 10.0, 100.0, 100.0), cerchio("b", 10.0, 130.0, 100.0), g, cerchio("c", 10.0, 300.0, 100.0), esterno],
            vec![],
        );
        let s = compila(&p).unwrap();
        // g: AABB [90, 140] × [90, 110] → P = (115, 100); ruotato di 90°: ±10 in x, ±25 in y.
        // e: unione di [105, 125] × [75, 125] e [290, 310] × [90, 110] → P = (207.5, 100).
        let (ie, e) = per_nome(&s, "e");
        assert_eq!((e.x, e.y), (207.5, 100.0));
        let (ig, g) = per_nome(&s, "g");
        assert_eq!(g.genitore, Some(ie));
        assert_eq!((g.x, g.y), (115.0 - 207.5, 0.0));
        let (_, a) = per_nome(&s, "a");
        assert_eq!(a.genitore, Some(ig));
        assert_eq!((a.x, a.y), (-15.0, 0.0));
        let nomi: Vec<&str> = s.attori.iter().map(|a| a.nome.as_str()).collect();
        assert_eq!(nomi, ["e", "g", "a", "b", "c"]);
    }

    #[test]
    fn cambia_colore_e_gruppi() {
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0)],
            vec![
                (0.0, 1.0, vec![("c", Verbo::CambiaColore(Colore::rgb(255, 0, 0)), Easing::Morbido)]),
                (1.0, 2.0, vec![("c", Verbo::CambiaColore(Colore { r: 0, g: 0, b: 255, a: 0.5 }), Easing::Morbido)]),
            ],
        );
        let s = compila(&p).unwrap();
        let v: Vec<Valore> = animazione(&s, "c", Canale::Colore).keyframe.iter().map(|k| k.valore).collect();
        assert_eq!(
            v,
            [
                Valore::Colore(Colore::BIANCO),
                Valore::Colore(Colore::rgb(255, 0, 0)),
                Valore::Colore(Colore { r: 0, g: 0, b: 255, a: 0.5 }),
            ]
        );
        // Sovrapposizione sul canale colore.
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0)],
            vec![
                (0.0, 2.0, vec![("c", Verbo::CambiaColore(Colore::BIANCO), Easing::Morbido)]),
                (1.0, 3.0, vec![("c", Verbo::CambiaColore(Colore::BIANCO), Easing::Morbido)]),
            ],
        );
        assert!(compila(&p).unwrap_err().messaggio.contains("cambiano il colore nello stesso momento"));
        // Su un gruppo è vietato.
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0), gruppo("g", &["c"])],
            vec![(0.0, 1.0, vec![("g", Verbo::CambiaColore(Colore::BIANCO), Easing::Morbido)])],
        );
        let e = compila(&p).unwrap_err();
        assert_eq!(e.messaggio, "\"g\" è un gruppo e non ha un colore da cambiare");
        assert_eq!(e.suggerimento.as_deref(), Some("cambia il colore dei singoli membri"));
    }

    fn keyframe_totali(s: &ScenaCompilata) -> usize {
        s.attori.iter().flat_map(|a| &a.animazioni).map(|a| a.keyframe.len()).sum()
    }

    #[test]
    fn tetto_globale_dei_keyframe() {
        // Due quarti di giro da 9 campioni: 10 keyframe ciascuno (il campione 0 si fonde con lo
        // stato iniziale), 20 in tutto.
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0), cerchio("p", 5.0, 300.0, 150.0), cerchio("q", 5.0, 100.0, 150.0)],
            vec![(
                0.0,
                1.0,
                vec![
                    ("p", orbita("c", 100.0, 0.25, true), Easing::Lineare),
                    ("q", orbita("c", 100.0, 0.25, true), Easing::Lineare),
                ],
            )],
        );
        let s = compila_con_tetto(&p, 19).map(|_| ()).unwrap_err();
        assert_eq!(s.messaggio, "animazione troppo complessa: più di 19 fotogrammi chiave in totale");
        assert_eq!(
            s.suggerimento.as_deref(),
            Some("riduci il numero di attori animati, i giri delle orbite o il ritmo delle pulsazioni")
        );
        let s = compila_con_tetto(&p, 20).unwrap();
        assert_eq!(keyframe_totali(&s), 20);
    }

    #[test]
    fn esempi_molto_sotto_il_tetto_globale() {
        for sorgente in [
            include_str!("../../examples/primo_test.joy"),
            include_str!("../../examples/livello1_motion.joy"),
            include_str!("../../examples/stato_iniziale.joy"),
        ] {
            let s = compila(&analizza(sorgente).unwrap()).unwrap();
            assert!(keyframe_totali(&s) < limiti::MAX_KEYFRAME_TOTALI / 100, "{}", keyframe_totali(&s));
        }
    }

    #[test]
    fn orbita_fuori_dai_limiti() {
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 900_000.0, 150.0), cerchio("p", 5.0, 900_000.0, 350_000.0)],
            vec![(0.0, 1.0, vec![("p", orbita("c", 200_000.0, 1.0, false), Easing::Lineare)])],
        );
        let e = compila(&p).unwrap_err();
        assert_eq!(e.messaggio, "coordinate fuori dai limiti (massimo ±1000000 pixel)");
        assert_eq!(e.riga, Some(5));
    }

    #[test]
    fn azioni_ammesse_su_un_gruppo() {
        let p = scena(
            400.0,
            300.0,
            vec![cerchio("c", 10.0, 200.0, 150.0), gruppo("g", &["c"])],
            vec![(
                0.0,
                1.0,
                vec![
                    ("g", Verbo::AppareSfumando, Easing::Morbido),
                    ("g", Verbo::Ruota(90.0), Easing::Morbido),
                    ("g", Verbo::SiSpostaA { x: 100.0, y: 100.0 }, Easing::Morbido),
                    ("g", Verbo::ScalaFinoA(2.0), Easing::Morbido),
                ],
            )],
        );
        let s = compila(&p).unwrap();
        let (_, g) = per_nome(&s, "g");
        assert_eq!(g.opacita_iniziale, 0.0);
        assert_eq!(g.animazioni.len(), 4);
        assert_eq!(traslazioni(&s, "g").last(), Some(&(-100.0, -50.0)));
        // Nessun avviso: la rotazione è sul gruppo, non sul cerchio.
        assert!(s.avvisi.is_empty());
    }

    #[test]
    fn suggerimento_non_propone_l_attore_stesso() {
        let p = programma("Crea un cerchio chiamato \"a\" a destra di \"b\" di 10.\n", |_| {});
        let e = compila(&p).unwrap_err();
        assert_eq!(e.messaggio, "non esiste nessun attore chiamato \"b\"");
        assert_ne!(e.suggerimento.as_deref(), Some("forse intendevi \"a\"?"));
    }

    // ------------------------------------------------ v0.2.1: `appare sfumando` e opacità invariata

    /// Valori dei keyframe di opacità dell'attore `nome`.
    fn opacita(s: &ScenaCompilata, nome: &str) -> Vec<f64> {
        let (i, _) = per_nome(s, nome);
        keyframe(s, i, Canale::Opacita)
            .into_iter()
            .map(|v| match v {
                Valore::Opacita(o) => o,
                altro => panic!("valore inatteso {altro:?}"),
            })
            .collect()
    }

    fn compila_testo(corpo: &str) -> ScenaCompilata {
        compila(&programma(corpo, |_| {})).unwrap()
    }

    fn testi_avvisi(s: &ScenaCompilata) -> Vec<String> {
        s.avvisi.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn appare_con_opacita_0_va_a_1_senza_avvisi() {
        let s = compila_testo(
            "Crea un quadrato chiamato \"q\" con opacità 0 al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"q\" appare sfumando.\n",
        );
        assert_eq!(opacita(&s, "q"), [0.0, 1.0]);
        assert_eq!(per_nome(&s, "q").1.opacita_iniziale, 0.0);
        assert!(s.avvisi.is_empty(), "{:?}", testi_avvisi(&s));
    }

    #[test]
    fn appare_con_opacita_parziale_va_al_valore_dichiarato() {
        let s = compila_testo(
            "Crea un quadrato chiamato \"q\" con opacità 0.6 al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"q\" appare sfumando.\n",
        );
        assert_eq!(opacita(&s, "q"), [0.0, 0.6]);
        assert!(s.avvisi.is_empty());
    }

    #[test]
    fn doppio_appare_avvisa_sul_secondo() {
        let s = compila_testo(
            "Crea un quadrato chiamato \"q\" con opacità 0.6 al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"q\" appare sfumando.\n\
             Poi per 1 secondo:\n\
             \x20 \"q\" appare sfumando e scivola verso destra di 10.\n",
        );
        assert_eq!(
            testi_avvisi(&s),
            ["Avviso alla riga 6: \"q\" appare sfumando ma la sua opacità è già 0.6: l'azione non cambia nulla\n  \
              Suggerimento: togli «appare sfumando», oppure fallo prima scomparire con «scompare sfumando»"]
        );
    }

    #[test]
    fn scompare_su_attore_gia_scomparso_avvisa() {
        let s = compila_testo(
            "Crea un quadrato chiamato \"q\" al centro.\n\
             Crea un quadrato chiamato \"z\" con opacità 0 al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"q\" scompare sfumando.\n\
             \x20 \"z\" scompare sfumando.\n\
             Dal secondo 1 al secondo 2:\n\
             \x20 \"q\" scompare sfumando.\n",
        );
        let scompare = |riga: usize, nome: &str| {
            format!(
                "Avviso alla riga {riga}: \"{nome}\" scompare sfumando ma la sua opacità è già 0: \
                 l'azione non cambia nulla\n  \
                 Suggerimento: togli «scompare sfumando», oppure fallo prima apparire con «appare sfumando»"
            )
        };
        // Il primo `scompare` di "q" (da 1 a 0) è valido; "z" è creato invisibile.
        assert_eq!(testi_avvisi(&s), [scompare(6, "z"), scompare(8, "q")]);
    }

    #[test]
    fn cambia_opacita_al_valore_attuale_avvisa() {
        let s = compila_testo(
            "Crea un quadrato chiamato \"q\" al centro.\n\
             Crea un quadrato chiamato \"r\" al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"q\" cambia opacità a 1.\n\
             \x20 \"r\" cambia opacità a 0.5.\n\
             Dal secondo 1 al secondo 2:\n\
             \x20 \"r\" cambia opacità a 0.5.\n",
        );
        let cambia = |riga: usize, nome: &str, v: &str| {
            format!(
                "Avviso alla riga {riga}: \"{nome}\" cambia opacità a {v} ma la sua opacità è già {v}: \
                 l'azione non cambia nulla\n  \
                 Suggerimento: togli «cambia opacità a …», oppure scegli un valore diverso da quello attuale"
            )
        };
        assert_eq!(testi_avvisi(&s), [cambia(5, "q", "1"), cambia(8, "r", "0.5")]);
    }

    #[test]
    fn appare_dopo_scompare_e_altri_canali_senza_avvisi() {
        let s = compila_testo(
            "Crea un quadrato chiamato \"q\" al centro.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"q\" appare sfumando.\n\
             Poi per 1 secondo:\n\
             \x20 \"q\" scompare sfumando.\n\
             Poi per 1 secondo:\n\
             \x20 \"q\" appare sfumando e si ingrandisce fino a 1.\n",
        );
        assert_eq!(opacita(&s, "q"), [0.0, 1.0, 0.0, 1.0]);
        assert!(s.avvisi.is_empty(), "{:?}", testi_avvisi(&s));
    }

    #[test]
    fn gruppo_con_opacita_0_appare_fino_a_1() {
        let s = compila_testo(
            "Crea un quadrato chiamato \"q\" al centro.\n\
             Crea un gruppo chiamato \"g\" con \"q\" e opacità 0.\n\
             Dal secondo 0 al secondo 1:\n\
             \x20 \"g\" appare sfumando.\n\
             \x20 \"q\" appare sfumando.\n\
             Poi per 1 secondo:\n\
             \x20 \"g\" appare sfumando.\n",
        );
        assert_eq!(opacita(&s, "g"), [0.0, 1.0, 1.0]);
        assert_eq!(per_nome(&s, "g").1.opacita_iniziale, 0.0);
        // Canali indipendenti: "q" dentro il gruppo invisibile non avvisa; il secondo
        // `appare` del gruppo sì.
        assert_eq!(
            testi_avvisi(&s),
            ["Avviso alla riga 8: \"g\" appare sfumando ma la sua opacità è già 1: l'azione non cambia nulla\n  \
              Suggerimento: togli «appare sfumando», oppure fallo prima scomparire con «scompare sfumando»"]
        );
    }
}
