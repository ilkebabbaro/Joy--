//! Packaging e I/O: dal file `.joy` al file `.html` su disco.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crate::codegen::OpzioniHtml;
use crate::errore::{Avviso, Errore, Risultato};
use crate::security::limiti;
use crate::{codegen, compiler, parser};

/// Opzioni di generazione dell'output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Opzioni {
    /// `true` per HTML indentato e leggibile (`--pretty`), `false` per output compatto.
    pub leggibile: bool,
    /// `true` per omettere il mini-player (`--senza-player`, B.5). Predefinito: `false`
    /// (player incluso). Si traduce in `OpzioniHtml { player: !senza_player }`.
    pub senza_player: bool,
}

/// Esito di una compilazione riuscita: il documento HTML e gli avvisi non bloccanti
/// (SPECIFICA B.6), ordinati per riga.
#[derive(Debug, Clone, PartialEq)]
pub struct Compilazione {
    /// Documento HTML5 auto-contenuto.
    pub html: String,
    /// Avvisi non bloccanti; non impediscono la generazione dell'HTML.
    pub avvisi: Vec<Avviso>,
    /// Durata totale della scena in secondi (`ScenaCompilata.durata`), per `joypp render`.
    pub durata: f64,
    /// Larghezza del palco in pixel.
    pub larghezza: f64,
    /// Altezza del palco in pixel.
    pub altezza: f64,
    /// FPS dichiarati dall'intestazione del palco (`a N fotogrammi`), se presenti.
    pub fps: Option<u32>,
}

/// Esito di [`compila_file`] e [`osserva`]: byte scritti sul disco e avvisi da mostrare.
#[derive(Debug, Clone, PartialEq)]
pub struct Esito {
    /// Numero di byte scritti nel file di uscita.
    pub byte: usize,
    /// Avvisi non bloccanti della compilazione, in ordine di riga.
    pub avvisi: Vec<Avviso>,
}

/// Compila un sorgente Joy++ in memoria e restituisce HTML e avvisi.
///
/// È il punto unico della pipeline (limite di dimensione, parse, compile, codegen).
///
/// # Errori
/// Il primo errore di parsing o di compilazione, oppure un errore se il sorgente
/// supera [`limiti::MAX_SORGENTE_BYTE`].
pub fn compila_con_avvisi(sorgente: &str, opzioni: &Opzioni) -> Risultato<Compilazione> {
    if sorgente.len() as u64 > limiti::MAX_SORGENTE_BYTE {
        return Err(Errore::globale(format!(
            "il sorgente è troppo grande (massimo {} byte)",
            limiti::MAX_SORGENTE_BYTE
        )));
    }
    let programma = parser::analizza(sorgente)?;
    let scena = compiler::compila(&programma)?;
    let opzioni_html =
        OpzioniHtml { leggibile: opzioni.leggibile, player: !opzioni.senza_player };
    let html = codegen::genera_html(&scena, &opzioni_html);
    Ok(Compilazione {
        html,
        durata: scena.durata,
        larghezza: scena.larghezza,
        altezza: scena.altezza,
        fps: scena.fps,
        avvisi: scena.avvisi,
    })
}

/// Compila un sorgente Joy++ in memoria e restituisce il solo documento HTML.
///
/// Gli avvisi vengono scartati: per riceverli usa [`compila_con_avvisi`].
///
/// # Errori
/// Gli stessi di [`compila_con_avvisi`].
pub fn compila_sorgente(sorgente: &str, opzioni: &Opzioni) -> Risultato<String> {
    compila_con_avvisi(sorgente, opzioni).map(|c| c.html)
}

/// Percorso di output predefinito: stesso nome del sorgente con estensione `.html`.
pub fn output_predefinito(sorgente: &Path) -> PathBuf {
    sorgente.with_extension("html")
}

/// Legge `ingresso`, lo compila e scrive l'HTML in `uscita`.
/// Restituisce un [`Esito`] con i byte scritti e gli avvisi della compilazione.
///
/// # Errori
/// File mancante o illeggibile, file troppo grande, non UTF-8, errori di compilazione,
/// output coincidente con l'ingresso, errori di scrittura.
pub fn compila_file(ingresso: &Path, uscita: &Path, opzioni: &Opzioni) -> Risultato<Esito> {
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
    let Compilazione { html, avvisi, .. } = compila_con_avvisi(&sorgente, opzioni)?;
    if let Some(cartella) = uscita.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(cartella).map_err(|e| {
            Errore::globale(format!("impossibile creare la cartella «{}»: {e}", cartella.display()))
        })?;
    }
    fs::write(uscita, &html).map_err(|e| {
        Errore::globale(format!("impossibile scrivere «{}»: {e}", uscita.display()))
    })?;
    Ok(Esito { byte: html.len(), avvisi })
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
/// `notifica` riceve l'[`Esito`] di ogni compilazione riuscita o fallita (anche la prima);
/// gli avvisi vengono quindi ristampati a ogni ricompilazione riuscita.
/// Usa il polling dell'orario di modifica, così non servono dipendenze esterne.
pub fn osserva(
    ingresso: &Path,
    uscita: &Path,
    opzioni: &Opzioni,
    intervallo: Duration,
    mut notifica: impl FnMut(Risultato<Esito>),
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
