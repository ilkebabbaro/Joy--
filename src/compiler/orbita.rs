//! Precalcolo geometrico di `orbita` (SPECIFICA B.4.1).

use std::f64::consts::PI;

/// Distanza (in pixel) sotto la quale non si salta e l'angolo di partenza è 0.
pub(super) const SOGLIA_PX: f64 = 0.5;

/// Campioni per giro: il minimo che mantiene la sagitta (errore della corda) ≤ 0.5 px,
/// limitato a `36..=360`. Per raggi ≤ 1 px sono 36.
pub(super) fn campioni_per_giro(raggio: f64) -> usize {
    if raggio <= 1.0 {
        return 36;
    }
    let angolo = 2.0 * (1.0 - SOGLIA_PX / raggio).acos();
    let n = (2.0 * PI / angolo).ceil();
    // `clamp` su f64 prima della conversione: nessun overflow, NaN impossibile qui.
    n.clamp(36.0, 360.0) as usize
}

/// Numero totale di campioni `N = max(1, ⌈n_giro × giri⌉)`, come `f64` per poterlo
/// confrontare con i limiti prima di qualsiasi conversione.
pub(super) fn numero_campioni(raggio: f64, giri: f64) -> f64 {
    (campioni_per_giro(raggio) as f64 * giri).ceil().max(1.0)
}

/// Angolo di partenza `θ0` (radianti, y verso il basso) e distanza `d` tra il punto `p` e
/// il centro `c`. Se `d < 0.5` px l'angolo è 0 (a destra del centro).
pub(super) fn angolo_iniziale(c: (f64, f64), p: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (p.0 - c.0, p.1 - c.1);
    let d = dx.hypot(dy);
    (if d < SOGLIA_PX { 0.0 } else { dy.atan2(dx) }, d)
}

/// Angolo del campione con progresso già "addolcito" `e = E(k/N)`:
/// `θ0 + σ·2π·giri·e`, con `σ = +1` per il verso orario.
pub(super) fn angolo(theta0: f64, giri: f64, orario: bool, e: f64) -> f64 {
    let sigma = if orario { 1.0 } else { -1.0 };
    theta0 + sigma * 2.0 * PI * giri * e
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn campioni_con_criterio_della_sagitta() {
        assert_eq!(campioni_per_giro(0.5), 36);
        assert_eq!(campioni_per_giro(1.0), 36);
        assert_eq!(campioni_per_giro(10.0), 36);
        assert_eq!(campioni_per_giro(200.0), 45);
        assert_eq!(campioni_per_giro(1_000_000.0), 360);
        // La sagitta resta entro 0.5 px.
        for r in [50.0, 200.0, 1234.0] {
            let n = campioni_per_giro(r) as f64;
            let sagitta = r * (1.0 - (PI / n).cos());
            assert!(sagitta <= 0.5 + 1e-9 || n >= 360.0, "r = {r}");
        }
        assert_eq!(numero_campioni(200.0, 2.0), 90.0);
        assert_eq!(numero_campioni(10.0, 0.25), 9.0);
        assert_eq!(numero_campioni(10.0, 0.001), 1.0);
    }

    #[test]
    fn angolo_di_partenza_e_verso() {
        let (t, d) = angolo_iniziale((0.0, 0.0), (0.0, 10.0));
        assert!((t - PI / 2.0).abs() < 1e-12 && d == 10.0);
        assert_eq!(angolo_iniziale((5.0, 5.0), (5.2, 5.0)).0, 0.0);
        assert!((angolo(0.0, 0.25, true, 1.0) - PI / 2.0).abs() < 1e-12);
        assert!((angolo(0.0, 0.25, false, 1.0) + PI / 2.0).abs() < 1e-12);
    }
}
