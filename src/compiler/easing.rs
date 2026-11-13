//! Curve di movimento (easing) pre-calcolate.
//!
//! Le curve "fisiche" (`elastico`, `rimbalzo`) non sono esprimibili con una
//! `cubic-bezier`: vengono campionate qui, una volta per tutte, e convertite nella
//! funzione CSS `linear(...)`, che il browser interpola senza calcoli a runtime.

use crate::ast::Easing;

/// Numero di campioni per le curve campionate.
const CAMPIONI: usize = 40;

/// Curva di temporizzazione pronta per essere emessa in CSS.
#[derive(Debug, Clone, PartialEq)]
pub enum Curva {
    /// `linear`
    Lineare,
    /// `cubic-bezier(x1, y1, x2, y2)`
    CubicBezier([f64; 4]),
    /// `linear(p0, p1, ..., pN)`: punti equidistanti nel tempo.
    Campionata(Vec<f64>),
}

/// Converte un [`Easing`] dell'AST nella relativa [`Curva`].
pub fn curva(easing: Easing) -> Curva {
    match easing {
        Easing::Lineare => Curva::Lineare,
        Easing::Morbido => Curva::CubicBezier([0.42, 0.0, 0.58, 1.0]),
        Easing::Scatto => Curva::CubicBezier([0.16, 1.0, 0.3, 1.0]),
        Easing::Elastico => campiona(elastico),
        Easing::Rimbalzo => campiona(rimbalzo),
    }
}

fn campiona(f: fn(f64) -> f64) -> Curva {
    Curva::Campionata(
        (0..=CAMPIONI)
            .map(|i| {
                let t = i as f64 / CAMPIONI as f64;
                (f(t) * 10_000.0).round() / 10_000.0
            })
            .collect(),
    )
}

/// *Ease-out elastic*: supera il bersaglio e oscilla fino a fermarsi (effetto molla).
pub fn elastico(t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let c4 = (2.0 * std::f64::consts::PI) / 3.0;
    2f64.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

/// *Ease-out bounce*: arriva al bersaglio e rimbalza con ampiezza decrescente.
pub fn rimbalzo(t: f64) -> f64 {
    const N1: f64 = 7.5625;
    const D1: f64 = 2.75;
    let t = t.clamp(0.0, 1.0);
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn estremi_delle_curve() {
        for f in [elastico as fn(f64) -> f64, rimbalzo] {
            assert!((f(0.0)).abs() < 1e-9);
            assert!((f(1.0) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn elastico_supera_il_bersaglio() {
        let Curva::Campionata(punti) = curva(Easing::Elastico) else {
            unreachable!()
        };
        assert_eq!(punti.len(), CAMPIONI + 1);
        assert!(punti.iter().any(|p| *p > 1.0));
    }
}
