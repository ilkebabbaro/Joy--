//! Esperimento isolato sulla cattura degli screenshot (ciclo v0.2.2, Fase 3).
//!
//! NON fa parte della suite normale: ogni test è `#[ignore]`. Esecuzione:
//!
//! ```text
//! cargo test --release --test esperimento_cattura -- --ignored --nocapture --test-threads=1
//! ```
//!
//! La cartella di lavoro si legge da `JOYPP_ESPERIMENTO_DIR`, per esempio
//! `C:\Users\xhoni\AppData\Local\Temp\claude\b2eb6e1a7a6a7f6e\2a22d48b-fdb6-4b78-b9cf-0ed1238f7383\scratchpad\cattura`.
//! Se la variabile manca, o mancano browser o ffmpeg, il test stampa `esperimento saltato: <motivo>`
//! e termina con successo. Tutti i file generati (PNG, raw, diff, CSV) vanno SOLO lì.
//!
//! Il client CDP strumentato riproduce i passi 4-9 di SPECIFICA B.9.5 con gli stessi argomenti del
//! browser (`Browser::avvia`), la stessa viewport e lo stesso JS (`__joy_seek`, invariato). Per ogni
//! screenshot misura, separatamente e in microsecondi: invio, primo byte, ricezione, JSON, copie,
//! base64. Il seek è misurato come in produzione (andata e ritorno di `Runtime.evaluate`).
//!
//! Variabili opzionali: `JOYPP_ESPERIMENTO_RIPETIZIONI` (default 3), `JOYPP_ESPERIMENTO_SCENE`
//! e `JOYPP_ESPERIMENTO_VARIANTI` (elenchi separati da virgola, filtri).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use joypp::exporter::render::browser::{Browser, Sistema, cerca_browser};
use joypp::exporter::render::ffmpeg::cerca_ffmpeg;
use joypp::exporter::render::websocket::{casuale, codifica_frame, handshake, leggi_frame};
use joypp::exporter::render::{base64, dimensioni_png, json, url_file};
use joypp::exporter::{Opzioni, compila_con_avvisi};

const OP_CONTINUA: u8 = 0x0;
const OP_TESTO: u8 = 0x1;
const OP_CHIUDI: u8 = 0x8;
const OP_PING: u8 = 0x9;
const OP_PONG: u8 = 0xA;
const TIMEOUT: Duration = Duration::from_secs(30);
const WARMUP: usize = 5;

fn us(d: Duration) -> u64 {
    u64::try_from(d.as_micros()).unwrap_or(u64::MAX)
}

// ------------------------------------------------------------------ client CDP strumentato

/// Messaggio ricevuto, con gli istanti del primo byte e del completamento.
struct Msg {
    testo: String,
    primo: Instant,
    fine: Instant,
}

/// Campioni di un singolo fotogramma, in µs e byte.
#[derive(Default, Clone, Debug)]
struct Camp {
    seek: u64,
    invio: u64,
    primo: u64,
    ric: u64,
    json: u64,
    copie: u64,
    b64: u64,
    b_testo: u64,
    b_png: u64,
    ev: u64,
}

/// Client CDP minimale con un proprio `TcpStream`.
struct Client {
    s: TcpStream,
    id: u64,
    sess: Option<String>,
    eventi: u64,
    caricato: bool,
}

impl Client {
    fn connetti(porta: u16, percorso: &str) -> Result<Self, String> {
        let ind = SocketAddr::from((Ipv4Addr::LOCALHOST, porta));
        let mut s = TcpStream::connect_timeout(&ind, TIMEOUT).map_err(|e| e.to_string())?;
        s.set_read_timeout(Some(TIMEOUT)).map_err(|e| e.to_string())?;
        s.set_write_timeout(Some(TIMEOUT)).map_err(|e| e.to_string())?;
        let _ = s.set_nodelay(true);
        handshake(&mut s, &format!("127.0.0.1:{porta}"), percorso).map_err(|e| format!("{e:?}"))?;
        Ok(Self { s, id: 1, sess: None, eventi: 0, caricato: false })
    }

    fn timeout(&self, d: Duration) {
        let _ = self.s.set_read_timeout(Some(d));
    }

    /// Serializza e scrive il comando; restituisce l'id.
    fn invia(&mut self, metodo: &str, params: &str, globale: bool) -> Result<u64, String> {
        let id = self.id;
        self.id += 1;
        let sess = match (&self.sess, globale) {
            (Some(s), false) => format!(",\"sessionId\":{}", json::stringa(s)),
            _ => String::new(),
        };
        let msg = format!("{{\"id\":{id},\"method\":{},\"params\":{params}{sess}}}", json::stringa(metodo));
        let m = casuale().to_le_bytes();
        let frame = codifica_frame(OP_TESTO, msg.as_bytes(), [m[0], m[1], m[2], m[3]]);
        self.s.write_all(&frame).map_err(|e| e.to_string())?;
        self.s.flush().map_err(|e| e.to_string())?;
        Ok(id)
    }

    /// Legge il prossimo messaggio di testo completo (come `Connessione::ricevi`), con `peek`
    /// bloccante per fissare l'istante del primo byte.
    fn leggi_messaggio(&mut self) -> Result<Msg, String> {
        let mut b = [0u8; 1];
        let n = self.s.peek(&mut b).map_err(|e| format!("peek: {e}"))?;
        if n == 0 {
            return Err("connessione chiusa".into());
        }
        let primo = Instant::now();
        let mut acc: Vec<u8> = Vec::new();
        let mut in_corso = false;
        loop {
            let f = leggi_frame(&mut self.s).map_err(|e| format!("{e:?}"))?;
            match f.opcode {
                OP_PING => {
                    let m = casuale().to_le_bytes();
                    let pong = codifica_frame(OP_PONG, &f.dati, [m[0], m[1], m[2], m[3]]);
                    self.s.write_all(&pong).map_err(|e| e.to_string())?;
                }
                OP_PONG => {}
                OP_CHIUDI => return Err("close dal server".into()),
                OP_TESTO | OP_CONTINUA => {
                    if (f.opcode == OP_TESTO) == in_corso {
                        return Err("frammenti non validi".into());
                    }
                    acc.extend_from_slice(&f.dati);
                    in_corso = true;
                    if f.fin {
                        let fine = Instant::now();
                        let testo = String::from_utf8(acc).map_err(|_| "non UTF-8".to_string())?;
                        return Ok(Msg { testo, primo, fine });
                    }
                }
                o => return Err(format!("opcode {o}")),
            }
        }
    }

    fn classifica_evento(&mut self, testo: &str) {
        self.eventi += 1;
        let testa: String = testo.chars().take(80).collect();
        if testa.contains("\"Page.loadEventFired\"") {
            self.caricato = true;
        }
    }

    /// Attende la risposta con l'`id` dato; gli eventi vengono contati e ignorati.
    fn attendi(&mut self, id: u64) -> Result<Msg, String> {
        let prefisso = format!("{{\"id\":{id},");
        loop {
            let m = self.leggi_messaggio()?;
            if m.testo.starts_with(&prefisso) {
                return Ok(m);
            }
            self.classifica_evento(&m.testo.clone());
        }
    }

    /// Comando generico non cronometrato: restituisce `result` (clonato).
    fn comando(&mut self, metodo: &str, params: &str, globale: bool) -> Result<json::Valore, String> {
        let id = self.invia(metodo, params, globale)?;
        let m = self.attendi(id)?;
        let v = json::analizza(&m.testo)?;
        if let Some(e) = v.campo("error") {
            return Err(format!("{metodo}: {}", e.campo("message").and_then(json::Valore::testo).unwrap_or("errore")));
        }
        Ok(v.campo("result").cloned().unwrap_or(json::Valore::Null))
    }

    fn attendi_caricamento(&mut self) -> Result<(), String> {
        while !self.caricato {
            let m = self.leggi_messaggio()?;
            self.classifica_evento(&m.testo.clone());
        }
        Ok(())
    }

    /// `Runtime.evaluate` con `awaitPromise` (come `__joy_seek` in produzione); durata in µs.
    fn valuta(&mut self, espressione: &str) -> Result<(u64, json::Valore), String> {
        let t0 = Instant::now();
        let id = self.invia(
            "Runtime.evaluate",
            &format!("{{\"expression\":{},\"awaitPromise\":true,\"returnByValue\":true}}", json::stringa(espressione)),
            false,
        )?;
        let m = self.attendi(id)?;
        let v = json::analizza(&m.testo)?;
        let d = us(t0.elapsed());
        if let Some(e) = v.campo("error") {
            return Err(format!("Runtime.evaluate: {}", e.campo("message").and_then(json::Valore::testo).unwrap_or("?")));
        }
        let r = v.campo("result").cloned().unwrap_or(json::Valore::Null);
        if let Some(e) = r.campo("exceptionDetails") {
            return Err(format!("eccezione JS: {e:?}"));
        }
        let val = r.campo("result").and_then(|x| x.campo("value")).cloned().unwrap_or(json::Valore::Null);
        Ok((d, val))
    }

    /// `__joy_seek(t)`: restituisce la durata in µs, verificando l'istante restituito.
    fn seek(&mut self, t: f64) -> Result<u64, String> {
        let (d, v) = self.valuta(&format!("window.__joy_seek({t})"))?;
        match v.numero() {
            Some(x) if (x - t).abs() <= 1e-6 => Ok(d),
            _ => Err(format!("seek ha restituito {v:?} invece di {t}")),
        }
    }

    /// Screenshot con i parametri dati, scomponendo la latenza.
    fn screenshot(&mut self, params: &str) -> Result<(Camp, Vec<u8>), String> {
        let ev0 = self.eventi;
        let t0 = Instant::now();
        let id = self.invia("Page.captureScreenshot", params, false)?;
        let inviato = Instant::now();
        let mut c = Camp { invio: us(inviato - t0), ..Camp::default() };
        let m = self.attendi(id)?;
        c.primo = us(m.primo.saturating_duration_since(inviato));
        c.ric = us(m.fine.saturating_duration_since(m.primo));
        c.ev = self.eventi - ev0;

        let t = Instant::now();
        let v = json::analizza(&m.testo)?;
        c.json = us(t.elapsed());
        if let Some(e) = v.campo("error") {
            return Err(format!("captureScreenshot: {}", e.campo("message").and_then(json::Valore::testo).unwrap_or("errore")));
        }
        // Come `Sessione::comando` in produzione: `result` viene clonato (copia dell'intero base64).
        let t = Instant::now();
        let r = v.campo("result").cloned().ok_or("risposta senza result")?;
        c.copie = us(t.elapsed());
        let dati = r.campo("data").and_then(json::Valore::testo).ok_or("screenshot senza dati")?;
        c.b_testo = dati.len() as u64;
        let t = Instant::now();
        let png = base64::decodifica(dati)?;
        c.b64 = us(t.elapsed());
        c.b_png = png.len() as u64;
        // Rilascio delle strutture temporanee (anch'esso una copia/allocazione che produzione paga).
        let t = Instant::now();
        drop(r);
        drop(v);
        drop(m);
        c.copie += us(t.elapsed());
        Ok((c, png))
    }
}

// ------------------------------------------------------------------ scene, varianti, run

#[derive(Clone)]
struct Scena {
    nome: String,
    w: u32,
    h: u32,
    d_ms: f64,
    url: Option<String>, // None = about:blank
    m: usize,
}

const P_PNG: &str = r#"{"format":"png","fromSurface":true,"captureBeyondViewport":false}"#;
const P_V1: &str = r#"{"format":"png","fromSurface":true,"captureBeyondViewport":false,"optimizeForSpeed":true}"#;
const P_JPEG: &str = r#"{"format":"jpeg","quality":100,"fromSurface":true,"captureBeyondViewport":false}"#;
const P_WEBP: &str = r#"{"format":"webp","quality":100,"fromSurface":true,"captureBeyondViewport":false}"#;

fn parametri(var: &str) -> &'static str {
    match var {
        "V1" => P_V1,
        "JPEG" => P_JPEG,
        "WEBP" => P_WEBP,
        _ => P_PNG,
    }
}

fn estensione(var: &str) -> &'static str {
    match var {
        "JPEG" => "jpg",
        "WEBP" => "webp",
        _ => "png",
    }
}

struct Ctx {
    dir: PathBuf,
    browser: PathBuf,
    ffmpeg: PathBuf,
}

#[derive(Debug)]
struct Riga {
    scena: String,
    var: String,
    rep: usize,
    k: usize,
    c: Camp,
    errore: String,
}

fn intestazione_csv() -> &'static str {
    "scena,var,rep,k,warm,seek,invio,primo,ric,json,copie,b64,b_testo,b_png,ev,errore\n"
}

fn riga_csv(r: &Riga) -> String {
    let c = &r.c;
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        r.scena,
        r.var,
        r.rep,
        r.k,
        u8::from(r.k < WARMUP),
        c.seek,
        c.invio,
        c.primo,
        c.ric,
        c.json,
        c.copie,
        c.b64,
        c.b_testo,
        c.b_png,
        c.ev,
        r.errore.replace([',', '\n', '\r'], ";")
    )
}

fn accoda_file(percorso: &Path, intest: &str, contenuto: &str) {
    let nuovo = !percorso.exists();
    let mut f = fs::OpenOptions::new().create(true).append(true).open(percorso).expect("apertura csv");
    if nuovo {
        f.write_all(intest.as_bytes()).expect("scrittura csv");
    }
    f.write_all(contenuto.as_bytes()).expect("scrittura csv");
}

/// Working set approssimato (byte) dei processi msedge con questo profilo (solo Windows).
fn working_set(profilo: &Path) -> Option<u64> {
    if !cfg!(windows) {
        return None;
    }
    let cmd = format!(
        "(Get-CimInstance Win32_Process -Filter \"Name='msedge.exe'\" | Where-Object {{ $_.CommandLine -like '*{}*' }} | Measure-Object -Property WorkingSetSize -Sum).Sum",
        profilo.display()
    );
    let o = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &cmd]).output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().parse().ok()
}

fn mediana(v: &mut [u64]) -> u64 {
    v.sort_unstable();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// Un run: browser nuovo, pagina preparata come B.9.5, M fotogrammi con la variante data.
fn esegui_run(ctx: &Ctx, sc: &Scena, var: &str, rep: usize) -> Result<(), String> {
    let id_run = format!("{}_{}_r{}", sc.nome, var, rep);
    let profilo = ctx.dir.join("profili").join(&id_run);
    let _ = fs::remove_dir_all(&profilo);
    fs::create_dir_all(&profilo).map_err(|e| e.to_string())?;
    let mut browser = Browser::avvia(&ctx.browser, &profilo, sc.w, sc.h).map_err(|e| e.messaggio)?;
    let (porta, percorso) = browser.attendi_porta(&profilo, TIMEOUT).map_err(|e| e.messaggio)?;
    let mut c = Client::connetti(porta, &percorso)?;

    // Passi 4-9 di B.9.5.
    let r = c.comando("Target.createTarget", "{\"url\":\"about:blank\"}", true)?;
    let target = r.campo("targetId").and_then(json::Valore::testo).ok_or("targetId")?.to_string();
    let r = c.comando(
        "Target.attachToTarget",
        &format!("{{\"targetId\":{},\"flatten\":true}}", json::stringa(&target)),
        true,
    )?;
    c.sess = Some(r.campo("sessionId").and_then(json::Valore::testo).ok_or("sessionId")?.to_string());
    c.comando(
        "Emulation.setDeviceMetricsOverride",
        &format!("{{\"width\":{},\"height\":{},\"deviceScaleFactor\":1,\"mobile\":false}}", sc.w, sc.h),
        false,
    )?;
    c.comando("Emulation.setDefaultBackgroundColorOverride", "{\"color\":{\"r\":0,\"g\":0,\"b\":0,\"a\":1}}", false)?;
    c.comando("Page.enable", "{}", false)?;
    if let Some(url) = &sc.url {
        let r = c.comando("Page.navigate", &format!("{{\"url\":{}}}", json::stringa(url)), false)?;
        if let Some(t) = r.campo("errorText").and_then(json::Valore::testo) {
            return Err(format!("navigate: {t}"));
        }
        c.attendi_caricamento()?;
        c.eventi = 0;
        let (_, v) = c.valuta("typeof window.__joy_seek")?;
        if v.testo() != Some("function") {
            return Err("__joy_seek assente".into());
        }
    }
    let versione = c
        .comando("Browser.getVersion", "{}", true)
        .ok()
        .and_then(|r| r.campo("product").and_then(json::Valore::testo).map(String::from))
        .unwrap_or_default();

    let ws_prima = if var == "V0" || var == "V1" { working_set(&profilo) } else { None };
    let params = parametri(var);
    let mut righe: Vec<Riga> = Vec::new();
    let mut immagini: Vec<Vec<u8>> = Vec::new();
    let mut primo_errore: Option<String> = None;
    for k in 0..sc.m {
        let t = if sc.m > 1 { sc.d_ms * k as f64 / (sc.m - 1) as f64 } else { 0.0 };
        let esito = (|| -> Result<(Camp, Vec<u8>), String> {
            let seek = if sc.url.is_some() { c.seek(t)? } else { 0 };
            let (mut cm, mut img) = c.screenshot(params)?;
            if var == "C" {
                // Secondo screenshot consecutivo senza nuovo seek: si registra solo questo.
                (cm, img) = c.screenshot(params)?;
            }
            cm.seek = seek;
            Ok((cm, img))
        })();
        match esito {
            Ok((cm, img)) => {
                if estensione(var) == "png" {
                    let (w, h) = dimensioni_png(&img)?;
                    if (w, h) != (sc.w, sc.h) {
                        return Err(format!("screenshot {w}x{h} invece di {}x{}", sc.w, sc.h));
                    }
                }
                righe.push(Riga { scena: sc.nome.clone(), var: var.into(), rep, k, c: cm, errore: String::new() });
                immagini.push(img);
            }
            Err(e) => {
                primo_errore.get_or_insert_with(|| e.clone());
                righe.push(Riga { scena: sc.nome.clone(), var: var.into(), rep, k, c: Camp::default(), errore: e });
                break;
            }
        }
    }
    let ws_dopo = if var == "V0" || var == "V1" { working_set(&profilo) } else { None };

    // Riferimenti (solo V0, dopo il ciclo: non perturbano le misure).
    let mut rif = String::new();
    if var == "V0" && primo_errore.is_none() {
        let mut triv = Vec::new();
        for _ in 0..30 {
            if let Ok((d, _)) = c.valuta("1") {
                triv.push(d);
            }
        }
        let mut raf = Vec::new();
        c.timeout(Duration::from_secs(5));
        for _ in 0..30 {
            match c.valuta("new Promise(r=>requestAnimationFrame(()=>r(1)))") {
                Ok((d, _)) => raf.push(d),
                Err(_) => break,
            }
        }
        let mut interv = 0.0;
        if let Ok((_, json::Valore::Lista(l))) = c.valuta(
            "new Promise(res=>{const t=[];function f(x){t.push(x);if(t.length>=31)res(t);else requestAnimationFrame(f)};requestAnimationFrame(f)})",
        ) {
            let t: Vec<f64> = l.iter().filter_map(json::Valore::numero).collect();
            if t.len() > 1 {
                interv = (t[t.len() - 1] - t[0]) / (t.len() - 1) as f64;
            }
        }
        c.timeout(TIMEOUT);
        let mut t2 = triv.clone();
        let mut r2 = raf.clone();
        let _ = writeln!(
            rif,
            "{},{},{},{},{},{},{:.3}",
            sc.nome,
            rep,
            mediana(&mut t2),
            triv.iter().sum::<u64>() / triv.len().max(1) as u64,
            mediana(&mut r2),
            raf.iter().sum::<u64>() / raf.len().max(1) as u64,
            interv
        );
    }

    let _ = c.comando("Browser.close", "{}", true);
    browser.chiudi(Duration::from_secs(5));
    drop(browser);
    let _ = fs::remove_dir_all(&profilo);

    // Scrittura dei risultati (fuori dal ciclo misurato).
    let csv: String = righe.iter().map(riga_csv).collect();
    accoda_file(&ctx.dir.join("campioni.csv"), intestazione_csv(), &csv);
    if !rif.is_empty() {
        accoda_file(&ctx.dir.join("riferimenti.csv"), "scena,rep,triv_p50_us,triv_media_us,raf_p50_us,raf_media_us,intervallo_raf_ms\n", &rif);
    }
    if let (Some(a), Some(b)) = (ws_prima, ws_dopo) {
        accoda_file(&ctx.dir.join("memoria.csv"), "scena,var,rep,ws_prima,ws_dopo\n", &format!("{},{var},{rep},{a},{b}\n", sc.nome));
    }
    let cartella_img = ctx.dir.join("img").join(&sc.nome).join(format!("{var}_r{rep}"));
    let _ = fs::remove_dir_all(&cartella_img);
    fs::create_dir_all(&cartella_img).map_err(|e| e.to_string())?;
    for (k, img) in immagini.iter().enumerate() {
        fs::write(cartella_img.join(format!("{k:03}.{}", estensione(var))), img).map_err(|e| e.to_string())?;
    }
    println!(
        "  run {id_run}: {} campioni, {}{}",
        righe.len(),
        primo_errore.map_or("nessun errore".to_string(), |e| format!("ERRORE: {e}")),
        if versione.is_empty() { String::new() } else { format!(" [{versione}]") }
    );
    Ok(())
}

// ------------------------------------------------------------------ preparazione scene

fn radice() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compila_scena(dir: &Path, nome: &str, sorgente: &str, m: usize) -> Result<Scena, String> {
    let c = compila_con_avvisi(sorgente, &Opzioni { leggibile: false, senza_player: true }).map_err(|e| e.messaggio)?;
    let cart = dir.join("scene");
    fs::create_dir_all(&cart).map_err(|e| e.to_string())?;
    let file = cart.join(format!("{nome}.html"));
    fs::write(&file, &c.html).map_err(|e| e.to_string())?;
    let canonico = fs::canonicalize(&file).map_err(|e| e.to_string())?;
    let url = url_file(&canonico.to_string_lossy()).map_err(|e| e.messaggio)?;
    Ok(Scena {
        nome: nome.into(),
        w: c.larghezza.round() as u32,
        h: c.altezza.round() as u32,
        d_ms: (c.durata * 1000.0 * 10_000.0).round() / 10_000.0,
        url: Some(url),
        m,
    })
}

fn sorgente_vuota(w: u32, h: u32, colore: &str, d: f64) -> String {
    format!(
        "Prepara un palco grande {w} per {h} con sfondo {colore} a 60 fotogrammi.\n\
         Crea un quadrato chiamato \"p\" con lato 4 e colore {colore} a 10, 10.\n\
         Dal secondo 0 al secondo {d}:\n  \"p\" ruota di 10 gradi con movimento lineare.\n"
    )
}

fn elenco_env(nome: &str) -> Option<Vec<String>> {
    std::env::var(nome).ok().map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
}

struct Config {
    ripetizioni: usize,
    m720: usize,
    m4k: usize,
    sotto: &'static str,
    scene: Option<Vec<String>>,
    varianti: Option<Vec<String>>,
}

fn ambiente(sotto: &str) -> Result<Ctx, String> {
    let base = std::env::var("JOYPP_ESPERIMENTO_DIR").map_err(|_| "JOYPP_ESPERIMENTO_DIR non impostata".to_string())?;
    let dir = PathBuf::from(base).join(sotto);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let var = |k: &str| std::env::var(k).ok();
    let esiste = |p: &Path| p.is_file();
    let s = Sistema::corrente();
    let browser = cerca_browser(s, &var, &esiste).map_err(|e| format!("browser non trovato ({})", e.messaggio))?;
    let ffmpeg = cerca_ffmpeg(s, &var, &esiste).map_err(|e| format!("ffmpeg non trovato ({})", e.messaggio))?;
    Ok(Ctx { dir, browser, ffmpeg })
}

fn esegui_esperimento(cfg: &Config) -> Option<Ctx> {
    let ctx = match ambiente(cfg.sotto) {
        Ok(c) => c,
        Err(m) => {
            println!("esperimento saltato: {m}");
            return None;
        }
    };
    for f in ["campioni.csv", "riferimenti.csv", "memoria.csv"] {
        let _ = fs::remove_file(ctx.dir.join(f));
    }
    for d in ["img", "profili", "pixel"] {
        let _ = fs::remove_dir_all(ctx.dir.join(d));
    }
    let ex = |f: &str| fs::read_to_string(radice().join("examples").join(f)).expect("esempio");
    let mut scene: Vec<Scena> = Vec::new();
    let ammessa = |n: &str| cfg.scene.as_ref().is_none_or(|v| v.iter().any(|x| x == n));
    let mut aggiungi = |r: Result<Scena, String>, n: &str| {
        if ammessa(n) {
            scene.push(r.expect("scena"));
        }
    };
    aggiungi(compila_scena(&ctx.dir, "vuota720", &sorgente_vuota(1280, 720, "#0B0B14", 6.0), cfg.m720), "vuota720");
    aggiungi(compila_scena(&ctx.dir, "livello1", &ex("livello1_motion.joy"), cfg.m720), "livello1");
    aggiungi(compila_scena(&ctx.dir, "vuota4k", &sorgente_vuota(3840, 2160, "#050914", 30.0), cfg.m4k), "vuota4k");
    aggiungi(compila_scena(&ctx.dir, "cattedrale", &ex("cattedrale.joy"), cfg.m4k), "cattedrale");
    aggiungi(compila_scena(&ctx.dir, "voxel", &ex("JOYPP_VOXEL_OVERDRIVE_v021.joy"), cfg.m4k), "voxel");
    let blank = |n: &str, w, h, m| Scena { nome: n.into(), w, h, d_ms: 0.0, url: None, m };
    if ammessa("blank720") {
        scene.push(blank("blank720", 1280, 720, cfg.m720));
    }
    if ammessa("blank4k") {
        scene.push(blank("blank4k", 3840, 2160, cfg.m4k));
    }
    for s in &scene {
        println!("scena {}: {}x{}, D = {} ms, M = {}", s.nome, s.w, s.h, s.d_ms, s.m);
    }
    for rep in 0..cfg.ripetizioni {
        for sc in &scene {
            let mut varianti: Vec<&str> =
                if sc.url.is_some() { vec!["V0", "V1", "C", "JPEG", "WEBP"] } else { vec!["V0", "V1"] };
            if let Some(f) = &cfg.varianti {
                varianti.retain(|v| f.iter().any(|x| x == v));
            }
            if rep % 2 == 1 {
                varianti.reverse();
            }
            for v in varianti {
                if let Err(e) = esegui_run(&ctx, sc, v, rep) {
                    println!("  run {}_{v}_r{rep}: FALLITO: {e}", sc.nome);
                    accoda_file(
                        &ctx.dir.join("campioni.csv"),
                        intestazione_csv(),
                        &riga_csv(&Riga { scena: sc.nome.clone(), var: v.into(), rep, k: 0, c: Camp::default(), errore: format!("run fallito: {e}") }),
                    );
                }
            }
        }
    }
    Some(ctx)
}

// ------------------------------------------------------------------ riepilogo

#[derive(Clone, Default)]
struct R {
    scena: String,
    var: String,
    rep: usize,
    k: usize,
    warm: bool,
    c: Camp,
    err: bool,
}

fn leggi_csv(dir: &Path) -> Vec<R> {
    let t = fs::read_to_string(dir.join("campioni.csv")).unwrap_or_default();
    t.lines()
        .skip(1)
        .filter_map(|l| {
            let f: Vec<&str> = l.splitn(16, ',').collect();
            if f.len() < 16 {
                return None;
            }
            let n = |i: usize| f[i].parse::<u64>().unwrap_or(0);
            Some(R {
                scena: f[0].into(),
                var: f[1].into(),
                rep: n(2) as usize,
                k: n(3) as usize,
                warm: n(4) == 1,
                c: Camp { seek: n(5), invio: n(6), primo: n(7), ric: n(8), json: n(9), copie: n(10), b64: n(11), b_testo: n(12), b_png: n(13), ev: n(14) },
                err: !f[15].is_empty(),
            })
        })
        .collect()
}

fn shot(c: &Camp) -> u64 {
    c.invio + c.primo + c.ric
}
fn ciclo(c: &Camp) -> u64 {
    c.seek + shot(c) + c.json + c.copie + c.b64
}

struct St {
    media: f64,
    p50: f64,
    p95: f64,
    max: f64,
    min: f64,
}

fn stat(v: &[f64]) -> St {
    if v.is_empty() {
        return St { media: 0.0, p50: 0.0, p95: 0.0, max: 0.0, min: 0.0 };
    }
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let n = s.len();
    let pos = |p: f64| s[(((p * n as f64).ceil() as usize).max(1) - 1).min(n - 1)];
    St { media: s.iter().sum::<f64>() / n as f64, p50: pos(0.5), p95: pos(0.95), max: s[n - 1], min: s[0] }
}

fn ms(x: f64) -> f64 {
    x / 1000.0
}

fn riepilogo(dir: &Path) -> String {
    let righe = leggi_csv(dir);
    let mut o = String::new();
    let mut gruppi: BTreeMap<(String, String), Vec<&R>> = BTreeMap::new();
    for r in &righe {
        gruppi.entry((r.scena.clone(), r.var.clone())).or_default().push(r);
    }
    let ordine_scene = ["vuota720", "livello1", "blank720", "vuota4k", "cattedrale", "voxel", "blank4k"];
    let ordine_var = ["V0", "V1", "C", "JPEG", "WEBP"];
    let _ = writeln!(o, "## Scomposizione a regime (ms; esclusi i primi {WARMUP} fotogrammi; ripetizioni accorpate)\n");
    let _ = writeln!(o, "| scena | var | n | err | seek | invio | primo_byte | ricezione | json | copie | base64 | shot(i+p+r) | shot p50 | shot p95 | shot max | shot min | ciclo | KiB testo | KiB img |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for sc in ordine_scene {
        for v in ordine_var {
            let Some(g) = gruppi.get(&(sc.to_string(), v.to_string())) else { continue };
            let errori = g.iter().filter(|r| r.err).count();
            let ok: Vec<&&R> = g.iter().filter(|r| !r.err && !r.warm).collect();
            let col = |f: &dyn Fn(&Camp) -> u64| ok.iter().map(|r| f(&r.c) as f64).collect::<Vec<_>>();
            let m = |f: &dyn Fn(&Camp) -> u64| ms(stat(&col(f)).media);
            let s = stat(&col(&shot));
            let _ = writeln!(
                o,
                "| {sc} | {v} | {} | {errori} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.0} | {:.0} |",
                ok.len(),
                m(&|c| c.seek),
                m(&|c| c.invio),
                m(&|c| c.primo),
                m(&|c| c.ric),
                m(&|c| c.json),
                m(&|c| c.copie),
                m(&|c| c.b64),
                ms(s.media),
                ms(s.p50),
                ms(s.p95),
                ms(s.max),
                ms(s.min),
                m(&ciclo),
                stat(&col(&|c| c.b_testo)).media / 1024.0,
                stat(&col(&|c| c.b_png)).media / 1024.0,
            );
        }
    }

    let _ = writeln!(o, "\n## A freddo (ms): k = 0 (primo screenshot dopo il caricamento, media sulle ripetizioni) e k = 1..{} (media)\n", WARMUP - 1);
    let _ = writeln!(o, "| scena | var | k=0 seek | k=0 shot | k=0 ciclo | k=1..4 seek | k=1..4 shot | k=1..4 ciclo | regime shot | regime ciclo |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|---|---|");
    for sc in ordine_scene {
        for v in ordine_var {
            let Some(g) = gruppi.get(&(sc.to_string(), v.to_string())) else { continue };
            let sel = |a: usize, b: usize, f: &dyn Fn(&Camp) -> u64| {
                stat(&g.iter().filter(|r| !r.err && r.k >= a && r.k <= b).map(|r| f(&r.c) as f64).collect::<Vec<_>>()).media
            };
            let _ = writeln!(
                o,
                "| {sc} | {v} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |",
                ms(sel(0, 0, &|c| c.seek)),
                ms(sel(0, 0, &shot)),
                ms(sel(0, 0, &ciclo)),
                ms(sel(1, WARMUP - 1, &|c| c.seek)),
                ms(sel(1, WARMUP - 1, &shot)),
                ms(sel(1, WARMUP - 1, &ciclo)),
                ms(sel(WARMUP, usize::MAX, &shot)),
                ms(sel(WARMUP, usize::MAX, &ciclo)),
            );
        }
    }

    // Per ripetizione: media di shot e ciclo, e miglioramenti appaiati rispetto a V0.
    let media_rep = |sc: &str, v: &str, rep: usize, f: &dyn Fn(&Camp) -> u64| -> Option<f64> {
        let g = gruppi.get(&(sc.to_string(), v.to_string()))?;
        let x: Vec<f64> = g.iter().filter(|r| r.rep == rep && !r.err && !r.warm).map(|r| f(&r.c) as f64).collect();
        (!x.is_empty()).then(|| stat(&x).media)
    };
    let reps: usize = righe.iter().map(|r| r.rep + 1).max().unwrap_or(0);
    let _ = writeln!(o, "\n## Miglioramento rispetto a V0 (media per ripetizione, appaiate; + = più veloce)\n");
    let _ = writeln!(o, "| scena | confronto | metrica | per ripetizione (%) | media % | min % | max % | V0 per rip. (ms) |");
    let _ = writeln!(o, "|---|---|---|---|---|---|---|---|");
    for sc in ordine_scene {
        for v in ["V1", "C", "JPEG", "WEBP"] {
            for (nome, f) in [("shot", &shot as &dyn Fn(&Camp) -> u64), ("ciclo", &ciclo)] {
                let mut p = Vec::new();
                let mut base = Vec::new();
                for rep in 0..reps {
                    if let (Some(a), Some(b)) = (media_rep(sc, "V0", rep, f), media_rep(sc, v, rep, f)) {
                        p.push((a - b) / a * 100.0);
                        base.push(ms(a));
                    }
                }
                if p.is_empty() {
                    continue;
                }
                let s = stat(&p);
                let el: Vec<String> = p.iter().map(|x| format!("{x:+.1}")).collect();
                let bl: Vec<String> = base.iter().map(|x| format!("{x:.1}")).collect();
                let _ = writeln!(o, "| {sc} | {v} vs V0 | {nome} | {} | {:+.1} | {:+.1} | {:+.1} | {} |", el.join(" / "), s.media, s.min, s.max, bl.join(" / "));
            }
        }
    }

    let _ = writeln!(o, "\n## Statistiche complete per componente (ms, a regime, ripetizioni accorpate): media / p50 / p95 / max / min\n");
    for sc in ordine_scene {
        for v in ordine_var {
            let Some(g) = gruppi.get(&(sc.to_string(), v.to_string())) else { continue };
            let ok: Vec<&&R> = g.iter().filter(|r| !r.err && !r.warm).collect();
            if ok.is_empty() {
                continue;
            }
            let _ = writeln!(o, "{sc} {v}:");
            for (n, f) in [
                ("seek", &(|c: &Camp| c.seek) as &dyn Fn(&Camp) -> u64),
                ("invio", &|c| c.invio),
                ("primo_byte", &|c| c.primo),
                ("ricezione", &|c| c.ric),
                ("json", &|c| c.json),
                ("copie", &|c| c.copie),
                ("base64", &|c| c.b64),
                ("shot", &shot),
                ("ciclo", &ciclo),
            ] {
                let s = stat(&ok.iter().map(|r| f(&r.c) as f64).collect::<Vec<_>>());
                let _ = writeln!(o, "  {n:<11} {:>8.3} / {:>8.3} / {:>8.3} / {:>8.3} / {:>8.3}", ms(s.media), ms(s.p50), ms(s.p95), ms(s.max), ms(s.min));
            }
        }
    }

    if let Ok(t) = fs::read_to_string(dir.join("riferimenti.csv")) {
        let _ = writeln!(o, "\n## Riferimenti (V0, dopo il ciclo; ms)\n");
        let _ = writeln!(o, "| scena | rep | andata/ritorno `1` p50 | media | rAF andata/ritorno p50 | media | intervallo tra rAF |");
        let _ = writeln!(o, "|---|---|---|---|---|---|---|");
        for l in t.lines().skip(1) {
            let f: Vec<&str> = l.split(',').collect();
            if f.len() == 7 {
                let n = |i: usize| f[i].parse::<f64>().unwrap_or(0.0) / 1000.0;
                let _ = writeln!(o, "| {} | {} | {:.3} | {:.3} | {:.3} | {:.3} | {} |", f[0], f[1], n(2), n(3), n(4), n(5), f[6]);
            }
        }
    }
    if let Ok(t) = fs::read_to_string(dir.join("memoria.csv")) {
        let _ = writeln!(o, "\n## Working set approssimato dei processi del browser (MiB, da Win32_Process; misura approssimata)\n");
        let _ = writeln!(o, "| scena | var | rep | prima | dopo |");
        let _ = writeln!(o, "|---|---|---|---|---|");
        for l in t.lines().skip(1) {
            let f: Vec<&str> = l.split(',').collect();
            if f.len() == 5 {
                let m = |i: usize| f[i].parse::<f64>().unwrap_or(0.0) / 1_048_576.0;
                let _ = writeln!(o, "| {} | {} | {} | {:.0} | {:.0} |", f[0], f[1], f[2], m(3), m(4));
            }
        }
    }
    let errori: Vec<String> = fs::read_to_string(dir.join("campioni.csv"))
        .unwrap_or_default()
        .lines()
        .skip(1)
        .filter(|l| l.rsplit(',').next().is_some_and(|e| !e.is_empty()))
        .map(String::from)
        .collect();
    let _ = writeln!(o, "\n## Errori/timeout: {}", errori.len());
    for e in errori.iter().take(20) {
        let _ = writeln!(o, "  {e}");
    }
    o
}

// ------------------------------------------------------------------ confronto dei pixel

fn decodifica_raw(ffmpeg: &Path, file: &Path) -> Option<Vec<u8>> {
    let o = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(file)
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .output()
        .ok()?;
    o.status.success().then_some(o.stdout)
}

fn elenca(cartella: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(cartella)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// Confronta due cartelle di PNG fotogramma per fotogramma; riporta anche l'analisi delle differenze.
fn confronta(ctx: &Ctx, w: usize, a: &Path, b: &Path, etichetta: &str, out: &mut String) {
    let fa = elenca(a);
    let fb = elenca(b);
    let n = fa.len().min(fb.len());
    let (mut uguali_byte, mut uguali_pixel) = (0, 0);
    let mut diff_lines = Vec::new();
    for i in 0..n {
        let (ba, bb) = (fs::read(&fa[i]).unwrap_or_default(), fs::read(&fb[i]).unwrap_or_default());
        if ba == bb {
            uguali_byte += 1;
            uguali_pixel += 1;
            continue;
        }
        let (Some(ra), Some(rb)) = (decodifica_raw(&ctx.ffmpeg, &fa[i]), decodifica_raw(&ctx.ffmpeg, &fb[i])) else {
            diff_lines.push(format!("    fotogramma {i}: decodifica ffmpeg fallita"));
            continue;
        };
        if ra == rb {
            uguali_pixel += 1;
            continue;
        }
        let (mut n_diff, mut max_c) = (0u64, 0u8);
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for (p, (pa, pb)) in ra.as_chunks::<4>().0.iter().zip(rb.as_chunks::<4>().0.iter()).enumerate() {
            if pa != pb {
                n_diff += 1;
                for c in 0..4 {
                    max_c = max_c.max(pa[c].abs_diff(pb[c]));
                }
                let (x, y) = (p % w, p / w);
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
        let mut riga = format!(
            "    fotogramma {i}: {n_diff} pixel diversi, differenza massima per canale {max_c}, riquadro x {x0}..{x1} y {y0}..{y1}"
        );
        if diff_lines.len() < 3 {
            let cart = ctx.dir.join("pixel");
            let _ = fs::create_dir_all(&cart);
            let nome = format!("diff_{}_{i:03}.png", etichetta.replace([' ', '/'], "_"));
            let ok = Command::new(&ctx.ffmpeg)
                .args(["-v", "error", "-y", "-i"])
                .arg(&fa[i])
                .arg("-i")
                .arg(&fb[i])
                .args(["-filter_complex", "blend=all_mode=difference,lutrgb=r=clip(val*32\\,0\\,255):g=clip(val*32\\,0\\,255):b=clip(val*32\\,0\\,255)", "-frames:v", "1"])
                .arg(cart.join(&nome))
                .status()
                .is_ok_and(|s| s.success());
            if ok {
                let _ = write!(riga, " (diff x32: {})", cart.join(&nome).display());
            }
        }
        diff_lines.push(riga);
    }
    let _ = writeln!(out, "- {etichetta}: {uguali_byte}/{n} PNG identici byte per byte, {uguali_pixel}/{n} identici a livello di pixel");
    for l in diff_lines.iter().take(8) {
        let _ = writeln!(out, "{l}");
    }
    if diff_lines.len() > 8 {
        let _ = writeln!(out, "    ... altre {} differenze", diff_lines.len() - 8);
    }
}

fn confronto_pixel(ctx: &Ctx, reps: usize) -> String {
    let mut o = String::from("## Confronto dei pixel (PNG decodificati con ffmpeg in RGBA)\n\n");
    for (sc, w) in [("vuota720", 1280), ("livello1", 1280), ("vuota4k", 3840), ("cattedrale", 3840), ("voxel", 3840)] {
        let img = ctx.dir.join("img").join(sc);
        if !img.exists() {
            continue;
        }
        let _ = writeln!(o, "{sc}:");
        for r in 0..reps {
            let (a, b) = (img.join(format!("V0_r{r}")), img.join(format!("V1_r{r}")));
            if a.exists() && b.exists() {
                confronta(ctx, w, &a, &b, &format!("{sc} V0 vs V1 (rep {r})"), &mut o);
            }
        }
        for r in 1..reps {
            let (a, b) = (img.join("V0_r0"), img.join(format!("V0_r{r}")));
            if a.exists() && b.exists() {
                confronta(ctx, w, &a, &b, &format!("{sc} V0 rep0 vs V0 rep{r} (ripetibilita)"), &mut o);
            }
        }
        for r in 1..reps {
            let (a, b) = (img.join("V1_r0"), img.join(format!("V1_r{r}")));
            if a.exists() && b.exists() {
                confronta(ctx, w, &a, &b, &format!("{sc} V1 rep0 vs V1 rep{r} (ripetibilita)"), &mut o);
            }
        }
    }
    o
}

fn finale(ctx: &Ctx, reps: usize) {
    let mut testo = riepilogo(&ctx.dir);
    println!("{testo}");
    let px = confronto_pixel(ctx, reps);
    println!("{px}");
    testo.push('\n');
    testo.push_str(&px);
    let _ = fs::write(ctx.dir.join("riepilogo.md"), testo);
}

// ------------------------------------------------------------------ esperimento S e caratterizzazione

/// Apre browser e pagina come in produzione (passi 4-9 di B.9.5); restituisce anche il profilo.
fn apri(ctx: &Ctx, sc: &Scena, id: &str) -> Result<(Browser, Client, PathBuf), String> {
    let profilo = ctx.dir.join("profili").join(id);
    let _ = fs::remove_dir_all(&profilo);
    fs::create_dir_all(&profilo).map_err(|e| e.to_string())?;
    let mut browser = Browser::avvia(&ctx.browser, &profilo, sc.w, sc.h).map_err(|e| e.messaggio)?;
    let (porta, percorso) = browser.attendi_porta(&profilo, TIMEOUT).map_err(|e| e.messaggio)?;
    let mut c = Client::connetti(porta, &percorso)?;
    let r = c.comando("Target.createTarget", "{\"url\":\"about:blank\"}", true)?;
    let target = r.campo("targetId").and_then(json::Valore::testo).ok_or("targetId")?.to_string();
    let r = c.comando(
        "Target.attachToTarget",
        &format!("{{\"targetId\":{},\"flatten\":true}}", json::stringa(&target)),
        true,
    )?;
    c.sess = Some(r.campo("sessionId").and_then(json::Valore::testo).ok_or("sessionId")?.to_string());
    c.comando(
        "Emulation.setDeviceMetricsOverride",
        &format!("{{\"width\":{},\"height\":{},\"deviceScaleFactor\":1,\"mobile\":false}}", sc.w, sc.h),
        false,
    )?;
    c.comando("Emulation.setDefaultBackgroundColorOverride", "{\"color\":{\"r\":0,\"g\":0,\"b\":0,\"a\":1}}", false)?;
    c.comando("Page.enable", "{}", false)?;
    let url = sc.url.as_ref().ok_or("scena senza pagina")?;
    let r = c.comando("Page.navigate", &format!("{{\"url\":{}}}", json::stringa(url)), false)?;
    if let Some(t) = r.campo("errorText").and_then(json::Valore::testo) {
        return Err(format!("navigate: {t}"));
    }
    c.attendi_caricamento()?;
    c.eventi = 0;
    let (_, v) = c.valuta("typeof window.__joy_seek")?;
    if v.testo() != Some("function") {
        return Err("__joy_seek assente".into());
    }
    Ok((browser, c, profilo))
}

fn chiudi(mut browser: Browser, mut c: Client, profilo: &Path) {
    let _ = c.comando("Browser.close", "{}", true);
    browser.chiudi(Duration::from_secs(5));
    drop(browser);
    let _ = fs::remove_dir_all(profilo);
}

fn scena_reale(ctx: &Ctx, nome: &str) -> Scena {
    let ex = |f: &str| fs::read_to_string(radice().join("examples").join(f)).expect("esempio");
    match nome {
        "livello1" => compila_scena(&ctx.dir, nome, &ex("livello1_motion.joy"), 60),
        "cattedrale" => compila_scena(&ctx.dir, nome, &ex("cattedrale.joy"), 30),
        _ => compila_scena(&ctx.dir, nome, &ex("JOYPP_VOXEL_OVERDRIVE_v021.joy"), 30),
    }
    .expect("scena")
}

fn istante_k(sc: &Scena, k: usize) -> f64 {
    sc.d_ms * k as f64 / (sc.m - 1) as f64
}

/// Pixel diversi e differenza massima per canale tra due buffer RGBA.
fn diff_raw(a: &[u8], b: &[u8]) -> (u64, u8) {
    let (mut n, mut m) = (0u64, 0u8);
    for (pa, pb) in a.as_chunks::<4>().0.iter().zip(b.as_chunks::<4>().0.iter()) {
        if pa != pb {
            n += 1;
            for c in 0..4 {
                m = m.max(pa[c].abs_diff(pb[c]));
            }
        }
    }
    (n, m)
}

/// Esperimento S: per ogni istante un solo seek, poi tre screenshot consecutivi (V0,V1,V0 oppure V1,V0,V1).
#[allow(clippy::needless_range_loop)]
fn esperimento_s(ctx: &Ctx, reps: usize) -> String {
    let mut o = String::from("## Esperimento S: stessa superficie (un solo seek, tre screenshot consecutivi)\n\n");
    let _ = fs::remove_dir_all(ctx.dir.join("img_s"));
    let _ = fs::remove_file(ctx.dir.join("stessa_superficie.csv"));
    for nome in ["voxel", "cattedrale", "livello1"] {
        let sc = scena_reale(ctx, nome);
        // statistiche: [ordine][..]
        let mut terne = [0u32; 2];
        let mut stabili = [0u32; 2]; // stessa variante, 1o == 3o
        let mut stabili_px = [0u64; 2];
        let mut stabili_max = [0u8; 2];
        let mut cross_tot = [0u32; 2];
        let mut cross_uguali = [0u32; 2];
        let mut cross_px = [0u64; 2];
        let mut cross_max = [0u8; 2];
        let mut cross_stab_tot = [0u32; 2]; // terne con superficie stabile: V1 == V0?
        let mut cross_stab_uguali = [0u32; 2];
        let mut tempi: [[Vec<u64>; 3]; 2] = Default::default();
        let mut righe_csv = String::new();
        for rep in 0..reps {
            let ord = rep % 2; // 0: V0,V1,V0; 1: V1,V0,V1
            let vars = if ord == 0 { ["V0", "V1", "V0"] } else { ["V1", "V0", "V1"] };
            let id = format!("S_{nome}_r{rep}");
            let (browser, mut c, profilo) = match apri(ctx, &sc, &id) {
                Ok(x) => x,
                Err(e) => {
                    println!("  S {id}: FALLITO {e}");
                    continue;
                }
            };
            let mut shots: Vec<[Vec<u8>; 3]> = Vec::new();
            for k in 0..sc.m {
                let r = (|| -> Result<[Vec<u8>; 3], String> {
                    c.seek(istante_k(&sc, k))?;
                    let mut v: Vec<Vec<u8>> = Vec::new();
                    for (i, var) in vars.iter().enumerate() {
                        let (cm, img) = c.screenshot(parametri(var))?;
                        tempi[ord][i].push(shot(&cm));
                        righe_csv.push_str(&format!("{nome},{rep},{k},{ord},{i},{var},{}\n", shot(&cm)));
                        v.push(img);
                    }
                    let [a, b, d]: [Vec<u8>; 3] = v.try_into().map_err(|_| "terna")?;
                    Ok([a, b, d])
                })();
                match r {
                    Ok(t) => shots.push(t),
                    Err(e) => {
                        println!("  S {id} k={k}: ERRORE {e}");
                        break;
                    }
                }
            }
            chiudi(browser, c, &profilo);
            let cart = ctx.dir.join("img_s").join(nome).join(format!("r{rep}"));
            let _ = fs::create_dir_all(&cart);
            for (k, t) in shots.iter().enumerate() {
                for (i, img) in t.iter().enumerate() {
                    let f = cart.join(format!("{k:03}_{i}_{}.png", vars[i]));
                    let _ = fs::write(&f, img);
                }
                let raw: Vec<Option<Vec<u8>>> = (0..3)
                    .map(|i| decodifica_raw(&ctx.ffmpeg, &cart.join(format!("{k:03}_{i}_{}.png", vars[i]))))
                    .collect();
                let (Some(r0), Some(r1), Some(r2)) = (&raw[0], &raw[1], &raw[2]) else {
                    println!("  S {id} k={k}: decodifica fallita");
                    continue;
                };
                terne[ord] += 1;
                let (n02, m02) = diff_raw(r0, r2);
                let stabile = n02 == 0;
                if stabile {
                    stabili[ord] += 1;
                } else {
                    stabili_px[ord] += n02;
                    stabili_max[ord] = stabili_max[ord].max(m02);
                }
                for (x, y) in [(r0, r1), (r1, r2)] {
                    let (n, m) = diff_raw(x, y);
                    cross_tot[ord] += 1;
                    if n == 0 {
                        cross_uguali[ord] += 1;
                    } else {
                        cross_px[ord] += n;
                        cross_max[ord] = cross_max[ord].max(m);
                    }
                    if stabile {
                        cross_stab_tot[ord] += 1;
                        if n == 0 {
                            cross_stab_uguali[ord] += 1;
                        }
                    }
                }
            }
            println!("  S {id}: {} terne", shots.len());
        }
        accoda_file(&ctx.dir.join("stessa_superficie.csv"), "scena,rep,k,ordine,posizione,var,shot_us\n", &righe_csv);
        let _ = writeln!(o, "### {nome} ({}x{}, {} istanti, {reps} ripetizioni)\n", sc.w, sc.h, sc.m);
        let _ = writeln!(o, "| ordine | terne | stabili (stessa variante 1a == 3a) | px diversi / max (instabili) | coppie V0-V1 uguali | px diversi / max (diverse) | con superficie stabile: V0-V1 uguali |");
        let _ = writeln!(o, "|---|---|---|---|---|---|---|");
        for ord in 0..2 {
            let nome_ord = if ord == 0 { "V0,V1,V0" } else { "V1,V0,V1" };
            let _ = writeln!(
                o,
                "| {nome_ord} | {} | {} | {} / {} | {}/{} | {} / {} | {}/{} |",
                terne[ord], stabili[ord], stabili_px[ord], stabili_max[ord], cross_uguali[ord], cross_tot[ord],
                cross_px[ord], cross_max[ord], cross_stab_uguali[ord], cross_stab_tot[ord]
            );
        }
        let _ = writeln!(o, "\nTempi dello screenshot (ms, media / p50 / p95 / max; tutti i fotogrammi):\n");
        for ord in 0..2 {
            let vars = if ord == 0 { ["V0", "V1", "V0"] } else { ["V1", "V0", "V1"] };
            for i in 0..3 {
                let s = stat(&tempi[ord][i].iter().map(|&x| x as f64).collect::<Vec<_>>());
                let _ = writeln!(o, "- ordine {}: posizione {} ({}): {:.2} / {:.2} / {:.2} / {:.2}", ord, i + 1, vars[i], ms(s.media), ms(s.p50), ms(s.p95), ms(s.max));
            }
        }
        o.push('\n');
    }
    o
}

/// Caratterizzazione di V0: 5 istanze, stessi istanti, classi distinte di pixel per fotogramma (voxel 4K).
fn caratterizza_v0(ctx: &Ctx, istanze: usize) -> String {
    let sc = scena_reale(ctx, "voxel");
    let mut o = String::from("## Non determinazione di V0 tra istanze (voxel 4K)\n\n");
    let _ = fs::remove_dir_all(ctx.dir.join("img_c"));
    let mut dir_i: Vec<PathBuf> = Vec::new();
    for n in 0..istanze {
        let id = format!("C_voxel_i{n}");
        let (browser, mut c, profilo) = match apri(ctx, &sc, &id) {
            Ok(x) => x,
            Err(e) => {
                let _ = writeln!(o, "istanza {n}: FALLITA {e}");
                continue;
            }
        };
        let cart = ctx.dir.join("img_c").join(format!("i{n}"));
        let _ = fs::create_dir_all(&cart);
        for k in 0..sc.m {
            let r = c.seek(istante_k(&sc, k)).and_then(|_| c.screenshot(P_PNG));
            match r {
                Ok((_, img)) => {
                    let _ = fs::write(cart.join(format!("{k:03}.png")), img);
                }
                Err(e) => {
                    println!("  C i{n} k={k}: ERRORE {e}");
                    break;
                }
            }
        }
        chiudi(browser, c, &profilo);
        dir_i.push(cart);
        println!("  C istanza {n} completata");
    }
    let mut istogramma: BTreeMap<usize, u32> = BTreeMap::new();
    let mut max_globale = 0u8;
    let mut px_max = 0u64;
    let _ = writeln!(o, "| fotogramma | classi distinte su {} istanze | differenza massima per canale | px diversi (max tra coppie) |", dir_i.len());
    let _ = writeln!(o, "|---|---|---|---|");
    for k in 0..sc.m {
        let raws: Vec<Vec<u8>> = dir_i
            .iter()
            .filter_map(|d| decodifica_raw(&ctx.ffmpeg, &d.join(format!("{k:03}.png"))))
            .collect();
        let mut classi: Vec<usize> = Vec::new(); // indice del rappresentante
        for (i, r) in raws.iter().enumerate() {
            if !classi.iter().any(|&j| raws[j] == *r) {
                classi.push(i);
            }
        }
        let (mut mx, mut pxm) = (0u8, 0u64);
        for a in 0..classi.len() {
            for b in a + 1..classi.len() {
                let (n, m) = diff_raw(&raws[classi[a]], &raws[classi[b]]);
                mx = mx.max(m);
                pxm = pxm.max(n);
            }
        }
        *istogramma.entry(classi.len()).or_default() += 1;
        max_globale = max_globale.max(mx);
        px_max = px_max.max(pxm);
        if classi.len() > 1 {
            let _ = writeln!(o, "| {k} | {} | {mx} | {pxm} |", classi.len());
        }
    }
    let _ = writeln!(o, "\nFotogrammi per numero di classi distinte: {istogramma:?}. Differenza massima per canale: {max_globale}; massimo di pixel diversi tra due classi: {px_max}. (Righe elencate: solo i fotogrammi con più di una classe.)");
    o
}

// ------------------------------------------------------------------ test

/// Esperimento completo: 3 ripetizioni, 60 istanti a 720p e 30 a 4K.
#[test]
#[ignore = "esperimento lungo: eseguire a mano (vedi la documentazione del file)"]
fn esperimento_completo() {
    let cfg = Config {
        ripetizioni: std::env::var("JOYPP_ESPERIMENTO_RIPETIZIONI").ok().and_then(|v| v.parse().ok()).unwrap_or(3),
        m720: 60,
        m4k: 30,
        sotto: "completo",
        scene: elenco_env("JOYPP_ESPERIMENTO_SCENE"),
        varianti: elenco_env("JOYPP_ESPERIMENTO_VARIANTI"),
    };
    if let Some(ctx) = esegui_esperimento(&cfg) {
        finale(&ctx, cfg.ripetizioni);
    }
}

/// Prova rapida del harness: una ripetizione, pochi istanti, due scene.
#[test]
#[ignore = "esperimento: eseguire a mano"]
fn esperimento_rapido() {
    let cfg = Config {
        ripetizioni: 2,
        m720: 8,
        m4k: 6,
        sotto: "rapido",
        scene: Some(vec!["vuota720".into(), "livello1".into(), "blank720".into(), "vuota4k".into()]),
        varianti: None,
    };
    if let Some(ctx) = esegui_esperimento(&cfg) {
        finale(&ctx, cfg.ripetizioni);
    }
}

/// Rielabora i CSV e le immagini già presenti in `<dir>/completo` senza rieseguire i browser.
#[test]
#[ignore = "esperimento: eseguire a mano"]
fn esperimento_riepilogo() {
    match ambiente("completo") {
        Ok(ctx) => finale(&ctx, 3),
        Err(m) => println!("esperimento saltato: {m}"),
    }
}

/// Esperimento S (stessa superficie) e caratterizzazione di V0 su 5 istanze: circa 10 minuti.
#[test]
#[ignore = "esperimento lungo: eseguire a mano"]
fn esperimento_stessa_superficie() {
    let ctx = match ambiente("stessa_superficie") {
        Ok(c) => c,
        Err(m) => {
            println!("esperimento saltato: {m}");
            return;
        }
    };
    let reps = std::env::var("JOYPP_ESPERIMENTO_RIPETIZIONI").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    let istanze = std::env::var("JOYPP_ESPERIMENTO_ISTANZE").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    let mut t = esperimento_s(&ctx, reps);
    t.push_str(&caratterizza_v0(&ctx, istanze));
    println!("{t}");
    let _ = fs::write(ctx.dir.join("riepilogo_s.md"), t);
}

/// Istanze ripetute di livello1 con i 361 istanti di produzione (60 fps): classi di PNG per
/// fotogramma e variante, e confronto V0/V1 sui pixel. Serve a stabilire se le differenze dei video
/// prima/dopo nascono già nello screenshot (non determinazione tra istanze) o nel codificatore.
#[test]
#[ignore = "esperimento lungo: eseguire a mano"]
fn esperimento_istanze_livello1() {
    let ctx = match ambiente("istanze_l1") {
        Ok(c) => c,
        Err(m) => {
            println!("esperimento saltato: {m}");
            return;
        }
    };
    let n_ist: usize = std::env::var("JOYPP_ESPERIMENTO_ISTANZE").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let mut sc = scena_reale(&ctx, "livello1");
    sc.m = 361;
    // per variante: istanza -> fotogrammi (PNG)
    let mut dati: BTreeMap<&str, Vec<Vec<Vec<u8>>>> = BTreeMap::new();
    for i in 0..n_ist {
        let ordine = if i % 2 == 0 { ["V0", "V1"] } else { ["V1", "V0"] };
        for var in ordine {
            let id = format!("L_{var}_i{i}");
            let Ok((browser, mut c, profilo)) = apri(&ctx, &sc, &id) else {
                println!("  {id}: apertura fallita");
                continue;
            };
            let mut frames = Vec::new();
            for k in 0..sc.m {
                match c.seek(istante_k(&sc, k)).and_then(|_| c.screenshot(parametri(var))) {
                    Ok((_, img)) => frames.push(img),
                    Err(e) => {
                        println!("  {id} k={k}: ERRORE {e}");
                        break;
                    }
                }
            }
            chiudi(browser, c, &profilo);
            println!("  {id}: {} fotogrammi", frames.len());
            dati.entry(var).or_default().push(frames);
        }
    }
    let mut o = String::from("## livello1 720p, 361 istanti, istanze ripetute\n\n");
    let cart = ctx.dir.join("tmp");
    let _ = fs::create_dir_all(&cart);
    let mut rappr: BTreeMap<&str, Vec<Vec<Vec<u8>>>> = BTreeMap::new(); // var -> frame -> classi (PNG distinti)
    for (var, ist) in &dati {
        let mut con_piu_classi = Vec::new();
        let mut per_frame: Vec<Vec<Vec<u8>>> = Vec::new();
        for k in 0..sc.m {
            let mut classi: Vec<Vec<u8>> = Vec::new();
            for f in ist {
                if let Some(img) = f.get(k).filter(|i| !classi.contains(i)) {
                    classi.push(img.clone());
                }
            }
            if classi.len() > 1 {
                con_piu_classi.push(k);
            }
            per_frame.push(classi);
        }
        let _ = writeln!(
            o,
            "- {var}: {} istanze; fotogrammi con più di una classe di PNG tra istanze: {} (primi: {:?})",
            ist.len(),
            con_piu_classi.len(),
            &con_piu_classi[..con_piu_classi.len().min(12)]
        );
        rappr.insert(var, per_frame);
    }
    // Natura delle differenze tra istanze: la classe minoritaria somiglia al fotogramma
    // precedente/successivo della classe maggioritaria (superficie «vecchia»/«nuova»)?
    for (var, ist) in &dati {
        let maggioritaria = |k: usize| -> Option<Vec<u8>> {
            let mut conta: Vec<(Vec<u8>, usize)> = Vec::new();
            for f in ist {
                if let Some(img) = f.get(k) {
                    match conta.iter_mut().find(|(i, _)| i == img) {
                        Some(c) => c.1 += 1,
                        None => conta.push((img.clone(), 1)),
                    }
                }
            }
            conta.into_iter().max_by_key(|(_, n)| *n).map(|(i, _)| i)
        };
        let dec = |img: &[u8], nome: &str| -> Option<Vec<u8>> {
            let f = cart.join(nome);
            fs::write(&f, img).ok()?;
            decodifica_raw(&ctx.ffmpeg, &f)
        };
        let mut visti = 0;
        for k in 1..sc.m - 1 {
            let Some(mag) = maggioritaria(k) else { continue };
            for (n_i, f) in ist.iter().enumerate() {
                let Some(img) = f.get(k) else { continue };
                if *img == mag || visti >= 12 {
                    continue;
                }
                visti += 1;
                let (Some(rm), Some(ri)) = (dec(&mag, "m.png"), dec(img, "i.png")) else { continue };
                let prev = maggioritaria(k - 1).and_then(|p| dec(&p, "p.png"));
                let next = maggioritaria(k + 1).and_then(|p| dec(&p, "n.png"));
                let (d_m, d_p, d_n) = (diff_raw(&ri, &rm), prev.map(|p| diff_raw(&ri, &p)), next.map(|p| diff_raw(&ri, &p)));
                let _ = writeln!(
                    o,
                    "  {var} istanza {n_i} fotogramma {k}: contro maggioranza(k) {} px (max {}), contro maggioranza(k-1) {:?}, contro maggioranza(k+1) {:?}",
                    d_m.0, d_m.1, d_p, d_n
                );
            }
        }
    }
    // confronto V0/V1 sui pixel (rappresentanti delle classi)
    let (mut uguali, mut tot, mut max_px, mut max_c) = (0, 0, 0u64, 0u8);
    let mut primi: Vec<usize> = Vec::new();
    if let (Some(a), Some(b)) = (rappr.get("V0"), rappr.get("V1")) {
        for k in 0..sc.m {
            let (Some(ca), Some(cb)) = (a.get(k), b.get(k)) else { continue };
            let mut tutti_uguali = true;
            for x in ca {
                for y in cb {
                    let (fa, fb) = (cart.join("a.png"), cart.join("b.png"));
                    let _ = fs::write(&fa, x);
                    let _ = fs::write(&fb, y);
                    let (Some(ra), Some(rb)) = (decodifica_raw(&ctx.ffmpeg, &fa), decodifica_raw(&ctx.ffmpeg, &fb)) else {
                        continue;
                    };
                    let (n, m) = diff_raw(&ra, &rb);
                    if n > 0 {
                        tutti_uguali = false;
                        max_px = max_px.max(n);
                        max_c = max_c.max(m);
                    }
                }
            }
            tot += 1;
            if tutti_uguali {
                uguali += 1;
            } else {
                primi.push(k);
            }
        }
    }
    let _ = writeln!(
        o,
        "- V0 contro V1 (tutte le classi): fotogrammi con pixel identici {uguali}/{tot}; max pixel diversi {max_px}, max canale {max_c}; primi fotogrammi diversi: {:?}",
        &primi[..primi.len().min(12)]
    );
    println!("{o}");
    let _ = fs::write(ctx.dir.join("riepilogo_istanze.md"), o);
}
