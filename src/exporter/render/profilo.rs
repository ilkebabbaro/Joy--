//! Diagnostica del render (`joypp render --profilo` / `--profilo-json`, schema `joypp-profilo/1`,
//! SPECIFICA C.1): tipi dei dati misurati, rapporto leggibile, JSON stabile e scrittura atomica.
//!
//! Le misure sono riempite da [`super::render_file`]; qui si trovano le statistiche
//! ([`Statistica::da_microsecondi`]), i due formati ([`Profilo::testo`], [`Profilo::json`]) e i
//! controlli sui percorsi ([`verifica_percorso_profilo`], [`scrivi_profilo_json`]).

use std::ffi::OsString;
use std::fs;
use std::path::Path;

use crate::errore::{Errore, Risultato};
use crate::exporter::stesso_file;
use super::browser::ripulisci_riga;
use super::json;

/// Identificativo dello schema del JSON di `--profilo-json` (C.1.4).
pub const SCHEMA: &str = "joypp-profilo/1";

/// Rapporto completo di un render riuscito con diagnostica attiva.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Profilo {
    /// Identificativo dello schema JSON: sempre `"joypp-profilo/1"`.
    pub schema: String,
    /// Ambiente di esecuzione (versione, sistema, browser, ffmpeg); nessun percorso.
    pub ambiente: Ambiente,
    /// Misure della scena compilata.
    pub scena: MisureScena,
    /// Misure del video prodotto.
    pub video: MisureVideo,
    /// Durate delle fasi del render, in millisecondi.
    pub fasi_ms: Fasi,
    /// Statistiche per fotogramma, in millisecondi.
    pub fotogramma_ms: PerFotogramma,
    /// Misure del processo ffmpeg.
    pub ffmpeg: MisureFfmpeg,
    /// Volumi di dati scambiati (PNG e base64).
    pub dati: Dati,
    /// Indici di prestazione derivati dalle misure.
    pub prestazioni: Prestazioni,
    /// Memoria: picco misurato (quando disponibile) e stima dei buffer.
    pub memoria: Memoria,
}

/// Ambiente di esecuzione del render.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ambiente {
    /// Versione di joypp (versione del pacchetto).
    pub joypp: String,
    /// Profilo di compilazione: `"debug"` o `"release"`.
    pub build: String,
    /// Sistema operativo (`std::env::consts::OS`).
    pub os: String,
    /// Architettura (`std::env::consts::ARCH`).
    pub arch: String,
    /// Thread logici disponibili; `None` se ignoti.
    pub thread_logici: Option<usize>,
    /// Campo `product` di `Browser.getVersion`; `None` se non disponibile.
    pub browser: Option<String>,
    /// Prima riga di `ffmpeg -version`, ripulita dai caratteri di controllo; `None` se ignota.
    pub ffmpeg: Option<String>,
}

/// Misure della scena compilata.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MisureScena {
    /// Numero di attori (non gruppi).
    pub attori: usize,
    /// Numero di gruppi.
    pub gruppi: usize,
    /// Somma delle animazioni di tutti gli attori.
    pub animazioni: usize,
    /// Dimensione dell'HTML generato, in byte.
    pub byte_html: u64,
    /// Durata della scena, in millisecondi.
    pub durata_ms: f64,
}

/// Misure del video prodotto.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MisureVideo {
    /// Numero di fotogrammi scritti.
    pub fotogrammi: u64,
    /// Larghezza finale del video, in pixel.
    pub larghezza: u32,
    /// Altezza finale del video, in pixel.
    pub altezza: u32,
    /// Fotogrammi al secondo.
    pub fps: u32,
    /// Qualità H.264 (CRF) usata.
    pub crf: u8,
    /// Preset di codifica (sempre `"medium"`).
    pub preset: String,
    /// Dimensione del file video, in byte.
    pub byte: u64,
}

/// Durate delle fasi del render, in millisecondi.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fasi {
    /// Lettura del sorgente.
    pub lettura: f64,
    /// Analisi lessicale e sintattica.
    pub analisi: f64,
    /// Compilazione (AST e timeline).
    pub compilazione: f64,
    /// Generazione dell'HTML della scena.
    pub generazione_html: f64,
    /// Ricerca e verifica di ffmpeg e browser, cartella temporanea, percorsi, parziale.
    pub preparazione: f64,
    /// Avvio del browser e attesa di `DevToolsActivePort`.
    pub avvio_browser: f64,
    /// Connessione CDP (senza `Browser.getVersion`).
    pub connessione_cdp: f64,
    /// Apertura della pagina, viewport, navigazione e controllo di `__joy_seek`.
    pub caricamento_pagina: f64,
    /// Avvio del processo ffmpeg.
    pub avvio_ffmpeg: f64,
    /// Intero ciclo sui fotogrammi.
    pub ciclo_fotogrammi: f64,
    /// Chiusura ordinata del browser.
    pub chiusura_browser: f64,
    /// Chiusura di ffmpeg e codifica residua.
    pub finalizzazione_ffmpeg: f64,
    /// Rinomina del file parziale e lettura della dimensione.
    pub pubblicazione: f64,
    /// Dall'ingresso in `render_file` all'uscita.
    pub totale: f64,
}

/// Statistiche per fotogramma di ciascuna fase del ciclo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PerFotogramma {
    /// Andata e ritorno di `Runtime.evaluate` con `__joy_seek`.
    pub seek: Statistica,
    /// Andata e ritorno di `Page.captureScreenshot` (misura combinata).
    pub screenshot: Statistica,
    /// Decodifica base64 dello screenshot.
    pub base64: Statistica,
    /// Analisi delle dimensioni PNG e confronto con il palco.
    pub verifica_png: Statistica,
    /// Tempo bloccato dentro l'invio a ffmpeg (coda piena).
    pub attesa_coda: Statistica,
}

/// Statistica su una serie di durate, in millisecondi (percentili nearest-rank).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Statistica {
    /// Somma delle durate.
    pub totale: f64,
    /// Media aritmetica.
    pub media: f64,
    /// Durata minima.
    pub min: f64,
    /// Mediana (50° percentile).
    pub p50: f64,
    /// 95° percentile.
    pub p95: f64,
    /// Durata massima.
    pub max: f64,
}

/// Misure del processo ffmpeg.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MisureFfmpeg {
    /// Somma dei `write_all` nel thread di scrittura, in millisecondi.
    pub scrittura_pipe_ms: f64,
    /// Capacità della coda dei fotogrammi (sempre 4).
    pub coda_capacita: usize,
    /// Invii che hanno trovato la coda piena.
    pub fotogrammi_con_coda_piena: u64,
}

/// Volumi di dati scambiati durante il render.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dati {
    /// Byte PNG totali catturati.
    pub byte_png_totali: u64,
    /// Dimensione del PNG più grande, in byte.
    pub byte_png_max: u64,
    /// Byte base64 totali ricevuti dal browser.
    pub byte_base64_totali: u64,
}

/// Indici di prestazione derivati dalle misure.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Prestazioni {
    /// Fotogrammi al secondo del solo ciclo: `fps_ciclo = N / (ciclo_fotogrammi_ms / 1000)`.
    /// `None` se il denominatore è 0 o il risultato non è finito.
    pub fps_ciclo: Option<f64>,
    /// Fotogrammi al secondo dell'intero render: `fps_totale = N / (totale_ms / 1000)`.
    /// `None` se il denominatore è 0 o il risultato non è finito.
    pub fps_totale: Option<f64>,
    /// Secondi di video prodotti per secondo di render:
    /// `rapporto_tempo_reale = (N / fps_video) / (totale_ms / 1000)`, con `fps_video` il
    /// frame rate del video. `None` se il denominatore è 0 o il risultato non è finito.
    pub rapporto_tempo_reale: Option<f64>,
}

/// Memoria: picco del processo (se misurabile) e stima dei buffer di fotogrammi.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Memoria {
    /// Picco di memoria residente di joypp (`VmHWM`), in byte; `None` dove non misurabile.
    pub picco_processo_byte: Option<u64>,
    /// Origine di `picco_processo_byte`: `"VmHWM"` oppure `"non disponibile"`.
    pub picco_processo_fonte: String,
    /// Stima del limite superiore dei PNG in memoria: `(coda + 2) × byte_png_max`.
    pub stima_buffer_fotogrammi_byte: u64,
}

impl Statistica {
    /// Statistica di una serie di durate espresse in microsecondi, convertita in millisecondi.
    ///
    /// Percentili *nearest-rank*: il q-esimo percentile è `v[ceil(q·n) − 1]` sui valori ordinati.
    /// Con la lista vuota tutti i campi valgono 0.
    pub fn da_microsecondi(valori: &[u64]) -> Statistica {
        if valori.is_empty() {
            return Statistica::default();
        }
        let mut ordinati = valori.to_vec();
        ordinati.sort_unstable();
        let n = ordinati.len();
        // Rango nearest-rank con aritmetica intera: ceil(percento·n / 100), poi indice da zero.
        let percentile = |percento: usize| {
            let rango = (percento * n).div_ceil(100).clamp(1, n);
            ordinati.get(rango - 1).copied().unwrap_or(0)
        };
        let somma: u128 = ordinati.iter().map(|&v| u128::from(v)).sum();
        let ms = |us: u64| us as f64 / 1000.0;
        let totale = somma as f64 / 1000.0;
        Statistica {
            totale,
            media: totale / n as f64,
            min: ms(ordinati.first().copied().unwrap_or(0)),
            p50: ms(percentile(50)),
            p95: ms(percentile(95)),
            max: ms(ordinati.last().copied().unwrap_or(0)),
        }
    }
}

/// Formatta un numero con 3 decimali; `null` se non finito (il JSON non ammette `NaN`).
fn numero_json(v: f64) -> String {
    if v.is_finite() { format!("{v:.3}") } else { "null".to_string() }
}

/// Formatta un `Option` numerico come numero o `null`.
fn opzione_json<T: ToString>(v: Option<T>) -> String {
    v.map_or_else(|| "null".to_string(), |x| x.to_string())
}

/// Formatta una stringa opzionale come stringa JSON (escape di `json::stringa`) o `null`.
fn testo_opzionale_json(v: Option<&str>) -> String {
    v.map_or_else(|| "null".to_string(), json::stringa)
}

/// Oggetto JSON con chiavi nell'ordine dato (le chiavi sono costanti ASCII senza escape).
fn oggetto(campi: &[(&str, String)]) -> String {
    let voci: Vec<String> = campi.iter().map(|(k, v)| format!("{}:{v}", json::stringa(k))).collect();
    format!("{{{}}}", voci.join(","))
}

/// Statistica come oggetto `{totale, media, min, p50, p95, max}` in millisecondi.
fn statistica_json(s: &Statistica) -> String {
    oggetto(&[
        ("totale", numero_json(s.totale)),
        ("media", numero_json(s.media)),
        ("min", numero_json(s.min)),
        ("p50", numero_json(s.p50)),
        ("p95", numero_json(s.p95)),
        ("max", numero_json(s.max)),
    ])
}

/// Millisecondi di una durata in forma testuale con 3 decimali.
fn ms_testo(v: f64) -> String {
    format!("{v:.3}")
}

/// Valore opzionale numerico per il rapporto: «non disponibile» se manca.
fn opzionale_testo(v: Option<f64>) -> String {
    v.map_or_else(|| "non disponibile".to_string(), ms_testo)
}

/// Quota percentuale di `parte` su `totale`, o «non disponibile» se il totale non è positivo.
fn quota(parte: f64, totale: f64) -> String {
    if totale > 0.0 && (parte / totale * 100.0).is_finite() {
        format!("{:>6.1}%", parte / totale * 100.0)
    } else {
        "non disponibile".to_string()
    }
}

/// Riga di fase: nome, tempo in ms e quota del totale.
fn riga_fase(nome: &str, ms: f64, totale: f64) -> String {
    format!("  {nome:<22}{:>12} ms  {}", ms_testo(ms), quota(ms, totale))
}

/// Riga per fotogramma: media, p50, p95 e max in ms.
fn riga_statistica(nome: &str, s: &Statistica) -> String {
    format!(
        "  {nome:<22}{:>10} / {:>10} / {:>10} / {:>10}",
        ms_testo(s.media),
        ms_testo(s.p50),
        ms_testo(s.p95),
        ms_testo(s.max)
    )
}

impl Profilo {
    /// Rapporto JSON `joypp-profilo/1` (C.1.4): chiavi in ordine fisso, `null` dove indicato,
    /// tempi in millisecondi con 3 decimali, nessun percorso.
    pub fn json(&self) -> String {
        let a = &self.ambiente;
        let s = &self.scena;
        let v = &self.video;
        let f = &self.fasi_ms;
        let p = &self.fotogramma_ms;
        let m = &self.ffmpeg;
        let d = &self.dati;
        let pr = &self.prestazioni;
        let me = &self.memoria;
        oggetto(&[
            ("schema", json::stringa(SCHEMA)),
            (
                "ambiente",
                oggetto(&[
                    ("joypp", json::stringa(&a.joypp)),
                    ("build", json::stringa(&a.build)),
                    ("os", json::stringa(&a.os)),
                    ("arch", json::stringa(&a.arch)),
                    ("thread_logici", opzione_json(a.thread_logici)),
                    ("browser", testo_opzionale_json(a.browser.as_deref())),
                    ("ffmpeg", testo_opzionale_json(a.ffmpeg.as_deref())),
                ]),
            ),
            (
                "scena",
                oggetto(&[
                    ("attori", s.attori.to_string()),
                    ("gruppi", s.gruppi.to_string()),
                    ("animazioni", s.animazioni.to_string()),
                    ("byte_html", s.byte_html.to_string()),
                    ("durata_ms", numero_json(s.durata_ms)),
                ]),
            ),
            (
                "video",
                oggetto(&[
                    ("fotogrammi", v.fotogrammi.to_string()),
                    ("larghezza", v.larghezza.to_string()),
                    ("altezza", v.altezza.to_string()),
                    ("fps", v.fps.to_string()),
                    ("crf", v.crf.to_string()),
                    ("preset", json::stringa(&v.preset)),
                    ("byte", v.byte.to_string()),
                ]),
            ),
            (
                "fasi_ms",
                oggetto(&[
                    ("lettura", numero_json(f.lettura)),
                    ("analisi", numero_json(f.analisi)),
                    ("compilazione", numero_json(f.compilazione)),
                    ("generazione_html", numero_json(f.generazione_html)),
                    ("preparazione", numero_json(f.preparazione)),
                    ("avvio_browser", numero_json(f.avvio_browser)),
                    ("connessione_cdp", numero_json(f.connessione_cdp)),
                    ("caricamento_pagina", numero_json(f.caricamento_pagina)),
                    ("avvio_ffmpeg", numero_json(f.avvio_ffmpeg)),
                    ("ciclo_fotogrammi", numero_json(f.ciclo_fotogrammi)),
                    ("chiusura_browser", numero_json(f.chiusura_browser)),
                    ("finalizzazione_ffmpeg", numero_json(f.finalizzazione_ffmpeg)),
                    ("pubblicazione", numero_json(f.pubblicazione)),
                    ("totale", numero_json(f.totale)),
                ]),
            ),
            (
                "fotogramma_ms",
                oggetto(&[
                    ("seek", statistica_json(&p.seek)),
                    ("screenshot", statistica_json(&p.screenshot)),
                    ("base64", statistica_json(&p.base64)),
                    ("verifica_png", statistica_json(&p.verifica_png)),
                    ("attesa_coda", statistica_json(&p.attesa_coda)),
                ]),
            ),
            (
                "ffmpeg",
                oggetto(&[
                    ("scrittura_pipe_ms", numero_json(m.scrittura_pipe_ms)),
                    ("coda_capacita", m.coda_capacita.to_string()),
                    ("fotogrammi_con_coda_piena", m.fotogrammi_con_coda_piena.to_string()),
                ]),
            ),
            (
                "dati",
                oggetto(&[
                    ("byte_png_totali", d.byte_png_totali.to_string()),
                    ("byte_png_max", d.byte_png_max.to_string()),
                    ("byte_base64_totali", d.byte_base64_totali.to_string()),
                ]),
            ),
            (
                "prestazioni",
                oggetto(&[
                    ("fps_ciclo", opzione_numero_json(pr.fps_ciclo)),
                    ("fps_totale", opzione_numero_json(pr.fps_totale)),
                    ("rapporto_tempo_reale", opzione_numero_json(pr.rapporto_tempo_reale)),
                ]),
            ),
            (
                "memoria",
                oggetto(&[
                    ("picco_processo_byte", opzione_json(me.picco_processo_byte)),
                    ("picco_processo_fonte", json::stringa(&me.picco_processo_fonte)),
                    ("stima_buffer_fotogrammi_byte", me.stima_buffer_fotogrammi_byte.to_string()),
                ]),
            ),
        ])
    }

    /// Rapporto leggibile (C.1.5) per stderr: sezioni Scena, Video, Fasi, Per fotogramma,
    /// ffmpeg, Dati, Prestazioni e Memoria. Le righe stimate dicono «(stima)», quelle assenti
    /// «non disponibile». Nessun percorso.
    pub fn testo(&self) -> String {
        let a = &self.ambiente;
        let s = &self.scena;
        let v = &self.video;
        let f = &self.fasi_ms;
        let p = &self.fotogramma_ms;
        let m = &self.ffmpeg;
        let d = &self.dati;
        let pr = &self.prestazioni;
        let me = &self.memoria;
        let thread = a.thread_logici.map_or_else(|| "thread non disponibili".to_string(), |n| format!("{n} thread"));
        let mut r: Vec<String> = Vec::new();
        r.push(format!(
            "Profilo del render (joypp {}, {}, {} {}, {thread})",
            a.joypp, a.build, a.os, a.arch
        ));
        r.push(String::new());
        r.push("Scena".to_string());
        r.push(format!(
            "  attori {}, gruppi {}, animazioni {}, HTML {} byte, durata {} ms",
            s.attori,
            s.gruppi,
            s.animazioni,
            s.byte_html,
            ms_testo(s.durata_ms)
        ));
        r.push(String::new());
        r.push("Video".to_string());
        r.push(format!(
            "  fotogrammi {}, {}×{}, {} fps, CRF {}, preset {}, {} byte",
            v.fotogrammi, v.larghezza, v.altezza, v.fps, v.crf, v.preset, v.byte
        ));
        r.push(String::new());
        r.push("Fasi (tempo, quota del totale)".to_string());
        let totale = f.totale;
        let fasi: [(&str, f64); 14] = [
            ("lettura", f.lettura),
            ("analisi", f.analisi),
            ("compilazione", f.compilazione),
            ("generazione_html", f.generazione_html),
            ("preparazione", f.preparazione),
            ("avvio_browser", f.avvio_browser),
            ("connessione_cdp", f.connessione_cdp),
            ("caricamento_pagina", f.caricamento_pagina),
            ("avvio_ffmpeg", f.avvio_ffmpeg),
            ("ciclo_fotogrammi", f.ciclo_fotogrammi),
            ("chiusura_browser", f.chiusura_browser),
            ("finalizzazione_ffmpeg", f.finalizzazione_ffmpeg),
            ("pubblicazione", f.pubblicazione),
            ("totale", f.totale),
        ];
        for (nome, ms) in fasi {
            r.push(riga_fase(nome, ms, totale));
        }
        r.push(String::new());
        r.push("Per fotogramma (ms: media / p50 / p95 / max)".to_string());
        r.push(riga_statistica("seek", &p.seek));
        r.push(riga_statistica("screenshot", &p.screenshot));
        r.push(riga_statistica("base64", &p.base64));
        r.push(riga_statistica("verifica_png", &p.verifica_png));
        r.push(riga_statistica("attesa_coda", &p.attesa_coda));
        r.push(String::new());
        r.push("ffmpeg".to_string());
        // Le stringhe dell'ambiente passano da `ripulisci_riga` anche qui (difesa in profondità).
        let versione = a.ffmpeg.as_deref().map_or_else(|| "non disponibile".to_string(), ripulisci_riga);
        let browser = a.browser.as_deref().map_or_else(|| "non disponibile".to_string(), ripulisci_riga);
        r.push(format!("  versione: {versione}"));
        r.push(format!("  browser: {browser}"));
        r.push(format!("  scrittura pipe: {} ms", ms_testo(m.scrittura_pipe_ms)));
        r.push(format!(
            "  coda: capacità {}, fotogrammi con coda piena {}",
            m.coda_capacita, m.fotogrammi_con_coda_piena
        ));
        r.push(String::new());
        r.push("Dati".to_string());
        r.push(format!(
            "  PNG totali {} byte (massimo {} byte), base64 {} byte",
            d.byte_png_totali, d.byte_png_max, d.byte_base64_totali
        ));
        r.push(String::new());
        r.push("Prestazioni".to_string());
        r.push(format!("  fps del ciclo: {}", opzionale_testo(pr.fps_ciclo)));
        r.push(format!("  fps del render: {}", opzionale_testo(pr.fps_totale)));
        r.push(format!(
            "  secondi di video per secondo di render: {}",
            opzionale_testo(pr.rapporto_tempo_reale)
        ));
        r.push(String::new());
        r.push("Memoria".to_string());
        let picco = match me.picco_processo_byte {
            Some(b) => format!("{b} byte ({})", me.picco_processo_fonte),
            None => "non disponibile".to_string(),
        };
        r.push(format!("  picco del processo joypp: {picco}"));
        r.push(format!(
            "  buffer dei fotogrammi: {} byte (stima)",
            me.stima_buffer_fotogrammi_byte
        ));
        r.join("\n")
    }
}

/// Come [`numero_json`] per un `Option`: `null` se assente.
fn opzione_numero_json(v: Option<f64>) -> String {
    v.map_or_else(|| "null".to_string(), numero_json)
}

/// Forma lessicale di un percorso: assoluto, con `.` rimossi e `..` risolti senza toccare il
/// file system. Su Windows la forma è minuscola (il file system non distingue le maiuscole).
fn forma_lessicale(p: &Path) -> String {
    use std::path::Component;
    let assoluto = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let mut componenti: Vec<Component> = Vec::new();
    for c in assoluto.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(componenti.last(), Some(Component::Normal(_))) {
                    componenti.pop();
                }
            }
            altro => componenti.push(altro),
        }
    }
    let testo = componenti.into_iter().collect::<std::path::PathBuf>().to_string_lossy().into_owned();
    if cfg!(windows) { testo.to_lowercase() } else { testo }
}

/// Due percorsi indicano lo stesso file se coincidono per la normalizzazione lessicale o, quando
/// esistono entrambi, per quella canonica (`stesso_file`).
fn stessa_destinazione(a: &Path, b: &Path) -> bool {
    forma_lessicale(a) == forma_lessicale(b) || stesso_file(a, b)
}

/// Controllo economico, senza creare nulla: il percorso del JSON deve essere non vuoto, non una
/// cartella esistente e non avere un antenato esistente che non sia una cartella.
fn controlla_scrivibilita(json: &Path) -> Risultato<()> {
    let errore = |motivo: String| Errore::globale(format!("impossibile scrivere «{}»: {motivo}", json.display()));
    if json.as_os_str().is_empty() {
        return Err(errore("percorso vuoto".to_string()));
    }
    if json.is_dir() {
        return Err(errore("è una cartella".to_string()));
    }
    for antenato in json.ancestors().skip(1).filter(|a| !a.as_os_str().is_empty()) {
        if antenato.exists() && !antenato.is_dir() {
            return Err(errore(format!("«{}» non è una cartella", antenato.display())));
        }
    }
    Ok(())
}

/// Controlla il percorso del profilo prima di avviare processi o scrivere file.
///
/// Verifica che il percorso sia scrivibile in linea di principio (vedi [`controlla_scrivibilita`])
/// e che non coincida con il sorgente né con il video, né lessicalmente (`./x`, `d/../x`,
/// percorso assoluto contro relativo, maiuscole su Windows) né, se esistono, canonicamente.
///
/// # Errori
/// `impossibile scrivere «X»: …` per percorsi non scrivibili, oppure
/// `il file del profilo deve essere diverso dal sorgente e dal video («X»)`, dove `X` è il
/// percorso del JSON.
pub fn verifica_percorso_profilo(sorgente: &Path, video: &Path, json: &Path) -> Risultato<()> {
    controlla_scrivibilita(json)?;
    if stessa_destinazione(json, sorgente) || stessa_destinazione(json, video) {
        return Err(Errore::globale(format!(
            "il file del profilo deve essere diverso dal sorgente e dal video («{}»)",
            json.display()
        )));
    }
    Ok(())
}

/// Scrive `testo` in `percorso` in modo atomico: crea la cartella se manca, scrive
/// `<file>.joypp-parziale` nella stessa cartella e poi lo rinomina in `<file>`. In caso di errore
/// il parziale viene rimosso e un `<file>` preesistente resta intatto.
///
/// # Errori
/// `impossibile creare la cartella «X»: …` oppure `impossibile scrivere «X»: …`.
pub fn scrivi_profilo_json(percorso: &Path, testo: &str) -> Risultato<()> {
    if let Some(cartella) = percorso.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(cartella).map_err(|e| {
            Errore::globale(format!("impossibile creare la cartella «{}»: {e}", cartella.display()))
        })?;
    }
    let mut nome: OsString = percorso.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    nome.push(".joypp-parziale");
    let parziale = percorso.with_file_name(nome);
    let errore = |e: std::io::Error| Errore::globale(format!("impossibile scrivere «{}»: {e}", percorso.display()));
    if let Err(e) = fs::write(&parziale, testo) {
        let _ = fs::remove_file(&parziale);
        return Err(errore(e));
    }
    if let Err(e) = fs::rename(&parziale, percorso) {
        let _ = fs::remove_file(&parziale);
        return Err(errore(e));
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    fn profilo_di_prova() -> Profilo {
        Profilo {
            schema: SCHEMA.to_string(),
            ambiente: Ambiente {
                joypp: "0.2.1".into(),
                build: "debug".into(),
                os: "windows".into(),
                arch: "x86_64".into(),
                thread_logici: Some(8),
                browser: Some("HeadlessChrome 120 \"x\"".into()),
                ffmpeg: None,
            },
            scena: MisureScena { attori: 2, gruppi: 1, animazioni: 5, byte_html: 900, durata_ms: 2000.0 },
            video: MisureVideo {
                fotogrammi: 121,
                larghezza: 64,
                altezza: 36,
                fps: 60,
                crf: 18,
                preset: "medium".into(),
                byte: 4096,
            },
            fasi_ms: Fasi { lettura: 1.0, totale: 1000.0, ciclo_fotogrammi: 800.0, ..Fasi::default() },
            fotogramma_ms: PerFotogramma {
                seek: Statistica::da_microsecondi(&[1000, 2000, 3000]),
                ..PerFotogramma::default()
            },
            ffmpeg: MisureFfmpeg { scrittura_pipe_ms: 2.5, coda_capacita: 4, fotogrammi_con_coda_piena: 0 },
            dati: Dati { byte_png_totali: 10, byte_png_max: 5, byte_base64_totali: 14 },
            prestazioni: Prestazioni {
                fps_ciclo: Some(151.25),
                fps_totale: None,
                rapporto_tempo_reale: Some(2.0),
            },
            memoria: Memoria {
                picco_processo_byte: None,
                picco_processo_fonte: "non disponibile".into(),
                stima_buffer_fotogrammi_byte: 30,
            },
        }
    }

    #[test]
    fn statistica_nearest_rank() {
        let valori: Vec<u64> = (1..=20).map(|i| i * 1000).collect();
        let s = Statistica::da_microsecondi(&valori);
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 20.0);
        assert_eq!(s.p50, 10.0);
        assert_eq!(s.p95, 19.0);
        assert_eq!(s.totale, 210.0);
        assert!((s.media - 10.5).abs() < 1e-12);
    }

    #[test]
    fn statistica_lista_vuota_e_singola() {
        assert_eq!(Statistica::da_microsecondi(&[]), Statistica::default());
        let s = Statistica::da_microsecondi(&[2500]);
        assert_eq!((s.totale, s.media, s.min, s.p50, s.p95, s.max), (2.5, 2.5, 2.5, 2.5, 2.5, 2.5));
    }

    #[test]
    fn statistica_ordina_l_ingresso() {
        let s = Statistica::da_microsecondi(&[3000, 1000, 2000]);
        assert_eq!((s.min, s.p50, s.max), (1.0, 2.0, 3.0));
    }

    #[test]
    fn json_valido_con_schema_e_ordine_fisso() {
        let j = profilo_di_prova().json();
        assert!(json::analizza(&j).is_ok(), "JSON non valido: {j}");
        assert!(j.starts_with("{\"schema\":\"joypp-profilo/1\",\"ambiente\":{\"joypp\":\"0.2.1\""));
        let ordine = [
            "\"schema\"", "\"ambiente\"", "\"scena\"", "\"video\"", "\"fasi_ms\"",
            "\"fotogramma_ms\"", "\"ffmpeg\"", "\"dati\"", "\"prestazioni\"", "\"memoria\"",
        ];
        // Ogni chiave va cercata dopo la precedente: «ffmpeg» compare anche dentro `ambiente`.
        let mut inizio = 0;
        for chiave in ordine {
            let p = j[inizio..].find(chiave).unwrap_or_else(|| panic!("chiave {chiave} assente o fuori ordine"));
            inizio += p + chiave.len();
        }
    }

    #[test]
    fn json_null_e_numeri_a_tre_decimali() {
        let j = profilo_di_prova().json();
        assert!(j.contains("\"browser\":\"HeadlessChrome 120 \\\"x\\\"\""));
        assert!(j.contains("\"ffmpeg\":null"));
        assert!(j.contains("\"picco_processo_byte\":null"));
        assert!(j.contains("\"fps_totale\":null"));
        assert!(j.contains("\"fps_ciclo\":151.250"));
        assert!(j.contains("\"totale\":1000.000"));
        assert!(j.contains("\"durata_ms\":2000.000"));
    }

    #[test]
    fn json_senza_percorsi() {
        // L'unico «/» ammesso è quello dello schema «joypp-profilo/1».
        let j = profilo_di_prova().json();
        assert_eq!(j.matches('/').count(), 1, "{j}");
        assert!(!j.contains('\\') || j.contains("\\\""));
    }

    #[test]
    fn testo_con_stima_e_non_disponibile() {
        let t = profilo_di_prova().testo();
        assert!(t.starts_with("Profilo del render (joypp 0.2.1, debug, windows x86_64, 8 thread)"));
        assert!(t.contains("buffer dei fotogrammi: 30 byte (stima)"));
        assert!(t.contains("picco del processo joypp: non disponibile"));
        assert!(t.contains("fps del render: non disponibile"));
        assert!(t.contains("versione: non disponibile"));
        assert!(t.contains("fps del ciclo: 151.250"));
        assert!(!t.contains(".mp4") && !t.contains('\\'));
    }

    #[test]
    fn prestazioni_none_con_denominatore_zero() {
        let mut p = profilo_di_prova();
        p.fasi_ms.totale = 0.0;
        p.prestazioni = Prestazioni { fps_ciclo: None, fps_totale: None, rapporto_tempo_reale: None };
        assert!(p.json().contains("\"fps_ciclo\":null,\"fps_totale\":null,\"rapporto_tempo_reale\":null"));
        assert!(p.testo().contains("fps del ciclo: non disponibile"));
        assert!(!p.testo().contains("NaN") && !p.json().contains("NaN"));
    }

    #[test]
    fn percorso_del_profilo_in_conflitto() {
        // `Cargo.toml` esiste nella radice del pacchetto, dove cargo esegue i test.
        let src = Path::new("Cargo.toml");
        let video = Path::new("video-inesistente.mp4");
        assert!(verifica_percorso_profilo(src, video, Path::new("p.json")).is_ok());
        assert!(verifica_percorso_profilo(src, video, Path::new("video-inesistente.mp4")).is_err());
        let e = verifica_percorso_profilo(src, video, Path::new("./Cargo.toml")).unwrap_err();
        assert_eq!(
            e.messaggio,
            "il file del profilo deve essere diverso dal sorgente e dal video («./Cargo.toml»)"
        );
    }

    /// Esito di `verifica_percorso_profilo` con sorgente e video inesistenti (il video non esiste
    /// ancora quando si fa il primo controllo).
    fn verifica_con_video(video: &str, json: &Path) -> Risultato<()> {
        verifica_percorso_profilo(Path::new("sorgente-inesistente.joy"), Path::new(video), json)
    }

    #[test]
    fn conflitto_lessicale_con_il_video() {
        assert!(verifica_con_video("x.mp4", Path::new("./x.mp4")).is_err());
        assert!(verifica_con_video("x.mp4", Path::new("d/../x.mp4")).is_err());
        assert!(verifica_con_video("x.mp4", Path::new("d/./../x.mp4")).is_err());
        let assoluto = std::env::current_dir().unwrap_or_default().join("x.mp4");
        assert!(verifica_con_video("x.mp4", &assoluto).is_err());
        assert!(verifica_con_video("x.mp4", Path::new("d/../p.json")).is_ok());
        assert!(verifica_con_video("x.mp4", Path::new("d/x.json")).is_ok());
    }

    #[test]
    fn conflitto_con_il_sorgente_lessicale() {
        let sorgente = Path::new("scena.joy");
        let e = verifica_percorso_profilo(sorgente, Path::new("v.mp4"), Path::new("./scena.joy")).unwrap_err();
        assert!(e.messaggio.starts_with("il file del profilo deve essere diverso"));
        assert!(verifica_percorso_profilo(sorgente, Path::new("v.mp4"), Path::new("x/../scena.joy")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn conflitto_maiuscole_su_windows() {
        assert!(verifica_con_video("out.mp4", Path::new("OUT.MP4")).is_err());
        assert!(verifica_con_video("out.mp4", Path::new(r".\Out.Mp4")).is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn maiuscole_distinte_altrove() {
        assert!(verifica_con_video("out.mp4", Path::new("OUT.MP4")).is_ok());
    }

    #[test]
    fn percorso_json_vuoto_o_cartella_rifiutati() {
        let e = verifica_con_video("v.mp4", Path::new("")).unwrap_err();
        assert_eq!(e.messaggio, "impossibile scrivere «»: percorso vuoto");
        // `src` è una cartella esistente nella radice del pacchetto (cwd dei test).
        let e = verifica_con_video("v.mp4", Path::new("src")).unwrap_err();
        assert_eq!(e.messaggio, "impossibile scrivere «src»: è una cartella");
    }

    #[test]
    fn antenato_che_non_e_una_cartella_rifiutato() {
        // `Cargo.toml` è un file esistente nella radice del pacchetto.
        let json = Path::new("Cargo.toml").join("p.json");
        let e = verifica_con_video("v.mp4", &json).unwrap_err();
        assert!(e.messaggio.starts_with("impossibile scrivere «"));
        assert!(e.messaggio.ends_with("»: «Cargo.toml» non è una cartella"), "{}", e.messaggio);
    }

    #[test]
    fn controllo_non_crea_nulla() {
        let cartella = format!("nonesiste-profilo-{}", std::process::id());
        let json = Path::new(&cartella).join("sotto").join("p.json");
        assert!(verifica_con_video("v.mp4", &json).is_ok());
        assert!(!Path::new(&cartella).exists());
    }

    fn cartella_di_prova(nome: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("joypp-profilo-{nome}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn scrittura_atomica_del_json() {
        let cartella = cartella_di_prova("ok");
        let file = cartella.join("sotto").join("p.json");
        scrivi_profilo_json(&file, "{\"a\":1}").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "{\"a\":1}");
        assert!(!cartella.join("sotto").join("p.json.joypp-parziale").exists());
        scrivi_profilo_json(&file, "{\"a\":2}").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "{\"a\":2}");
        let _ = fs::remove_dir_all(&cartella);
    }

    #[test]
    fn scrittura_fallita_non_lascia_parziali() {
        let cartella = cartella_di_prova("errore");
        fs::create_dir_all(&cartella).unwrap();
        // Una cartella con lo stesso nome del file impedisce la rinomina.
        let file = cartella.join("occupato.json");
        fs::create_dir_all(&file).unwrap();
        let e = scrivi_profilo_json(&file, "{}").unwrap_err();
        assert!(e.messaggio.starts_with("impossibile scrivere «"));
        assert!(!cartella.join("occupato.json.joypp-parziale").exists());
        assert!(file.is_dir());
        let _ = fs::remove_dir_all(&cartella);
    }
}
