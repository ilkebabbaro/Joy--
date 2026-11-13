//! Ingombri (AABB) degli attori e dei gruppi e fuori campo (SPECIFICA B.0.4, B.2.1, B.3.4).
//!
//! Un ingombro è descritto da quattro estensioni misurate dal centro dell'attore (o dal
//! pivot del gruppo): tutte le funzioni lavorano in coordinate relative a quel centro.

use crate::ast::{Direzione, Forma, Stile};

/// Margine di sicurezza del fuori campo, in pixel (B.2.1).
pub(super) const MARGINE_FUORI_CAMPO: f64 = 2.0;

/// Estensioni di un ingombro dal centro: il riquadro è `[-sinistra, destra] × [-alto, basso]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Estensioni {
    /// Estensione verso sinistra (x decrescente).
    pub sinistra: f64,
    /// Estensione verso destra.
    pub destra: f64,
    /// Estensione verso l'alto (y decrescente).
    pub alto: f64,
    /// Estensione verso il basso.
    pub basso: f64,
}

impl Estensioni {
    /// Ingombro simmetrico di semilarghezza `ex` e semialtezza `ey`.
    pub fn simmetriche(ex: f64, ey: f64) -> Self {
        Self { sinistra: ex, destra: ex, alto: ey, basso: ey }
    }

    /// Ingombro dal riquadro `[x0, x1] × [y0, y1]` (coordinate relative al centro).
    pub fn da_riquadro(x0: f64, x1: f64, y0: f64, y1: f64) -> Self {
        Self { sinistra: -x0, destra: x1, alto: -y0, basso: y1 }
    }

    /// I quattro vertici del riquadro.
    fn vertici(self) -> [(f64, f64); 4] {
        [
            (-self.sinistra, -self.alto),
            (self.destra, -self.alto),
            (self.destra, self.basso),
            (-self.sinistra, self.basso),
        ]
    }

    /// AABB del riquadro ruotato di `gradi` (orario sullo schermo, come `rotate` in CSS)
    /// attorno al centro e scalato di `scala`.
    pub fn ruota_scala(self, gradi: f64, scala: f64) -> Self {
        let (sin, cos) = gradi.to_radians().sin_cos();
        let (mut x0, mut x1, mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY, f64::NEG_INFINITY);
        for (x, y) in self.vertici() {
            let (rx, ry) = ((x * cos - y * sin) * scala, (x * sin + y * cos) * scala);
            x0 = x0.min(rx);
            x1 = x1.max(rx);
            y0 = y0.min(ry);
            y1 = y1.max(ry);
        }
        // `normalizza` toglie lo zero negativo (es. con scala 0).
        Self::da_riquadro(x0, x1, y0, y1).normalizza()
    }

    /// Cerchio circoscritto: raggio `scala × (massima distanza tra il centro e i vertici)`.
    pub fn cerchio_circoscritto(self, scala: f64) -> Self {
        let raggio = self.vertici().iter().map(|(x, y)| x.hypot(*y)).fold(0.0, f64::max) * scala;
        Self::simmetriche(raggio, raggio)
    }

    fn normalizza(self) -> Self {
        let pulisci = |v: f64| if v == 0.0 { 0.0 } else { v };
        Self {
            sinistra: pulisci(self.sinistra),
            destra: pulisci(self.destra),
            alto: pulisci(self.alto),
            basso: pulisci(self.basso),
        }
    }
}

/// Dimensioni di base `(w, h)` prima di scala e rotazione (B.0.4). Il bordo delle forme è
/// già incluso (`box-sizing:border-box`); per i testi si usa la stima prudente. Un gruppo
/// non ha dimensioni proprie: `(0, 0)`.
pub(super) fn dimensioni_base(forma: &Forma, stile: &Stile) -> (f64, f64) {
    match forma {
        Forma::Cerchio { raggio } => (2.0 * raggio, 2.0 * raggio),
        Forma::Rettangolo { larghezza, altezza, .. } => (*larghezza, *altezza),
        Forma::Testo { contenuto, grandezza, .. } => {
            let bordo = 2.0 * stile.bordo.map_or(0.0, |(spessore, _)| spessore);
            let mut righe = 0usize;
            let mut piu_lunga = 0.0_f64;
            for riga in contenuto.split('\n') {
                righe += 1;
                let em: f64 = riga.chars().map(|c| if (c as u32) < 0x1100 { 1.0 } else { 1.25 }).sum();
                piu_lunga = piu_lunga.max(em);
            }
            (grandezza * piu_lunga + bordo, 1.5 * grandezza * righe as f64 + bordo)
        }
        Forma::Gruppo { .. } => (0.0, 0.0),
    }
}

/// Ingombro di un attore semplice con scala `scala` e rotazione costante `gradi`.
/// Per il cerchio vale sempre `scala × raggio`, qualunque sia la rotazione.
fn ingombro_ruotato(forma: &Forma, stile: &Stile, scala: f64, gradi: f64) -> Estensioni {
    if let Forma::Cerchio { raggio } = forma {
        let r = scala.abs() * raggio;
        return Estensioni::simmetriche(r, r);
    }
    let (w, h) = dimensioni_base(forma, stile);
    Estensioni::simmetriche(w / 2.0, h / 2.0).ruota_scala(gradi, scala.abs())
}

/// Ingombro iniziale di un attore semplice (B.0.4): scala `s0` e rotazione `r0` dichiarate.
pub(super) fn ingombro_iniziale(forma: &Forma, stile: &Stile) -> Estensioni {
    ingombro_ruotato(forma, stile, stile.scala, stile.rotazione)
}

/// Ingombro prudente di un attore semplice (B.0.4). `scala_rif` è `max(|min|, |max|)`
/// dell'estensione del canale `scala`; `ruota` indica se il canale `rotazione` ha segmenti.
pub(super) fn ingombro_prudente(forma: &Forma, stile: &Stile, scala_rif: f64, ruota: bool) -> Estensioni {
    if !ruota {
        return ingombro_ruotato(forma, stile, scala_rif, stile.rotazione);
    }
    if let Forma::Cerchio { raggio } = forma {
        let r = scala_rif * raggio;
        return Estensioni::simmetriche(r, r);
    }
    let (w, h) = dimensioni_base(forma, stile);
    Estensioni::simmetriche(w / 2.0, h / 2.0).cerchio_circoscritto(scala_rif)
}

/// Applica a un riquadro relativo al pivot la trasformazione di un gruppo (B.3.4): rotazione
/// costante `r0` e scala `scala`, oppure, se il gruppo ruota, il cerchio circoscritto.
pub(super) fn trasforma_gruppo(riquadro: Estensioni, r0: f64, scala: f64, ruota: bool) -> Estensioni {
    if ruota { riquadro.cerchio_circoscritto(scala.abs()) } else { riquadro.ruota_scala(r0, scala.abs()) }
}

/// Coordinata (nel sistema del genitore) del centro fuori campo dal lato `lato` (B.2.1),
/// per un palco `larghezza × altezza` e un ingombro prudente `e`. È un'ascissa per
/// sinistra/destra e un'ordinata per alto/basso.
pub(super) fn fuori_campo(lato: Direzione, e: Estensioni, larghezza: f64, altezza: f64) -> f64 {
    match lato {
        Direzione::Sinistra => -e.destra - MARGINE_FUORI_CAMPO,
        Direzione::Destra => larghezza + e.sinistra + MARGINE_FUORI_CAMPO,
        Direzione::Alto => -e.basso - MARGINE_FUORI_CAMPO,
        Direzione::Basso => altezza + e.alto + MARGINE_FUORI_CAMPO,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn quasi(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    fn testo(contenuto: &str, grandezza: f64) -> Forma {
        Forma::Testo { contenuto: contenuto.into(), grandezza, font: None, grassetto: false }
    }

    #[test]
    fn stima_dei_testi() {
        let s = Stile::default();
        assert_eq!(dimensioni_base(&testo("SISTEMA ORBITALE", 36.0), &s), (576.0, 54.0));
        // Righe multiple: conta la più lunga; i caratteri CJK pesano 1.25 em.
        assert_eq!(dimensioni_base(&testo("ab\nabcd", 10.0), &s), (40.0, 30.0));
        assert_eq!(dimensioni_base(&testo("日本", 10.0), &s), (25.0, 15.0));
        let con_bordo = Stile { bordo: Some((3.0, crate::ast::Colore::BIANCO)), ..Stile::default() };
        assert_eq!(dimensioni_base(&testo("a", 10.0), &con_bordo), (16.0, 21.0));
    }

    #[test]
    fn ingombro_iniziale_ruotato_e_scalato() {
        let q = Forma::Rettangolo { larghezza: 100.0, altezza: 50.0, arrotondamento: 0.0 };
        let s = Stile { scala: 2.0, rotazione: 90.0, ..Stile::default() };
        let e = ingombro_iniziale(&q, &s);
        assert!(quasi(e.destra, 50.0) && quasi(e.basso, 100.0), "{e:?}");
        let s45 = Stile { rotazione: 45.0, ..Stile::default() };
        let e = ingombro_iniziale(&Forma::Rettangolo { larghezza: 10.0, altezza: 10.0, arrotondamento: 0.0 }, &s45);
        assert!(quasi(e.destra, 50f64.sqrt()), "{e:?}");
        // Il cerchio ignora la rotazione.
        let c = ingombro_iniziale(&Forma::Cerchio { raggio: 10.0 }, &Stile { scala: 0.5, ..s45 });
        assert_eq!(c, Estensioni::simmetriche(5.0, 5.0));
    }

    #[test]
    fn ingombro_prudente_con_rotazione_animata() {
        let q = Forma::Rettangolo { larghezza: 6.0, altezza: 8.0, arrotondamento: 0.0 };
        let e = ingombro_prudente(&q, &Stile::default(), 2.0, true);
        assert_eq!(e, Estensioni::simmetriche(10.0, 10.0));
        let e = ingombro_prudente(&q, &Stile::default(), 2.0, false);
        assert_eq!(e, Estensioni::simmetriche(6.0, 8.0));
    }

    #[test]
    fn gruppo_asimmetrico_ruotato() {
        let r = Estensioni::da_riquadro(-10.0, 30.0, -5.0, 5.0);
        let e = trasforma_gruppo(r, 90.0, 1.0, false);
        assert!(quasi(e.sinistra, 5.0) && quasi(e.destra, 5.0), "{e:?}");
        assert!(quasi(e.alto, 10.0) && quasi(e.basso, 30.0), "{e:?}");
        let c = trasforma_gruppo(r, 0.0, 2.0, true);
        assert!(quasi(c.destra, 2.0 * 925f64.sqrt()), "{c:?}");
    }

    #[test]
    fn coordinate_fuori_campo() {
        let e = Estensioni { sinistra: 1.0, destra: 2.0, alto: 3.0, basso: 4.0 };
        assert_eq!(fuori_campo(Direzione::Sinistra, e, 800.0, 600.0), -4.0);
        assert_eq!(fuori_campo(Direzione::Destra, e, 800.0, 600.0), 803.0);
        assert_eq!(fuori_campo(Direzione::Alto, e, 800.0, 600.0), -6.0);
        assert_eq!(fuori_campo(Direzione::Basso, e, 800.0, 600.0), 605.0);
    }
}
