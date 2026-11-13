//! Tipo di errore unico della pipeline Joy++.
//!
//! Ogni fase (parser, compilatore, generatore, exporter) restituisce un [`Errore`]
//! leggibile da un essere umano: messaggio in italiano, numero di riga (quando noto)
//! e un suggerimento di correzione opzionale.
//!
//! Gli [`Avviso`] (SPECIFICA B.6) segnalano invece problemi **non bloccanti**: la
//! compilazione riesce comunque e l'HTML viene prodotto.

use std::fmt;

/// Errore di compilazione mostrato all'utente.
#[derive(Debug, Clone, PartialEq)]
pub struct Errore {
    /// Riga del sorgente `.joy` (a partire da 1); `None` per errori globali (es. I/O).
    pub riga: Option<usize>,
    /// Descrizione del problema, in italiano.
    pub messaggio: String,
    /// Suggerimento di correzione, se disponibile.
    pub suggerimento: Option<String>,
}

impl Errore {
    /// Crea un errore legato a una riga del sorgente.
    pub fn alla_riga(riga: usize, messaggio: impl Into<String>) -> Self {
        Self {
            riga: Some(riga),
            messaggio: messaggio.into(),
            suggerimento: None,
        }
    }

    /// Crea un errore globale, non legato a una riga specifica.
    pub fn globale(messaggio: impl Into<String>) -> Self {
        Self {
            riga: None,
            messaggio: messaggio.into(),
            suggerimento: None,
        }
    }

    /// Aggiunge (o sostituisce) il suggerimento di correzione.
    pub fn con_suggerimento(mut self, suggerimento: impl Into<String>) -> Self {
        self.suggerimento = Some(suggerimento.into());
        self
    }
}

impl fmt::Display for Errore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.riga {
            Some(riga) => write!(f, "Errore alla riga {riga}: {}", self.messaggio)?,
            None => write!(f, "Errore: {}", self.messaggio)?,
        }
        if let Some(s) = &self.suggerimento {
            write!(f, "\n  Suggerimento: {s}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Errore {}

/// Avviso non bloccante del compilatore (SPECIFICA B.6).
///
/// Ha lo stesso formato di [`Errore`], con `Avviso` al posto di `Errore`; è sempre
/// legato a una riga del sorgente. Non cambia l'esito della compilazione.
#[derive(Debug, Clone, PartialEq)]
pub struct Avviso {
    /// Riga del sorgente `.joy` (a partire da 1).
    pub riga: usize,
    /// Descrizione del problema, in italiano.
    pub messaggio: String,
    /// Suggerimento di correzione, se disponibile.
    pub suggerimento: Option<String>,
}

impl Avviso {
    /// Crea un avviso legato a una riga del sorgente.
    pub fn alla_riga(riga: usize, messaggio: impl Into<String>) -> Self {
        Self {
            riga,
            messaggio: messaggio.into(),
            suggerimento: None,
        }
    }

    /// Aggiunge (o sostituisce) il suggerimento di correzione.
    pub fn con_suggerimento(mut self, suggerimento: impl Into<String>) -> Self {
        self.suggerimento = Some(suggerimento.into());
        self
    }
}

impl fmt::Display for Avviso {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Avviso alla riga {}: {}", self.riga, self.messaggio)?;
        if let Some(s) = &self.suggerimento {
            write!(f, "\n  Suggerimento: {s}")?;
        }
        Ok(())
    }
}

/// Alias di comodo per i risultati della pipeline.
pub type Risultato<T> = Result<T, Errore>;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn formato_avviso_con_e_senza_suggerimento() {
        let a = Avviso::alla_riga(4, "messaggio");
        assert_eq!(a.to_string(), "Avviso alla riga 4: messaggio");
        let a = a.con_suggerimento("fai così");
        assert_eq!(a.to_string(), "Avviso alla riga 4: messaggio\n  Suggerimento: fai così");
    }
}
