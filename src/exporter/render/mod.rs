//! `joypp render`: esportazione in MP4 (SPECIFICA B.9).
//!
//! Compila il sorgente, apre l'HTML in un browser headless pilotato via CDP, chiama
//! `window.__joy_seek(t_k)` per ogni fotogramma, cattura un PNG e lo passa a `ffmpeg`.
//!
//! Moduli: [`base64`], [`json`], [`websocket`] e [`cdp`] formano il client CDP con la sola
//! libreria standard (D-B9-1); [`browser`] e [`ffmpeg`] cercano e lanciano i processi.

pub mod base64;
pub mod browser;
pub mod cdp;
pub mod ffmpeg;
pub mod json;
pub mod profilo;
pub mod websocket;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::ast::Colore;
use crate::errore::{Errore, Risultato};
use crate::security::limiti;
use crate::exporter::{Compilazione, Opzioni, compila_misurando, leggi_sorgente};

use browser::{Browser, Sistema, ripulisci_riga};
use cdp::{ErroreCdp, Sessione};
use ffmpeg::TempiFfmpeg;
use json::Valore;
use profilo::{
    Ambiente, Dati, Fasi, MisureFfmpeg, MisureScena, MisureVideo, Memoria, PerFotogramma, Prestazioni,
    Profilo, Statistica,
};


/// FPS predefinito quando né `--fps` né l'intestazione lo indicano.
pub const FPS_PREDEFINITO: u32 = 60;
/// CRF predefinito.
pub const CRF_PREDEFINITO: u8 = 18;
/// Parametri di `Page.captureScreenshot` (SPECIFICA B.9.5; misure della Fase 3 in
/// `benches/README.md`). `optimizeForSpeed` rende la compressione PNG del browser più rapida,
/// senza perdita: i pixel decodificati sono identici, i file sono più grandi (circa +38%). Il
/// ciclo per fotogramma scende del 21-35% a 4K. Cambiare questa stringa è una decisione di
/// specifica: il test `parametri_screenshot_fissi` la blocca.
const PARAMETRI_SCREENSHOT: &str =
    "{\"format\":\"png\",\"fromSurface\":true,\"captureBeyondViewport\":false,\"optimizeForSpeed\":true}";

/// Opzioni del video.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpzioniVideo {
    /// `--fps`; `None` = usa l'intestazione `a N fotogrammi`, altrimenti 60 (B.9.1).
    pub fps: Option<u32>,
    /// `--crf` (0-51).
    pub crf: u8,
    /// Misure di diagnostica attive (`--profilo` o `--profilo-json`).
    pub profilo: bool,
}

impl Default for OpzioniVideo {
    fn default() -> Self {
        Self { fps: None, crf: CRF_PREDEFINITO, profilo: false }
    }
}

/// Esito di un render riuscito.
#[derive(Debug, Clone, PartialEq)]
pub struct EsitoVideo {
    /// Numero di fotogrammi scritti.
    pub fotogrammi: u64,
    /// Larghezza finale del video (dopo l'eventuale pad).
    pub larghezza: u32,
    /// Altezza finale del video (dopo l'eventuale pad).
    pub altezza: u32,
    /// Fotogrammi al secondo effettivi.
    pub fps: u32,
    /// Dimensione del file scritto, in byte.
    pub byte: u64,
    /// Rapporto di diagnostica: `Some` se e solo se `OpzioniVideo::profilo` era attivo.
    pub profilo: Option<profilo::Profilo>,
}

impl EsitoVideo {
    /// Durata del video in secondi (`N / FPS`).
    pub fn durata_video(&self) -> f64 {
        self.fotogrammi as f64 / f64::from(self.fps)
    }
}

/// Notifiche di avanzamento verso la CLI.
#[derive(Debug, Clone, PartialEq)]
pub enum Evento {
    /// Una riga di avviso già formattata, da stampare su stderr prima del render.
    Avviso(String),
    /// Fotogramma `fatti` su `totale` completato.
    Fotogramma {
        /// Fotogrammi completati.
        fatti: u64,
        /// Fotogrammi totali.
        totale: u64,
    },
}

/// Piano di campionamento e dimensioni, calcolato prima di avviare processi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piano {
    /// Durata della scena in ms (4 decimali, come nell'HTML).
    pub d_ms: f64,
    /// Larghezza del palco.
    pub larghezza: u32,
    /// Altezza del palco.
    pub altezza: u32,
    /// FPS effettivi.
    pub fps: u32,
    /// Numero di fotogrammi.
    pub n: u64,
}

impl Piano {
    /// Dimensioni finali del video (pari).
    pub fn dimensioni_video(&self) -> (u32, u32) {
        (self.larghezza + self.larghezza % 2, self.altezza + self.altezza % 2)
    }
}

/// Precedenza di B.9.1: `--fps`, poi l'intestazione, poi 60.
pub fn fps_effettivo(opzione: Option<u32>, intestazione: Option<u32>) -> u32 {
    opzione.or(intestazione).unwrap_or(FPS_PREDEFINITO)
}

/// Numero di fotogrammi `N = ⌈D·fps/1000 − 10⁻⁹⌉ + 1` (B.9.3).
pub fn numero_fotogrammi(d_ms: f64, fps: u32) -> u64 {
    (d_ms * f64::from(fps) / 1000.0 - 1e-9).ceil().max(0.0) as u64 + 1
}

/// Istante `t_k = min(k·1000/fps, D)` in ms (B.9.3), calcolato da `k`.
pub fn istante(k: u64, fps: u32, d_ms: f64) -> f64 {
    (k as f64 * 1000.0 / f64::from(fps)).min(d_ms)
}

/// Sfondo composto su nero (`canale × alfa`, arrotondato).
pub fn sfondo_su_nero(c: Colore) -> [u8; 3] {
    let a = c.a.clamp(0.0, 1.0);
    let f = |v: u8| (f64::from(v) * a).round() as u8;
    [f(c.r), f(c.g), f(c.b)]
}

/// Controlli di B.9.7 che non avviano processi, nell'ordine della specifica.
///
/// # Errori
/// Durata 0, dimensioni non intere, palco troppo grande, troppi fotogrammi.
pub fn pianifica(c: &Compilazione, fps_opzione: Option<u32>) -> Risultato<Piano> {
    if c.durata <= 0.0 {
        return Err(Errore::globale("la scena non ha una timeline: il video sarebbe un solo fotogramma")
            .con_suggerimento("aggiungi almeno un blocco temporale, es. «Dal secondo 0 al secondo 3:»"));
    }
    let intera = |v: f64| (v - v.round()).abs() < 1e-9 && v >= 1.0;
    if !intera(c.larghezza) || !intera(c.altezza) {
        return Err(Errore::globale(format!(
            "per il video le dimensioni del palco devono essere intere (trovato {} × {})",
            c.larghezza, c.altezza
        )));
    }
    let troppo_grande = || {
        Errore::globale(format!(
            "il palco è troppo grande per il video (massimo {} pixel per lato e {} pixel in tutto)",
            limiti::MAX_LATO_VIDEO,
            limiti::MAX_PIXEL_VIDEO
        ))
    };
    if c.larghezza > f64::from(limiti::MAX_LATO_VIDEO) || c.altezza > f64::from(limiti::MAX_LATO_VIDEO) {
        return Err(troppo_grande());
    }
    let (larghezza, altezza) = (c.larghezza.round() as u32, c.altezza.round() as u32);
    let piano_dim = Piano { d_ms: 0.0, larghezza, altezza, fps: 1, n: 0 };
    let (w2, h2) = piano_dim.dimensioni_video();
    if w2 > limiti::MAX_LATO_VIDEO
        || h2 > limiti::MAX_LATO_VIDEO
        || u64::from(w2) * u64::from(h2) > limiti::MAX_PIXEL_VIDEO
    {
        return Err(troppo_grande());
    }
    let d_ms = (c.durata * 1000.0 * 10_000.0).round() / 10_000.0;
    let fps = fps_effettivo(fps_opzione, c.fps);
    let n = numero_fotogrammi(d_ms, fps);
    if n > limiti::MAX_FOTOGRAMMI_VIDEO {
        return Err(Errore::globale(format!(
            "il video avrebbe troppi fotogrammi: {n} (massimo {})",
            limiti::MAX_FOTOGRAMMI_VIDEO
        ))
        .con_suggerimento("riduci --fps o la durata della scena"));
    }
    Ok(Piano { d_ms, larghezza, altezza, fps, n })
}

/// Codifica percent-encoding dei byte fuori da `A-Z a-z 0-9 - . _ ~ /` (e dal `:` dell'unità).
fn codifica_url(s: &str, unita: bool) -> String {
    let mut o = String::with_capacity(s.len());
    for (i, b) in s.bytes().enumerate() {
        let lecito = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~' | b'/')
            || (unita && i == 1 && b == b':');
        if lecito {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// Trasforma il percorso assoluto (testo) in un URL `file://` (B.9.4).
///
/// # Errori
/// Percorso UNC (`\\server\…`), non utilizzabile.
pub fn url_file(percorso: &str) -> Risultato<String> {
    let unc = || {
        Errore::globale(format!("la cartella temporanea «{percorso}» non è utilizzabile: percorso di rete (UNC)"))
            .con_suggerimento("imposta TEMP/TMPDIR su una cartella locale")
    };
    let senza = match percorso.strip_prefix(r"\\?\") {
        Some(r) if r.starts_with("UNC\\") => return Err(unc()),
        Some(r) => r,
        None => percorso,
    };
    if senza.starts_with(r"\\") || senza.starts_with("//") {
        return Err(unc());
    }
    let barre = senza.replace('\\', "/");
    let b = barre.as_bytes();
    let unita = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/';
    if unita {
        Ok(format!("file:///{}", codifica_url(&barre, true)))
    } else {
        Ok(format!("file://{}", codifica_url(&barre, false)))
    }
}

/// Legge `(larghezza, altezza)` dall'intestazione `IHDR` di un PNG.
///
/// # Errori
/// Firma o intestazione non valide.
pub fn dimensioni_png(d: &[u8]) -> Result<(u32, u32), String> {
    if d.len() < 24 || d[..8] != [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A] {
        return Err("firma PNG non valida".into());
    }
    if &d[12..16] != b"IHDR" {
        return Err("intestazione IHDR mancante".into());
    }
    let n = |i: usize| u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]);
    Ok((n(16), n(20)))
}

/// Cartella temporanea di un render; il `Drop` la rimuove sempre (B.9.4).
struct CartellaTemporanea {
    percorso: PathBuf,
}

impl CartellaTemporanea {
    fn crea() -> Risultato<Self> {
        let base = std::env::temp_dir();
        let pid = std::process::id();
        let mut ultimo = String::new();
        for n in 0..100 {
            let p = base.join(format!("joypp-render-{pid}-{n}"));
            #[cfg(unix)]
            let b = {
                use std::os::unix::fs::DirBuilderExt;
                let mut b = fs::DirBuilder::new();
                b.mode(0o700);
                b
            };
            #[cfg(not(unix))]
            let b = fs::DirBuilder::new();
            match b.create(&p) {
                Ok(()) => return Ok(Self { percorso: p }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    ultimo = e.to_string();
                    break;
                }
            }
        }
        if ultimo.is_empty() {
            ultimo = "troppe cartelle esistenti".into();
        }
        Err(Errore::globale(format!(
            "la cartella temporanea «{}» non è utilizzabile: {ultimo}",
            base.display()
        ))
        .con_suggerimento("imposta TEMP/TMPDIR su una cartella locale"))
    }
}

impl Drop for CartellaTemporanea {
    fn drop(&mut self) {
        // Su Windows i processi figli del browser possono tenere i file ancora per un istante.
        let mut errore = None;
        for _ in 0..30 {
            match fs::remove_dir_all(&self.percorso) {
                Ok(()) => return,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
                Err(e) => errore = Some(e),
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if let Some(e) = errore {
            eprintln!(
                "Attenzione: impossibile rimuovere la cartella temporanea «{}»: {e}",
                self.percorso.display()
            );
        }
    }
}

/// File parziale di ffmpeg: rimosso al `Drop` se non è stato rinominato.
struct Parziale {
    percorso: PathBuf,
    attivo: bool,
}

impl Drop for Parziale {
    fn drop(&mut self) {
        if self.attivo {
            let _ = fs::remove_file(&self.percorso);
        }
    }
}

fn errore_scrittura(percorso: &Path, e: impl std::fmt::Display) -> Errore {
    Errore::globale(format!("impossibile scrivere «{}»: {e}", percorso.display()))
}

/// Traduce un errore CDP nel messaggio di B.9.7. `contesto` è per esempio
/// `comando «Runtime.evaluate», fotogramma 3/10`.
fn mappa_cdp(e: ErroreCdp, browser: &mut Browser, contesto: &str) -> Errore {
    match e {
        ErroreCdp::Timeout => Errore::globale(format!(
            "il browser non ha risposto entro 30 secondi ({contesto}){}",
            browser.righe_stderr()
        )),
        ErroreCdp::Browser(m) => Errore::globale(format!("il browser ha segnalato un errore: {m}")),
        ErroreCdp::Protocollo(m) => Errore::globale(format!("risposta del browser non valida: {m}")),
        ErroreCdp::TroppoGrande => Errore::globale("messaggio del browser troppo grande (massimo 256 MiB)"),
        ErroreCdp::Chiuso | ErroreCdp::Io(_) => {
            // Il processo può impiegare un istante a risultare terminato.
            for _ in 0..20 {
                if browser.uscito().is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if browser.uscito().is_some() {
                browser.errore_chiusura()
            } else {
                let dettaglio = match e {
                    ErroreCdp::Io(m) => m,
                    _ => "connessione chiusa".to_string(),
                };
                Errore::globale(format!("risposta del browser non valida: {dettaglio}"))
            }
        }
    }
}

/// Esegue un comando CDP con timeout standard, traducendo gli errori.
fn comando(
    s: &mut Sessione,
    b: &mut Browser,
    metodo: &str,
    params: &str,
    contesto: &str,
) -> Risultato<Valore> {
    s.comando(metodo, params, limiti::TIMEOUT_COMANDO_CDP)
        .map_err(|e| mappa_cdp(e, b, &format!("comando «{metodo}»{contesto}")))
}

fn errore_interno(m: &str) -> Errore {
    Errore::globale(format!("errore interno: {m}"))
        .con_suggerimento("è un bug di Joy++: segnalalo allegando il file .joy")
}

/// Apre la pagina e verifica che `__joy_seek` esista (B.9.5, passi 4-9).
fn prepara_pagina(s: &mut Sessione, b: &mut Browser, url: &str, p: &Piano) -> Risultato<()> {
    let r = comando(s, b, "Target.createTarget", "{\"url\":\"about:blank\"}", "")?;
    let target = r
        .campo("targetId")
        .and_then(Valore::testo)
        .ok_or_else(|| Errore::globale("risposta del browser non valida: targetId mancante"))?
        .to_string();
    let r = comando(
        s,
        b,
        "Target.attachToTarget",
        &format!("{{\"targetId\":{},\"flatten\":true}}", json::stringa(&target)),
        "",
    )?;
    let sessione = r
        .campo("sessionId")
        .and_then(Valore::testo)
        .ok_or_else(|| Errore::globale("risposta del browser non valida: sessionId mancante"))?
        .to_string();
    s.imposta_sessione(sessione);
    comando(
        s,
        b,
        "Emulation.setDeviceMetricsOverride",
        &format!(
            "{{\"width\":{},\"height\":{},\"deviceScaleFactor\":1,\"mobile\":false}}",
            p.larghezza, p.altezza
        ),
        "",
    )?;
    comando(
        s,
        b,
        "Emulation.setDefaultBackgroundColorOverride",
        "{\"color\":{\"r\":0,\"g\":0,\"b\":0,\"a\":1}}",
        "",
    )?;
    comando(s, b, "Page.enable", "{}", "")?;
    let r = comando(s, b, "Page.navigate", &format!("{{\"url\":{}}}", json::stringa(url)), "")?;
    if let Some(t) = r.campo("errorText").and_then(Valore::testo) {
        return Err(Errore::globale(format!("il browser ha segnalato un errore: {t}")));
    }
    s.attendi_evento("Page.loadEventFired", limiti::TIMEOUT_COMANDO_CDP)
        .map_err(|e| mappa_cdp(e, b, "attesa del caricamento della pagina"))?;
    s.svuota_eventi();
    let r = comando(
        s,
        b,
        "Runtime.evaluate",
        "{\"expression\":\"typeof window.__joy_seek\",\"returnByValue\":true}",
        "",
    )?;
    match cdp::valore_valutato(&r) {
        Ok(v) if v.testo() == Some("function") => Ok(()),
        _ => Err(errore_interno("la pagina generata non espone __joy_seek")),
    }
}

/// Campioni grezzi della diagnostica (C.1.3), raccolti solo con `--profilo`: durate per
/// fotogramma in microsecondi e byte. Con il profilo spento non vengono mai creati.
#[derive(Debug, Default)]
struct Campioni {
    /// Andata e ritorno di `Runtime.evaluate` con `__joy_seek`, in µs.
    seek: Vec<u64>,
    /// Andata e ritorno di `Page.captureScreenshot`, in µs.
    screenshot: Vec<u64>,
    /// Decodifica base64, in µs.
    base64: Vec<u64>,
    /// Verifica delle dimensioni PNG, in µs.
    verifica_png: Vec<u64>,
    /// Tempo dentro `Ffmpeg::invia`, in µs.
    attesa_coda: Vec<u64>,
    /// Byte PNG totali.
    byte_png_totali: u64,
    /// Byte del PNG più grande.
    byte_png_max: u64,
    /// Byte base64 totali ricevuti dal browser.
    byte_base64_totali: u64,
}

/// Durata in microsecondi, saturata a `u64::MAX`.
fn micros(d: Duration) -> u64 {
    u64::try_from(d.as_micros()).unwrap_or(u64::MAX)
}

/// Durata in millisecondi.
fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Registra una durata nel campo indicato, solo se il profilo è attivo.
fn registra(
    campioni: &mut Option<Campioni>,
    campo: impl FnOnce(&mut Campioni) -> &mut Vec<u64>,
    d: Duration,
) {
    if let Some(c) = campioni.as_mut() {
        campo(c).push(micros(d));
    }
}

/// Frequenza `numeratore / secondi`, con `secondi = durata_ms / 1000`. `None` se il
/// denominatore non è positivo o il risultato non è finito (C.1.3, «Prestazioni»).
fn quoziente(numeratore: f64, durata_ms: f64) -> Option<f64> {
    let secondi = durata_ms / 1000.0;
    if secondi <= 0.0 {
        return None;
    }
    let risultato = numeratore / secondi;
    risultato.is_finite().then_some(risultato)
}

/// Picco di memoria residente di joypp (`VmHWM`), in byte. Solo Linux: altrove `None`.
#[cfg(target_os = "linux")]
fn picco_processo() -> Option<u64> {
    let stato = fs::read_to_string("/proc/self/status").ok()?;
    let riga = stato.lines().find(|r| r.starts_with("VmHWM:"))?;
    let kb: u64 = riga.split_whitespace().nth(1)?.parse().ok()?;
    kb.checked_mul(1024)
}

/// Picco di memoria residente: non misurabile senza `unsafe` o dipendenze (C.1.3).
#[cfg(not(target_os = "linux"))]
fn picco_processo() -> Option<u64> {
    None
}

/// Versione del browser (`product` di `Browser.getVersion`). Un errore non interrompe il render:
/// il valore è `None` (C.1.2).
fn leggi_versione_browser(s: &mut Sessione) -> Option<String> {
    s.comando_globale("Browser.getVersion", "{}", limiti::TIMEOUT_COMANDO_CDP)
        .ok()
        .and_then(|r| r.campo("product").and_then(Valore::testo).map(ripulisci_riga))
}

/// Assembla il rapporto di diagnostica a render riuscito (C.1.3).
fn costruisci_profilo(
    c: Campioni,
    fasi: Fasi,
    scena: MisureScena,
    video: MisureVideo,
    tempi_ffmpeg: TempiFfmpeg,
    versione_browser: Option<String>,
    versione_ffmpeg: Option<String>,
) -> Profilo {
    let picco = picco_processo();
    let n = video.fotogrammi as f64;
    let secondi_video = n / f64::from(video.fps);
    let prestazioni = Prestazioni {
        fps_ciclo: quoziente(n, fasi.ciclo_fotogrammi),
        fps_totale: quoziente(n, fasi.totale),
        rapporto_tempo_reale: quoziente(secondi_video, fasi.totale),
    };
    let stima = (ffmpeg::CODA_FOTOGRAMMI as u64 + 2).saturating_mul(c.byte_png_max);
    Profilo {
        schema: profilo::SCHEMA.to_string(),
        ambiente: Ambiente {
            joypp: env!("CARGO_PKG_VERSION").to_string(),
            build: if cfg!(debug_assertions) { "debug" } else { "release" }.to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            thread_logici: std::thread::available_parallelism().ok().map(std::num::NonZeroUsize::get),
            browser: versione_browser,
            ffmpeg: versione_ffmpeg,
        },
        scena,
        video,
        fasi_ms: fasi,
        fotogramma_ms: PerFotogramma {
            seek: Statistica::da_microsecondi(&c.seek),
            screenshot: Statistica::da_microsecondi(&c.screenshot),
            base64: Statistica::da_microsecondi(&c.base64),
            verifica_png: Statistica::da_microsecondi(&c.verifica_png),
            attesa_coda: Statistica::da_microsecondi(&c.attesa_coda),
        },
        ffmpeg: MisureFfmpeg {
            scrittura_pipe_ms: ms(tempi_ffmpeg.scrittura_pipe),
            coda_capacita: ffmpeg::CODA_FOTOGRAMMI,
            fotogrammi_con_coda_piena: tempi_ffmpeg.coda_piena,
        },
        dati: Dati {
            byte_png_totali: c.byte_png_totali,
            byte_png_max: c.byte_png_max,
            byte_base64_totali: c.byte_base64_totali,
        },
        prestazioni,
        memoria: Memoria {
            picco_processo_byte: picco,
            picco_processo_fonte: if picco.is_some() { "VmHWM" } else { "non disponibile" }.to_string(),
            stima_buffer_fotogrammi_byte: stima,
        },
    }
}

/// Cattura il fotogramma `k` (posizionamento con `__joy_seek` + screenshot PNG). Con il profilo
/// attivo registra le durate di `seek`, `screenshot`, `base64` e `verifica_png` (C.1.3).
fn cattura(
    s: &mut Sessione,
    b: &mut Browser,
    p: &Piano,
    k: u64,
    campioni: &mut Option<Campioni>,
) -> Risultato<Vec<u8>> {
    let t = istante(k, p.fps, p.d_ms);
    let ctx = format!(", fotogramma {}/{}", k + 1, p.n);
    let espressione = format!("window.__joy_seek({t})");
    let inizio = Instant::now();
    let r = comando(
        s,
        b,
        "Runtime.evaluate",
        &format!(
            "{{\"expression\":{},\"awaitPromise\":true,\"returnByValue\":true}}",
            json::stringa(&espressione)
        ),
        &ctx,
    )?;
    let v = cdp::valore_valutato(&r).map_err(|m| errore_interno(&format!("__joy_seek ha fallito: {m}")))?;
    match v.numero() {
        Some(x) if (x - t).abs() <= 1e-6 => {}
        _ => return Err(errore_interno("__joy_seek ha restituito un istante diverso da quello richiesto")),
    }
    registra(campioni, |c| &mut c.seek, inizio.elapsed());

    let inizio = Instant::now();
    let r = comando(
        s,
        b,
        "Page.captureScreenshot",
        PARAMETRI_SCREENSHOT,
        &ctx,
    )?;
    registra(campioni, |c| &mut c.screenshot, inizio.elapsed());
    let dati = r
        .campo("data")
        .and_then(Valore::testo)
        .ok_or_else(|| Errore::globale("risposta del browser non valida: screenshot senza dati"))?;

    let inizio = Instant::now();
    let png = base64::decodifica(dati)
        .map_err(|m| Errore::globale(format!("risposta del browser non valida: {m}")))?;
    registra(campioni, |c| &mut c.base64, inizio.elapsed());
    if let Some(c) = campioni.as_mut() {
        c.byte_base64_totali += dati.len() as u64;
    }

    let inizio = Instant::now();
    let (w, h) = dimensioni_png(&png).map_err(|m| Errore::globale(format!("risposta del browser non valida: {m}")))?;
    if (w, h) != (p.larghezza, p.altezza) {
        return Err(Errore::globale(format!(
            "lo screenshot è {w}×{h} invece di {}×{}",
            p.larghezza, p.altezza
        ))
        .con_suggerimento(
            "il browser ridimensiona la pagina: segnala il problema indicando browser e sistema operativo",
        ));
    }
    registra(campioni, |c| &mut c.verifica_png, inizio.elapsed());
    if let Some(c) = campioni.as_mut() {
        let n = png.len() as u64;
        c.byte_png_totali += n;
        c.byte_png_max = c.byte_png_max.max(n);
    }
    Ok(png)
}

/// Esporta `ingresso` in un MP4 `uscita` (B.9). `progresso` riceve avvisi e avanzamento.
///
/// Con `opzioni.profilo` il rapporto di diagnostica (C.1) è in [`EsitoVideo::profilo`]; le misure
/// non cambiano il video né l'ordine dei fotogrammi.
///
/// Su ogni errore browser, ffmpeg, file parziale e cartella temporanea vengono ripuliti e un
/// eventuale `uscita` preesistente resta intatta.
///
/// # Errori
/// Tutti quelli elencati in B.9.7.
pub fn render_file(
    ingresso: &Path,
    uscita: &Path,
    opzioni: &OpzioniVideo,
    progresso: &mut dyn FnMut(Evento),
) -> Risultato<EsitoVideo> {
    let inizio_totale = Instant::now();
    let mut fasi = Fasi::default();
    let mut campioni: Option<Campioni> = opzioni.profilo.then(Campioni::default);

    let t = Instant::now();
    let sorgente = leggi_sorgente(ingresso, uscita, "-o video.mp4")?;
    fasi.lettura = ms(t.elapsed());
    let (compilazione, sfondo, misure_compilazione) =
        compila_misurando(&sorgente, &Opzioni { leggibile: false, senza_player: true })?;
    fasi.analisi = ms(misure_compilazione.analisi);
    fasi.compilazione = ms(misure_compilazione.compilazione);
    fasi.generazione_html = ms(misure_compilazione.generazione_html);
    for a in &compilazione.avvisi {
        progresso(Evento::Avviso(a.to_string()));
    }
    let piano = pianifica(&compilazione, opzioni.fps)?;
    let (w2, h2) = piano.dimensioni_video();
    if (w2, h2) != (piano.larghezza, piano.altezza) {
        progresso(Evento::Avviso(format!(
            "Attenzione: il palco {}×{} ha dimensioni dispari: il video sarà {w2}×{h2} (aggiunto un bordo a destra/in basso del colore dello sfondo)",
            piano.larghezza, piano.altezza
        )));
    }

    let inizio_preparazione = Instant::now();
    let var = |k: &str| std::env::var(k).ok();
    let esiste = |p: &Path| p.is_file();
    let sistema = Sistema::corrente();
    let exe_ffmpeg = ffmpeg::cerca_ffmpeg(sistema, &var, &esiste)?;
    let versione_ffmpeg = ffmpeg::verifica(&exe_ffmpeg, opzioni.profilo)?;
    let exe_browser = browser::cerca_browser(sistema, &var, &esiste)?;

    // Ordine di dichiarazione = inverso dell'ordine di rimozione: prima ffmpeg e browser,
    // poi il file parziale, infine la cartella temporanea (B.9.4).
    let cartella = CartellaTemporanea::crea()?;
    let file_scena = cartella.percorso.join("scena.html");
    let profilo = cartella.percorso.join("profilo");
    fs::write(&file_scena, &compilazione.html).map_err(|e| errore_scrittura(&file_scena, e))?;
    fs::create_dir(&profilo).map_err(|e| errore_scrittura(&profilo, e))?;
    let canonico = fs::canonicalize(&file_scena).map_err(|e| errore_scrittura(&file_scena, e))?;
    let url = url_file(&canonico.to_string_lossy())?;

    let assoluta = std::path::absolute(uscita).map_err(|e| errore_scrittura(uscita, e))?;
    if let Some(dir) = assoluta.parent() {
        fs::create_dir_all(dir).map_err(|e| {
            Errore::globale(format!("impossibile creare la cartella «{}»: {e}", dir.display()))
        })?;
    }
    let mut nome_parziale = assoluta.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    nome_parziale.push(".joypp-parziale");
    let mut parziale = Parziale { percorso: assoluta.with_file_name(nome_parziale), attivo: true };
    fasi.preparazione = ms(inizio_preparazione.elapsed());

    let t = Instant::now();
    let mut browser = Browser::avvia(&exe_browser, &profilo, piano.larghezza, piano.altezza)?;
    let (porta, percorso_ws) = browser.attendi_porta(&profilo, limiti::TIMEOUT_AVVIO_BROWSER)?;
    fasi.avvio_browser = ms(t.elapsed());

    let t = Instant::now();
    let mut sessione = Sessione::connetti(porta, &percorso_ws, limiti::TIMEOUT_COMANDO_CDP)
        .map_err(|e| mappa_cdp(e, &mut browser, "connessione"))?;
    fasi.connessione_cdp = ms(t.elapsed());

    // Solo con profilo: la versione del browser non rientra in `connessione_cdp` né in
    // `caricamento_pagina`.
    let versione_browser = if opzioni.profilo { leggi_versione_browser(&mut sessione) } else { None };

    let t = Instant::now();
    prepara_pagina(&mut sessione, &mut browser, &url, &piano)?;
    fasi.caricamento_pagina = ms(t.elapsed());

    let argomenti = ffmpeg::argomenti_ffmpeg(
        piano.larghezza,
        piano.altezza,
        piano.fps,
        opzioni.crf,
        sfondo_su_nero(sfondo),
        &parziale.percorso,
    );
    let t = Instant::now();
    let mut codificatore = ffmpeg::Ffmpeg::avvia(&exe_ffmpeg, &argomenti)?;
    fasi.avvio_ffmpeg = ms(t.elapsed());

    progresso(Evento::Fotogramma { fatti: 0, totale: piano.n });
    let t = Instant::now();
    let mut ffmpeg_chiuso = false;
    for k in 0..piano.n {
        let png = cattura(&mut sessione, &mut browser, &piano, k, &mut campioni)?;
        let inizio_coda = Instant::now();
        let accettato = codificatore.invia(png);
        registra(&mut campioni, |c| &mut c.attesa_coda, inizio_coda.elapsed());
        if !accettato {
            ffmpeg_chiuso = true;
            break;
        }
        progresso(Evento::Fotogramma { fatti: k + 1, totale: piano.n });
    }
    fasi.ciclo_fotogrammi = ms(t.elapsed());

    // Chiusura ordinata del browser prima della fine di ffmpeg.
    let t = Instant::now();
    let _ = sessione.comando_globale("Browser.close", "{}", Duration::from_secs(2));
    browser.chiudi(limiti::TIMEOUT_CHIUSURA);
    fasi.chiusura_browser = ms(t.elapsed());

    let t = Instant::now();
    let tempi_ffmpeg = codificatore.finisci()?;
    fasi.finalizzazione_ffmpeg = ms(t.elapsed());
    if ffmpeg_chiuso {
        return Err(Errore::globale("ffmpeg ha chiuso la pipe prima della fine"));
    }

    let t = Instant::now();
    fs::rename(&parziale.percorso, &assoluta).map_err(|e| errore_scrittura(&assoluta, e))?;
    parziale.attivo = false;
    let byte = fs::metadata(&assoluta).map(|m| m.len()).map_err(|e| errore_scrittura(&assoluta, e))?;
    fasi.pubblicazione = ms(t.elapsed());
    fasi.totale = ms(inizio_totale.elapsed());

    let video = MisureVideo {
        fotogrammi: piano.n,
        larghezza: w2,
        altezza: h2,
        fps: piano.fps,
        crf: opzioni.crf,
        preset: "medium".to_string(),
        byte,
    };
    let profilo = campioni.map(|c| {
        let scena = MisureScena {
            attori: misure_compilazione.attori,
            gruppi: misure_compilazione.gruppi,
            animazioni: misure_compilazione.animazioni,
            byte_html: compilazione.html.len() as u64,
            durata_ms: compilazione.durata * 1000.0,
        };
        costruisci_profilo(c, fasi, scena, video, tempi_ffmpeg, versione_browser, versione_ffmpeg)
    });
    Ok(EsitoVideo {
        fotogrammi: piano.n,
        larghezza: w2,
        altezza: h2,
        fps: piano.fps,
        byte,
        profilo,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    fn compilazione(durata: f64, l: f64, a: f64, fps: Option<u32>) -> Compilazione {
        Compilazione { html: String::new(), avvisi: vec![], durata, larghezza: l, altezza: a, fps }
    }

    #[test]
    fn precedenza_fps() {
        assert_eq!(fps_effettivo(None, None), 60);
        assert_eq!(fps_effettivo(None, Some(30)), 30);
        assert_eq!(fps_effettivo(Some(60), Some(30)), 60);
        assert_eq!(fps_effettivo(Some(24), None), 24);
    }

    #[test]
    fn tabella_di_campionamento() {
        assert_eq!(numero_fotogrammi(8500.0, 60), 511);
        assert!((istante(509, 60, 8500.0) - 8483.333333).abs() < 1e-5);
        assert_eq!(istante(510, 60, 8500.0), 8500.0);
        assert_eq!(numero_fotogrammi(1000.0, 30), 31);
        assert_eq!(istante(30, 30, 1000.0), 1000.0);
        assert_eq!(numero_fotogrammi(1010.0, 60), 62);
        assert_eq!(istante(60, 60, 1010.0), 1000.0);
        assert_eq!(istante(61, 60, 1010.0), 1010.0);
        assert_eq!(numero_fotogrammi(300.0, 1), 2);
        assert_eq!(istante(0, 1, 300.0), 0.0);
        assert_eq!(istante(1, 1, 300.0), 300.0);
        assert_eq!(numero_fotogrammi(500.0, 10), 6);
    }

    #[test]
    fn ultimo_istante_e_d_e_istanti_crescenti() {
        for (d, fps) in [(8500.0, 60), (1010.0, 60), (333.3333, 24), (3_600_000.0, 60), (1.0, 240)] {
            let n = numero_fotogrammi(d, fps);
            assert_eq!(istante(n - 1, fps, d), d, "d={d} fps={fps}");
            let mut prec = -1.0;
            for k in 0..n.min(5000) {
                let t = istante(k, fps, d);
                assert!(t > prec, "non crescente a k={k}");
                prec = t;
            }
        }
    }

    #[test]
    fn pianificazione_ok_e_default_dall_intestazione() {
        let p = pianifica(&compilazione(8.5, 1280.0, 720.0, Some(30)), None).unwrap();
        assert_eq!((p.fps, p.n, p.d_ms), (30, 256, 8500.0));
        let p = pianifica(&compilazione(0.5, 64.0, 36.0, None), Some(10)).unwrap();
        assert_eq!((p.fps, p.n), (10, 6));
        assert_eq!(p.dimensioni_video(), (64, 36));
        let p = pianifica(&compilazione(1.0, 63.0, 35.0, None), None).unwrap();
        assert_eq!(p.dimensioni_video(), (64, 36));
    }

    #[test]
    fn errori_di_pianificazione() {
        let e = pianifica(&compilazione(0.0, 64.0, 36.0, None), None).unwrap_err();
        assert_eq!(e.messaggio, "la scena non ha una timeline: il video sarebbe un solo fotogramma");
        let e = pianifica(&compilazione(1.0, 800.5, 600.0, None), None).unwrap_err();
        assert_eq!(e.messaggio, "per il video le dimensioni del palco devono essere intere (trovato 800.5 × 600)");
        let e = pianifica(&compilazione(1.0, 8193.0, 100.0, None), None).unwrap_err();
        assert!(e.messaggio.starts_with("il palco è troppo grande per il video"));
        let e = pianifica(&compilazione(1.0, 8191.0, 8191.0, None), None).unwrap_err();
        assert!(e.messaggio.contains("35389440 pixel"));
        let e = pianifica(&compilazione(3600.0, 100.0, 100.0, None), Some(61)).unwrap_err();
        assert_eq!(e.messaggio, "il video avrebbe troppi fotogrammi: 219601 (massimo 216001)");
        assert!(pianifica(&compilazione(3600.0, 100.0, 100.0, None), None).is_ok());
    }

    #[test]
    fn sfondo_composto_su_nero() {
        let c = |r, g, b, a| Colore { r, g, b, a };
        assert_eq!(sfondo_su_nero(c(255, 0, 0, 0.5)), [0x80, 0, 0]);
        assert_eq!(sfondo_su_nero(c(0x0f, 0x17, 0x2a, 1.0)), [0x0f, 0x17, 0x2a]);
        assert_eq!(sfondo_su_nero(c(200, 200, 200, 0.0)), [0, 0, 0]);
    }

    #[test]
    fn url_windows() {
        assert_eq!(
            url_file(r"C:\Users\Mario Rossi\AppData\Local\Temp\joypp-render-1-0\scena.html").unwrap(),
            "file:///C:/Users/Mario%20Rossi/AppData/Local/Temp/joypp-render-1-0/scena.html"
        );
        assert_eq!(
            url_file(r"\\?\C:\Temp\scena.html").unwrap(),
            "file:///C:/Temp/scena.html"
        );
    }

    #[test]
    fn url_non_ascii_e_unix() {
        assert_eq!(url_file("/tmp/è #1/scena.html").unwrap(), "file:///tmp/%C3%A8%20%231/scena.html");
        assert_eq!(url_file(r"C:\Utenti\Zoë\s.html").unwrap(), "file:///C:/Utenti/Zo%C3%AB/s.html");
    }

    #[test]
    fn url_rifiuta_unc() {
        assert!(url_file(r"\\server\share\scena.html").is_err());
        assert!(url_file(r"\\?\UNC\server\share\scena.html").is_err());
    }

    #[test]
    fn intestazione_png() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&64u32.to_be_bytes());
        png.extend_from_slice(&36u32.to_be_bytes());
        png.extend_from_slice(&[8, 6, 0, 0, 0]);
        assert_eq!(dimensioni_png(&png), Ok((64, 36)));
        assert!(dimensioni_png(&png[..20]).is_err());
        let mut sbagliato = png.clone();
        sbagliato[0] = 0;
        assert!(dimensioni_png(&sbagliato).is_err());
        let mut senza_ihdr = png;
        senza_ihdr[12] = b'X';
        assert!(dimensioni_png(&senza_ihdr).is_err());
    }

    #[test]
    fn parametri_screenshot_fissi() {
        assert_eq!(
            PARAMETRI_SCREENSHOT,
            "{\"format\":\"png\",\"fromSurface\":true,\"captureBeyondViewport\":false,\"optimizeForSpeed\":true}"
        );
    }

    #[test]
    fn prestazioni_con_denominatore_non_positivo_sono_assenti() {
        assert_eq!(quoziente(10.0, 1000.0), Some(10.0));
        assert_eq!(quoziente(10.0, 0.0), None);
        assert_eq!(quoziente(10.0, -5.0), None);
        assert_eq!(quoziente(f64::INFINITY, 1000.0), None);
    }

    #[test]
    fn durata_video_dell_esito() {
        let e = EsitoVideo { fotogrammi: 511, larghezza: 2, altezza: 2, fps: 60, byte: 1, profilo: None };
        assert!((e.durata_video() - 8.5166).abs() < 1e-3);
    }
}
