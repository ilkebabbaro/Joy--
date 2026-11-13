//! Test di sicurezza: XSS, CSS injection e limiti anti-DoS.

use joypp::{Opzioni, compila_sorgente};

const PALCO: &str = "Prepara un palco grande 400 per 300.\n";

fn html(src: &str) -> String {
    compila_sorgente(src, &Opzioni::default()).unwrap()
}

fn fallisce(src: &str) -> joypp::Errore {
    compila_sorgente(src, &Opzioni::default()).expect_err("doveva fallire")
}

#[test]
fn testo_con_script_viene_neutralizzato() {
    let out = html(&format!(
        "{PALCO}Crea un testo chiamato \"t\" con testo \"</style><script>alert('x')</script>&amp;\".\n"
    ));
    assert!(!out.contains("<script>alert"));
    assert!(!out.contains("</style><script>"));
    assert!(out.contains("&lt;/style&gt;&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;&amp;amp;"));
    // L'unico <script> è quello generato dal compilatore per adattare il palco.
    assert_eq!(out.matches("<script>").count(), 1);
}

#[test]
fn nome_attore_non_esce_dall_attributo() {
    let out = html(&format!("{PALCO}Crea un cerchio chiamato \"x' onload='alert(1)><b>\".\n"));
    assert!(out.contains("data-nome=\"x&#39; onload=&#39;alert(1)&gt;&lt;b&gt;\""));
    assert!(!out.contains("<b>"));
}

#[test]
fn nomi_utente_non_diventano_identificatori_css() {
    let out = html(&format!(
        "{PALCO}Crea un cerchio chiamato \"}}body{{display:none\".\nDal secondo 0 al secondo 1:\n  \"}}body{{display:none\" appare sfumando.\n"
    ));
    // Il nome può comparire solo come valore (escape-ato) di `data-nome`, mai dentro <style>.
    let stile = out.split("<style>").nth(1).and_then(|s| s.split("</style>").next()).unwrap();
    assert!(!stile.contains("display:none"));
    assert!(stile.contains("@keyframes a0-opacita"));
}

#[test]
fn font_con_caratteri_pericolosi_rifiutato() {
    for font in ["Arial;}body{display:none", "a<b", "a\\b", "x'y", "url(evil)"] {
        let e = fallisce(&format!("{PALCO}Crea un testo chiamato \"t\" con testo \"a\" e font \"{font}\".\n"));
        assert_eq!(e.riga, Some(2), "{font}");
    }
}

#[test]
fn colori_non_letterali_rifiutati() {
    for colore in ["expression(alert(1))", "url(x)", "#12;}", "rgb(1,2,3);x"] {
        assert!(
            compila_sorgente(&format!("{PALCO}Crea un cerchio chiamato \"a\" con colore {colore}.\n"), &Opzioni::default()).is_err(),
            "{colore}"
        );
    }
}

#[test]
fn colori_riemessi_numericamente() {
    let out = html(&format!("{PALCO}Crea un cerchio chiamato \"a\" con colore #ABC.\n"));
    assert!(out.contains("background:#aabbcc"));
}

#[test]
fn caratteri_di_controllo_rifiutati() {
    let e = fallisce(&format!("{PALCO}Crea un cerchio chiamato \"a\u{1b}[31m\".\n"));
    assert!(!e.to_string().contains('\u{1b}'));
    assert_eq!(e.riga, Some(2));
}

#[test]
fn limite_dimensione_sorgente() {
    let enorme = format!("{PALCO}# {}\n", "x".repeat(2 * 1024 * 1024));
    assert!(fallisce(&enorme).messaggio.contains("troppo grande"));
}

#[test]
fn limite_durata_scena() {
    let e = fallisce(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 999999:\n  \"a\" appare sfumando.\n"
    ));
    assert!(e.messaggio.contains("durata massima"));
}

#[test]
fn limite_keyframe_pulsazione() {
    let e = fallisce(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 3600:\n  \"a\" pulsa a ritmo 1200.\n"
    ));
    assert!(e.messaggio.contains("battiti"), "{e}");
}

#[test]
fn limite_palco_e_dimensioni() {
    assert!(compila_sorgente("Prepara un palco grande 0 per 10.", &Opzioni::default()).is_err());
    assert!(compila_sorgente("Prepara un palco grande 99999999 per 10.", &Opzioni::default()).is_err());
    assert!(compila_sorgente(&format!("{PALCO}Crea un cerchio chiamato \"a\" con raggio -5.\n"), &Opzioni::default()).is_err());
}

#[test]
fn limite_nome_attore() {
    let nome = "n".repeat(500);
    let e = fallisce(&format!("{PALCO}Crea un cerchio chiamato \"{nome}\".\n"));
    assert!(e.messaggio.contains("troppo lungo"));
}

#[test]
fn input_casuali_non_causano_panic() {
    // Fuzzing deterministico leggero: frammenti combinati in modo pseudo-casuale.
    let frammenti = [
        "Prepara", "palco", "grande", "Crea", "cerchio", "chiamato", "\"a\"", "\"", "con", "raggio",
        "-1", "1.5", ".", ":", ",", "#", "#FFF", "rgb(", ")", "Dal", "secondo", "al", "Poi", "per",
        "  ", "pulsa", "a", "ritmo", "di", "e", "\n", "\n  ", "si", "sposta", "’", "à", "\t",
    ];
    let mut stato: u64 = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..3000 {
        let mut src = String::from(PALCO);
        for _ in 0..12 {
            stato ^= stato << 13;
            stato ^= stato >> 7;
            stato ^= stato << 17;
            src.push_str(frammenti[(stato % frammenti.len() as u64) as usize]);
            src.push(' ');
        }
        let _ = compila_sorgente(&src, &Opzioni::default());
    }
}
