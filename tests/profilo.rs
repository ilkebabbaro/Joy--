//! Test d'integrazione della diagnostica del render (SPECIFICA C.1.7): `--profilo` e
//! `--profilo-json`.
//!
//! La prima parte non richiede browser né ffmpeg e gira sempre. La seconda esegue render reali:
//! se mancano stampa `test saltato: <motivo>`; con `JOYPP_TEST_RENDER=1` la loro assenza è un
//! fallimento. I test unitari di `src/exporter/render/profilo.rs` e `src/exporter/cli.rs`
//! coprono già i casi di base: qui ci sono quelli di integrazione e di contorno.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use joypp::exporter::cli::{Comando, interpreta};
use joypp::exporter::render::browser::{Sistema, cerca_browser};
use joypp::exporter::render::ffmpeg::{argomenti_ffmpeg, cerca_ffmpeg};
use joypp::exporter::render::json::{Valore, analizza};
use joypp::exporter::render::profilo::{
    Ambiente, Dati, Fasi, Memoria, MisureFfmpeg, MisureScena, MisureVideo, PerFotogramma, Prestazioni,
    Profilo, SCHEMA, Statistica, verifica_percorso_profilo,
};
use joypp::exporter::render::{Evento, OpzioniVideo, render_file};

// ---------------------------------------------------------------- utilità

fn args(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

fn cartella(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("joypp-test-profilo-{nome}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn profilo_base() -> Profilo {
    Profilo {
        schema: SCHEMA.to_string(),
        ambiente: Ambiente {
            joypp: "0.2.1".into(),
            build: "release".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
            thread_logici: None,
            browser: None,
            ffmpeg: None,
        },
        scena: MisureScena { attori: 1, gruppi: 0, animazioni: 1, byte_html: 100, durata_ms: 500.0 },
        video: MisureVideo {
            fotogrammi: 6,
            larghezza: 64,
            altezza: 36,
            fps: 10,
            crf: 18,
            preset: "medium".into(),
            byte: 1000,
        },
        fasi_ms: Fasi::default(),
        fotogramma_ms: PerFotogramma::default(),
        ffmpeg: MisureFfmpeg { scrittura_pipe_ms: 0.0, coda_capacita: 4, fotogrammi_con_coda_piena: 0 },
        dati: Dati { byte_png_totali: 0, byte_png_max: 0, byte_base64_totali: 0 },
        prestazioni: Prestazioni { fps_ciclo: None, fps_totale: None, rapporto_tempo_reale: None },
        memoria: Memoria {
            picco_processo_byte: None,
            picco_processo_fonte: "non disponibile".into(),
            stima_buffer_fotogrammi_byte: 0,
        },
    }
}

fn chiavi(v: &Valore) -> Vec<&str> {
    match v {
        Valore::Oggetto(c) => c.iter().map(|(k, _)| k.as_str()).collect(),
        _ => panic!("oggetto atteso"),
    }
}

// ---------------------------------------------------------------- CLI (senza browser)

#[test]
fn cli_profilo_e_profilo_json_impostano_le_misure() {
    let Comando::Render(r) = interpreta(args("render s.joy --profilo")).unwrap() else { panic!() };
    assert!(r.profilo_testo && r.profilo_json.is_none() && r.video.profilo);
    let Comando::Render(r) = interpreta(args("render s.joy --profilo-json p.json")).unwrap() else { panic!() };
    assert!(!r.profilo_testo && r.profilo_json.as_deref() == Some(Path::new("p.json")) && r.video.profilo);
    let Comando::Render(r) = interpreta(args("render s.joy --fps 30 --crf 20")).unwrap() else { panic!() };
    assert!(!r.video.profilo, "senza le opzioni le misure sono spente");
}

#[test]
fn cli_ultima_vince_e_ordine_indifferente() {
    let Comando::Render(r) =
        interpreta(args("render s.joy --profilo-json a.json --profilo --profilo-json b.json -o v.mp4")).unwrap()
    else {
        panic!()
    };
    assert_eq!(r.profilo_json, Some(PathBuf::from("b.json")));
    assert!(r.profilo_testo);
    let Comando::Render(r) = interpreta(args("render --profilo s.joy")).unwrap() else { panic!() };
    assert!(r.profilo_testo && r.ingresso == Path::new("s.joy"));
}

#[test]
fn cli_errori_d_uso() {
    let e = interpreta(args("render s.joy --profilo-json")).unwrap_err();
    assert_eq!(e.messaggio, "l'opzione --profilo-json richiede un valore");
    let e = interpreta(args("render s.joy --profilo-json=x.json")).unwrap_err();
    assert!(e.messaggio.contains("--profilo-json=x.json"), "{}", e.messaggio);
    let e = interpreta(args("s.joy --profilo")).unwrap_err();
    assert!(e.messaggio.contains("vale solo con «joypp render»"));
    let e = interpreta(args("--profilo-json x.json s.joy")).unwrap_err();
    assert!(e.messaggio.contains("vale solo con «joypp render»"));
}

/// Gli errori d'uso terminano con exit 2 e mostrano l'aiuto; `-h` elenca le due opzioni.
#[test]
fn binario_exit_2_e_aiuto() {
    let exe = env!("CARGO_BIN_EXE_joypp");
    for a in [&["s.joy", "--profilo"][..], &["render", "s.joy", "--profilo-json"][..]] {
        let o = Command::new(exe).args(a).output().unwrap();
        assert_eq!(o.status.code(), Some(2), "{a:?}");
        let e = String::from_utf8_lossy(&o.stderr);
        assert!(e.contains("--profilo"), "l'aiuto deve seguire l'errore: {e}");
    }
    let o = Command::new(exe).arg("-h").output().unwrap();
    let t = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(t.contains("--profilo") && t.contains("--profilo-json"), "{t}");
}

// ---------------------------------------------------------------- Statistica

#[test]
fn statistica_casi_di_contorno() {
    // n = 100: p50 = 50° valore, p95 = 95° valore.
    let v: Vec<u64> = (1..=100).map(|i| i * 1000).collect();
    let s = Statistica::da_microsecondi(&v);
    assert_eq!((s.p50, s.p95, s.min, s.max), (50.0, 95.0, 1.0, 100.0));
    // n = 2: p50 = 1° valore (ceil(1) = 1), p95 = 2°.
    let s = Statistica::da_microsecondi(&[9000, 1000]);
    assert_eq!((s.p50, s.p95), (1.0, 9.0));
    // n = 3: p50 = ceil(1.5) = 2° valore.
    let s = Statistica::da_microsecondi(&[30, 10, 20]);
    assert_eq!(s.p50, 0.02);
    // Valori uguali; microsecondi non multipli di 1000.
    let s = Statistica::da_microsecondi(&[1500, 1500, 1500]);
    assert_eq!((s.min, s.media, s.max, s.totale), (1.5, 1.5, 1.5, 4.5));
    // Somma grande senza overflow.
    let s = Statistica::da_microsecondi(&[u64::MAX, u64::MAX]);
    assert!(s.totale.is_finite() && s.media.is_finite());
    assert_eq!(Statistica::da_microsecondi(&[]).p95, 0.0);
}

// ---------------------------------------------------------------- Profilo::json / testo

#[test]
fn json_chiavi_di_primo_livello_e_sezioni() {
    let j = analizza(&profilo_base().json()).unwrap();
    assert_eq!(
        chiavi(&j),
        ["schema", "ambiente", "scena", "video", "fasi_ms", "fotogramma_ms", "ffmpeg", "dati", "prestazioni", "memoria"]
    );
    assert_eq!(j.campo("schema").unwrap().testo(), Some("joypp-profilo/1"));
    assert_eq!(
        chiavi(j.campo("ambiente").unwrap()),
        ["joypp", "build", "os", "arch", "thread_logici", "browser", "ffmpeg"]
    );
    assert_eq!(chiavi(j.campo("scena").unwrap()), ["attori", "gruppi", "animazioni", "byte_html", "durata_ms"]);
    assert_eq!(
        chiavi(j.campo("video").unwrap()),
        ["fotogrammi", "larghezza", "altezza", "fps", "crf", "preset", "byte"]
    );
    assert_eq!(
        chiavi(j.campo("fasi_ms").unwrap()),
        [
            "lettura", "analisi", "compilazione", "generazione_html", "preparazione", "avvio_browser",
            "connessione_cdp", "caricamento_pagina", "avvio_ffmpeg", "ciclo_fotogrammi", "chiusura_browser",
            "finalizzazione_ffmpeg", "pubblicazione", "totale"
        ]
    );
    let pf = j.campo("fotogramma_ms").unwrap();
    assert_eq!(chiavi(pf), ["seek", "screenshot", "base64", "verifica_png", "attesa_coda"]);
    assert_eq!(chiavi(pf.campo("seek").unwrap()), ["totale", "media", "min", "p50", "p95", "max"]);
    assert_eq!(
        chiavi(j.campo("ffmpeg").unwrap()),
        ["scrittura_pipe_ms", "coda_capacita", "fotogrammi_con_coda_piena"]
    );
    assert_eq!(chiavi(j.campo("dati").unwrap()), ["byte_png_totali", "byte_png_max", "byte_base64_totali"]);
    assert_eq!(
        chiavi(j.campo("prestazioni").unwrap()),
        ["fps_ciclo", "fps_totale", "rapporto_tempo_reale"]
    );
    assert_eq!(
        chiavi(j.campo("memoria").unwrap()),
        ["picco_processo_byte", "picco_processo_fonte", "stima_buffer_fotogrammi_byte"]
    );
    // null dove indicato.
    assert_eq!(j.campo("ambiente").unwrap().campo("thread_logici"), Some(&Valore::Null));
    assert_eq!(j.campo("ambiente").unwrap().campo("browser"), Some(&Valore::Null));
    assert_eq!(j.campo("prestazioni").unwrap().campo("fps_ciclo"), Some(&Valore::Null));
}

#[test]
fn json_escape_di_stringhe_ostili_con_andata_e_ritorno() {
    let ostile = "Chrome \"x\" \\ </script><b>\n\r\t\u{1}\u{1f} fine";
    let mut p = profilo_base();
    p.ambiente.browser = Some(ostile.to_string());
    p.ambiente.ffmpeg = Some("ffmpeg \\ \"v\"".to_string());
    let testo = p.json();
    assert!(!testo.contains('\n') && !testo.contains('\r') && !testo.contains('\t'), "newline grezzi nel JSON");
    assert!(!testo.chars().any(|c| (c as u32) < 0x20), "caratteri di controllo non escapati");
    let j = analizza(&testo).expect("JSON con stringhe ostili deve restare valido");
    assert_eq!(j.campo("ambiente").unwrap().campo("browser").unwrap().testo(), Some(ostile));
    assert_eq!(j.campo("ambiente").unwrap().campo("ffmpeg").unwrap().testo(), Some("ffmpeg \\ \"v\""));
}

#[test]
fn json_numeri_non_finiti_diventano_null() {
    let mut p = profilo_base();
    p.fasi_ms.totale = f64::NAN;
    p.scena.durata_ms = f64::INFINITY;
    p.prestazioni.fps_ciclo = Some(f64::NAN);
    p.prestazioni.rapporto_tempo_reale = Some(f64::NEG_INFINITY);
    let t = p.json();
    assert!(!t.contains("NaN") && !t.contains("inf"), "{t}");
    let j = analizza(&t).unwrap();
    assert_eq!(j.campo("fasi_ms").unwrap().campo("totale"), Some(&Valore::Null));
    assert_eq!(j.campo("scena").unwrap().campo("durata_ms"), Some(&Valore::Null));
    assert_eq!(j.campo("prestazioni").unwrap().campo("fps_ciclo"), Some(&Valore::Null));
    assert_eq!(j.campo("prestazioni").unwrap().campo("rapporto_tempo_reale"), Some(&Valore::Null));
}

#[test]
fn json_e_testo_senza_percorsi() {
    let p = profilo_base();
    let t = p.json();
    for vietato in ["\\\\", "C:", ".mp4", ".joy", "Users", "joypp-render-"] {
        assert!(!t.contains(vietato), "percorso nel JSON ({vietato}): {t}");
    }
    let r = p.testo();
    for vietato in ["\\", "C:", ".mp4", ".joy", "Users", "joypp-render-"] {
        assert!(!r.contains(vietato), "percorso nel rapporto ({vietato}): {r}");
    }
}

#[test]
fn testo_stima_e_non_disponibile() {
    let r = profilo_base().testo();
    assert!(r.lines().next().unwrap().starts_with("Profilo del render (joypp 0.2.1, release, windows x86_64"));
    assert!(r.contains("(stima)"));
    assert!(r.contains("non disponibile"));
    // Con memoria misurata la riga del picco riporta il valore.
    let mut p = profilo_base();
    p.memoria.picco_processo_byte = Some(12345);
    p.memoria.picco_processo_fonte = "VmHWM".into();
    let r = p.testo();
    assert!(r.contains("12345"), "{r}");
    assert!(r.contains("(stima)"), "la stima del buffer resta etichettata");
}

// ---------------------------------------------------------------- verifica_percorso_profilo

#[test]
fn percorso_profilo_normalizzazione() {
    let d = cartella("percorsi");
    let sorgente = d.join("scena.joy");
    fs::write(&sorgente, "x").unwrap();
    let video = d.join("v.mp4");
    // Percorso diverso: accettato.
    assert!(verifica_percorso_profilo(&sorgente, &video, &d.join("p.json")).is_ok());
    // Stesso file scritto con `..` e con `.`.
    fs::create_dir_all(d.join("sub")).unwrap();
    let alias = d.join("sub").join("..").join("scena.joy");
    let e = verifica_percorso_profilo(&sorgente, &video, &alias).unwrap_err();
    assert!(
        e.messaggio.starts_with("il file del profilo deve essere diverso dal sorgente e dal video («"),
        "{}",
        e.messaggio
    );
    assert!(verifica_percorso_profilo(&sorgente, &video, &d.join(".").join("scena.joy")).is_err());
    // Coincidenza con il video, che non esiste ancora (confronto letterale).
    assert!(verifica_percorso_profilo(&sorgente, &video, &video).is_err());
    // Profilo dentro una cartella inesistente: va bene (sarà creata).
    assert!(verifica_percorso_profilo(&sorgente, &video, &d.join("nuova").join("p.json")).is_ok());
    fs::remove_dir_all(&d).ok();
}

/// Senza browser: il conflitto è rilevato prima di avviare qualsiasi processo (exit 1).
#[test]
fn binario_rifiuta_profilo_uguale_al_sorgente_o_al_video() {
    let d = cartella("conflitto");
    let sorgente = d.join("scena.joy");
    let contenuto = "Prepara un palco grande 64 per 36 con sfondo #0F172A.\n";
    fs::write(&sorgente, contenuto).unwrap();
    let video = d.join("v.mp4");
    let exe = env!("CARGO_BIN_EXE_joypp");
    for conflitto in [&sorgente, &video] {
        let o = Command::new(exe)
            .args(["render", sorgente.to_str().unwrap(), "-o", video.to_str().unwrap(), "--profilo-json"])
            .arg(conflitto)
            .output()
            .unwrap();
        assert_eq!(o.status.code(), Some(1), "{}", String::from_utf8_lossy(&o.stderr));
        let e = String::from_utf8_lossy(&o.stderr);
        assert!(e.contains("il file del profilo deve essere diverso dal sorgente e dal video"), "{e}");
        assert!(!video.exists());
    }
    assert_eq!(fs::read_to_string(&sorgente).unwrap(), contenuto, "il sorgente non va toccato");
    fs::remove_dir_all(&d).ok();
}

// ---------------------------------------------------------------- non intrusività (senza browser)

#[test]
fn argomenti_ffmpeg_non_dipendono_dal_profilo() {
    let p = Path::new("/o/v.mp4.joypp-parziale");
    let a = argomenti_ffmpeg(1280, 720, 60, 18, [1, 2, 3], p);
    let b = argomenti_ffmpeg(1280, 720, 60, 18, [1, 2, 3], p);
    assert_eq!(a, b);
    let testo: Vec<String> = a.iter().map(|s| s.to_string_lossy().into_owned()).collect();
    assert!(!testo.iter().any(|s| s.contains("profil")), "{testo:?}");
    assert_eq!(&testo[..4], ["-hide_banner", "-loglevel", "error", "-nostats"]);
    assert!(testo.last().unwrap().starts_with("file:"));
    assert!(testo.windows(2).any(|w| w == ["-preset", "medium"]));
    assert!(testo.windows(2).any(|w| w == ["-crf", "18"]));
}

#[test]
fn opzioni_video_predefinite_senza_misure() {
    let o = OpzioniVideo::default();
    assert!(!o.profilo);
    assert_eq!(o.crf, 18);
}

// ---------------------------------------------------------------- con browser e ffmpeg

fn ambiente(k: &str) -> Option<String> {
    std::env::var(k).ok()
}

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

/// `true` se il test deve proseguire; altrimenti stampa «test saltato» (o fallisce in CI).
fn si_puo_renderizzare() -> bool {
    match manca_qualcosa() {
        None => true,
        Some(m) => {
            if ambiente("JOYPP_TEST_RENDER").as_deref() == Some("1") {
                panic!("render non eseguibile con JOYPP_TEST_RENDER=1: {m}");
            }
            eprintln!("test saltato: {m}");
            false
        }
    }
}

/// I test con render reali sono serializzati: confrontano le cartelle `joypp-render-*` della temp.
static SERIALE: Mutex<()> = Mutex::new(());

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

const SCENA: &str = "Prepara un palco grande 64 per 36 con sfondo #0F172A.\nCrea un cerchio chiamato \"a\" con raggio 10 e colore #F59E0B al centro.\nDal secondo 0 al secondo 0.5:\n  \"a\" appare sfumando.\n";

fn senza_residui(d: &Path) {
    for e in fs::read_dir(d).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        assert!(!n.ends_with(".joypp-parziale"), "parziale residuo: {n}");
    }
}

/// Render in-process con e senza profilo: `EsitoVideo.profilo` è `Some` se e solo se attivo.
#[test]
fn esito_video_profilo_some_solo_se_attivo() {
    if !si_puo_renderizzare() {
        return;
    }
    let _serie = SERIALE.lock().unwrap_or_else(|e| e.into_inner());
    let d = cartella("esito");
    let src = d.join("s.joy");
    fs::write(&src, SCENA).unwrap();
    let prima = cartelle_render();
    let mut progresso = |_e: Evento| {};
    let con = render_file(&src, &d.join("a.mp4"), &OpzioniVideo { fps: Some(10), crf: 18, profilo: true }, &mut progresso)
        .unwrap();
    let senza = render_file(&src, &d.join("b.mp4"), &OpzioniVideo { fps: Some(10), crf: 18, profilo: false }, &mut progresso)
        .unwrap();
    assert!(senza.profilo.is_none());
    let p = con.profilo.as_ref().expect("profilo atteso");
    assert_eq!(con.fotogrammi, 6);
    assert_eq!(p.schema, SCHEMA);
    assert_eq!(p.video.fotogrammi, 6);
    assert_eq!((p.video.larghezza, p.video.altezza, p.video.fps, p.video.crf), (64, 36, 10, 18));
    assert_eq!(p.video.preset, "medium");
    assert_eq!(p.video.byte, con.byte);
    assert!(p.fotogramma_ms.seek.totale > 0.0);
    assert!(p.fasi_ms.totale >= p.fasi_ms.ciclo_fotogrammi && p.fasi_ms.ciclo_fotogrammi > 0.0);
    assert!(p.dati.byte_png_max > 0 && p.dati.byte_png_totali >= p.dati.byte_png_max);
    assert_eq!(p.ffmpeg.coda_capacita, 4);
    assert_eq!(p.memoria.stima_buffer_fotogrammi_byte, 6 * p.dati.byte_png_max);
    assert!(p.ambiente.browser.is_some() && p.ambiente.ffmpeg.is_some());
    assert!(p.prestazioni.fps_ciclo.is_some_and(f64::is_finite));
    // Con e senza misure: stessi fotogrammi e dimensioni.
    assert_eq!((con.fotogrammi, con.larghezza, con.altezza), (senza.fotogrammi, senza.larghezza, senza.altezza));
    assert!(analizza(&p.json()).is_ok());
    senza_residui(&d);
    let dopo = cartelle_render();
    let nuove: Vec<_> = dopo.difference(&prima).collect();
    assert!(nuove.is_empty(), "cartelle joypp-render-* residue: {nuove:?}");
    fs::remove_dir_all(&d).ok();
}

/// `joypp render --profilo-json` + `--profilo`: file JSON valido, rapporto su stderr, nessun parziale.
#[test]
fn binario_scrive_il_json_e_il_rapporto() {
    if !si_puo_renderizzare() {
        return;
    }
    let _serie = SERIALE.lock().unwrap_or_else(|e| e.into_inner());
    let d = cartella("binario");
    let src = d.join("s.joy");
    fs::write(&src, SCENA).unwrap();
    let video = d.join("v.mp4");
    let json = d.join("annidata").join("p.json");
    let prima = cartelle_render();
    let o = Command::new(env!("CARGO_BIN_EXE_joypp"))
        .args(["render", src.to_str().unwrap(), "-o", video.to_str().unwrap(), "--fps", "10", "--profilo", "--profilo-json"])
        .arg(&json)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout).into_owned();
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    assert!(o.status.success(), "{err}");
    assert!(out.contains("6 fotogrammi, 64×36, 10 fps"), "{out}");
    assert!(err.contains("Profilo del render (joypp "), "{err}");
    assert!(!err.contains(d.to_str().unwrap()) || err.matches(d.to_str().unwrap()).count() == 0);
    let testo = fs::read_to_string(&json).unwrap();
    let j = analizza(&testo).expect("JSON non valido");
    assert_eq!(j.campo("schema").unwrap().testo(), Some("joypp-profilo/1"));
    assert_eq!(j.campo("video").unwrap().campo("fotogrammi").unwrap().numero(), Some(6.0));
    assert!(!testo.contains(d.to_str().unwrap()), "il JSON contiene un percorso");
    senza_residui(&d);
    senza_residui(&d.join("annidata"));
    assert!(cartelle_render().difference(&prima).next().is_none());
    // Senza opzioni: nessun rapporto.
    let video2 = d.join("v2.mp4");
    let o = Command::new(env!("CARGO_BIN_EXE_joypp"))
        .args(["render", src.to_str().unwrap(), "-o", video2.to_str().unwrap(), "--fps", "10"])
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(!String::from_utf8_lossy(&o.stderr).contains("Profilo del render"), "senza opzioni nessun rapporto");
    fs::remove_dir_all(&d).ok();
}

/// Pre-validazione D1: un `--profilo-json` che è una cartella esistente dà exit 1 PRIMA di avviare
/// processi: nessun video, nessun parziale, nessuna cartella `joypp-render-*`, browser non avviato.
/// Non richiede browser, quindi non viene mai saltato. L'errore di scrittura DOPO il render resta
/// coperto dal test unitario `scrittura_fallita_non_lascia_parziali` di `profilo.rs`.
#[test]
fn binario_profilo_json_cartella_errore_prima_dei_processi() {
    let _serie = SERIALE.lock().unwrap_or_else(|e| e.into_inner());
    let d = cartella("scrittura");
    let src = d.join("s.joy");
    fs::write(&src, SCENA).unwrap();
    let video = d.join("v.mp4");
    let json = d.join("occupato.json");
    fs::create_dir_all(&json).unwrap();
    let prima = cartelle_render();
    let o = Command::new(env!("CARGO_BIN_EXE_joypp"))
        .args(["render", src.to_str().unwrap(), "-o", video.to_str().unwrap(), "--fps", "10", "--profilo-json"])
        .arg(&json)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(err.contains("impossibile scrivere «") && err.contains("è una cartella"), "{err}");
    assert!(!video.exists(), "nessun video deve essere creato");
    assert!(json.is_dir());
    senza_residui(&d);
    assert!(cartelle_render().difference(&prima).next().is_none(), "il browser non doveva partire");
    fs::remove_dir_all(&d).ok();
}

/// Regressione B1: il JSON non deve mai poter sovrascrivere il video, comunque sia scritto il
/// percorso (confronto lessicale perché il video non esiste ancora). Nessun browser avviato.
#[test]
fn binario_b1_profilo_json_uguale_al_video_non_lo_sovrascrive() {
    let _serie = SERIALE.lock().unwrap_or_else(|e| e.into_inner());
    let mut varianti: Vec<&str> = vec!["./out.mp4", "nuova/../out.mp4", "out.mp4"];
    if cfg!(windows) {
        varianti.push("./OUT.MP4");
    }
    for preesistente in [false, true] {
        for v in &varianti {
            let d = cartella("b1");
            fs::write(d.join("s.joy"), SCENA).unwrap();
            let vecchio = b"vecchio video non toccare";
            if preesistente {
                fs::write(d.join("out.mp4"), vecchio).unwrap();
            }
            let prima = cartelle_render();
            let o = Command::new(env!("CARGO_BIN_EXE_joypp"))
                .current_dir(&d)
                .args(["render", "s.joy", "-o", "out.mp4", "--fps", "10", "--profilo-json", v])
                .output()
                .unwrap();
            let err = String::from_utf8_lossy(&o.stderr);
            assert_eq!(o.status.code(), Some(1), "{v} (preesistente={preesistente}): {err}");
            assert!(err.contains("il file del profilo deve essere diverso dal sorgente e dal video"), "{v}: {err}");
            if preesistente {
                assert_eq!(fs::read(d.join("out.mp4")).unwrap(), vecchio, "{v}: il video preesistente è cambiato");
            } else {
                assert!(!d.join("out.mp4").exists(), "{v}: out.mp4 non deve esistere");
            }
            senza_residui(&d);
            assert!(!d.join("nuova").exists(), "{v}: nessuna cartella creata");
            assert!(cartelle_render().difference(&prima).next().is_none(), "{v}: il browser non doveva partire");
            fs::remove_dir_all(&d).ok();
        }
    }
}
