//! Accumulo dei keyframe di un canale nel tempo, valutazione della posizione corrente
//! (SPECIFICA B.0.2) ed estensione di un canale (B.0.4).
//!
//! I keyframe sono tenuti in secondi finché il canale è in costruzione: così si può valutare
//! un canale in un istante qualsiasi (serve a `orbita`). Solo alla fine diventano gli
//! offset percentuali di [`Keyframe`].

use std::cell::Cell;
use std::rc::Rc;

use super::easing::Curva;
use super::{Canale, EPSILON, Keyframe};
use crate::errore::{Errore, Risultato};
use crate::security::limiti;

/// Tetto globale dei keyframe (somma su tutti i canali di tutti gli attori e gruppi),
/// condiviso da tutti gli [`Accumulatore`] di una compilazione. Il conteggio cresce a ogni
/// keyframe nuovo (non per quelli fusi) e la compilazione si ferma appena lo supera, senza
/// generare il resto: il costo resta limitato anche nel caso peggiore.
#[derive(Debug, Clone)]
pub(super) struct Tetto {
    usati: Rc<Cell<usize>>,
    massimo: usize,
}

impl Tetto {
    /// Tetto di `massimo` keyframe in tutto (in produzione `MAX_KEYFRAME_TOTALI`).
    pub fn nuovo(massimo: usize) -> Self {
        Self { usati: Rc::new(Cell::new(0)), massimo }
    }

    /// Conta un keyframe nuovo.
    ///
    /// # Errori
    /// Il totale supera il tetto.
    fn conta(&self) -> Risultato<()> {
        let usati = self.usati.get().saturating_add(1);
        if usati > self.massimo {
            return Err(Errore::globale(format!(
                "animazione troppo complessa: più di {} fotogrammi chiave in totale",
                self.massimo
            ))
            .con_suggerimento("riduci il numero di attori animati, i giri delle orbite o il ritmo delle pulsazioni"));
        }
        self.usati.set(usati);
        Ok(())
    }

    /// Keyframe contati finora.
    #[cfg(test)]
    pub fn usati(&self) -> usize {
        self.usati.get()
    }
}

/// Valore interno di un canale: `[v, 0, 0, 0]` per i canali scalari, `[dx, dy, 0, 0]` per la
/// traslazione, `[r, g, b, a]` per il colore.
pub(super) type Vett = [f64; 4];

/// Vettore nullo.
pub(super) const ZERO: Vett = [0.0; 4];

/// Fattore con cui si allarga un tratto con curva che supera il bersaglio (B.0.4).
const FATTORE_SOVRAELONGAZIONE: f64 = 1.36;

/// Un keyframe in secondi.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Punto {
    /// Istante in secondi.
    pub tempo: f64,
    /// Valore del canale.
    pub valore: Vett,
    /// Curva fino al keyframe successivo (`None` = lineare).
    pub curva: Option<Curva>,
}

/// Accumula keyframe fondendo quelli che cadono nello stesso offset percentuale.
#[derive(Debug, Clone)]
pub(super) struct Accumulatore {
    durata: f64,
    punti: Vec<Punto>,
    tetto: Tetto,
}

impl Accumulatore {
    /// Accumulatore vuoto per una scena lunga `durata` secondi, che conta i propri keyframe
    /// nel tetto globale `tetto`.
    pub fn nuovo(durata: f64, tetto: &Tetto) -> Self {
        Self { durata, punti: Vec::new(), tetto: tetto.clone() }
    }

    fn offset(&self, tempo: f64) -> f64 {
        if self.durata > 0.0 { (tempo / self.durata).clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Aggiunge un keyframe. Se cade nello stesso offset dell'ultimo, lo sostituisce nel
    /// valore (e nella curva, se `curva` è `Some`).
    ///
    /// # Errori
    /// Più di `MAX_KEYFRAME` keyframe nello stesso canale, o tetto globale superato.
    pub fn aggiungi(&mut self, tempo: f64, valore: Vett, curva: Option<Curva>) -> Risultato<()> {
        let offset = self.offset(tempo);
        if let Some(ultimo) = self.punti.last()
            && (self.offset(ultimo.tempo) - offset).abs() < EPSILON
        {
            if let Some(ultimo) = self.punti.last_mut() {
                ultimo.valore = valore;
                if curva.is_some() {
                    ultimo.curva = curva;
                }
            }
            return Ok(());
        }
        if self.punti.len() >= limiti::MAX_KEYFRAME {
            return Err(Errore::globale(format!(
                "animazione troppo complessa: più di {} fotogrammi chiave",
                limiti::MAX_KEYFRAME
            ))
            .con_suggerimento("riduci il ritmo delle pulsazioni, i giri delle orbite o la durata dei blocchi"));
        }
        self.tetto.conta()?;
        self.punti.push(Punto { tempo, valore, curva });
        Ok(())
    }

    /// Keyframe accumulati finora, in ordine di tempo.
    #[cfg(test)]
    pub fn punti(&self) -> &[Punto] {
        &self.punti
    }

    /// Valore del canale al **limite sinistro** di `t` (SPECIFICA B.0.2): un keyframe che
    /// inizia esattamente in `t` non conta ancora; un tratto in corso si valuta con la curva
    /// del suo keyframe sinistro. `None` se non c'è ancora nessun keyframe.
    pub fn valuta_prima_di(&self, t: f64) -> Option<Vett> {
        let primo = self.punti.first()?;
        if t <= primo.tempo + EPSILON {
            return Some(primo.valore);
        }
        let j = self.punti.iter().rposition(|k| k.tempo < t - EPSILON)?;
        let a = &self.punti[j];
        let Some(b) = self.punti.get(j + 1) else {
            return Some(a.valore);
        };
        let ampiezza = b.tempo - a.tempo;
        let progresso = if ampiezza > 0.0 { ((t - a.tempo) / ampiezza).clamp(0.0, 1.0) } else { 1.0 };
        let e = match &a.curva {
            None => progresso,
            // Limite sinistro di `steps(1,end)`: il valore resta quello del keyframe sinistro.
            Some(Curva::Gradino) => 0.0,
            Some(c) => c.valuta(progresso),
        };
        Some(interpola(a.valore, b.valore, e))
    }

    /// Estensione del canale (B.0.4): minimo e massimo, componente per componente, dei
    /// valori di tutti i keyframe, più `partenza + 1.36 × (arrivo − partenza)` per ogni
    /// tratto la cui curva supera il bersaglio. `None` se non c'è nessun keyframe.
    pub fn estensione(&self) -> Option<(Vett, Vett)> {
        let primo = self.punti.first()?;
        let (mut min, mut max) = (primo.valore, primo.valore);
        let mut includi = |v: Vett| {
            for c in 0..4 {
                min[c] = min[c].min(v[c]);
                max[c] = max[c].max(v[c]);
            }
        };
        for (i, k) in self.punti.iter().enumerate() {
            includi(k.valore);
            if let (Some(c), Some(succ)) = (&k.curva, self.punti.get(i + 1))
                && c.supera_bersaglio()
            {
                includi(interpola(k.valore, succ.valore, FATTORE_SOVRAELONGAZIONE));
            }
        }
        Some((min, max))
    }

    /// Converte i keyframe in quelli pubblici, con offset da 0 a 1.
    pub fn in_keyframe(self, canale: Canale) -> Vec<Keyframe> {
        let durata = self.durata;
        self.punti
            .into_iter()
            .map(|p| Keyframe {
                offset: if durata > 0.0 { (p.tempo / durata).clamp(0.0, 1.0) } else { 0.0 },
                valore: canale.valore(p.valore),
                curva: p.curva,
            })
            .collect()
    }
}

/// `a + e × (b − a)`, componente per componente.
pub(super) fn interpola(a: Vett, b: Vett, e: f64) -> Vett {
    [a[0] + (b[0] - a[0]) * e, a[1] + (b[1] - a[1]) * e, a[2] + (b[2] - a[2]) * e, a[3] + (b[3] - a[3]) * e]
}

#[cfg(test)]
mod test {
    use super::*;

    fn v(x: f64) -> Vett {
        [x, 0.0, 0.0, 0.0]
    }

    fn acc(durata: f64) -> Accumulatore {
        Accumulatore::nuovo(durata, &Tetto::nuovo(usize::MAX))
    }

    #[test]
    fn tetto_globale_condiviso_tra_canali() {
        let tetto = Tetto::nuovo(3);
        let mut a = Accumulatore::nuovo(10.0, &tetto);
        let mut b = Accumulatore::nuovo(10.0, &tetto);
        a.aggiungi(0.0, v(0.0), None).unwrap();
        // I keyframe fusi non contano.
        a.aggiungi(0.0, v(1.0), None).unwrap();
        b.aggiungi(0.0, v(0.0), None).unwrap();
        b.aggiungi(1.0, v(0.0), None).unwrap();
        assert_eq!(tetto.usati(), 3);
        let e = a.aggiungi(2.0, v(0.0), None).unwrap_err();
        assert_eq!(e.messaggio, "animazione troppo complessa: più di 3 fotogrammi chiave in totale");
        assert_eq!(
            e.suggerimento.as_deref(),
            Some("riduci il numero di attori animati, i giri delle orbite o il ritmo delle pulsazioni")
        );
    }

    #[test]
    fn fusione_nello_stesso_istante() {
        let mut a = acc(10.0);
        a.aggiungi(1.0, v(1.0), None).unwrap();
        a.aggiungi(1.0, v(2.0), Some(Curva::Gradino)).unwrap();
        assert_eq!(a.punti().len(), 1);
        assert_eq!(a.punti()[0].valore, v(2.0));
        assert_eq!(a.punti()[0].curva, Some(Curva::Gradino));
    }

    #[test]
    fn limite_sinistro() {
        let mut a = acc(10.0);
        a.aggiungi(0.0, v(0.0), None).unwrap();
        a.aggiungi(2.0, v(0.0), Some(Curva::Lineare)).unwrap();
        a.aggiungi(4.0, v(10.0), None).unwrap();
        assert_eq!(a.valuta_prima_di(0.0), Some(v(0.0)));
        assert_eq!(a.valuta_prima_di(3.0), Some(v(5.0)));
        assert_eq!(a.valuta_prima_di(4.0), Some(v(10.0)));
        assert_eq!(a.valuta_prima_di(9.0), Some(v(10.0)));
        // Salto: in t0 + δ il limite sinistro è ancora il valore prima del salto.
        let mut s = acc(10.0);
        s.aggiungi(0.0, v(0.0), Some(Curva::Gradino)).unwrap();
        s.aggiungi(0.5, v(100.0), None).unwrap();
        assert_eq!(s.valuta_prima_di(0.5), Some(v(0.0)));
        assert_eq!(s.valuta_prima_di(0.6), Some(v(100.0)));
        assert_eq!(acc(1.0).valuta_prima_di(0.5), None);
    }

    #[test]
    fn estensione_con_elastico() {
        let mut a = acc(10.0);
        a.aggiungi(0.0, v(0.0), Some(super::super::easing::curva(crate::ast::Easing::Elastico))).unwrap();
        a.aggiungi(1.0, v(100.0), None).unwrap();
        let (min, max) = a.estensione().unwrap();
        assert_eq!(min[0], 0.0);
        assert!((max[0] - 136.0).abs() < 1e-9);
    }
}
