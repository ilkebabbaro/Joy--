//! Test d'integrazione REALE di `joypp render` (SPECIFICA B.9.9): browser headless e ffmpeg.
//!
//! Se un browser compatibile o ffmpeg non vengono trovati il test stampa `test saltato: <motivo>`
//! e termina con successo. Con `JOYPP_TEST_RENDER=1` la loro assenza è invece un fallimento (CI).
//! I video e le cartelle temporanee stanno nella cartella temp di sistema e vengono rimossi.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use joypp::exporter::render::browser::{Sistema, cerca_browser};
use joypp::exporter::render::ffmpeg::cerca_ffmpeg;

/// Tempo massimo ragionevole per ciascun render di una scena minuscola.
const TETTO: Duration = Duration::from_secs(60);

fn ambiente(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

/// `Some(motivo)` se il test va saltato (o fa fallire con `JOYPP_TEST_RENDER=1`).
fn manca_qualcosa() -> Option<String> {
    let esiste = |p: &Path| p.is_file();
    let s = Sistema::corrente();
    if let Err(e) = cerca_ffmpeg(s, &ambiente, &esiste) {
        return Some(format!("ffmpeg non trovato ({})", e.messaggio));
    }
    if let Err(e) = cerca_browser(s, &ambiente, &esiste) {
        return Some(format!("browser non trovato ({})", e.messaggio));
    }
    None
}

/// Nomi delle cartelle `joypp-render-*` presenti nella cartella temporanea di sistema.
fn cartelle_render() -> BTreeSet<String> {
    fs::read_dir(std::env::temp_dir())
        .map(|it| {
            it.filter_map(Result::ok)
                .map(|v| v.file_name().to_string_lossy().into_owned())
                .filter(|n| n.starts_with("joypp-render-"))
                .collect()
        })
        .unwrap_or_default()
}

/// Valori `chiave=valore` di `ffprobe`, se disponibile.
fn ffprobe(video: &Path) -> Option<Vec<(String, String)>> {
    let out = Command::new("ffprobe")
        .args([
            "-v", "error", "-select_streams", "v:0", "-count_frames", "-show_entries",
            "stream=codec_name,pix_fmt,width,height,nb_read_frames", "-of", "default=noprint_wrappers=1",
        ])
        .arg(video)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let t = String::from_utf8_lossy(&out.stdout).into_owned();
    Some(
        t.lines()
            .filter_map(|l| l.split_once('='))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect(),
    )
}

fn valore<'a>(v: &'a [(String, String)], chiave: &str) -> &'a str {
    v.iter().find(|(k, _)| k == chiave).map(|(_, x)| x.as_str()).unwrap_or_else(|| panic!("ffprobe: {chiave} assente"))
}

fn scena(larghezza: u32, altezza: u32) -> String {
    format!(
        "Prepara un palco grande {larghezza} per {altezza} con sfondo #0F172A.\nCrea un cerchio chiamato \"a\" con raggio 10 e colore #F59E0B al centro.\nDal secondo 0 al secondo 0.5:\n  \"a\" appare sfumando.\n"
    )
}

/// Esegue `joypp render` e controlla file, `ftyp`, ffprobe, tempi. Restituisce lo stderr.
fn renderizza(nome: &str, larghezza: u32, altezza: u32, attesi: (u32, u32), fps: u32, n_atteso: u64) -> String {
    let dir = std::env::temp_dir().join(format!("joypp-test-render-{nome}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let sorgente = dir.join("scena.joy");
    fs::write(&sorgente, scena(larghezza, altezza)).unwrap();
    let video: PathBuf = dir.join("uscita.mp4");

    let prima = cartelle_render();
    let inizio = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_joypp"))
        .args(["render", sorgente.to_str().unwrap(), "-o", video.to_str().unwrap(), "--fps", &fps.to_string()])
        .output()
        .unwrap();
    let durata = inizio.elapsed();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    eprintln!("render {nome}: {:.1} s", durata.as_secs_f64());

    assert!(out.status.success(), "render fallito ({:?}):\n{stderr}", out.status.code());
    assert!(durata < TETTO, "render troppo lento: {durata:?}");
    assert!(
        stdout.contains(&format!("{n_atteso} fotogrammi, {}×{}, {fps} fps", attesi.0, attesi.1)),
        "stdout inatteso: {stdout}"
    );

    // Il file esiste, è un MP4 (box `ftyp` ai byte 4..8) e non resta il parziale.
    let byte = fs::read(&video).unwrap();
    assert!(byte.len() > 8 + 100, "file troppo piccolo: {}", byte.len());
    assert_eq!(&byte[4..8], b"ftyp", "manca il box ftyp");
    assert!(!dir.join("uscita.mp4.joypp-parziale").exists(), "parziale rimasto");

    // Nessuna cartella joypp-render-* nuova nella temp di sistema.
    let dopo = cartelle_render();
    let nuove: Vec<_> = dopo.difference(&prima).collect();
    assert!(nuove.is_empty(), "cartelle temporanee rimaste: {nuove:?}");

    match ffprobe(&video) {
        Some(v) => {
            assert_eq!(valore(&v, "codec_name"), "h264");
            assert_eq!(valore(&v, "pix_fmt"), "yuv420p");
            assert_eq!(valore(&v, "width"), attesi.0.to_string());
            assert_eq!(valore(&v, "height"), attesi.1.to_string());
            assert_eq!(valore(&v, "nb_read_frames"), n_atteso.to_string(), "numero di fotogrammi");
        }
        None => eprintln!("ffprobe non disponibile: controlli sul flusso saltati"),
    }
    fs::remove_dir_all(&dir).ok();
    stderr
}

#[test]
fn render_reale_di_una_scena_minuscola() {
    if let Some(motivo) = manca_qualcosa() {
        if ambiente("JOYPP_TEST_RENDER").as_deref() == Some("1") {
            panic!("render non eseguibile con JOYPP_TEST_RENDER=1: {motivo}");
        }
        eprintln!("test saltato: {motivo}");
        return;
    }
    // Palco 64x36, 0.5 s a 10 fps: N = ceil(0.5 * 10) + 1 = 6.
    let e = renderizza("64x36", 64, 36, (64, 36), 10, 6);
    assert!(!e.contains("dimensioni dispari"), "{e}");
    // Palco 63x35: video 64x36 (pad) e avviso; 0.5 s a 5 fps: N = ceil(2.5) + 1 = 4.
    let e = renderizza("63x35", 63, 35, (64, 36), 5, 4);
    assert!(e.contains("Attenzione: il palco 63×35 ha dimensioni dispari: il video sarà 64×36"), "{e}");
}
