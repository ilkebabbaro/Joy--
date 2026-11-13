//! Test end-to-end: esempi in `examples/`, regressioni in `tests/repro/` e CLI.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use joypp::{Opzioni, compila_sorgente};

fn radice() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn cartella_temporanea(nome: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("joypp-test-{nome}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn tutti_gli_esempi_compilano() {
    let mut trovati = 0;
    for voce in fs::read_dir(radice().join("examples")).unwrap() {
        let percorso = voce.unwrap().path();
        if percorso.extension().is_some_and(|e| e == "joy") {
            let src = fs::read_to_string(&percorso).unwrap();
            let html = compila_sorgente(&src, &Opzioni::default())
                .unwrap_or_else(|e| panic!("{}: {e}", percorso.display()));
            assert!(html.starts_with("<!DOCTYPE html>"));
            trovati += 1;
        }
    }
    assert!(trovati >= 1);
}

#[test]
fn primo_test_contiene_gli_elementi_attesi() {
    let src = fs::read_to_string(radice().join("examples/primo_test.joy")).unwrap();
    let html = compila_sorgente(&src, &Opzioni::default()).unwrap();
    // Auto-contenuto: nessuna risorsa esterna.
    for vietato in ["http://", "https://", "<link", "src=", "@import"] {
        assert!(!html.contains(vietato), "risorsa esterna trovata: {vietato}");
    }
    assert!(html.contains("#palco{position:relative;flex:none;width:1280px;height:720px;background:#0f172a"));
    assert!(html.contains("data-fps=\"60\""));
    assert!(html.contains("@keyframes a0-scala"));
    assert!(html.contains("@keyframes a1-traslazione"));
    assert!(html.contains("SISTEMA ORBITALE"));
    assert!(html.contains("&lt;animazioni in italiano&gt;"));
    // Solo proprietà composite animate.
    for k in html.split("@keyframes").skip(1) {
        let corpo = k.split("}}").next().unwrap_or_default();
        assert!(!corpo.contains("left:") && !corpo.contains("top:") && !corpo.contains("width:"));
    }
}

#[test]
fn output_leggibile_e_compatto_equivalenti() {
    let src = fs::read_to_string(radice().join("examples/primo_test.joy")).unwrap();
    let compatto = compila_sorgente(&src, &Opzioni { leggibile: false }).unwrap();
    let leggibile = compila_sorgente(&src, &Opzioni { leggibile: true }).unwrap();
    assert!(leggibile.len() > compatto.len());
    assert_eq!(compatto.lines().count(), 1);
    let senza_spazi = |s: &str| s.lines().map(str::trim_start).collect::<String>();
    assert_eq!(senza_spazi(&leggibile), compatto);
}

#[test]
fn file_di_regressione_in_repro() {
    // Ogni file in tests/repro/ dichiara nella prima riga l'esito atteso:
    // "# atteso: ok" oppure "# atteso: errore riga N".
    for voce in fs::read_dir(radice().join("tests/repro")).unwrap() {
        let percorso = voce.unwrap().path();
        if percorso.extension().is_none_or(|e| e != "joy") {
            continue;
        }
        let src = fs::read_to_string(&percorso).unwrap();
        let atteso = src.lines().next().unwrap_or_default().trim_start_matches("# atteso:").trim().to_string();
        let esito = compila_sorgente(&src, &Opzioni::default());
        match atteso.strip_prefix("errore riga ") {
            Some(riga) => {
                let e = esito.expect_err(&percorso.display().to_string());
                assert_eq!(e.riga, riga.parse().ok(), "{}: {e}", percorso.display());
            }
            None => {
                assert_eq!(atteso, "ok", "{}", percorso.display());
                esito.unwrap_or_else(|e| panic!("{}: {e}", percorso.display()));
            }
        }
    }
}

fn joypp(argomenti: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_joypp")).args(argomenti).output().unwrap()
}

#[test]
fn cli_compila_su_file() {
    let dir = cartella_temporanea("cli");
    let uscita = dir.join("output.html");
    let esempio = radice().join("examples/primo_test.joy");
    let out = joypp(&[esempio.to_str().unwrap(), "-o", uscita.to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let html = fs::read_to_string(&uscita).unwrap();
    assert!(html.contains("SISTEMA ORBITALE"));
    fs::remove_dir_all(dir).ok();
}

#[test]
fn cli_errore_di_compilazione_esce_con_codice_1() {
    let dir = cartella_temporanea("cli-errore");
    let sorgente = dir.join("rotto.joy");
    fs::write(&sorgente, "Prepara un palco grande 10 per 10.\nCrea un cerchio chiamato \"a\" con colore biancho.\n").unwrap();
    let out = joypp(&[sorgente.to_str().unwrap(), "-o", dir.join("x.html").to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Errore alla riga 2"));
    assert!(stderr.contains("forse intendevi «bianco»?"));
    assert!(!dir.join("x.html").exists());
    fs::remove_dir_all(dir).ok();
}

#[test]
fn cli_non_sovrascrive_il_sorgente() {
    let dir = cartella_temporanea("cli-sovrascrittura");
    let sorgente = dir.join("s.joy");
    fs::write(&sorgente, "Prepara un palco grande 10 per 10.\n").unwrap();
    let out = joypp(&[sorgente.to_str().unwrap(), "-o", sorgente.to_str().unwrap()]);
    assert!(!out.status.success());
    assert_eq!(fs::read_to_string(&sorgente).unwrap(), "Prepara un palco grande 10 per 10.\n");
    fs::remove_dir_all(dir).ok();
}

#[test]
fn cli_argomenti_non_validi() {
    assert_eq!(joypp(&[]).status.code(), Some(2));
    assert_eq!(joypp(&["--boh"]).status.code(), Some(2));
    assert!(joypp(&["--help"]).status.success());
    let versione = joypp(&["--version"]);
    assert!(String::from_utf8_lossy(&versione.stdout).contains(env!("CARGO_PKG_VERSION")));
    assert!(!joypp(&[Path::new("non_esiste.joy").to_str().unwrap()]).status.success());
}
