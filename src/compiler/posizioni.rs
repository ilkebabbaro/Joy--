//! Risoluzione delle posizioni dichiarate (SPECIFICA A.2, B.3.3, B.3.6).
//!
//! Le posizioni sono calcolate nel sistema del palco **come se nessun gruppo fosse
//! trasformato** (B.0.2): sono le coordinate in cui il parser e l'utente ragionano. Il pivot di
//! un gruppo è il centro dell'AABB degli ingombri iniziali dei suoi membri, quindi il gruppo
//! dipende da ciascun membro; le posizioni relative dipendono dal loro riferimento. Il grafo
//! delle dipendenze è visitato in profondità senza ricorsione, rilevando i cicli.

use std::collections::HashMap;

use super::gruppi::Albero;
use super::ingombro::{Estensioni, ingombro_iniziale, trasforma_gruppo};
use super::{MARGINE_ANCORE, controlla_coordinate, indice_attore};
use crate::ast::{Ancora, Forma, Posizione, Programma};
use crate::errore::{Errore, Risultato};

/// Posizioni dichiarate e ingombri iniziali, indicizzati come `Programma::attori`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Posizioni {
    /// Centro dichiarato (pivot `P` per i gruppi), in coordinate del palco non trasformate.
    pub centri: Vec<(f64, f64)>,
    /// Ingombro iniziale (B.0.4; per i gruppi B.3.4), relativo al centro.
    pub iniziali: Vec<Estensioni>,
}

#[derive(Clone, Copy, PartialEq)]
enum Stato {
    Nuovo,
    InCorso,
    Fatto,
}

/// Risolve centri e ingombri iniziali di tutti gli attori e gruppi.
///
/// # Errori
/// Riferimenti inesistenti, posizioni circolari (anche attraverso un gruppo), coordinate
/// fuori dai limiti.
pub(super) fn risolvi(
    programma: &Programma,
    indici: &HashMap<&str, usize>,
    albero: &Albero,
) -> Risultato<Posizioni> {
    let attori = &programma.attori;
    let n = attori.len();
    let mut stato = vec![Stato::Nuovo; n];
    let mut risolte: Vec<Option<((f64, f64), Estensioni)>> = vec![None; n];

    // k-esima dipendenza di `nodo`, se esiste.
    let dipendenza = |nodo: usize, k: usize| -> Risultato<Option<usize>> {
        let attore = &attori[nodo];
        match (&attore.forma, &attore.posizione) {
            (Forma::Gruppo { .. }, _) => Ok(albero.figli[nodo].get(k).copied()),
            (_, Posizione::Relativa { riferimento, .. }) if k == 0 => {
                indice_attore(riferimento, indici, attore.riga, Some(&attore.nome)).map(Some)
            }
            _ => Ok(None),
        }
    };

    for inizio in 0..n {
        if stato[inizio] == Stato::Fatto {
            continue;
        }
        stato[inizio] = Stato::InCorso;
        let mut pila: Vec<(usize, usize)> = vec![(inizio, 0)];
        while let Some(&(nodo, k)) = pila.last() {
            if let Some(d) = dipendenza(nodo, k)? {
                if let Some(cima) = pila.last_mut() {
                    cima.1 += 1;
                }
                match stato[d] {
                    Stato::Fatto => {}
                    Stato::Nuovo => {
                        stato[d] = Stato::InCorso;
                        pila.push((d, 0));
                    }
                    Stato::InCorso => {
                        let da = pila.iter().position(|(i, _)| *i == d).unwrap_or(0);
                        let nomi: Vec<String> =
                            pila[da..].iter().map(|(i, _)| format!("\"{}\"", attori[*i].nome)).collect();
                        return Err(Errore::alla_riga(
                            attori[d].riga,
                            format!("posizioni circolari: {} dipendono l'uno dall'altro", nomi.join(" → ")),
                        )
                        .con_suggerimento(
                            "almeno uno degli attori deve avere una posizione assoluta o ancorata al palco \
                             (un membro non può essere posizionato rispetto al proprio gruppo)",
                        ));
                    }
                }
                continue;
            }
            risolte[nodo] = Some(calcola(programma, albero, &risolte, nodo, indici)?);
            stato[nodo] = Stato::Fatto;
            pila.pop();
        }
    }

    let mut centri = Vec::with_capacity(n);
    let mut iniziali = Vec::with_capacity(n);
    for r in risolte {
        let (c, e) = r.ok_or_else(|| Errore::globale("errore interno: posizione non risolta"))?;
        centri.push(c);
        iniziali.push(e);
    }
    Ok(Posizioni { centri, iniziali })
}

/// Calcola centro e ingombro iniziale di `nodo`, le cui dipendenze sono già risolte.
fn calcola(
    programma: &Programma,
    albero: &Albero,
    risolte: &[Option<((f64, f64), Estensioni)>],
    nodo: usize,
    indici: &HashMap<&str, usize>,
) -> Risultato<((f64, f64), Estensioni)> {
    let attore = &programma.attori[nodo];
    let interno = || Errore::globale("errore interno: dipendenza di posizione non risolta");
    if let Forma::Gruppo { .. } = attore.forma {
        let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for &m in &albero.figli[nodo] {
            let ((cx, cy), e) = risolte[m].ok_or_else(interno)?;
            x0 = x0.min(cx - e.sinistra);
            x1 = x1.max(cx + e.destra);
            y0 = y0.min(cy - e.alto);
            y1 = y1.max(cy + e.basso);
        }
        if !(x0 <= x1 && y0 <= y1) {
            return Err(Errore::alla_riga(
                attore.riga,
                format!("il gruppo \"{}\" deve contenere almeno un membro", attore.nome),
            ));
        }
        let p = controlla_coordinate((x0 + x1) / 2.0, (y0 + y1) / 2.0, attore.riga)?;
        let riquadro = Estensioni::simmetriche((x1 - x0) / 2.0, (y1 - y0) / 2.0);
        let iniziale = trasforma_gruppo(riquadro, attore.stile.rotazione, attore.stile.scala, false);
        return Ok((p, iniziale));
    }
    let (l, h) = (programma.palco.larghezza, programma.palco.altezza);
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
            let ((rx, ry), _) = risolte[k].ok_or_else(interno)?;
            let (dx, dy) = direzione.vettore();
            (rx + dx * distanza, ry + dy * distanza)
        }
    };
    let punto = controlla_coordinate(punto.0, punto.1, attore.riga)?;
    Ok((punto, ingombro_iniziale(&attore.forma, &attore.stile)))
}
