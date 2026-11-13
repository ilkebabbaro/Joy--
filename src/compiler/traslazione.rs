//! Canale `traslazione` (SPECIFICA B.0.5 passo 3, B.2, B.4.1).
//!
//! Si procede un livello di annidamento alla volta, dal più interno al più esterno: un gruppo
//! si calcola dopo tutti i suoi discendenti, perché il suo ingombro prudente usa le loro
//! estensioni (B.3.4). Dentro un livello i segmenti di tutti gli attori sono elaborati per
//! istante di inizio crescente (a parità, per riga), così `orbita` trova già calcolata la
//! posizione dell'attore attorno a cui gira.

use super::easing::{Curva, curva};
use super::gruppi::Albero;
use super::ingombro::{Estensioni, fuori_campo, ingombro_prudente, trasforma_gruppo};
use super::keyframe::{Accumulatore, Tetto, Vett, ZERO};
use super::orbita::{self, SOGLIA_PX};
use super::posizioni::Posizioni;
use super::{Effetto, Segmento, applica_base, controlla_coordinate};
use crate::ast::{Direzione, Forma, Programma};
use crate::errore::{Errore, Risultato};
use crate::security::limiti;

/// Durata minima del salto istantaneo, in secondi (B.0.3).
const DELTA_MINIMO: f64 = 0.001;

/// Frazione della durata della scena usata per il salto, se maggiore del minimo (B.0.3).
const DELTA_RELATIVO: f64 = 2e-6;

/// Dati già risolti di cui ha bisogno il canale `traslazione`.
pub(super) struct Contesto<'a> {
    /// Programma sorgente.
    pub programma: &'a Programma,
    /// Albero dei gruppi.
    pub albero: &'a Albero,
    /// Posizioni dichiarate e ingombri iniziali.
    pub posizioni: &'a Posizioni,
    /// Durata della scena in secondi.
    pub durata: f64,
    /// `s_rif` di ogni attore: `max(|min|, |max|)` dell'estensione del canale `scala`.
    pub scala_rif: &'a [f64],
    /// `true` se il canale `rotazione` dell'attore ha segmenti.
    pub ruota: &'a [bool],
    /// Tetto globale dei keyframe della compilazione.
    pub tetto: &'a Tetto,
}

/// Stato del calcolo, indicizzato come `Programma::attori`.
struct Stato {
    accumulatori: Vec<Option<Accumulatore>>,
    corrente: Vec<Vett>,
    prudente: Vec<Estensioni>,
    estensione: Vec<(Vett, Vett)>,
}

/// Calcola i keyframe del canale `traslazione` di tutti gli attori.
///
/// `segmenti[i]` sono i segmenti di traslazione dell'attore `i`, già ordinati e senza
/// sovrapposizioni. Restituisce un accumulatore (completo fino alla fine della scena) per ogni
/// attore che ha almeno un segmento.
///
/// # Errori
/// Coordinate fuori dai limiti, orbite troppo lunghe, troppi fotogrammi chiave.
pub(super) fn calcola(ctx: &Contesto, segmenti: &[Vec<Segmento>]) -> Risultato<Vec<Option<Accumulatore>>> {
    let n = ctx.programma.attori.len();
    let mut st = Stato {
        accumulatori: vec![None; n],
        corrente: vec![ZERO; n],
        prudente: vec![Estensioni::simmetriche(0.0, 0.0); n],
        estensione: vec![(ZERO, ZERO); n],
    };
    let massimo = ctx.albero.livello.iter().copied().max().unwrap_or(0);
    for livello in (0..=massimo).rev() {
        let nodi: Vec<usize> = (0..n).filter(|&i| ctx.albero.livello[i] == livello).collect();
        // I discendenti sono già completi: gli ingombri prudenti di questo livello sono noti.
        for &i in &nodi {
            st.prudente[i] = ingombro_prudente_di(ctx, i, &st);
        }
        for &i in &nodi {
            let Some(primo) = segmenti[i].first() else {
                continue;
            };
            // `entra da` come primo movimento: l'attore parte già fuori campo (B.2.3).
            let iniziale = match primo.effetto {
                Effetto::EntraDa(lato) => partenza_entra(ctx, i, lato, st.prudente[i], primo.riga)?,
                _ => ZERO,
            };
            let mut acc = Accumulatore::nuovo(ctx.durata, ctx.tetto);
            acc.aggiungi(0.0, iniziale, None)?;
            st.accumulatori[i] = Some(acc);
            st.corrente[i] = iniziale;
        }
        let mut coda: Vec<(usize, usize)> =
            nodi.iter().flat_map(|&i| (0..segmenti[i].len()).map(move |k| (i, k))).collect();
        coda.sort_by(|&(i, k), &(j, h)| {
            let (a, b) = (&segmenti[i][k], &segmenti[j][h]);
            a.inizio.total_cmp(&b.inizio).then(a.riga.cmp(&b.riga))
        });
        for (i, k) in coda {
            applica(ctx, i, &segmenti[i][k], k == 0, &mut st)?;
        }
        for &i in &nodi {
            if let Some(acc) = st.accumulatori[i].as_mut() {
                acc.aggiungi(ctx.durata, st.corrente[i], None)?;
                st.estensione[i] = acc.estensione().unwrap_or((ZERO, ZERO));
            }
        }
    }
    Ok(st.accumulatori)
}

/// Ingombro prudente (B.0.4, per i gruppi B.3.4) relativo al centro dichiarato.
fn ingombro_prudente_di(ctx: &Contesto, i: usize, st: &Stato) -> Estensioni {
    let attore = &ctx.programma.attori[i];
    if !matches!(attore.forma, Forma::Gruppo { .. }) {
        return ingombro_prudente(&attore.forma, &attore.stile, ctx.scala_rif[i], ctx.ruota[i]);
    }
    let (px, py) = ctx.posizioni.centri[i];
    let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
    for &m in &ctx.albero.figli[i] {
        let (mx, my) = ctx.posizioni.centri[m];
        let e = st.prudente[m];
        let (min, max) = st.estensione[m];
        x0 = x0.min(mx - px - e.sinistra + min[0]);
        x1 = x1.max(mx - px + e.destra + max[0]);
        y0 = y0.min(my - py - e.alto + min[1]);
        y1 = y1.max(my - py + e.basso + max[1]);
    }
    if !(x0 <= x1 && y0 <= y1) {
        return Estensioni::simmetriche(0.0, 0.0);
    }
    let riquadro = Estensioni::da_riquadro(x0, x1, y0, y1);
    trasforma_gruppo(riquadro, attore.stile.rotazione, ctx.scala_rif[i], ctx.ruota[i])
}

/// Valore del canale nel punto di partenza di `entra da` (B.2.3): fuori campo sull'asse del
/// lato, posizione dichiarata sull'altro asse.
fn partenza_entra(ctx: &Contesto, i: usize, lato: Direzione, e: Estensioni, riga: usize) -> Risultato<Vett> {
    let palco = &ctx.programma.palco;
    let f = fuori_campo(lato, e, palco.larghezza, palco.altezza);
    let (x0, y0) = ctx.posizioni.centri[i];
    Ok(match lato {
        Direzione::Sinistra | Direzione::Destra => {
            controlla_coordinate(f, y0, riga)?;
            [f - x0, 0.0, 0.0, 0.0]
        }
        Direzione::Alto | Direzione::Basso => {
            controlla_coordinate(x0, f, riga)?;
            [0.0, f - y0, 0.0, 0.0]
        }
    })
}

/// Durata `δ` del salto istantaneo (B.0.3) per un segmento `[t0, t1]`.
fn delta_salto(durata: f64, t0: f64, t1: f64) -> f64 {
    DELTA_MINIMO.max(DELTA_RELATIVO * durata).min(((t1 - t0) / 2.0).max(0.0))
}

/// Centro dell'orbita `C` (B.4.1): il pivot se `altro` è il genitore diretto di `i`,
/// altrimenti la posizione corrente di `altro` al limite sinistro di `t0`.
fn centro_orbita(ctx: &Contesto, i: usize, altro: usize, t0: f64, st: &Stato) -> (f64, f64) {
    let (cx, cy) = ctx.posizioni.centri[altro];
    if ctx.albero.genitore[i] == Some(altro) {
        return (cx, cy);
    }
    let d = st.accumulatori[altro].as_ref().and_then(|a| a.valuta_prima_di(t0)).unwrap_or(ZERO);
    (cx + d[0], cy + d[1])
}

/// Applica un segmento di traslazione dell'attore `i`. `primo` indica il primo segmento
/// di traslazione dell'attore.
fn applica(ctx: &Contesto, i: usize, s: &Segmento, primo: bool, st: &mut Stato) -> Risultato<()> {
    let (x0, y0) = ctx.posizioni.centri[i];
    let palco = &ctx.programma.palco;
    let attuale = st.corrente[i];
    let c = curva(s.easing);
    let centro = match s.effetto {
        Effetto::Orbita { centro, .. } => centro_orbita(ctx, i, centro, s.inizio, st),
        _ => (0.0, 0.0),
    };
    let prudente = st.prudente[i];
    let partenza = match s.effetto {
        Effetto::EntraDa(lato) => partenza_entra(ctx, i, lato, prudente, s.riga)?,
        _ => ZERO,
    };
    let acc = st
        .accumulatori[i]
        .as_mut()
        .ok_or_else(|| Errore::globale("errore interno: canale di traslazione non inizializzato"))?;
    match s.effetto {
        Effetto::EsceVerso(lato) => {
            let f = fuori_campo(lato, prudente, palco.larghezza, palco.altezza);
            let arrivo = match lato {
                Direzione::Sinistra | Direzione::Destra => {
                    controlla_coordinate(f, y0 + attuale[1], s.riga)?;
                    [f - x0, attuale[1], 0.0, 0.0]
                }
                Direzione::Alto | Direzione::Basso => {
                    controlla_coordinate(x0 + attuale[0], f, s.riga)?;
                    [attuale[0], f - y0, 0.0, 0.0]
                }
            };
            acc.aggiungi(s.inizio, attuale, Some(c))?;
            st.corrente[i] = arrivo;
            acc.aggiungi(s.fine, arrivo, None)
        }
        Effetto::EntraDa(_) => {
            if primo {
                // Il canale parte già dal fuori campo (`attuale == partenza`).
                acc.aggiungi(s.inizio, attuale, Some(c))?;
            } else {
                let distanza = (partenza[0] - attuale[0]).hypot(partenza[1] - attuale[1]);
                if distanza > SOGLIA_PX {
                    let delta = delta_salto(ctx.durata, s.inizio, s.fine);
                    acc.aggiungi(s.inizio, attuale, Some(Curva::Gradino))?;
                    acc.aggiungi(s.inizio + delta, partenza, Some(c))?;
                } else {
                    acc.aggiungi(s.inizio, attuale, Some(c))?;
                }
            }
            st.corrente[i] = ZERO;
            acc.aggiungi(s.fine, ZERO, None)
        }
        Effetto::Orbita { raggio, giri, orario, .. } => {
            let n = orbita::numero_campioni(raggio, giri);
            let massimo = limiti::MAX_KEYFRAME as f64;
            if n.is_nan() || n + 1.0 > massimo {
                return Err(Errore::alla_riga(
                    s.riga,
                    format!("orbita troppo lunga: {:.0} fotogrammi chiave (massimo {})", n + 1.0, limiti::MAX_KEYFRAME),
                )
                .con_suggerimento("riduci i giri"));
            }
            // Una sola verifica per orbita: tutti i campioni stanno nel quadrato
            // `centro ± raggio`, che deve restare entro MAX_COORDINATA.
            controlla_coordinate(centro.0.abs() + raggio, centro.1.abs() + raggio, s.riga)?;
            let (theta0, d) = orbita::angolo_iniziale(centro, (x0 + attuale[0], y0 + attuale[1]));
            let punto = |theta: f64| -> Vett {
                [centro.0 + raggio * theta.cos() - x0, centro.1 + raggio * theta.sin() - y0, 0.0, 0.0]
            };
            let mut t_inizio = s.inizio;
            if (d - raggio).abs() > SOGLIA_PX {
                acc.aggiungi(s.inizio, attuale, Some(Curva::Gradino))?;
                t_inizio += delta_salto(ctx.durata, s.inizio, s.fine);
            }
            // `n` è intero, positivo e ≤ MAX_KEYFRAME: la conversione è esatta.
            let campioni = n as usize;
            let mut ultimo = attuale;
            for k in 0..=campioni {
                let p = k as f64 / n;
                let t = t_inizio + (s.fine - t_inizio) * p;
                ultimo = punto(orbita::angolo(theta0, giri, orario, c.valuta(p)));
                // Tra i campioni l'interpolazione è lineare: l'easing è già nei campioni.
                acc.aggiungi(t, ultimo, if k == 0 { Some(Curva::Lineare) } else { None })?;
            }
            st.corrente[i] = ultimo;
            Ok(())
        }
        Effetto::Verso(_) | Effetto::Delta(_) | Effetto::Pulsazione { .. } => {
            applica_base(acc, &mut st.corrente[i], s)
        }
    }
}
