//! CLI `joypp`: compila un file `.joy` in un singolo file `.html` auto-contenuto.
//!
//! ```text
//! joypp <file.joy> [-o <file.html>] [--pretty] [--watch] [--senza-player]
//! joypp render <file.joy> [-o <file.mp4>] [--fps N] [--crf N] [--profilo] [--profilo-json <file.json>]
//! ```
//!
//! L'interpretazione degli argomenti vive in `joypp::exporter::cli` (testata a parte) e il
//! render video in `joypp::exporter::render`; qui restano solo aiuto, stampe ed exit code.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use joypp::exporter::cli::{
    ArgomentiCompila, ArgomentiRender, Comando, interpreta, uscita_video_predefinita,
};
use joypp::exporter::render::{Evento, render_file};
use joypp::exporter::{Esito, compila_file, osserva, output_predefinito};

const AIUTO: &str = "\
Joy++ — compilatore di animazioni in italiano naturale

Uso:
  joypp <file.joy> [opzioni]
  joypp render <file.joy> [opzioni]

Opzioni:
  -o, --output <file.html>   file di destinazione (predefinito: <file>.html)
      --pretty               HTML indentato e leggibile (predefinito: compatto)
      --senza-player         omette il mini-player dall'HTML (predefinito: incluso)
      --watch                ricompila automaticamente a ogni salvataggio
  -h, --help                 mostra questo aiuto
  -V, --version              mostra la versione

joypp render — esporta la scena in un video MP4 (H.264, richiede un browser e ffmpeg):
  -o, --output <file.mp4>    file di destinazione (predefinito: <file>.mp4)
      --fps <N>              fotogrammi al secondo, da 1 a 240 (predefinito: quello
                             dell'intestazione «a N fotogrammi», altrimenti 60)
      --crf <N>              qualità H.264, da 0 (senza perdita) a 51 (predefinito: 18)
      --profilo              stampa su stderr il rapporto di diagnostica del render
      --profilo-json <file>  scrive il rapporto di diagnostica in JSON (joypp-profilo/1)
  Non ammesse con render: --watch, --pretty, --senza-player.
  Il video ha le dimensioni del palco. Un file chiamato «render» si indica con ./render.

Variabili d'ambiente:
  JOYPP_BROWSER              percorso di Chrome, Chromium o Edge (predefinito: ricerca automatica)
  JOYPP_FFMPEG               percorso di ffmpeg (predefinito: ricerca nel PATH)

Esempi:
  joypp examples/primo_test.joy -o output.html
  joypp render examples/sistema_orbitale.joy -o video.mp4 --fps 30";

/// Stampa gli avvisi su stderr (in ordine di riga), poi il messaggio di successo su stdout.
/// Gli avvisi non cambiano l'exit code (SPECIFICA B.6).
fn successo(ingresso: &Path, uscita: &Path, esito: &Esito) {
    for avviso in &esito.avvisi {
        eprintln!("{avviso}");
    }
    println!("✔ {} → {} ({} byte)", ingresso.display(), uscita.display(), esito.byte);
}

/// Stampa il progresso su stderr (B.9.6): su terminale una riga riscritta con `\r` al massimo
/// ogni 100 ms, altrimenti una riga a ogni 10%.
struct Avanzamento {
    terminale: bool,
    ultima_stampa: Option<Instant>,
    ultimo_decile: u64,
    riga_aperta: bool,
}

impl Avanzamento {
    fn nuovo() -> Self {
        Self {
            terminale: std::io::stderr().is_terminal(),
            ultima_stampa: None,
            ultimo_decile: 0,
            riga_aperta: false,
        }
    }

    fn evento(&mut self, e: Evento) {
        match e {
            Evento::Avviso(testo) => {
                self.chiudi_riga();
                eprintln!("{testo}");
            }
            Evento::Fotogramma { fatti, totale } => {
                let percento = fatti * 100 / totale.max(1);
                let linea = format!("Render: fotogramma {fatti}/{totale} ({percento}%)");
                if self.terminale {
                    let finito = fatti == totale;
                    let scaduto = self
                        .ultima_stampa
                        .is_none_or(|t| t.elapsed() >= Duration::from_millis(100));
                    if finito || scaduto {
                        eprint!("\r{linea}");
                        let _ = std::io::stderr().flush();
                        self.ultima_stampa = Some(Instant::now());
                        self.riga_aperta = !finito;
                        if finito {
                            eprintln!();
                        }
                    }
                } else if percento / 10 > self.ultimo_decile || (fatti == totale && self.ultimo_decile < 10) {
                    self.ultimo_decile = percento / 10;
                    eprintln!("{linea}");
                }
            }
        }
    }

    /// Va a capo se una riga di progresso è ancora aperta (per non mescolarla con altri messaggi).
    fn chiudi_riga(&mut self) {
        if self.riga_aperta {
            eprintln!();
            self.riga_aperta = false;
        }
    }
}

/// Esegue `joypp render`.
fn esegui_render(a: ArgomentiRender) -> ExitCode {
    if a.ingresso.extension().and_then(|e| e.to_str()) != Some("joy") {
        eprintln!("Attenzione: «{}» non ha estensione .joy", a.ingresso.display());
    }
    let uscita = a.uscita.unwrap_or_else(|| uscita_video_predefinita(&a.ingresso));
    if uscita.extension().and_then(|e| e.to_str()) != Some("mp4") {
        eprintln!(
            "Attenzione: «{}» non ha estensione .mp4: il file sarà comunque un video MP4",
            uscita.display()
        );
    }
    let mut avanzamento = Avanzamento::nuovo();
    let esito = render_file(&a.ingresso, &uscita, &a.video, &mut |e| avanzamento.evento(e));
    avanzamento.chiudi_riga();
    match esito {
        Ok(e) => {
            println!(
                "✔ {} → {} ({} fotogrammi, {}×{}, {} fps, {:.2} s, {} byte)",
                a.ingresso.display(),
                uscita.display(),
                e.fotogrammi,
                e.larghezza,
                e.altezza,
                e.fps,
                e.durata_video(),
                e.byte
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: ArgomentiCompila = match interpreta(std::env::args().skip(1)) {
        Ok(Comando::Aiuto) => {
            println!("{AIUTO}");
            return ExitCode::SUCCESS;
        }
        Ok(Comando::Versione) => {
            println!("joypp {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Ok(Comando::Render(a)) => return esegui_render(a),
        Ok(Comando::Compila(a)) => a,
        Err(errore) => {
            eprintln!("{errore}\n\n{AIUTO}");
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
