//! Generazione dell'output HTML5 auto-contenuto.
//!
//! Nella v0.1 è disponibile solo il backend DOM ([`dom`]); i backend SVG e Canvas
//! verranno aggiunti con linee, tracciati e sistemi particellari.

pub mod dom;

pub use dom::genera_html;

/// Formatta un numero per CSS: al massimo 4 decimali, senza zeri finali né `-0`.
pub fn numero(v: f64) -> String {
    let arrotondato = (v * 10_000.0).round() / 10_000.0;
    if arrotondato == 0.0 || !arrotondato.is_finite() {
        return "0".into();
    }
    let testo = format!("{arrotondato:.4}");
    testo.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod test {
    use super::numero;

    #[test]
    fn formattazione_numeri() {
        assert_eq!(numero(1.0), "1");
        assert_eq!(numero(0.30000000000000004), "0.3");
        assert_eq!(numero(-0.0), "0");
        assert_eq!(numero(-12.5), "-12.5");
        assert_eq!(numero(1.0 / 3.0), "0.3333");
    }
}
