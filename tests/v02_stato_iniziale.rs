//! Test v0.2 blocchi B.1 (stato iniziale `scala`/`rotazione`) e B.6 (avvisi),
//! più le correzioni diagnostiche collegate.

use joypp::compiler::{Canale, ScenaCompilata, Valore, compila};
use joypp::parser::analizza;
use joypp::{Avviso, Opzioni, compila_sorgente};

const PALCO: &str = "Prepara un palco grande 1000 per 500.\n";

fn scena(src: &str) -> ScenaCompilata {
    compila(&analizza(src).unwrap()).unwrap()
}

fn errore_compilatore(src: &str) -> joypp::Errore {
    compila(&analizza(src).unwrap()).expect_err("doveva fallire")
}

fn errore_parser(src: &str) -> joypp::Errore {
    analizza(src).expect_err("doveva fallire")
}

fn html(src: &str) -> String {
    compila_sorgente(src, &Opzioni::default()).unwrap()
}

fn valori(s: &ScenaCompilata, canale: Canale) -> Vec<Valore> {
    s.attori[0]
        .animazioni
        .iter()
        .find(|a| a.canale == canale)
        .unwrap_or_else(|| panic!("canale {canale:?} assente"))
        .keyframe
        .iter()
        .map(|k| k.valore)
        .collect()
}

/// Regola statica `#aN{...}` dell'attore `n` nell'HTML compatto.
fn regola_statica(html: &str, n: usize) -> String {
    let inizio = html.find(&format!("#a{n}{{")).unwrap_or_else(|| panic!("regola #a{n} assente"));
    let resto = &html[inizio..];
    resto[..resto.find('}').unwrap()].to_string()
}

fn stile(src: &str) -> (f64, f64) {
    let p = analizza(&format!("{PALCO}{src}")).unwrap();
    (p.attori[0].stile.scala, p.attori[0].stile.rotazione)
}

// ------------------------------------------------------------ parser B.1

#[test]
fn parser_scala_e_rotazione_su_tutti_i_tipi() {
    let casi = [
        "Crea un cerchio chiamato \"a\" con raggio 10, scala 2 e rotazione 30 al centro.\n",
        "Crea un quadrato chiamato \"a\" con lato 10, scala 2 e rotazione 30 al centro.\n",
        "Crea un rettangolo chiamato \"a\" con larghezza 10 e altezza 5, scala 2 e rotazione 30 al centro.\n",
        "Crea un testo chiamato \"a\" con testo \"ciao\", scala 2 e rotazione 30 al centro.\n",
        "Crea un titolo chiamato \"a\" con testo \"ciao\", scala 2 e rotazione 30 al centro.\n",
    ];
    for c in casi {
        assert_eq!(stile(c), (2.0, 30.0), "{c}");
    }
}

#[test]
fn parser_rotazione_con_e_senza_gradi() {
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione 45.\n").1, 45.0);
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione 45 gradi.\n").1, 45.0);
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione 1 grado.\n").1, 1.0);
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione -8 gradi.\n").1, -8.0);
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione -8.5.\n").1, -8.5);
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione 370.\n").1, 370.0, "non normalizzata");
}

#[test]
fn parser_predefiniti() {
    assert_eq!(stile("Crea un quadrato chiamato \"a\".\n"), (1.0, 0.0));
}

#[test]
fn parser_ordine_e_separatori() {
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione 10 e scala 3.\n"), (3.0, 10.0));
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con scala 3, rotazione 10 gradi.\n"), (3.0, 10.0));
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con rotazione 10 con scala 0.5 al centro.\n"), (0.5, 10.0));
    assert_eq!(stile("Crea un quadrato chiamato \"a\" con lato 5, scala 0.\n").0, 0.0);
    assert_eq!(
        stile("Crea un quadrato chiamato \"a\" con opacità 0.5, rotazione -10 gradi e scala 2 e lato 5 in alto.\n"),
        (2.0, -10.0)
    );
}

#[test]
fn parser_ripetute_vale_l_ultima() {
    assert_eq!(
        stile("Crea un quadrato chiamato \"a\" con scala 2, rotazione 10, scala 5 e rotazione -20 gradi.\n"),
        (5.0, -20.0)
    );
}

#[test]
fn parser_numero_mancante_dopo_scala() {
    let e = errore_parser(&format!("{PALCO}\nCrea un quadrato chiamato \"a\" con scala al centro.\n"));
    assert_eq!(e.riga, Some(3));
    assert!(e.messaggio.contains("mi aspettavo un numero dopo «scala»"), "{e}");
}

#[test]
fn parser_numero_mancante_dopo_rotazione() {
    let e = errore_parser(&format!("{PALCO}Crea un quadrato chiamato \"a\" con rotazione gradi.\n"));
    assert_eq!(e.riga, Some(2));
    assert!(e.messaggio.contains("mi aspettavo un numero dopo «rotazione»"), "{e}");
}

// ------------------------------------------------------- compilatore B.1

#[test]
fn compilatore_scala_negativa() {
    let e = errore_compilatore(&format!("{PALCO}\nCrea un quadrato chiamato \"a\" con scala -1.\n"));
    assert_eq!(e.riga, Some(3));
    assert_eq!(e.messaggio, "la scala iniziale non può essere negativa");
    assert_eq!(e.suggerimento.as_deref(), Some("usa un valore da 0 in su, es. «scala 0.5»"));
}

#[test]
fn compilatore_scala_troppo_grande() {
    let e = errore_compilatore(&format!("{PALCO}Crea un quadrato chiamato \"a\" con scala 1001.\n"));
    assert_eq!(e.riga, Some(2));
    assert_eq!(e.messaggio, "la scala iniziale è troppo grande (massimo 1000)");
    // Il limite è incluso.
    scena(&format!("{PALCO}Crea un quadrato chiamato \"a\" con scala 1000.\n"));
}

#[test]
fn compilatore_rotazione_troppo_grande() {
    for g in ["360001", "-360001"] {
        let e = errore_compilatore(&format!("{PALCO}Crea un quadrato chiamato \"a\" con rotazione {g}.\n"));
        assert_eq!(e.riga, Some(2));
        assert_eq!(e.messaggio, "la rotazione iniziale è troppo grande (massimo 360000 gradi)");
    }
    scena(&format!("{PALCO}Crea un quadrato chiamato \"a\" con rotazione 360000.\n"));
    scena(&format!("{PALCO}Crea un quadrato chiamato \"a\" con rotazione -360000.\n"));
}

#[test]
fn compilatore_stato_iniziale_esposto_sull_attore() {
    let s = scena(&format!("{PALCO}Crea un quadrato chiamato \"a\" con scala 0.5 e rotazione -30.\n"));
    assert_eq!(s.attori[0].scala_iniziale, 0.5);
    assert_eq!(s.attori[0].rotazione_iniziale, -30.0);
}

#[test]
fn compilatore_primo_keyframe_uguale_allo_stato_iniziale() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con scala 0.5 e rotazione 20.\n\
         Dal secondo 1 al secondo 2:\n  \"a\" ruota di 10 gradi e si ingrandisce fino a 2.\n"
    ));
    assert_eq!(valori(&s, Canale::Scala)[0], Valore::Scala(0.5));
    assert_eq!(valori(&s, Canale::Rotazione)[0], Valore::Rotazione(20.0));
}

#[test]
fn compilatore_ruota_e_relativo_a_r0() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con rotazione 45.\n\
         Dal secondo 0 al secondo 1:\n  \"a\" ruota di 90 gradi.\n"
    ));
    assert_eq!(valori(&s, Canale::Rotazione), vec![Valore::Rotazione(45.0), Valore::Rotazione(135.0)]);
}

#[test]
fn compilatore_ingrandisce_e_assoluto_da_s0() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con scala 0.\n\
         Dal secondo 0 al secondo 1:\n  \"a\" si ingrandisce fino a 3.\n"
    ));
    assert_eq!(valori(&s, Canale::Scala), vec![Valore::Scala(0.0), Valore::Scala(3.0)]);
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con scala 4.\n\
         Dal secondo 0 al secondo 1:\n  \"a\" si rimpicciolisce fino a 1.\n"
    ));
    assert_eq!(valori(&s, Canale::Scala), vec![Valore::Scala(4.0), Valore::Scala(1.0)]);
}

#[test]
fn compilatore_pulsa_attorno_a_s0() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con scala 2.\n\
         Dal secondo 0 al secondo 1:\n  \"a\" pulsa intensamente a ritmo 60.\n"
    ));
    let v = valori(&s, Canale::Scala);
    assert_eq!(v[0], Valore::Scala(2.0));
    assert_eq!(v[1], Valore::Scala(2.5), "b × (1 + 0.25)");
    assert_eq!(*v.last().unwrap(), Valore::Scala(2.0));
}

#[test]
fn compilatore_pulsa_con_s0_zero_resta_zero() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con scala 0.\n\
         Dal secondo 0 al secondo 2:\n  \"a\" pulsa intensamente a ritmo 60.\n"
    ));
    assert!(valori(&s, Canale::Scala).iter().all(|v| *v == Valore::Scala(0.0)));
}

#[test]
fn compilatore_appare_non_tocca_scala_e_rotazione() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con scala 2 e rotazione 45.\n\
         Dal secondo 0 al secondo 1:\n  \"a\" appare sfumando.\n"
    ));
    let canali: Vec<Canale> = s.attori[0].animazioni.iter().map(|a| a.canale).collect();
    assert_eq!(canali, vec![Canale::Opacita]);
    assert_eq!(s.attori[0].scala_iniziale, 2.0);
    assert_eq!(s.attori[0].rotazione_iniziale, 45.0);
}

// --------------------------------------------------------- renderer B.1

#[test]
fn renderer_scale_e_rotate_nella_regola_statica() {
    let h = html(&format!("{PALCO}Crea un quadrato chiamato \"a\" con scala 0.5 e rotazione -30.\n"));
    let r = regola_statica(&h, 0);
    assert!(r.contains(";scale:0.5"), "{r}");
    assert!(r.contains(";rotate:-30deg"), "{r}");
}

#[test]
fn renderer_solo_uno_dei_due() {
    let h = html(&format!("{PALCO}Crea un quadrato chiamato \"a\" con scala 2.\n"));
    let r = regola_statica(&h, 0);
    assert!(r.contains(";scale:2") && !r.contains(";rotate:"), "{r}");
    let h = html(&format!("{PALCO}Crea un quadrato chiamato \"a\" con rotazione 10.\n"));
    let r = regola_statica(&h, 0);
    assert!(r.contains(";rotate:10deg") && !r.contains(";scale:"), "{r}");
}

#[test]
fn renderer_presenti_anche_con_animazioni() {
    let h = html(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\" con scala 0 e rotazione 45.\n\
         Dal secondo 0 al secondo 1:\n  \"a\" si ingrandisce fino a 1 e ruota di 90 gradi.\n"
    ));
    let r = regola_statica(&h, 0);
    assert!(r.contains(";scale:0") && r.contains(";rotate:45deg"), "{r}");
    assert!(r.contains("animation:"), "{r}");
}

#[test]
fn renderer_assenti_con_i_default() {
    let h = html(&format!("{PALCO}Crea un quadrato chiamato \"a\" con scala 1 e rotazione 0.\n"));
    let r = regola_statica(&h, 0);
    assert!(!r.contains("scale:") && !r.contains("rotate:"), "{r}");
    let h = html(&format!(
        "{PALCO}Crea un quadrato chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" ruota di 90 gradi.\n"
    ));
    let r = regola_statica(&h, 0);
    assert!(!r.contains(";rotate:") && !r.contains(";scale:"), "{r}");
}

#[test]
fn regressione_esempi_v01_senza_scale_rotate_statici() {
    for esempio in ["primo_test", "livello1_motion"] {
        let src = std::fs::read_to_string(format!("{}/examples/{esempio}.joy", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let h = html(&src);
        let mut regole = 0;
        for pezzo in h.split("#a").skip(1) {
            let Some(fine_cifre) = pezzo.find(|c: char| !c.is_ascii_digit()) else { continue };
            let (testa, resto) = pezzo.split_at(fine_cifre);
            if testa.is_empty() || !resto.starts_with('{') {
                continue;
            }
            let regola = &resto[..resto.find('}').unwrap()];
            assert!(!regola.contains(";scale:"), "{esempio}: #a{testa}{regola}");
            assert!(!regola.contains(";rotate:"), "{esempio}: #a{testa}{regola}");
            regole += 1;
        }
        assert!(regole >= 1, "{esempio}: nessuna regola statica trovata");
    }
}

// ------------------------------------------------------------ avvisi B.6

const MESSAGGIO_C: &str = "\"c\" è un cerchio: la rotazione non cambia il suo aspetto";

#[test]
fn avviso_ruota_su_cerchio() {
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\nDal secondo 0 al secondo 1:\n  \"c\" ruota di 90 gradi.\n"
    ));
    assert_eq!(s.avvisi.len(), 1);
    let a = &s.avvisi[0];
    assert_eq!(a.riga, 5);
    assert_eq!(a.messaggio, MESSAGGIO_C);
    assert_eq!(
        a.suggerimento.as_deref(),
        Some("togli «ruota di …», oppure usa un quadrato o un rettangolo se vuoi vederlo girare")
    );
}

#[test]
fn avviso_rotazione_iniziale_su_cerchio() {
    let s = scena(&format!("{PALCO}\nCrea un cerchio chiamato \"c\" con rotazione 30.\n"));
    assert_eq!(s.avvisi.len(), 1);
    assert_eq!(s.avvisi[0].riga, 3);
    assert_eq!(s.avvisi[0].messaggio, MESSAGGIO_C);
    assert_eq!(s.avvisi[0].suggerimento.as_deref(), Some("togli «rotazione …» dalla creazione del cerchio"));
}

#[test]
fn nessun_avviso_con_rotazione_zero_o_scala() {
    let s = scena(&format!("{PALCO}Crea un cerchio chiamato \"c\" con rotazione 0 e scala 2.\n"));
    assert!(s.avvisi.is_empty(), "{:?}", s.avvisi);
}

#[test]
fn nessun_avviso_su_quadrato_o_testo() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"q\" con rotazione 10.\n\
         Crea un testo chiamato \"t\" con testo \"x\" e rotazione 10.\n\
         Dal secondo 0 al secondo 1:\n  \"q\" ruota di 90 gradi.\n  \"t\" ruota di 90 gradi.\n"
    ));
    assert!(s.avvisi.is_empty(), "{:?}", s.avvisi);
}

#[test]
fn avvisi_ordinati_per_riga() {
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\n\
         Crea un cerchio chiamato \"d\" con rotazione 5.\n\
         Dal secondo 0 al secondo 1:\n  \"d\" ruota di 5 gradi.\n  \"c\" ruota di 5 gradi.\n"
    ));
    let righe: Vec<usize> = s.avvisi.iter().map(|a| a.riga).collect();
    assert_eq!(righe, vec![3, 5, 6]);
}

#[test]
fn avvisi_stessa_riga_e_stesso_messaggio_non_raggiungibile() {
    // La deduplicazione (B.6) richiede due `ruota di` sulla stessa riga e sullo stesso attore,
    // ma due azioni dello stesso attore sul canale rotazione nello stesso blocco si
    // sovrappongono: la sintassi attuale le rifiuta, quindi il caso non è raggiungibile.
    let src = format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" ruota di 10 gradi e ruota di 20 gradi.\n"
    );
    if let Ok(p) = analizza(&src) {
        assert!(compila(&p).is_err(), "doveva essere una sovrapposizione");
    }
    // Sulla stessa riga attori diversi producono messaggi diversi: nessuna fusione.
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\nCrea un cerchio chiamato \"d\".\n\
         Dal secondo 0 al secondo 1:\n  \"c\" ruota di 5 gradi.\n  \"d\" ruota di 5 gradi.\n"
    ));
    assert_eq!(s.avvisi.len(), 2);
}

#[test]
fn avviso_display() {
    let a = Avviso::alla_riga(7, "msg").con_suggerimento("sugg");
    assert_eq!(a.to_string(), "Avviso alla riga 7: msg\n  Suggerimento: sugg");
}

#[test]
fn compila_sorgente_riesce_con_avvisi() {
    let src = format!("{PALCO}Crea un cerchio chiamato \"c\" con rotazione 30.\n");
    assert!(compila_sorgente(&src, &Opzioni::default()).is_ok());
    let c = joypp::exporter::compila_con_avvisi(&src, &Opzioni::default()).unwrap();
    assert_eq!(c.avvisi.len(), 1);
    assert!(c.html.starts_with("<!DOCTYPE html>"));
}

// ------------------------------------------------ correzioni diagnostiche

#[test]
fn riferimento_sconosciuto_non_suggerisce_se_stesso() {
    let e = errore_compilatore(&format!("{PALCO}Crea un cerchio chiamato \"a\" a destra di \"b\" di 10.\n"));
    assert_eq!(e.riga, Some(2));
    assert_ne!(e.suggerimento.as_deref(), Some("forse intendevi \"a\"?"), "{e}");
}

#[test]
fn nessun_errore_contiene_1_secondi() {
    let sorgenti = [
        format!("{PALCO}Crea un cerchio chiamato \"a\".\nDopo 1 secondo.\n"),
        format!("{PALCO}Crea un cerchio chiamato \"a\".\nDopo 1 secondo:\n"),
        format!("{PALCO}Crea un cerchio chiamato \"a\".\nDopo qualcosa per 1 secondo:\n"),
        format!("{PALCO}Crea un cerchio chiamato \"a\".\nDopo:\n"),
        format!("{PALCO}Crea un cerchio chiamato \"a\".\nBoh 1 secondo:\n"),
        format!("{PALCO}Crea un cerchio chiamato \"a\".\nDopo -1 per 2 secondi:\n"),
    ];
    for s in &sorgenti {
        let e = errore_parser(s);
        assert!(!e.to_string().contains("1 secondi"), "{s}\n{e}");
    }
    let e = errore_compilatore(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\n\
         Dal secondo 0 al secondo 2:\n  \"a\" si sposta a 1, 1.\n\
         Dal secondo 1 al secondo 3:\n  \"a\" scivola verso alto di 5.\n"
    ));
    assert!(!e.to_string().contains("1 secondi"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap().contains("1 secondo:"), "{e}");
}
