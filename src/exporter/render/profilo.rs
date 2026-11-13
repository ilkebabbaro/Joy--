//! Diagnostica del render (`joypp render --profilo` / `--profilo-json`, schema `joypp-profilo/1`,
//! contratto C.1.3-C.1.6): tipi dei dati misurati.
//!
//! In questo modulo ci sono solo i tipi. Le misure sono riempite da [`super::render_file`] e la
//! formattazione (testo e JSON) arriverà nel passo successivo.

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
