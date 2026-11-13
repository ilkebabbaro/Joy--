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
    // Il CSS del player contiene `display:none!important` (modalità render, B.9.2): quindi si
    // cerca il payload nella sua forma completa e si verifica che la regola `body{...}` non esista.
    assert!(!stile.contains("body{display:none"));
    assert!(!stile.contains("}body{display:none"));
    // Struttura del CSS identica a quella con un nome innocuo: il payload non ha aggiunto regole.
    let controllo = html(&format!(
        "{PALCO}Crea un cerchio chiamato \"x\".\nDal secondo 0 al secondo 1:\n  \"x\" appare sfumando.\n"
    ));
    let stile_c = controllo.split("<style>").nth(1).and_then(|s| s.split("</style>").next()).unwrap();
    assert_eq!(stile.matches('{').count(), stile_c.matches('{').count());
    assert_eq!(stile.matches('}').count(), stile_c.matches('}').count());
    assert_eq!(stile, stile_c, "il CSS non dipende dal nome dell'attore");
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
        // Frammenti del blocco v0.2 2B: gruppi, orbita, entra/esce, cambia colore.
        "gruppo", "un gruppo", "orbita", "attorno", "giri", "giro", "in senso orario", "antiorario",
        "entra", "dall", "dall'", "dal", "dalla", "da", "esce", "verso", "l'alto", "alto", "sinistra",
        "cambia colore in", "cambia colore", "colore", "rosso", "dentro", "\"b\"", "scala", "rotazione",
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

// ------------------------------------------------- blocco v0.2 2B (gruppi, orbita, ...)

/// Nessun carattere bidirezionale (override, isolati, marcatori) deve uscire in un messaggio.
fn ha_bidi(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{200E}' | '\u{200F}' | '\u{061C}'))
}

fn conta_div(html: &str) -> (usize, usize) {
    (html.matches("<div").count(), html.matches("</div>").count())
}

#[test]
fn gruppo_nome_e_membri_con_payload_xss() {
    let out = html(&format!(
        "{PALCO}Crea un cerchio chiamato \"<img src=x onerror=alert(1)>\".\n\
         Crea un cerchio chiamato \"x' onload='alert(1)><b>\".\n\
         Crea un gruppo chiamato \"</div><script>alert(2)</script>\" con \"<img src=x onerror=alert(1)>\" e \"x' onload='alert(1)><b>\".\n"
    ));
    assert!(!out.contains("<img"), "{out}");
    assert!(!out.contains("<b>"));
    assert!(!out.contains("<script>alert"));
    assert!(out.contains("data-nome=\"&lt;/div&gt;&lt;script&gt;alert(2)&lt;/script&gt;\""), "{out}");
    assert!(out.contains("data-nome=\"&lt;img src=x onerror=alert(1)&gt;\""));
    assert!(out.contains("data-nome=\"x&#39; onload=&#39;alert(1)&gt;&lt;b&gt;\""));
    // L'unico <script> è quello generato dal compilatore.
    assert_eq!(out.matches("<script>").count(), 1);
    // Il payload `</div>` nel nome non altera la struttura.
    let (aperti, chiusi) = conta_div(&out);
    assert_eq!(aperti, chiusi);
    // I nomi non finiscono mai dentro il CSS.
    let stile = out.split("<style>").nth(1).and_then(|s| s.split("</style>").next()).unwrap();
    assert!(!stile.contains("alert") && !stile.contains("onerror"));
}

#[test]
fn gruppo_nome_attore_bidi_non_esce_nei_messaggi() {
    // Il nome del membro contiene un override bidirezionale (U+202E): rifiutato, e il messaggio
    // d'errore (lista dei membri rifiutati compresa) non lo riemette.
    let bidi = "a\u{202E}b";
    for sorgente in [
        format!("{PALCO}Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\" e \"{bidi}\".\n"),
        format!("{PALCO}Crea un cerchio chiamato \"{bidi}\".\n"),
        format!("{PALCO}Crea un gruppo chiamato \"{bidi}\" con \"a\".\n"),
        format!(
            "{PALCO}Crea un cerchio chiamato \"a\".\nCrea un cerchio chiamato \"b\".\nDal secondo 0 al secondo 1:\n  \"a\" orbita attorno a \"{bidi}\" con raggio 5 per 1 giro.\n"
        ),
    ] {
        let e = fallisce(&sorgente);
        let testo = e.to_string();
        assert!(!ha_bidi(&testo), "il messaggio contiene un carattere bidirezionale: {testo:?}");
    }
}

#[test]
fn gruppo_nome_con_bidi_tra_i_membri_non_produce_html() {
    // Se qualcuno lo accettasse, l'HTML non deve contenere il carattere grezzo.
    for bidi in ['\u{202E}', '\u{2067}', '\u{200F}'] {
        let sorgente = format!(
            "{PALCO}Crea un cerchio chiamato \"x{bidi}y\".\nCrea un gruppo chiamato \"g\" con \"x{bidi}y\".\n"
        );
        if let Ok(out) = compila_sorgente(&sorgente, &Opzioni::default()) {
            assert!(!ha_bidi(&out), "carattere bidirezionale nell'HTML ({bidi:?})");
        }
    }
}

#[test]
fn gruppi_annidati_profondi_html_bilanciato() {
    let mut s = format!("{PALCO}Crea un cerchio chiamato \"a\".\n");
    let mut prec = "a".to_string();
    for i in 1..=16 {
        s.push_str(&format!("Crea un gruppo chiamato \"g{i}\" con \"{prec}\".\n"));
        prec = format!("g{i}");
    }
    s.push_str(&format!("Dal secondo 0 al secondo 1:\n  \"{prec}\" entra da sinistra.\n"));
    for opzioni in [Opzioni::default(), Opzioni { leggibile: true, ..Default::default() }] {
        let out = compila_sorgente(&s, &opzioni).unwrap();
        // Bilanciamento sull'intero documento (comprende il `<div id="joy-player">`).
        let (aperti, chiusi) = conta_div(&out);
        assert_eq!(aperti, chiusi);
        // Dentro `#palco`: #palco + 16 gruppi + il cerchio `.attore` + la sua `.forma`.
        // Il player (B.5) sta fuori dal palco e aggiunge un solo `<div>`.
        let palco = out.split("<div id=\"palco\"").nth(1).and_then(|s| s.split("<div id=\"joy-player\"").next()).unwrap();
        assert_eq!(palco.matches("<div").count() + 1, 19);
        assert_eq!(palco.matches("</div>").count(), 19);
        assert_eq!(aperti, 19 + 1, "palco + player");
        assert_eq!(out.matches("class=\"attore gruppo\"").count(), 16);
        assert_eq!(out.matches("class=\"forma\"").count(), 1, "solo il cerchio ha una .forma");
    }
}

#[test]
fn gruppi_oltre_la_profondita_massima_rifiutati() {
    let mut s = format!("{PALCO}Crea un cerchio chiamato \"a\".\n");
    let mut prec = "a".to_string();
    for i in 1..=17 {
        s.push_str(&format!("Crea un gruppo chiamato \"g{i}\" con \"{prec}\".\n"));
        prec = format!("g{i}");
    }
    assert!(fallisce(&s).messaggio.contains("troppo in profondità"));
}

#[test]
fn gruppi_contano_nel_limite_attori() {
    // 5001 cerchi (limite MAX_ATTORI = 5000) con un gruppo: rifiutato senza panic.
    let mut s = String::from(PALCO);
    for i in 0..5001 {
        s.push_str(&format!("Crea un cerchio chiamato \"c{i}\".\n"));
    }
    s.push_str("Crea un gruppo chiamato \"g\" con \"c0\".\n");
    assert!(compila_sorgente(&s, &Opzioni::default()).is_err());
}

#[test]
fn orbita_con_giri_enormi_errore_e_nessun_panic() {
    let base = format!(
        "{PALCO}Crea un cerchio chiamato \"s\" a 100, 100.\nCrea un cerchio chiamato \"t\" a 200, 100.\nDal secondo 0 al secondo 1:\n"
    );
    for giri in ["1000000", "1000000000000", "99999999999999999999999", "300"] {
        let e = fallisce(&format!("{base}  \"t\" orbita attorno a \"s\" con raggio 100 per {giri} giri.\n"));
        assert!(e.messaggio.contains("orbita troppo lunga"), "{giri}: {e}");
    }
    // Raggio e giri estremi insieme, e valori microscopici: nessun panic (Ok o Err).
    for (raggio, giri) in [("0.000001", "0.000001"), ("1000000", "5"), ("1", "1000000000000"), ("0.5", "0.0000000001")] {
        let _ = compila_sorgente(
            &format!("{base}  \"t\" orbita attorno a \"s\" con raggio {raggio} per {giri} giri.\n"),
            &Opzioni::default(),
        );
    }
}

#[test]
fn orbita_con_numeri_non_finiti_nessun_panic() {
    let base = format!(
        "{PALCO}Crea un cerchio chiamato \"s\" a 100, 100.\nCrea un cerchio chiamato \"t\" a 200, 100.\nDal secondo 0 al secondo 1:\n"
    );
    for (raggio, giri) in [("NaN", "1"), ("inf", "1"), ("100", "NaN"), ("100", "inf"), ("1e999", "1"), ("100", "1e999")] {
        let _ = compila_sorgente(
            &format!("{base}  \"t\" orbita attorno a \"s\" con raggio {raggio} per {giri} giri.\n"),
            &Opzioni::default(),
        );
    }
}

#[test]
fn entra_da_con_attori_enormi_errore_e_nessun_panic() {
    let e = fallisce(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con lato 100000 e scala 1000.\nDal secondo 0 al secondo 1:\n  \"a\" entra da sinistra.\n"
    ));
    assert!(e.messaggio.contains("fuori dai limiti"), "{e}");
}

#[test]
fn cambia_colore_riemesso_dai_valori_numerici() {
    let out = html(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" cambia colore in #ABCDEF.\n"
    ));
    assert!(out.contains("background-color:#abcdef"));
    for colore in ["expression(alert(1))", "url(x)", "#12;}", "rgb(1,2,3);x", "red;}body{display:none"] {
        assert!(
            compila_sorgente(
                &format!("{PALCO}Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" cambia colore in {colore}.\n"),
                &Opzioni::default()
            )
            .is_err(),
            "{colore}"
        );
    }
}

#[test]
fn input_casuali_con_gruppi_e_verbi_v02_non_causano_panic() {
    // Stesso fuzzing deterministico, ma con una base valida (attori, gruppo e blocco temporale)
    // così le frasi casuali arrivano fino al compilatore.
    let base = format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\nCrea un cerchio chiamato \"b\" a 100, 100.\n\
         Crea un gruppo chiamato \"g\" con \"a\".\nDal secondo 0 al secondo 2:\n"
    );
    let verbi = [
        "orbita attorno a", "entra da", "entra dall'", "entra dalla", "entra dal", "esce verso", "esce verso l'",
        "cambia colore in", "si sposta a", "pulsa", "scivola verso", "ruota di", "appare sfumando", "e",
    ];
    let oggetti = [
        "\"a\"", "\"b\"", "\"g\"", "\"x\"", "alto", "basso", "sinistra", "destra", "rosso", "#FFF", "#12",
        "con raggio", "per", "giri", "giro", "in senso orario", "in senso antiorario", "0", "-3", "2.5",
        "1e9", "100000", "con movimento elastico", "con movimento scatto", ".",
    ];
    let mut stato: u64 = 0xD1B5_4A32_D192_ED03;
    for _ in 0..3000 {
        let mut src = base.clone();
        for _ in 0..3 {
            src.push_str("  \"");
            src.push(['a', 'b', 'g'][(stato % 3) as usize]);
            src.push_str("\" ");
            for _ in 0..6 {
                stato ^= stato << 13;
                stato ^= stato >> 7;
                stato ^= stato << 17;
                let lista: &[&str] = if stato.is_multiple_of(3) { &verbi } else { &oggetti };
                src.push_str(lista[((stato >> 8) % lista.len() as u64) as usize]);
                src.push(' ');
            }
            src.push('\n');
        }
        let _ = compila_sorgente(&src, &Opzioni::default());
    }
}
