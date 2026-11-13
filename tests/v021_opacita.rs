//! Test v0.2.1: semantica di `appare sfumando` con `opacità 0` (A.3, B.3.5) e avviso
//! «opacità invariata» (B.6), dalla frase `.joy` all'HTML.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use joypp::compiler::{ScenaCompilata, compila};
use joypp::exporter::compila_con_avvisi;
use joypp::parser::analizza;
use joypp::{Avviso, Opzioni};

const PALCO: &str = "Prepara un palco grande 100 per 100.\n";

fn radice() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scena(src: &str) -> ScenaCompilata {
    compila(&analizza(src).expect("parser")).expect("compilatore")
}

fn avvisi(src: &str) -> Vec<Avviso> {
    scena(src).avvisi
}

fn html(src: &str) -> String {
    compila_con_avvisi(src, &Opzioni::default()).expect("compilazione").html
}

/// Corpo del primo `@keyframes aN-opacita{...}` dell'HTML (senza le graffe esterne finali).
fn keyframe_opacita(html: &str, n: usize) -> String {
    let marca = format!("@keyframes a{n}-opacita{{");
    let inizio = html.find(&marca).unwrap_or_else(|| panic!("manca {marca}"));
    let resto = &html[inizio + marca.len()..];
    resto.split("}}").next().unwrap().to_string()
}

const SUGG_APPARE: &str = "togli «appare sfumando», oppure fallo prima scomparire con «scompare sfumando»";
const SUGG_SCOMPARE: &str = "togli «scompare sfumando», oppure fallo prima apparire con «appare sfumando»";
const SUGG_CAMBIA: &str = "togli «cambia opacità a …», oppure scegli un valore diverso da quello attuale";

// ------------------------------------------------------------ regressione del bug

#[test]
fn testo_opacita_zero_appare_arriva_a_uno() {
    let src = format!(
        "{PALCO}Crea un testo chiamato \"t\" con testo \"ciao\" e opacità 0.\n\
         Dal secondo 0 al secondo 1:\n  \"t\" appare sfumando.\n"
    );
    let s = scena(&src);
    assert!(s.avvisi.is_empty(), "{:?}", s.avvisi);
    let k = keyframe_opacita(&html(&src), 0);
    assert!(k.starts_with("0%{opacity:0;"), "{k}");
    assert!(k.ends_with("100%{opacity:1"), "{k}");
}

#[test]
fn gruppo_opacita_zero_appare_arriva_a_uno() {
    let src = format!(
        "{PALCO}Crea un testo chiamato \"t\" con testo \"ciao\".\n\
         Crea un gruppo chiamato \"g\" con \"t\", opacità 0.\n\
         Dal secondo 0 al secondo 1:\n  \"g\" appare sfumando.\n"
    );
    let s = scena(&src);
    assert!(s.avvisi.is_empty(), "{:?}", s.avvisi);
    let h = html(&src);
    // Il gruppo è l'unico attore con animazione di opacità.
    assert_eq!(h.matches("-opacita{").count(), 1, "{h}");
    let k = keyframe_opacita(&h, 0);
    assert!(k.starts_with("0%{opacity:0;") && k.ends_with("100%{opacity:1"), "{k}");
}

#[test]
fn opacita_dichiarata_maggiore_di_zero_resta_il_valore_dichiarato() {
    let src = format!(
        "{PALCO}Crea un testo chiamato \"t\" con testo \"ciao\" e opacità 0.6.\n\
         Dal secondo 0 al secondo 1:\n  \"t\" appare sfumando.\n"
    );
    assert!(avvisi(&src).is_empty());
    let k = keyframe_opacita(&html(&src), 0);
    assert!(k.starts_with("0%{opacity:0;") && k.ends_with("100%{opacity:0.6"), "{k}");
    // Un gruppo con opacità 0.6 si comporta allo stesso modo.
    let g = format!(
        "{PALCO}Crea un testo chiamato \"t\" con testo \"x\".\n\
         Crea un gruppo chiamato \"g\" con \"t\", opacità 0.6.\n\
         Dal secondo 0 al secondo 1:\n  \"g\" appare sfumando.\n"
    );
    assert!(avvisi(&g).is_empty());
    assert!(keyframe_opacita(&html(&g), 0).ends_with("100%{opacity:0.6"));
}

#[test]
fn opacita_uno_dichiarata_va_a_uno() {
    let src = format!(
        "{PALCO}Crea un testo chiamato \"t\" con testo \"x\" e opacità 1.\n\
         Dal secondo 0 al secondo 1:\n  \"t\" appare sfumando.\n"
    );
    assert!(avvisi(&src).is_empty());
    assert!(keyframe_opacita(&html(&src), 0).ends_with("100%{opacity:1"));
}

// ------------------------------------------------------------ repro dalla sfida

#[test]
fn repro_appare_opacita_zero_contenuto_html() {
    let src = fs::read_to_string(radice().join("tests/repro/appare_opacita_zero.joy")).unwrap();
    let c = compila_con_avvisi(&src, &Opzioni::default()).unwrap();
    assert!(c.avvisi.is_empty(), "{:?}", c.avvisi);
    // 4 attori animati sull'opacità: g, c, a, b. Tutti vanno da 0 a 1.
    assert_eq!(c.html.matches("-opacita{").count(), 4);
    let mut trovati = 0;
    for pezzo in c.html.split("@keyframes ").skip(1) {
        let (nome, corpo) = pezzo.split_once('{').unwrap();
        if nome.ends_with("-opacita") {
            let corpo = corpo.split("}}").next().unwrap();
            // Gli attori che iniziano dopo il secondo 0 hanno prima un'attesa a 0 (`0%{opacity:0}`).
            assert!(corpo.starts_with("0%{opacity:0"), "{nome}: {corpo}");
            assert!(corpo.ends_with("100%{opacity:1"), "{nome}: {corpo}");
            trovati += 1;
        }
    }
    assert_eq!(trovati, 4);
}

// ------------------------------------------------------------ avvisi B.6

#[test]
fn avviso_appare_due_volte_di_seguito() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Dal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n\
         Dal secondo 1 al secondo 2:\n  \"c\" appare sfumando.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 6);
    assert_eq!(
        a[0].messaggio,
        "\"c\" appare sfumando ma la sua opacità è già 1: l'azione non cambia nulla"
    );
    assert_eq!(a[0].suggerimento.as_deref(), Some(SUGG_APPARE));
}

#[test]
fn avviso_appare_con_opacita_dichiarata_scrive_il_valore() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\" con opacità 0.6.\n\
         Dal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n\
         Dal secondo 1 al secondo 2:\n  \"c\" appare sfumando.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 6);
    assert_eq!(
        a[0].messaggio,
        "\"c\" appare sfumando ma la sua opacità è già 0.6: l'azione non cambia nulla"
    );
}

#[test]
fn avviso_appare_con_opacita_zero_due_volte_dice_uno() {
    // Con `opacità 0` il valore di arrivo di `appare` è 1 (A.3): il secondo `appare` è già a 1.
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\" con opacità 0.\n\
         Dal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n\
         Dal secondo 1 al secondo 2:\n  \"c\" appare sfumando.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(
        a[0].messaggio,
        "\"c\" appare sfumando ma la sua opacità è già 1: l'azione non cambia nulla"
    );
}

#[test]
fn avviso_scompare_su_attore_gia_scomparso() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Dal secondo 0 al secondo 1:\n  \"c\" scompare sfumando.\n\
         Dal secondo 1 al secondo 2:\n  \"c\" scompare sfumando.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 6);
    assert_eq!(
        a[0].messaggio,
        "\"c\" scompare sfumando ma la sua opacità è già 0: l'azione non cambia nulla"
    );
    assert_eq!(a[0].suggerimento.as_deref(), Some(SUGG_SCOMPARE));
}

#[test]
fn avviso_scompare_su_attore_creato_con_opacita_zero() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\" con opacità 0.\n\
         Dal secondo 0 al secondo 1:\n  \"c\" scompare sfumando.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 4);
    assert_eq!(a[0].suggerimento.as_deref(), Some(SUGG_SCOMPARE));
}

#[test]
fn avviso_cambia_opacita_a_valore_attuale() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Dal secondo 0 al secondo 1:\n  \"c\" cambia opacità a 1.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 4);
    assert_eq!(
        a[0].messaggio,
        "\"c\" cambia opacità a 1 ma la sua opacità è già 1: l'azione non cambia nulla"
    );
    assert_eq!(a[0].suggerimento.as_deref(), Some(SUGG_CAMBIA));
}

#[test]
fn avviso_cambia_opacita_ripetuto_su_valore_frazionario() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Dal secondo 0 al secondo 1:\n  \"c\" cambia opacità a 0.5.\n\
         Dal secondo 1 al secondo 2:\n  \"c\" cambia opacità a 0.5.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 6);
    assert_eq!(
        a[0].messaggio,
        "\"c\" cambia opacità a 0.5 ma la sua opacità è già 0.5: l'azione non cambia nulla"
    );
}

#[test]
fn avviso_valore_corrente_resta_fermo_tra_due_segmenti_non_contigui() {
    // Tra i due segmenti c'è una pausa: il valore resta fermo e il secondo `scompare` avvisa.
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Dal secondo 0 al secondo 1:\n  \"c\" scompare sfumando.\n\
         Dal secondo 5 al secondo 6:\n  \"c\" scompare sfumando.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 6);
}

#[test]
fn avviso_su_gruppo() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Crea un gruppo chiamato \"g\" con \"c\".\n\
         Dal secondo 0 al secondo 1:\n  \"g\" cambia opacità a 1.\n"
    ));
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].riga, 5);
    assert!(a[0].messaggio.starts_with("\"g\" cambia opacità a 1 ma"), "{}", a[0].messaggio);
}

#[test]
fn avvisi_ordinati_per_riga_con_quello_del_cerchio_ruotato() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Crea un cerchio chiamato \"d\" con rotazione 5.\n\
         Dal secondo 0 al secondo 1:\n  \"c\" ruota di 5 gradi e cambia opacità a 1.\n  \"d\" scompare sfumando.\n\
         Dal secondo 1 al secondo 2:\n  \"d\" scompare sfumando.\n"
    ));
    let righe: Vec<usize> = a.iter().map(|x| x.riga).collect();
    assert_eq!(righe, vec![3, 5, 5, 8], "{a:?}");
    assert!(a[0].messaggio.contains("è un cerchio"), "{:?}", a[0]);
    assert!(a[1].messaggio.contains("è un cerchio") || a[1].messaggio.contains("cambia opacità"));
    assert!(a.iter().any(|x| x.riga == 5 && x.messaggio.contains("cambia opacità a 1")), "{a:?}");
    assert!(a[3].messaggio.contains("scompare sfumando ma"), "{:?}", a[3]);
}

#[test]
fn avviso_display_completo() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia opacità a 1.\n"
    ));
    assert_eq!(
        a[0].to_string(),
        format!(
            "Avviso alla riga 4: \"c\" cambia opacità a 1 ma la sua opacità è già 1: l'azione non cambia nulla\n  Suggerimento: {SUGG_CAMBIA}"
        )
    );
}

#[test]
fn compilazione_riesce_con_avvisi_di_opacita() {
    let src = format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia opacità a 1.\n"
    );
    let c = compila_con_avvisi(&src, &Opzioni::default()).unwrap();
    assert_eq!(c.avvisi.len(), 1);
    assert!(c.html.starts_with("<!DOCTYPE html>"));
}

// ------------------------------------------------------------ nessun avviso

#[test]
fn nessun_avviso_appare_come_prima_azione() {
    for decl in ["", " con opacità 1", " con opacità 0", " con opacità 0.4"] {
        let a = avvisi(&format!(
            "{PALCO}Crea un cerchio chiamato \"c\"{decl}.\nDal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n"
        ));
        assert!(a.is_empty(), "decl={decl:?}: {a:?}");
    }
}

#[test]
fn nessun_avviso_appare_scompare_appare() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Dal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n\
         Dal secondo 1 al secondo 2:\n  \"c\" scompare sfumando.\n\
         Dal secondo 2 al secondo 3:\n  \"c\" appare sfumando.\n"
    ));
    assert!(a.is_empty(), "{a:?}");
}

#[test]
fn nessun_avviso_cambia_opacita_a_valore_diverso() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia opacità a 0.5.\n"
    ));
    assert!(a.is_empty(), "{a:?}");
}

#[test]
fn nessun_avviso_per_canali_indipendenti_tra_gruppo_e_membro() {
    let a = avvisi(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Crea un gruppo chiamato \"g\" con \"c\", opacità 0.\n\
         Dal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n\
         Dal secondo 1 al secondo 2:\n  \"g\" appare sfumando.\n"
    ));
    assert!(a.is_empty(), "{a:?}");
}

// ------------------------------------------------------------ CLI

#[test]
fn cli_avviso_opacita_invariata_exit_0_stderr_e_html_scritto() {
    let dir = std::env::temp_dir().join(format!("joypp-test-v021-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let sorgente = dir.join("opacita.joy");
    let uscita = dir.join("o.html");
    fs::write(
        &sorgente,
        "Prepara un palco grande 100 per 100.\nCrea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia opacità a 1.\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_joypp"))
        .args([sorgente.to_str().unwrap(), "-o", uscita.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(
        stderr.contains(
            "Avviso alla riga 4: \"c\" cambia opacità a 1 ma la sua opacità è già 1: l'azione non cambia nulla"
        ),
        "{stderr}"
    );
    assert!(stderr.contains(&format!("  Suggerimento: {SUGG_CAMBIA}")), "{stderr}");
    assert!(!stdout.contains("Avviso"), "{stdout}");
    assert!(stdout.contains('✔'), "{stdout}");
    assert!(fs::read_to_string(&uscita).unwrap().starts_with("<!DOCTYPE html>"));
    fs::remove_dir_all(dir).ok();
}

// ------------------------------------------------------------ esempi

#[test]
fn nessun_avviso_su_tutti_gli_esempi() {
    let mut trovati = 0;
    for voce in fs::read_dir(radice().join("examples")).unwrap() {
        let percorso = voce.unwrap().path();
        if percorso.extension().is_some_and(|e| e == "joy") {
            let src = fs::read_to_string(&percorso).unwrap();
            let c = compila_con_avvisi(&src, &Opzioni::default())
                .unwrap_or_else(|e| panic!("{}: {e}", percorso.display()));
            assert!(c.avvisi.is_empty(), "{}: {:?}", percorso.display(), c.avvisi);
            trovati += 1;
        }
    }
    assert_eq!(trovati, 5, "attesi 5 esempi in examples/");
}
