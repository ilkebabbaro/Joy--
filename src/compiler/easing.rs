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
    /// Salto istantaneo (SPECIFICA B.0.3), emesso esattamente come `steps(1,end)`.
    ///
    /// Non corrisponde a nessun [`Easing`] dell'utente: il compilatore lo assegna al
    /// keyframe in `t0` che precede di `δ` il keyframe con il valore di partenza di
    /// `entra da` (non iniziale) o di `orbita` (distanza diversa dal raggio). Con
    /// `steps(1,end)` il valore resta quello di `t0` fino a `t0 + δ` e poi cambia di colpo.
    Gradino,
}

/// Tolleranza della risoluzione numerica delle `cubic-bezier` (SPECIFICA B.0.2 chiede 1e-6).
const TOLLERANZA_BEZIER: f64 = 1e-7;

impl Curva {
    /// Valuta la curva nel progresso `p` (0.0-1.0, valori fuori intervallo sono limitati),
    /// come farebbe il browser: restituisce la frazione del cambiamento già compiuta.
    ///
    /// * `cubic-bezier`: si risolve `x(s) = p` per bisezione (tolleranza 1e-7) e si
    ///   restituisce `y(s)`.
    /// * Curve campionate: interpolazione lineare tra campioni equidistanti.
    /// * [`Curva::Gradino`] (`steps(1,end)`): 0 per `p < 1`, 1 per `p = 1`.
    pub fn valuta(&self, p: f64) -> f64 {
        let p = if p.is_nan() { 0.0 } else { p.clamp(0.0, 1.0) };
        match self {
            Curva::Lineare => p,
            Curva::CubicBezier([x1, y1, x2, y2]) => {
                let (mut basso, mut alto, mut s) = (0.0, 1.0, p);
                for _ in 0..64 {
                    let x = bezier(*x1, *x2, s);
                    if (x - p).abs() < TOLLERANZA_BEZIER {
                        break;
                    }
                    if x < p {
                        basso = s;
                    } else {
                        alto = s;
                    }
                    s = (basso + alto) / 2.0;
                }
                bezier(*y1, *y2, s)
            }
            Curva::Campionata(punti) => match punti.len() {
                0 => p,
                1 => punti[0],
                n => {
                    let ultimo = n - 1;
                    let pos = p * ultimo as f64;
                    let i = (pos.floor() as usize).min(ultimo - 1);
                    let frazione = pos - i as f64;
                    punti[i] + (punti[i + 1] - punti[i]) * frazione
                }
            },
            Curva::Gradino => {
                if p >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    /// `true` se la curva supera il bersaglio (oggi solo `elastico`): per l'estensione dei
    /// canali (SPECIFICA B.0.4) il tratto va allargato con il fattore 1.36.
    pub fn supera_bersaglio(&self) -> bool {
        matches!(self, Curva::Campionata(punti) if punti.iter().any(|v| *v > 1.0))
    }
}

/// Coordinata di una `cubic-bezier` con estremi 0 e 1 e punti di controllo `p1`, `p2`.
fn bezier(p1: f64, p2: f64, s: f64) -> f64 {
    let u = 1.0 - s;
    3.0 * u * u * s * p1 + 3.0 * u * s * s * p2 + s * s * s
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

    #[test]
    fn valutazione_delle_curve() {
        for e in [Easing::Lineare, Easing::Morbido, Easing::Scatto, Easing::Elastico, Easing::Rimbalzo] {
            let c = curva(e);
            assert!(c.valuta(0.0).abs() < 1e-6, "{e:?}");
            assert!((c.valuta(1.0) - 1.0).abs() < 1e-6, "{e:?}");
        }
        // `ease-in-out` è simmetrica: a metà vale 0.5.
        assert!((curva(Easing::Morbido).valuta(0.5) - 0.5).abs() < 1e-6);
        assert_eq!(Curva::Campionata(vec![0.0, 1.0, 0.0]).valuta(0.25), 0.5);
        assert_eq!(Curva::Gradino.valuta(0.999), 0.0);
        assert_eq!(Curva::Gradino.valuta(1.0), 1.0);
        assert!(curva(Easing::Elastico).supera_bersaglio());
        assert!(!curva(Easing::Rimbalzo).supera_bersaglio());
        assert!(!curva(Easing::Scatto).supera_bersaglio());
    }
}
