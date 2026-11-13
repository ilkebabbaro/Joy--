//! Packaging e I/O: dal file `.joy` al file `.html` su disco.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::errore::{Errore, Risultato};
use crate::security::limiti;
use crate::{codegen, compiler, parser};

/// Opzioni di generazione dell'output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Opzioni {
    /// `true` per HTML indentato e leggibile (`--pretty`), `false` per output compatto.
    pub leggibile: bool,
}

/// Compila un sorgente Joy++ in memoria e restituisce il documento HTML.
///
/// # Errori
/// Il primo errore di parsing o di compilazione, oppure un errore se il sorgente
/// supera [`limiti::MAX_SORGENTE_BYTE`].
pub fn compila_sorgente(sorgente: &str, opzioni: &Opzioni) -> Risultato<String> {
    if sorgente.len() as u64 > limiti::MAX_SORGENTE_BYTE {
        return Err(Errore::globale(format!(
            "il sorgente è troppo grande (massimo {} byte)",
            limiti::MAX_SORGENTE_BYTE
        )));
    }
    let programma = parser::analizza(sorgente)?;
    let scena = compiler::compila(&programma)?;
    Ok(codegen::genera_html(&scena, opzioni.leggibile))
}

/// Percorso di output predefinito: stesso nome del sorgente con estensione `.html`.
pub fn output_predefinito(sorgente: &Path) -> PathBuf {
    sorgente.with_extension("html")
}

/// Legge `ingresso`, lo compila e scrive l'HTML in `uscita`.
/// Restituisce il numero di byte scritti.
///
/// # Errori
/// File mancante o illeggibile, file troppo grande, non UTF-8, errori di compilazione,
/// output coincidente con l'ingresso, errori di scrittura.
pub fn compila_file(ingresso: &Path, uscita: &Path, opzioni: &Opzioni) -> Risultato<usize> {
    let metadati = fs::metadata(ingresso).map_err(|e| {
        Errore::globale(format!("impossibile leggere «{}»: {e}", ingresso.display()))
    })?;
    if metadati.len() > limiti::MAX_SORGENTE_BYTE {
        return Err(Errore::globale(format!(
            "«{}» è troppo grande (massimo {} byte)",
            ingresso.display(),
            limiti::MAX_SORGENTE_BYTE
        )));
    }
    if stesso_file(ingresso, uscita) {
        return Err(Errore::globale("il file di output coincide con il sorgente: lo sovrascriverei")
            .con_suggerimento("scegli un percorso diverso con -o, es. -o output.html"));
    }
    let byte = fs::read(ingresso).map_err(|e| {
        Errore::globale(format!("impossibile leggere «{}»: {e}", ingresso.display()))
    })?;
    let sorgente = String::from_utf8(byte).map_err(|_| {
        Errore::globale(format!("«{}» non è un file di testo UTF-8", ingresso.display()))
            .con_suggerimento("salva il file con codifica UTF-8")
    })?;
    let html = compila_sorgente(&sorgente, opzioni)?;
    if let Some(cartella) = uscita.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(cartella).map_err(|e| {
            Errore::globale(format!("impossibile creare la cartella «{}»: {e}", cartella.display()))
        })?;
    }
    fs::write(uscita, &html).map_err(|e| {
        Errore::globale(format!("impossibile scrivere «{}»: {e}", uscita.display()))
    })?;
    Ok(html.len())
}

fn stesso_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

fn data_modifica(percorso: &Path) -> Option<SystemTime> {
    fs::metadata(percorso).and_then(|m| m.modified()).ok()
}

/// Modalità *watch*: ricompila `ingresso` ogni volta che cambia, senza mai terminare.
///
/// `notifica` riceve l'esito di ogni compilazione (anche la prima).
/// Usa il polling dell'orario di modifica, così non servono dipendenze esterne.
pub fn osserva(
    ingresso: &Path,
    uscita: &Path,
    opzioni: &Opzioni,
    intervallo: Duration,
    mut notifica: impl FnMut(Risultato<usize>),
) -> ! {
    let mut ultima = data_modifica(ingresso);
    notifica(compila_file(ingresso, uscita, opzioni));
    loop {
        std::thread::sleep(intervallo);
        let attuale = data_modifica(ingresso);
        if attuale != ultima {
            ultima = attuale;
            notifica(compila_file(ingresso, uscita, opzioni));
        }
    }
}
