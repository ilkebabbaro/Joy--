//! CLI `joypp`: compila un file `.joy` in un singolo file `.html` auto-contenuto.
//!
//! ```text
//! joypp <file.joy> [-o <file.html>] [--pretty] [--watch] [--senza-player]
//! ```

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use joypp::Opzioni;
use joypp::exporter::{Esito, compila_file, osserva, output_predefinito};

const AIUTO: &str = "\
Joy++ — compilatore di animazioni in italiano naturale

Uso:
  joypp <file.joy> [opzioni]

Opzioni:
  -o, --output <file.html>   file di destinazione (predefinito: <file>.html)
      --pretty               HTML indentato e leggibile (predefinito: compatto)
      --senza-player         omette il mini-player dall'HTML (predefinito: incluso)
      --watch                ricompila automaticamente a ogni salvataggio
  -h, --help                 mostra questo aiuto
  -V, --version              mostra la versione

Esempio:
  joypp examples/primo_test.joy -o output.html";

/// Argomenti interpretati dalla riga di comando.
struct Argomenti {
    ingresso: PathBuf,
    uscita: Option<PathBuf>,
    opzioni: Opzioni,
    osserva: bool,
}

/// Cosa deve fare il programma.
enum Comando {
    Compila(Argomenti),
    Aiuto,
    Versione,
}

fn interpreta(argomenti: impl Iterator<Item = String>) -> Result<Comando, String> {
    let mut ingresso = None;
    let mut uscita = None;
    let mut opzioni = Opzioni::default();
    let mut osserva = false;
    let mut argomenti = argomenti;
    while let Some(a) = argomenti.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(Comando::Aiuto),
            "-V" | "--version" => return Ok(Comando::Versione),
            "-o" | "--output" => {
                let p = argomenti.next().ok_or("l'opzione -o richiede un percorso")?;
                uscita = Some(PathBuf::from(p));
            }
            "--pretty" => opzioni.leggibile = true,
            "--senza-player" => opzioni.senza_player = true,
            "--watch" => osserva = true,
            opzione if opzione.starts_with('-') && opzione.len() > 1 => {
                return Err(format!("opzione sconosciuta «{opzione}»"));
            }
            percorso => {
                if ingresso.is_some() {
                    return Err(format!("è ammesso un solo file sorgente (trovato anche «{percorso}»)"));
                }
                ingresso = Some(PathBuf::from(percorso));
            }
        }
    }
    let ingresso = ingresso.ok_or("manca il file sorgente .joy")?;
    Ok(Comando::Compila(Argomenti { ingresso, uscita, opzioni, osserva }))
}

/// Stampa gli avvisi su stderr (in ordine di riga), poi il messaggio di successo su stdout.
/// Gli avvisi non cambiano l'exit code (SPECIFICA B.6).
fn successo(ingresso: &Path, uscita: &Path, esito: &Esito) {
    for avviso in &esito.avvisi {
        eprintln!("{avviso}");
    }
    println!("✔ {} → {} ({} byte)", ingresso.display(), uscita.display(), esito.byte);
}

fn main() -> ExitCode {
    let args = match interpreta(std::env::args().skip(1)) {
        Ok(Comando::Aiuto) => {
            println!("{AIUTO}");
            return ExitCode::SUCCESS;
        }
        Ok(Comando::Versione) => {
            println!("joypp {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Ok(Comando::Compila(a)) => a,
        Err(messaggio) => {
            eprintln!("Errore: {messaggio}\n\n{AIUTO}");
            return ExitCode::from(2);
        }
    };

    if args.ingresso.extension().and_then(|e| e.to_str()) != Some("joy") {
        eprintln!("Attenzione: «{}» non ha estensione .joy", args.ingresso.display());
    }
    let uscita = args.uscita.unwrap_or_else(|| output_predefinito(&args.ingresso));

    if args.osserva {
        println!("In osservazione di {} (Ctrl+C per uscire)...", args.ingresso.display());
        osserva(&args.ingresso, &uscita, &args.opzioni, Duration::from_millis(500), |esito| {
            match esito {
                Ok(esito) => successo(&args.ingresso, &uscita, &esito),
                Err(e) => eprintln!("{e}"),
            }
        });
    }

    match compila_file(&args.ingresso, &uscita, &args.opzioni) {
        Ok(esito) => {
            successo(&args.ingresso, &uscita, &esito);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
