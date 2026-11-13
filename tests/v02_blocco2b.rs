//! Test d'integrazione del blocco v0.2 "2B" (SPECIFICA, Appendice B): gruppi (B.3),
//! `entra da` / `esce verso` (B.2), `orbita` (B.4.1), `cambia colore` (B.4.2) e l'esempio B.8.
//!
//! Tutti i test partono dalla frase `.joy` e arrivano alla `ScenaCompilata` o all'HTML.

use joypp::ast::{Direzione, Forma, Verbo};
use joypp::compiler::easing::Curva;
use joypp::compiler::{AttoreCompilato, Canale, ScenaCompilata, Valore, compila};
use joypp::parser::analizza;
use joypp::{Opzioni, compila_sorgente};

/// Palco 1000 × 500: il centro è (500, 250).
const PALCO: &str = "Prepara un palco grande 1000 per 500.\n";

const TOLLERANZA: f64 = 1e-3;

// ------------------------------------------------------------------ helper

fn scena(src: &str) -> ScenaCompilata {
    let p = analizza(src).unwrap_or_else(|e| panic!("parser: {e}\n--\n{src}"));
    compila(&p).unwrap_or_else(|e| panic!("compilatore: {e}\n--\n{src}"))
}

/// Errore (di parser o di compilatore) di un sorgente che deve fallire.
fn errore(src: &str) -> joypp::Errore {
    match analizza(src) {
        Err(e) => e,
        Ok(p) => compila(&p).expect_err(&format!("doveva fallire:\n{src}")),
    }
}

fn html(src: &str) -> String {
    compila_sorgente(src, &Opzioni::default()).unwrap_or_else(|e| panic!("{e}\n--\n{src}"))
}

fn att<'a>(s: &'a ScenaCompilata, nome: &str) -> &'a AttoreCompilato {
    s.attori.iter().find(|a| a.nome == nome).unwrap_or_else(|| panic!("attore {nome:?} assente"))
}

fn indice(s: &ScenaCompilata, nome: &str) -> usize {
    s.attori.iter().position(|a| a.nome == nome).unwrap_or_else(|| panic!("attore {nome:?} assente"))
}

/// Keyframe del canale `traslazione` come `(offset, dx, dy, curva)`.
fn trasl(s: &ScenaCompilata, nome: &str) -> Vec<(f64, f64, f64, Option<Curva>)> {
    att(s, nome)
        .animazioni
        .iter()
        .find(|a| a.canale == Canale::Traslazione)
        .unwrap_or_else(|| panic!("{nome}: canale traslazione assente"))
        .keyframe
        .iter()
        .map(|k| match k.valore {
            Valore::Traslazione(dx, dy) => (k.offset, dx, dy, k.curva.clone()),
            v => panic!("valore inatteso {v:?}"),
        })
        .collect()
}

fn ha_canale(s: &ScenaCompilata, nome: &str, canale: Canale) -> bool {
    att(s, nome).animazioni.iter().any(|a| a.canale == canale)
}

fn vicino(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLLERANZA
}

fn assert_vicino(a: f64, b: f64, cosa: &str) {
    assert!(vicino(a, b), "{cosa}: atteso {b}, trovato {a}");
}

/// Numero di `<div` aperti e chiusi nell'HTML: devono coincidere.
fn div_bilanciati(html: &str) -> bool {
    html.matches("<div").count() == html.matches("</div>").count()
}

/// Corpo (tra le graffe) della prima regola CSS che inizia con `selettore`.
fn regola(html: &str, selettore: &str) -> String {
    let i = html.find(&format!("{selettore}{{")).unwrap_or_else(|| panic!("regola {selettore} assente"));
    let resto = &html[i..];
    resto[..resto.find('}').unwrap()].to_string()
}

/// Tre cerchi per i test dei gruppi: a (100,100) r10, b (300,200) r20, c (600,300) r30.
const TRE_CERCHI: &str = "Crea un cerchio chiamato \"a\" con raggio 10 a 100, 100.\n\
     Crea un cerchio chiamato \"b\" con raggio 20 a 300, 200.\n\
     Crea un cerchio chiamato \"c\" con raggio 30 a 600, 300.\n";

fn src(corpo: &str) -> String {
    format!("{PALCO}{corpo}")
}

// ========================================================== gruppi: sintassi

#[test]
fn gruppo_sintassi_b32_esempi_validi() {
    let p = analizza(&src(
        "Crea un cerchio chiamato \"lettera_j\".\n\
         Crea un cerchio chiamato \"lettera_o\".\n\
         Crea un cerchio chiamato \"lettera_y\".\n\
         Crea un gruppo chiamato \"logo\" con \"lettera_j\", \"lettera_o\" e \"lettera_y\".\n\
         Crea un cerchio chiamato \"sole\".\n\
         Crea un cerchio chiamato \"terra\".\n\
         Crea un gruppo chiamato \"sistema\" con \"sole\" e \"terra\", scala 0.8 e livello 1.\n\
         Crea un testo chiamato \"titolo\" con testo \"x\".\n\
         Crea un gruppo chiamato \"scena\" con \"sistema\", \"titolo\" e opacità 0.9.\n",
    ))
    .unwrap();
    let g = |n: &str| p.attori.iter().find(|a| a.nome == n).unwrap();
    assert_eq!(
        g("logo").forma,
        Forma::Gruppo { membri: vec!["lettera_j".into(), "lettera_o".into(), "lettera_y".into()] }
    );
    assert_eq!(g("sistema").stile.scala, 0.8);
    assert_eq!(g("sistema").stile.livello, 1);
    assert_eq!(g("scena").stile.opacita, 0.9);
    assert_eq!(g("scena").forma, Forma::Gruppo { membri: vec!["sistema".into(), "titolo".into()] });
}

#[test]
fn gruppo_proprieta_ammesse_e_predefiniti() {
    let p = analizza(&src(
        "Crea un cerchio chiamato \"a\".\n\
         Crea un gruppo chiamato \"g\" con \"a\", rotazione 30, scala 2, livello 3 e opacità 0.5.\n\
         Crea un cerchio chiamato \"b\".\n\
         Crea un gruppo chiamato \"h\" con \"b\".\n",
    ))
    .unwrap();
    let g = &p.attori[1].stile;
    assert_eq!((g.rotazione, g.scala, g.livello, g.opacita), (30.0, 2.0, 3, 0.5));
    let h = &p.attori[3].stile;
    assert_eq!((h.rotazione, h.scala, h.livello, h.opacita), (0.0, 1.0, 0, 1.0));
}

// ============================================================ gruppi: errori

#[test]
fn gruppo_errore_membro_inesistente_con_suggerimento() {
    let e = errore(&src("Crea un cerchio chiamato \"sole\".\nCrea un gruppo chiamato \"g\" con \"sloe\".\n"));
    assert_eq!(e.riga, Some(3));
    assert!(e.messaggio.contains("non esiste nessun attore o gruppo chiamato \"sloe\""), "{e}");
    let sugg = e.suggerimento.as_deref().unwrap_or_default();
    assert!(sugg.contains("forse intendevi") && sugg.contains("sole"), "{e}");
}

#[test]
fn gruppo_errore_membro_creato_dopo_il_gruppo() {
    // Il gruppo è alla riga 3 e "b" alla riga 4: ma "b" esiste, quindi l'errore è sull'ordine.
    let e = errore(&src(
        "Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\" e \"b\".\nCrea un cerchio chiamato \"b\".\n",
    ));
    assert_eq!(e.riga, Some(3));
    assert!(e.messaggio.contains("\"b\" è creato alla riga 4, dopo il gruppo \"g\""), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("crea il gruppo dopo tutti i suoi membri"));
}

#[test]
fn gruppo_errore_appartenenza_unica() {
    let e = errore(&src(
        "Crea un cerchio chiamato \"a\".\n\
         Crea un gruppo chiamato \"g1\" con \"a\".\n\
         Crea un gruppo chiamato \"g2\" con \"a\".\n",
    ));
    assert_eq!(e.riga, Some(4));
    assert!(e.messaggio.contains("\"a\" fa già parte del gruppo \"g1\" (riga 3)"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("un solo gruppo"));
}

#[test]
fn gruppo_errore_membro_due_volte() {
    let e = errore(&src("Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\" e \"a\".\n"));
    assert_eq!(e.riga, Some(3));
    assert!(e.messaggio.contains("\"a\" compare due volte nel gruppo \"g\""), "{e}");
}

#[test]
fn gruppo_errore_contiene_se_stesso() {
    let e = errore(&src("Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\" e \"g\".\n"));
    assert_eq!(e.riga, Some(3));
    assert!(e.messaggio.contains("un gruppo non può contenere sé stesso"), "{e}");
}

#[test]
fn gruppo_errore_nessun_membro() {
    for frase in [
        "Crea un gruppo chiamato \"g\".\n",
        "Crea un gruppo chiamato \"g\" con scala 2.\n",
    ] {
        let e = errore(&src(frase));
        assert_eq!(e.riga, Some(2), "{frase}");
        assert!(e.messaggio.contains("il gruppo \"g\" deve contenere almeno un membro"), "{e}");
        assert!(e.suggerimento.as_deref().unwrap_or_default().contains("Crea un gruppo chiamato"), "{e}");
    }
}

#[test]
fn gruppo_errore_membri_dopo_le_proprieta() {
    let e = errore(&src(
        "Crea un cerchio chiamato \"a\".\nCrea un cerchio chiamato \"b\".\n\
         Crea un gruppo chiamato \"g\" con \"a\", scala 2 e \"b\".\n",
    ));
    assert_eq!(e.riga, Some(4));
    assert!(e.messaggio.contains("i membri del gruppo vanno elencati tutti subito dopo «con»"), "{e}");
}

#[test]
fn gruppo_errore_posizione_propria() {
    for pos in ["al centro", "in alto", "a 10, 10", "a destra di \"a\" di 10"] {
        let e = errore(&src(&format!(
            "Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\" {pos}.\n"
        )));
        assert_eq!(e.riga, Some(3), "{pos}");
        assert!(e.messaggio.contains("un gruppo non ha una posizione propria"), "{pos}: {e}");
        assert!(e.suggerimento.is_some(), "{pos}: {e}");
    }
}

#[test]
fn gruppo_errore_proprieta_non_applicabili() {
    for prop in ["colore rosso", "bordo 2 rosso", "raggio 10", "lato 10", "testo \"x\""] {
        let e = errore(&src(&format!(
            "Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\" e {prop}.\n"
        )));
        assert_eq!(e.riga, Some(3), "{prop}");
        assert!(e.messaggio.contains("non si applica a un gruppo") || e.messaggio.contains("i membri del gruppo"), "{prop}: {e}");
    }
    let e = errore(&src("Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\", colore rosso.\n"));
    assert!(e.messaggio.contains("la proprietà «colore» non si applica a un gruppo"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("singoli membri"), "{e}");
}

#[test]
fn gruppo_dentro_a_rifiutato() {
    let msg = "la posizione «dentro a» non esiste: i membri si elencano nella frase del gruppo";
    let e = errore(&src("Crea un cerchio chiamato \"a\" dentro a \"g\".\n"));
    assert_eq!(e.riga, Some(2));
    assert!(e.messaggio.contains(msg), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("Crea un gruppo chiamato \"g\" con \"a\" e \"b\"."));
    let e = errore(&src("Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\" dentro a \"h\".\n"));
    assert!(e.messaggio.contains(msg), "{e}");
    let e = errore(&src("Crea un cerchio chiamato \"a\" a destra dentro a \"g\".\n"));
    assert!(e.messaggio.contains(msg), "{e}");
}

#[test]
fn gruppo_cambia_colore_vietato() {
    let e = errore(&src(
        "Crea un cerchio chiamato \"a\".\nCrea un gruppo chiamato \"g\" con \"a\".\n\
         Dal secondo 0 al secondo 1:\n  \"g\" cambia colore in rosso.\n",
    ));
    assert_eq!(e.riga, Some(5));
    assert!(e.messaggio.contains("\"g\" è un gruppo e non ha un colore da cambiare"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("cambia il colore dei singoli membri"), "{e}");
}

// ================================================ gruppi: pivot e coordinate

#[test]
fn gruppo_pivot_e_centro_dell_aabb() {
    // a: cerchio r10 a (100,100) -> [90,110]x[90,110]; b: r20 a (300,200) -> [280,320]x[180,220].
    // AABB [90,320]x[90,220] -> P = (205, 155).
    let s = scena(&src(
        "Crea un cerchio chiamato \"a\" con raggio 10 a 100, 100.\n\
         Crea un cerchio chiamato \"b\" con raggio 20 a 300, 200.\n\
         Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\n",
    ));
    let g = att(&s, "g");
    assert_vicino(g.x, 205.0, "g.x");
    assert_vicino(g.y, 155.0, "g.y");
    assert_eq!(g.genitore, None);
    let (a, b) = (att(&s, "a"), att(&s, "b"));
    assert_eq!((a.genitore, b.genitore), (Some(0), Some(0)));
    assert_vicino(a.x, -105.0, "a.x");
    assert_vicino(a.y, -55.0, "a.y");
    assert_vicino(b.x, 95.0, "b.x");
    assert_vicino(b.y, 45.0, "b.y");
}

#[test]
fn gruppo_pivot_usa_scala_e_rotazione_iniziali_dei_membri() {
    // Quadrato lato 100 con scala 2 al centro (500,250): ingombro 100 per lato.
    // Insieme a un cerchio r10 a (800,250) -> AABB x [400, 810] -> P.x = 605.
    let s = scena(&src(
        "Crea un quadrato chiamato \"q\" con lato 100 e scala 2 al centro.\n\
         Crea un cerchio chiamato \"c\" con raggio 10 a 800, 250.\n\
         Crea un gruppo chiamato \"g\" con \"q\" e \"c\".\n",
    ));
    assert_vicino(att(&s, "g").x, 605.0, "g.x");
    assert_vicino(att(&s, "g").y, 250.0, "g.y");
}

#[test]
fn gruppo_scala_iniziale_non_sposta_i_membri_ma_il_css_la_applica_al_gruppo() {
    // Il pivot e le coordinate dei membri sono quelli PRIMA della trasformazione (B.3.3).
    let sorg = src(&format!("{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\" e \"b\", scala 0.5 e rotazione 20.\n"));
    let s = scena(&sorg);
    assert_vicino(att(&s, "g").x, 205.0, "g.x");
    assert_vicino(att(&s, "a").x, -105.0, "a.x");
    assert_eq!(att(&s, "g").scala_iniziale, 0.5);
    assert_eq!(att(&s, "g").rotazione_iniziale, 20.0);
    let h = html(&sorg);
    let r = regola(&h, &format!("#a{}", indice(&s, "g")));
    assert!(r.contains("scale:0.5") && r.contains("rotate:20deg"), "{r}");
    // I membri non ereditano la trasformazione nel loro CSS.
    let ra = regola(&h, &format!("#a{}", indice(&s, "a")));
    assert!(!ra.contains("scale:") && !ra.contains("rotate:"), "{ra}");
}

#[test]
fn gruppo_annidato_coordinate_relative_al_genitore() {
    // g1 = {a, b}: P1 = (205,155). g2 = {g1, c}.
    // Ingombro iniziale di g1: [90,320]x[90,220]; c r30 a (600,300): [570,630]x[270,330].
    // AABB di g2: [90,630]x[90,330] -> P2 = (360, 210).
    let s = scena(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g1\" con \"a\" e \"b\".\nCrea un gruppo chiamato \"g2\" con \"g1\" e \"c\".\n"
    )));
    let (g1, g2, a, c) = (att(&s, "g1"), att(&s, "g2"), att(&s, "a"), att(&s, "c"));
    assert_vicino(g2.x, 360.0, "g2.x");
    assert_vicino(g2.y, 210.0, "g2.y");
    assert_eq!(g2.genitore, None);
    assert_eq!(g1.genitore, Some(indice(&s, "g2")));
    // g1 relativo a P2: (205,155) - (360,210).
    assert_vicino(g1.x, -155.0, "g1.x");
    assert_vicino(g1.y, -55.0, "g1.y");
    // a relativo a P1 (non a P2).
    assert_vicino(a.x, -105.0, "a.x");
    assert_vicino(a.y, -55.0, "a.y");
    // c relativo a P2: (600,300) - (360,210).
    assert_vicino(c.x, 240.0, "c.x");
    assert_vicino(c.y, 90.0, "c.y");
}

// ===================================================== gruppi: pre-ordine

#[test]
fn gruppo_preordine_e_ordine_dei_fratelli() {
    // Righe: a=2, b=3, c=4, g1{a,c}=5. g1 prende il posto di "a" (riga minima): [g1, a, c, b].
    let s = scena(&src(
        "Crea un cerchio chiamato \"a\".\nCrea un cerchio chiamato \"b\".\nCrea un cerchio chiamato \"c\".\n\
         Crea un gruppo chiamato \"g1\" con \"a\" e \"c\".\n",
    ));
    let nomi: Vec<&str> = s.attori.iter().map(|a| a.nome.as_str()).collect();
    assert_eq!(nomi, ["g1", "a", "c", "b"]);
    let genitori: Vec<Option<usize>> = s.attori.iter().map(|a| a.genitore).collect();
    assert_eq!(genitori, [None, Some(0), Some(0), None]);
    let ids: Vec<&str> = s.attori.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["a0", "a1", "a2", "a3"]);
    assert_eq!(s.figli(None).collect::<Vec<_>>(), [0, 3]);
    assert_eq!(s.figli(Some(0)).collect::<Vec<_>>(), [1, 2]);
}

#[test]
fn gruppo_invarianti_del_preordine() {
    // Gruppi annidati e fratelli sparsi: verifica gli invarianti 1 e 2 di `ScenaCompilata::attori`.
    let s = scena(&src(
        "Crea un cerchio chiamato \"a\".\nCrea un cerchio chiamato \"b\".\nCrea un cerchio chiamato \"c\".\n\
         Crea un cerchio chiamato \"d\".\nCrea un gruppo chiamato \"g1\" con \"b\" e \"c\".\n\
         Crea un gruppo chiamato \"g2\" con \"a\" e \"g1\".\n",
    ));
    for (i, a) in s.attori.iter().enumerate() {
        if let Some(g) = a.genitore {
            assert!(g < i, "genitore prima dei figli");
            assert!(matches!(s.attori[g].forma, Forma::Gruppo { .. }));
        }
    }
    let nomi: Vec<&str> = s.attori.iter().map(|a| a.nome.as_str()).collect();
    assert_eq!(nomi, ["g2", "a", "g1", "b", "c", "d"]);
}

#[test]
fn senza_gruppi_ordine_e_id_della_v01() {
    let s = scena(&src(
        "Crea un cerchio chiamato \"x\".\nCrea un cerchio chiamato \"y\".\nCrea un cerchio chiamato \"z\".\n",
    ));
    let nomi: Vec<&str> = s.attori.iter().map(|a| a.nome.as_str()).collect();
    assert_eq!(nomi, ["x", "y", "z"]);
    assert!(s.attori.iter().all(|a| a.genitore.is_none()));
    assert_eq!(s.attori[2].id, "a2");
}

// ========================================================== gruppi: HTML

#[test]
fn gruppo_html_annidato_e_bilanciato_senza_forma() {
    let h = html(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g1\" con \"a\" e \"b\".\nCrea un gruppo chiamato \"g2\" con \"g1\" e \"c\".\n"
    )));
    assert!(div_bilanciati(&h), "div non bilanciati");
    // Pre-ordine: g2=a0, g1=a1, a=a2, b=a3, c=a4.
    assert!(h.contains("<div class=\"attore gruppo\" id=\"a0\" data-nome=\"g2\"><div class=\"attore gruppo\" id=\"a1\" data-nome=\"g1\"><div class=\"attore\" id=\"a2\" data-nome=\"a\">"), "{h}");
    // Nessuna `.forma` nei gruppi: 3 `.forma` per 3 cerchi, non 5.
    assert_eq!(h.matches("class=\"forma\"").count(), 3);
    assert_eq!(h.matches("class=\"attore gruppo\"").count(), 2);
    // Nessuna regola `#aK>.forma` per i gruppi.
    assert!(!h.contains("#a0>.forma") && !h.contains("#a1>.forma"));
    // Il gruppo chiude dopo i suoi membri: "c" è dentro g2 ma fuori da g1.
    let pos_c = h.find("data-nome=\"c\"").unwrap();
    let pos_b = h.find("data-nome=\"b\"").unwrap();
    assert!(pos_c > pos_b);
    let tra_b_e_c = &h[pos_b..pos_c];
    assert!(tra_b_e_c.matches("</div>").count() > tra_b_e_c.matches("<div").count(), "g1 deve chiudersi prima di c");
}

#[test]
fn gruppo_html_left_top_relativi_al_genitore() {
    let h = html(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\n"
    )));
    // g = a0 con P = (205,155); a = a1 a (-105,-55).
    let rg = regola(&h, "#a0");
    assert!(rg.contains("left:205px") && rg.contains("top:155px"), "{rg}");
    let ra = regola(&h, "#a1");
    assert!(ra.contains("left:-105px") && ra.contains("top:-55px"), "{ra}");
}

#[test]
fn gruppo_html_pretty_bilanciato() {
    let opz = Opzioni { leggibile: true, ..Default::default() };
    let h = compila_sorgente(
        &src(&format!("{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\n")),
        &opz,
    )
    .unwrap();
    assert!(div_bilanciati(&h));
}

#[test]
fn gruppo_profondita_16_ok_17_errore() {
    let catena = |n: usize| {
        let mut s = String::from("Crea un cerchio chiamato \"a\".\n");
        let mut prec = "a".to_string();
        for i in 1..=n {
            s.push_str(&format!("Crea un gruppo chiamato \"g{i}\" con \"{prec}\".\n"));
            prec = format!("g{i}");
        }
        src(&s)
    };
    let ok = catena(16);
    let sc = scena(&ok);
    assert_eq!(sc.attori.len(), 17);
    assert!(div_bilanciati(&html(&ok)));
    let e = errore(&catena(17));
    // Il gruppo g17 è alla riga 2 (palco) + 1 (a) + 17 = 19... (palco=1, a=2, g1=3, ..., g17=19).
    assert_eq!(e.riga, Some(19));
    assert!(e.messaggio.contains("gruppi annidati troppo in profondità (massimo 16 livelli)"), "{e}");
}

#[test]
fn gruppo_opacita_sul_wrapper_del_gruppo() {
    let h = html(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\" e \"b\", opacità 0.5 e livello 2.\n"
    )));
    let rg = regola(&h, "#a0");
    assert!(rg.contains("opacity:0.5") && rg.contains("z-index:2"), "{rg}");
    // I membri conservano la loro opacità (1): è il CSS a moltiplicarla.
    let ra = regola(&h, "#a1");
    assert!(ra.contains("opacity:1"), "{ra}");
}

#[test]
fn gruppo_appare_sfumando_parte_invisibile() {
    let s = scena(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\nDal secondo 0 al secondo 2:\n  \"g\" appare sfumando.\n"
    )));
    let g = att(&s, "g");
    let an = g.animazioni.iter().find(|a| a.canale == Canale::Opacita).expect("opacità");
    assert_eq!(an.keyframe.first().unwrap().valore, Valore::Opacita(0.0));
    assert_eq!(an.keyframe.last().unwrap().valore, Valore::Opacita(1.0));
    // I membri non sono toccati.
    assert!(att(&s, "a").animazioni.is_empty());
}

#[test]
fn gruppo_azioni_ammesse_e_canali_indipendenti_dai_membri() {
    // Gruppo e membro animano la traslazione nello stesso istante: canali indipendenti (B.0.1).
    let s = scena(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\n\
         Dal secondo 0 al secondo 2:\n  \"g\" si sposta a 500, 250 e ruota di 90 gradi e si ingrandisce fino a 2.\n  \"a\" scivola verso destra di 50.\n"
    )));
    assert!(ha_canale(&s, "g", Canale::Traslazione));
    assert!(ha_canale(&s, "g", Canale::Rotazione));
    assert!(ha_canale(&s, "g", Canale::Scala));
    // `si sposta a` porta il pivot (205,155) a (500,250): traslazione finale (295, 95).
    let t = trasl(&s, "g");
    let ultimo = t.last().unwrap();
    assert_vicino(ultimo.1, 295.0, "g dx");
    assert_vicino(ultimo.2, 95.0, "g dy");
    let ta = trasl(&s, "a");
    assert_vicino(ta.last().unwrap().1, 50.0, "a dx");
}

#[test]
fn gruppo_due_azioni_sullo_stesso_canale_si_sovrappongono() {
    let e = errore(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\".\nDal secondo 0 al secondo 2:\n  \"g\" ruota di 10 gradi.\nDal secondo 1 al secondo 3:\n  \"g\" ruota di 10 gradi.\n"
    )));
    assert!(e.messaggio.contains("la rotazione"), "{e}");
}

// ============================================== gruppi: posizioni relative

#[test]
fn gruppo_posizione_relativa_usa_il_pivot() {
    // P del gruppo = (205,155). "x" a destra del gruppo di 100 -> (305, 155).
    let s = scena(&src(
        "Crea un cerchio chiamato \"a\" con raggio 10 a 100, 100.\n\
         Crea un cerchio chiamato \"b\" con raggio 20 a 300, 200.\n\
         Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\n\
         Crea un cerchio chiamato \"x\" a destra di \"g\" di 100.\n",
    ));
    let x = att(&s, "x");
    assert_vicino(x.x, 305.0, "x.x");
    assert_vicino(x.y, 155.0, "x.y");
    assert_eq!(x.genitore, None);
}

#[test]
fn gruppo_posizione_riferimento_in_avanti_ammesso() {
    // "x" riferisce un gruppo creato dopo di lui (come nella v0.1 per gli attori).
    let s = scena(&src(
        "Crea un cerchio chiamato \"x\" sotto a \"g\" di 50.\n\
         Crea un cerchio chiamato \"a\" con raggio 10 a 100, 100.\n\
         Crea un gruppo chiamato \"g\" con \"a\".\n",
    ));
    assert_vicino(att(&s, "x").x, 100.0, "x.x");
    assert_vicino(att(&s, "x").y, 150.0, "x.y");
}

#[test]
fn gruppo_ciclo_membro_posizionato_rispetto_al_proprio_gruppo() {
    let e = errore(&src(
        "Crea un cerchio chiamato \"a\" a destra di \"g\" di 50.\n\
         Crea un gruppo chiamato \"g\" con \"a\".\n",
    ));
    assert!(e.messaggio.contains("posizioni circolari"), "{e}");
}

#[test]
fn gruppo_riferire_un_membro_da_fuori_usa_la_posizione_dichiarata() {
    let s = scena(&src(&format!(
        "{TRE_CERCHI}Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\nCrea un cerchio chiamato \"x\" sopra a \"a\" di 30.\n"
    )));
    let x = att(&s, "x");
    assert_vicino(x.x, 100.0, "x.x");
    assert_vicino(x.y, 70.0, "x.y");
}

// ================================================== entra da / esce verso

/// Quadrato lato 100 al centro (500,250), senza rotazione né scala: ingombro 50 per lato.
const Q: &str = "Crea un quadrato chiamato \"a\" con lato 100 a 500, 250.\n";

fn entra(dir: &str) -> ScenaCompilata {
    scena(&src(&format!("{Q}Dal secondo 1 al secondo 3:\n  \"a\" entra da {dir}.\n")))
}

#[test]
fn entra_da_le_quattro_direzioni_primo_movimento() {
    // M = 2: xF sinistra = -50-2; destra = 1000+50+2; yF alto = -52; basso = 500+52.
    let casi = [("sinistra", -552.0, 0.0), ("destra", 552.0, 0.0), ("alto", 0.0, -302.0), ("basso", 0.0, 302.0)];
    for (dir, dx, dy) in casi {
        let s = entra(dir);
        let t = trasl(&s, "a");
        // Primo movimento: già fuori campo a t = 0 e fino a t0, senza salto.
        assert_vicino(t[0].1, dx, &format!("{dir} dx0"));
        assert_vicino(t[0].2, dy, &format!("{dir} dy0"));
        assert_vicino(t[1].1, dx, &format!("{dir} dx a t0"));
        assert_vicino(t[1].2, dy, &format!("{dir} dy a t0"));
        assert_vicino(t[1].0, 1.0 / 3.0, "offset di t0");
        let ult = t.last().unwrap();
        assert_vicino(ult.1, 0.0, "arrivo dx");
        assert_vicino(ult.2, 0.0, "arrivo dy");
        assert!(t.iter().all(|k| k.3 != Some(Curva::Gradino)), "{dir}: nessun salto sul primo movimento");
        let h = html(&src(&format!("{Q}Dal secondo 1 al secondo 3:\n  \"a\" entra da {dir}.\n")));
        assert!(!h.contains("steps("), "{dir}");
    }
}

#[test]
fn entra_da_preposizioni_articolate() {
    let verbo = |frase: &str| {
        let p = analizza(&src(&format!("{Q}Dal secondo 0 al secondo 1:\n  \"a\" {frase}.\n"))).unwrap();
        p.timeline.blocchi[0].azioni[0].verbo.clone()
    };
    assert_eq!(verbo("entra da sinistra"), Verbo::EntraDa(Direzione::Sinistra));
    assert_eq!(verbo("entra dal basso"), Verbo::EntraDa(Direzione::Basso));
    assert_eq!(verbo("entra dall'alto"), Verbo::EntraDa(Direzione::Alto));
    assert_eq!(verbo("entra dalla destra"), Verbo::EntraDa(Direzione::Destra));
    assert_eq!(verbo("entra dalla sinistra"), Verbo::EntraDa(Direzione::Sinistra));
    assert_eq!(verbo("entra dall’alto"), Verbo::EntraDa(Direzione::Alto));
    assert_eq!(verbo("esce verso l'alto"), Verbo::EsceVerso(Direzione::Alto));
    assert_eq!(verbo("esce verso destra"), Verbo::EsceVerso(Direzione::Destra));
    assert_eq!(verbo("esce verso il basso"), Verbo::EsceVerso(Direzione::Basso));
    assert_eq!(verbo("esce verso sinistra"), Verbo::EsceVerso(Direzione::Sinistra));
}

#[test]
fn entra_da_con_movimento_e_concatenato() {
    let s = scena(&src(&format!(
        "{Q}Dal secondo 0 al secondo 2:\n  \"a\" entra da sinistra con movimento scatto e appare sfumando.\n"
    )));
    assert!(ha_canale(&s, "a", Canale::Traslazione));
    assert!(ha_canale(&s, "a", Canale::Opacita));
}

#[test]
fn esce_verso_le_quattro_direzioni() {
    let casi = [("sinistra", -552.0, 0.0), ("destra", 552.0, 0.0), ("alto", 0.0, -302.0), ("basso", 0.0, 302.0)];
    for (dir, dx, dy) in casi {
        let s = scena(&src(&format!("{Q}Dal secondo 0 al secondo 2:\n  \"a\" esce verso {dir}.\n")));
        let t = trasl(&s, "a");
        assert_vicino(t[0].1, 0.0, "parte dalla posizione dichiarata");
        let ult = t.last().unwrap();
        assert_vicino(ult.1, dx, &format!("{dir} dx"));
        assert_vicino(ult.2, dy, &format!("{dir} dy"));
    }
}

#[test]
fn esce_verso_cambia_solo_l_asse_del_movimento() {
    // Dopo `scivola verso basso di 20` la y corrente è +20: `esce verso destra` la mantiene.
    let s = scena(&src(&format!(
        "{Q}Dal secondo 0 al secondo 1:\n  \"a\" scivola verso basso di 20.\nPoi per 1 secondo:\n  \"a\" esce verso destra.\n"
    )));
    let ult = trasl(&s, "a").last().unwrap().clone();
    assert_vicino(ult.1, 552.0, "dx");
    assert_vicino(ult.2, 20.0, "dy resta quella corrente");
}

#[test]
fn entra_da_non_primo_movimento_salta_con_steps() {
    // Esce a destra (fino a t=2), poi rientra da sinistra: in t0 = 3 salta (B.0.3).
    let sorg = src(&format!(
        "{Q}Dal secondo 0 al secondo 2:\n  \"a\" esce verso destra.\nDal secondo 3 al secondo 5:\n  \"a\" entra da sinistra.\n"
    ));
    let s = scena(&sorg);
    let t = trasl(&s, "a");
    let durata = 5.0;
    let i = t.iter().position(|k| k.3 == Some(Curva::Gradino)).expect("keyframe con Gradino");
    assert_vicino(t[i].0, 3.0 / durata, "offset del salto");
    assert_vicino(t[i].1, 552.0, "valore corrente in t0");
    // Il keyframe successivo (t0 + δ) ha il punto di partenza fuori campo a sinistra.
    let delta = (0.001f64).max(2e-6 * durata);
    assert_vicino(t[i + 1].0, (3.0 + delta) / durata, "offset di t0 + δ");
    assert_vicino(t[i + 1].1, -552.0, "partenza a sinistra");
    assert_vicino(t.last().unwrap().1, 0.0, "arrivo alla posizione dichiarata");
    let h = html(&sorg);
    assert!(h.contains("animation-timing-function:steps(1,end)"), "manca steps(1,end)");
}

#[test]
fn entra_da_non_primo_dopo_si_sposta_a() {
    let s = scena(&src(&format!(
        "{Q}Dal secondo 0 al secondo 1:\n  \"a\" si sposta a 700, 250.\nDal secondo 2 al secondo 3:\n  \"a\" entra da alto.\n"
    )));
    let t = trasl(&s, "a");
    let i = t.iter().position(|k| k.3 == Some(Curva::Gradino)).expect("salto");
    assert_vicino(t[i].1, 200.0, "dx corrente");
    assert_vicino(t[i + 1].1, 0.0, "x di partenza = dichiarata");
    assert_vicino(t[i + 1].2, -302.0, "y fuori campo");
}

#[test]
fn fuori_campo_prudente_per_forme_ruotate_e_scalate() {
    let primo_dx = |decl: &str, verbi: &str| {
        let s = scena(&src(&format!(
            "Crea un {decl} a 500, 250.\nDal secondo 0 al secondo 2:\n  \"a\" {verbi}.\n"
        )));
        trasl(&s, "a")[0].1
    };
    // Quadrato lato 100 con rotazione iniziale 45: ex = 50(cos45 + sin45) = 70.7107.
    let ex = 50.0 * (std::f64::consts::FRAC_PI_4.cos() + std::f64::consts::FRAC_PI_4.sin());
    assert_vicino(
        primo_dx("quadrato chiamato \"a\" con lato 100 e rotazione 45", "entra da sinistra"),
        -ex - 2.0 - 500.0,
        "rotazione iniziale",
    );
    // Scala iniziale 2 senza rotazione: ex = 100.
    assert_vicino(
        primo_dx("quadrato chiamato \"a\" con lato 100 e scala 2", "entra da sinistra"),
        -100.0 - 2.0 - 500.0,
        "scala iniziale",
    );
    // Scala che passa da 1 a 3: s_rif = 3 -> ex = 150.
    assert_vicino(
        primo_dx("quadrato chiamato \"a\" con lato 100", "entra da sinistra e si ingrandisce fino a 3"),
        -150.0 - 2.0 - 500.0,
        "scala animata",
    );
    // Rotazione animata: cerchio circoscritto. Rettangolo 200x100: sqrt(100^2 + 50^2) = 111.803.
    let r = (100.0f64.powi(2) + 50.0f64.powi(2)).sqrt();
    assert_vicino(
        primo_dx(
            "rettangolo chiamato \"a\" con larghezza 200 e altezza 100",
            "entra da sinistra e ruota di 90 gradi",
        ),
        -r - 2.0 - 500.0,
        "rotazione animata",
    );
    // Verso l'alto la rotazione non sfrutta l'asse minore: stessa estensione.
    let s = scena(&src(
        "Crea un rettangolo chiamato \"a\" con larghezza 200 e altezza 100 a 500, 250.\n\
         Dal secondo 0 al secondo 2:\n  \"a\" entra da alto e ruota di 90 gradi.\n",
    ));
    assert_vicino(trasl(&s, "a")[0].2, -r - 2.0 - 250.0, "alto con rotazione animata");
}

#[test]
fn fuori_campo_prudente_per_i_testi() {
    // "ABCD" grandezza 20: w = 20*4 = 80 (ex = 40), h = 1.5*20 = 30 (ey = 15).
    let t = |dir: &str| {
        let s = scena(&src(&format!(
            "Crea un testo chiamato \"a\" con testo \"ABCD\" e grandezza 20 a 500, 250.\nDal secondo 0 al secondo 1:\n  \"a\" entra da {dir}.\n"
        )));
        trasl(&s, "a")[0].clone()
    };
    assert_vicino(t("sinistra").1, -40.0 - 2.0 - 500.0, "testo sinistra");
    assert_vicino(t("destra").1, 1000.0 + 40.0 + 2.0 - 500.0, "testo destra");
    assert_vicino(t("alto").2, -15.0 - 2.0 - 250.0, "testo alto");
    assert_vicino(t("basso").2, 500.0 + 15.0 + 2.0 - 250.0, "testo basso");
}

#[test]
fn fuori_campo_testi_cjk_e_titoli_usano_il_coefficiente_largo() {
    // Due caratteri CJK: k = 1.25 -> w = 20 * 2 * 1.25 = 50 (ex = 25).
    let s = scena(&src(
        "Crea un testo chiamato \"a\" con testo \"日本\" e grandezza 20 a 500, 250.\nDal secondo 0 al secondo 1:\n  \"a\" entra da sinistra.\n",
    ));
    assert_vicino(trasl(&s, "a")[0].1, -25.0 - 2.0 - 500.0, "testo CJK");
    // Titolo: grandezza 36, 16 caratteri latini -> w = 576 (come in B.8).
    let s = scena(&src(
        "Crea un titolo chiamato \"t\" con testo \"SISTEMA ORBITALE\", grandezza 36 a 500, 250.\nDal secondo 0 al secondo 1:\n  \"t\" entra da destra.\n",
    ));
    assert_vicino(trasl(&s, "t")[0].1, 1000.0 + 288.0 + 2.0 - 500.0, "titolo");
}

#[test]
fn entra_da_cerchio_e_gruppo() {
    // Cerchio r40 a (500,250): ex = 40.
    let s = scena(&src(
        "Crea un cerchio chiamato \"c\" con raggio 40 al centro.\nDal secondo 0 al secondo 1:\n  \"c\" entra da basso.\n",
    ));
    assert_vicino(trasl(&s, "c")[0].2, 500.0 + 40.0 + 2.0 - 250.0, "cerchio basso");
}

#[test]
fn entra_da_sovrapposto_a_scivola_e_errore() {
    for altro in ["\"a\" scivola verso destra di 10.", "\"a\" si sposta a 100, 100.", "\"a\" esce verso destra."] {
        let e = errore(&src(&format!("{Q}Dal secondo 0 al secondo 2:\n  \"a\" entra da sinistra.\n  {altro}\n")));
        assert_eq!(e.riga, Some(5), "{altro}");
        assert!(e.messaggio.contains("la posizione"), "{altro}: {e}");
        assert!(e.messaggio.contains("righe 4 e 5"), "{altro}: {e}");
    }
}

#[test]
fn entra_esce_errori_di_sintassi() {
    let e = errore(&src(&format!("{Q}Dal secondo 0 al secondo 1:\n  \"a\" entra sinistra.\n")));
    assert_eq!(e.riga, Some(4));
    assert!(e.messaggio.contains("mi aspettavo «da» dopo «entra»"), "{e}");
    let e = errore(&src(&format!("{Q}Dal secondo 0 al secondo 1:\n  \"a\" esce destra.\n")));
    assert!(e.messaggio.contains("mi aspettavo «verso» dopo «esce»"), "{e}");
    let e = errore(&src(&format!("{Q}Dal secondo 0 al secondo 1:\n  \"a\" entra da sinsitra.\n")));
    assert!(e.messaggio.contains("direzione non valida"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("sinistra"), "{e}");
}

#[test]
fn entra_da_coordinate_fuori_dai_limiti() {
    // Quadrato enorme (lato 100000, scala 20): l'estensione supera MAX_COORDINATA.
    let e = errore(&src(
        "Crea un quadrato chiamato \"a\" con lato 100000 e scala 20.\nDal secondo 0 al secondo 1:\n  \"a\" entra da sinistra.\n",
    ));
    assert!(e.messaggio.contains("coordinate fuori dai limiti"), "{e}");
}

// ================================================================= orbita

/// Due cerchi: "sole" al centro (500,250) e "terra" a (700,250), distanza 200.
const SOLE_TERRA: &str = "Crea un cerchio chiamato \"sole\" con raggio 30 a 500, 250.\n\
     Crea un cerchio chiamato \"terra\" con raggio 10 a 700, 250.\n";

fn orbita(frase: &str) -> ScenaCompilata {
    scena(&src(&format!("{SOLE_TERRA}Dal secondo 0 al secondo 4:\n  {frase}\n")))
}

fn n_giro(raggio: f64) -> usize {
    if raggio <= 1.0 {
        return 36;
    }
    let n = (2.0 * std::f64::consts::PI / (2.0 * (1.0 - 0.5 / raggio).acos())).ceil() as usize;
    n.clamp(36, 360)
}

#[test]
fn orbita_verso_predefinito_antiorario() {
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro con movimento lineare.");
    let t = trasl(&s, "terra");
    assert_vicino(t[0].1, 0.0, "parte da θ0 = 0, senza salto");
    // Antiorario sullo schermo (y verso il basso): θ decresce, quindi y diventa negativo.
    assert!(t[1].2 < 0.0, "antiorario: dy deve scendere sotto 0, trovato {}", t[1].2);
    // Dopo un quarto di giro (campione intero più vicino) il pianeta è sopra il sole.
    let n = n_giro(200.0);
    assert_eq!(t.len(), n + 1);
}

#[test]
fn orbita_in_senso_orario() {
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro in senso orario con movimento lineare.");
    let t = trasl(&s, "terra");
    assert!(t[1].2 > 0.0, "orario: dy deve salire sopra 0, trovato {}", t[1].2);
    // Esplicito antiorario = predefinito.
    let a = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro in senso antiorario con movimento lineare.");
    let d = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro con movimento lineare.");
    assert_eq!(trasl(&a, "terra"), trasl(&d, "terra"));
}

#[test]
fn orbita_giri_frazionari() {
    // 0.25 giri antiorari da θ0 = 0: arriva sopra il centro, (500, 50) -> traslazione (-200, -200).
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 0.25 giri con movimento lineare.");
    let u = trasl(&s, "terra").last().unwrap().clone();
    assert_vicino(u.1, -200.0, "0.25 giri dx");
    assert_vicino(u.2, -200.0, "0.25 giri dy");
    // 0.5 giri orari: arriva a sinistra del centro, traslazione (-400, 0).
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 0.5 giri in senso orario con movimento lineare.");
    let t = trasl(&s, "terra");
    let u = t.last().unwrap();
    assert_vicino(u.1, -400.0, "0.5 giri dx");
    assert_vicino(u.2, 0.0, "0.5 giri dy");
    assert_eq!(t.len(), (n_giro(200.0) as f64 * 0.5).ceil() as usize + 1);
    // 2.5 giri: campioni = ceil(n * 2.5).
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 2.5 giri con movimento lineare.");
    assert_eq!(trasl(&s, "terra").len(), (n_giro(200.0) as f64 * 2.5).ceil() as usize + 1);
}

#[test]
fn orbita_numero_di_campioni_nei_limiti() {
    // Raggio piccolo: clamp inferiore a 36; raggio grande: clamp superiore a 360.
    let casi = [(2.0, 36usize), (1.0, 36), (200.0, n_giro(200.0)), (100_000.0, 360)];
    for (raggio, atteso) in casi {
        // "terra" a distanza = raggio dal sole: nessun salto, quindi N + 1 keyframe.
        let s = scena(&src(&format!(
            "Crea un cerchio chiamato \"sole\" a 500, 250.\nCrea un cerchio chiamato \"terra\" a {}, 250.\n\
             Dal secondo 0 al secondo 4:\n  \"terra\" orbita attorno a \"sole\" con raggio {raggio} per 1 giro con movimento lineare.\n",
            500.0 + raggio
        )));
        let n = trasl(&s, "terra").len() - 1;
        assert_eq!(n, atteso, "raggio {raggio}");
        assert!((36..=360).contains(&atteso));
    }
    assert_eq!(n_giro(200.0), 45);
}

#[test]
fn orbita_nessun_salto_se_distanza_uguale_al_raggio() {
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro con movimento lineare.");
    assert!(trasl(&s, "terra").iter().all(|k| k.3 != Some(Curva::Gradino)));
    // Anche con scarto ≤ 0.5 px (distanza 200.3 contro raggio 200).
    let s = scena(&src(
        "Crea un cerchio chiamato \"sole\" a 500, 250.\nCrea un cerchio chiamato \"terra\" a 700.3, 250.\n\
         Dal secondo 0 al secondo 4:\n  \"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro.\n",
    ));
    assert!(trasl(&s, "terra").iter().all(|k| k.3 != Some(Curva::Gradino)), "scarto ≤ 0.5 px: niente salto");
}

#[test]
fn orbita_salto_sul_cerchio_se_distanza_diversa_dal_raggio() {
    // terra a (650,250): distanza 150, raggio 200, θ0 = 0 -> salta a (700,250): traslazione (50,0).
    let sorg = src(
        "Crea un cerchio chiamato \"sole\" a 500, 250.\nCrea un cerchio chiamato \"terra\" a 650, 250.\n\
         Dal secondo 1 al secondo 5:\n  \"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro con movimento lineare.\n",
    );
    let s = scena(&sorg);
    let t = trasl(&s, "terra");
    let i = t.iter().position(|k| k.3 == Some(Curva::Gradino)).expect("salto");
    assert_vicino(t[i].0, 1.0 / 5.0, "offset t0");
    assert_vicino(t[i].1, 0.0, "valore corrente");
    let delta = (0.001f64).max(2e-6 * 5.0);
    assert_vicino(t[i + 1].0, (1.0 + delta) / 5.0, "offset t0 + δ");
    assert_vicino(t[i + 1].1, 50.0, "punto sul cerchio dx");
    assert_vicino(t[i + 1].2, 0.0, "punto sul cerchio dy");
    assert!(html(&sorg).contains("steps(1,end)"));
    // L'arrivo di un giro intero coincide con il punto di partenza sul cerchio.
    let u = t.last().unwrap();
    assert_vicino(u.1, 50.0, "arrivo dx");
    assert_vicino(u.2, 0.0, "arrivo dy");
}

#[test]
fn orbita_angolo_di_partenza_dalla_posizione_corrente() {
    // terra sopra il sole a (500,50): θ0 = -π/2; un quarto di giro orario porta a (700,250).
    let s = scena(&src(
        "Crea un cerchio chiamato \"sole\" a 500, 250.\nCrea un cerchio chiamato \"terra\" a 500, 50.\n\
         Dal secondo 0 al secondo 2:\n  \"terra\" orbita attorno a \"sole\" con raggio 200 per 0.25 giri in senso orario.\n",
    ));
    let u = trasl(&s, "terra").last().unwrap().clone();
    assert_vicino(u.1, 200.0, "dx");
    assert_vicino(u.2, 200.0, "dy");
}

#[test]
fn orbita_i_campioni_stanno_sul_cerchio() {
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro con movimento lineare.");
    for (_, dx, dy, _) in trasl(&s, "terra") {
        let d = ((700.0 + dx - 500.0).powi(2) + (250.0 + dy - 250.0).powi(2)).sqrt();
        assert!((d - 200.0).abs() < 0.01, "il campione dista {d} dal centro");
    }
}

#[test]
fn orbita_attorno_al_gruppo_genitore_usa_il_pivot() {
    // g = {a (100,100), b (300,100)}: AABB x [90,310] -> P = (200, 100).
    // "a" orbita attorno a "g" (suo genitore diretto) con raggio 100 per 0.5 giri: parte da
    // (-100, 0) relativo a P, θ0 = π, e arriva a (+100, 0): traslazione finale (200, 0).
    let s = scena(&src(
        "Crea un cerchio chiamato \"a\" con raggio 10 a 100, 100.\nCrea un cerchio chiamato \"b\" con raggio 10 a 300, 100.\n\
         Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\n\
         Dal secondo 0 al secondo 2:\n  \"a\" orbita attorno a \"g\" con raggio 100 per 0.5 giri con movimento lineare.\n",
    ));
    let u = trasl(&s, "a").last().unwrap().clone();
    assert_vicino(u.1, 200.0, "dx");
    assert_vicino(u.2, 0.0, "dy");
    assert!(trasl(&s, "a").iter().all(|k| k.3 != Some(Curva::Gradino)), "distanza = raggio: niente salto");
}

#[test]
fn orbita_tra_fratelli_di_un_gruppo() {
    let s = scena(&src(
        "Crea un cerchio chiamato \"sole\" con raggio 30 a 400, 250.\nCrea un cerchio chiamato \"terra\" con raggio 10 a 600, 250.\n\
         Crea un gruppo chiamato \"sistema\" con \"sole\" e \"terra\".\n\
         Dal secondo 0 al secondo 2:\n  \"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro.\n",
    ));
    assert!(ha_canale(&s, "terra", Canale::Traslazione));
    assert!(!ha_canale(&s, "sole", Canale::Traslazione));
}

#[test]
fn orbita_il_centro_e_la_posizione_corrente_in_t0() {
    // "sole" si sposta a (600,250) prima; l'orbita (dopo) usa il centro (600,250).
    let s = scena(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 1:\n  \"sole\" si sposta a 600, 250.\nDal secondo 1 al secondo 3:\n  \"terra\" orbita attorno a \"sole\" con raggio 100 per 0.5 giri in senso orario con movimento lineare.\n"
    )));
    let t = trasl(&s, "terra");
    // terra (700,250) è già a distanza 100 dal nuovo centro: nessun salto, θ0 = 0; 0.5 giri
    // orari arrivano a (500,250): traslazione (-200, 0).
    assert!(t.iter().all(|k| k.3 != Some(Curva::Gradino)));
    let u = t.last().unwrap();
    assert_vicino(u.1, -200.0, "dx");
    assert_vicino(u.2, 0.0, "dy");
}

#[test]
fn orbita_errore_attorno_a_se_stesso() {
    let e = errore(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 1:\n  \"terra\" orbita attorno a \"terra\" con raggio 10 per 1 giro.\n"
    )));
    assert_eq!(e.riga, Some(5));
    assert!(e.messaggio.contains("\"terra\" non può orbitare attorno a sé stesso"), "{e}");
}

#[test]
fn orbita_errore_raggio_non_positivo() {
    for r in ["0", "-5"] {
        let e = errore(&src(&format!(
            "{SOLE_TERRA}Dal secondo 0 al secondo 1:\n  \"terra\" orbita attorno a \"sole\" con raggio {r} per 1 giro.\n"
        )));
        assert_eq!(e.riga, Some(5), "raggio {r}");
        assert!(
            e.messaggio.contains("il raggio dell'orbita deve essere maggiore di 0") || e.messaggio.contains("raggio"),
            "raggio {r}: {e}"
        );
    }
}

#[test]
fn orbita_errore_raggio_troppo_grande() {
    let e = errore(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 1:\n  \"terra\" orbita attorno a \"sole\" con raggio 2000000 per 1 giro.\n"
    )));
    assert!(e.messaggio.contains("troppo grande"), "{e}");
}

#[test]
fn orbita_errore_giri_non_positivi() {
    let e = errore(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 1:\n  \"terra\" orbita attorno a \"sole\" con raggio 10 per 0 giri.\n"
    )));
    assert!(e.messaggio.contains("il numero di giri deve essere maggiore di 0"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("in senso orario"), "{e}");
}

#[test]
fn orbita_errore_centro_inesistente_con_suggerimento() {
    let e = errore(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 1:\n  \"terra\" orbita attorno a \"soel\" con raggio 10 per 1 giro.\n"
    )));
    assert_eq!(e.riga, Some(5));
    assert!(e.messaggio.contains("non esiste nessun attore chiamato \"soel\""), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("sole"), "{e}");
}

#[test]
fn orbita_errore_gruppi_diversi() {
    let e = errore(&src(
        "Crea un cerchio chiamato \"a\".\nCrea un cerchio chiamato \"b\".\nCrea un cerchio chiamato \"c\".\n\
         Crea un gruppo chiamato \"g\" con \"a\" e \"b\".\n\
         Dal secondo 0 al secondo 1:\n  \"a\" orbita attorno a \"c\" con raggio 10 per 1 giro.\n",
    ));
    assert_eq!(e.riga, Some(7));
    assert!(e.messaggio.contains("\"a\" e \"c\" appartengono a gruppi diversi: l'orbita richiede lo stesso gruppo"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("stesso gruppo"), "{e}");
}

#[test]
fn orbita_errore_troppi_keyframe() {
    // 45 campioni per giro × 300 giri = 13500 > MAX_KEYFRAME (10000).
    let e = errore(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 4:\n  \"terra\" orbita attorno a \"sole\" con raggio 200 per 300 giri.\n"
    )));
    assert!(e.messaggio.contains("orbita troppo lunga"), "{e}");
    assert!(e.messaggio.contains("massimo 10000"), "{e}");
    assert!(e.suggerimento.as_deref().unwrap_or_default().contains("riduci i giri"), "{e}");
}

#[test]
fn orbita_appena_sotto_il_limite_di_keyframe_e_ammessa() {
    // 45 × 200 = 9000 campioni.
    let s = orbita("\"terra\" orbita attorno a \"sole\" con raggio 200 per 200 giri con movimento lineare.");
    assert_eq!(trasl(&s, "terra").len(), 9001);
}

#[test]
fn orbita_sovrapposta_a_un_altro_movimento_e_errore() {
    let e = errore(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 4:\n  \"terra\" orbita attorno a \"sole\" con raggio 200 per 1 giro.\n  \"terra\" scivola verso alto di 10.\n"
    )));
    assert!(e.messaggio.contains("la posizione"), "{e}");
}

#[test]
fn orbita_parser_forme_valide() {
    let p = analizza(&src(&format!(
        "{SOLE_TERRA}Dal secondo 0 al secondo 4:\n  \"terra\" orbita attorno a \"sole\" con raggio 200 per 2 giri con movimento lineare.\n\
         Poi per 1 secondo:\n  \"terra\" orbita attorno a \"sole\" con raggio 40 per 0.5 giri in senso orario.\n"
    )))
    .unwrap();
    assert_eq!(
        p.timeline.blocchi[0].azioni[0].verbo,
        Verbo::Orbita { centro: "sole".into(), raggio: 200.0, giri: 2.0, orario: false }
    );
    assert_eq!(
        p.timeline.blocchi[1].azioni[0].verbo,
        Verbo::Orbita { centro: "sole".into(), raggio: 40.0, giri: 0.5, orario: true }
    );
}

// ============================================================ cambia colore

#[test]
fn colore_forme_usano_background_color() {
    let h = html(&src(
        "Crea un cerchio chiamato \"c\" con colore rosso al centro.\nCrea un quadrato chiamato \"q\" con colore blu a 100, 100.\n\
         Dal secondo 0 al secondo 2:\n  \"c\" cambia colore in verde.\n  \"q\" cambia colore in #FDE68A.\n",
    ));
    assert!(h.contains("@keyframes a0-colore{"), "{h}");
    let kf = &h[h.find("@keyframes a0-colore").unwrap()..];
    let kf = &kf[..kf.find("}}").unwrap()];
    assert!(kf.contains("background-color:"), "{kf}");
    assert!(!kf.contains("{color:") && !kf.contains(";color:"), "{kf}");
    assert!(h.contains("#a0>.forma{"), "{h}");
    assert!(h.contains("animation:a0-colore 2s linear both"), "{h}");
    assert!(h.contains("background-color:#fde68a"), "il colore è riemesso dai valori numerici");
}

#[test]
fn colore_testi_usano_color() {
    let h = html(&src(
        "Crea un titolo chiamato \"t\" con testo \"CIAO\" e colore bianco in alto.\n\
         Crea un testo chiamato \"u\" con testo \"x\" e colore bianco in basso.\n\
         Dal secondo 0 al secondo 1:\n  \"t\" cambia colore in rosso.\n  \"u\" cambia colore in #00FF00.\n",
    ));
    let kf = &h[h.find("@keyframes a0-colore").unwrap()..];
    let kf = &kf[..kf.find("}}").unwrap()];
    assert!(kf.contains("{color:") || kf.contains("%{color:") || kf.contains(";color:"), "{kf}");
    assert!(!kf.contains("background-color"), "{kf}");
    let kf = &h[h.find("@keyframes a1-colore").unwrap()..];
    assert!(kf.contains("color:#00ff00"), "{kf}");
}

#[test]
fn colore_non_compare_in_will_change_e_non_tocca_il_wrapper() {
    let h = html(&src(
        "Crea un cerchio chiamato \"c\" con colore rosso al centro.\nDal secondo 0 al secondo 2:\n  \"c\" cambia colore in blu.\n",
    ));
    let r = regola(&h, "#a0");
    assert!(!r.contains("will-change"), "solo canale colore: niente will-change ({r})");
    assert!(!r.contains("animation"), "{r}");
    assert!(!h.contains("will-change:background-color") && !h.contains("will-change:color"));
    // Con altri canali, will-change resta limitato ai compositi.
    let h = html(&src(
        "Crea un cerchio chiamato \"c\" con colore rosso al centro.\nDal secondo 0 al secondo 2:\n  \"c\" cambia colore in blu e appare sfumando.\n",
    ));
    let r = regola(&h, "#a0");
    assert!(r.contains("will-change:opacity") && !r.contains("color"), "{r}");
}

#[test]
fn colore_valore_iniziale_e_il_colore_dichiarato() {
    let s = scena(&src(
        "Crea un cerchio chiamato \"c\" con colore #38BDF8 al centro.\nCrea un cerchio chiamato \"d\" a 100, 100.\n\
         Dal secondo 0 al secondo 2:\n  \"c\" cambia colore in #FDE68A.\n  \"d\" cambia colore in rosso.\n",
    ));
    let primo = |n: &str| {
        att(&s, n).animazioni.iter().find(|a| a.canale == Canale::Colore).unwrap().keyframe[0].valore
    };
    let Valore::Colore(c) = primo("c") else { panic!("non è un colore") };
    assert_eq!((c.r, c.g, c.b), (0x38, 0xBD, 0xF8));
    // Colore non dichiarato: bianco predefinito.
    let Valore::Colore(d) = primo("d") else { panic!("non è un colore") };
    assert_eq!((d.r, d.g, d.b, d.a), (255, 255, 255, 1.0));
    // L'ultimo keyframe è il colore di arrivo.
    let an = att(&s, "c").animazioni.iter().find(|a| a.canale == Canale::Colore).unwrap();
    let Valore::Colore(fin) = an.keyframe.last().unwrap().valore else { panic!() };
    assert_eq!((fin.r, fin.g, fin.b), (0xFD, 0xE6, 0x8A));
}

#[test]
fn colore_forme_accettate() {
    for forma in ["rosso", "#F00", "#FF0000", "#FF000080", "rgb(255, 0, 0)", "trasparente"] {
        let s = scena(&src(&format!(
            "Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia colore in {forma}.\n"
        )));
        assert!(ha_canale(&s, "c", Canale::Colore), "{forma}");
    }
    // Con trasparenza il colore è riemesso come rgba().
    let h = html(&src(
        "Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia colore in trasparente.\n",
    ));
    assert!(h.contains("rgba("), "{h}");
}

#[test]
fn colore_sovrapposizioni() {
    let e = errore(&src(
        "Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 2:\n  \"c\" cambia colore in rosso.\n\
         Dal secondo 1 al secondo 3:\n  \"c\" cambia colore in blu.\n",
    ));
    assert_eq!(e.riga, Some(6));
    assert!(e.messaggio.contains("il colore"), "{e}");
    assert!(e.messaggio.contains("righe 4 e 6"), "{e}");
    // Contigui: ammessi.
    let s = scena(&src(
        "Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia colore in rosso.\n\
         Poi per 1 secondo:\n  \"c\" cambia colore in blu.\n",
    ));
    assert!(ha_canale(&s, "c", Canale::Colore));
    // Indipendente dagli altri canali, anche nello stesso blocco e sulla stessa riga.
    let s = scena(&src(
        "Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 2:\n  \"c\" cambia colore in rosso e scivola verso basso di 20 e pulsa.\n",
    ));
    for canale in [Canale::Colore, Canale::Traslazione, Canale::Scala] {
        assert!(ha_canale(&s, "c", canale), "{canale:?}");
    }
}

#[test]
fn colore_errori_di_sintassi() {
    let e = errore(&src("Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia colore rosso.\n"));
    assert_eq!(e.riga, Some(4));
    assert!(e.messaggio.contains("mi aspettavo «in» dopo «cambia colore»"), "{e}");
    let e = errore(&src("Crea un cerchio chiamato \"c\".\nDal secondo 0 al secondo 1:\n  \"c\" cambia colore in #12.\n"));
    assert_eq!(e.riga, Some(4));
    assert!(e.messaggio.contains("colore"), "{e}");
}

#[test]
fn colore_il_canale_precede_la_regola_forma_solo_con_animazione() {
    let h = html(&src("Crea un cerchio chiamato \"c\" con colore rosso.\nDal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n"));
    assert!(!h.contains("-colore"), "senza `cambia colore` non c'è canale colore");
}

// ================================================================= B.8

const ESEMPIO_B8: &str = "# Esempio completo v0.2: stato iniziale, gruppi, ingressi/uscite, orbita, colore\n\
Prepara un palco grande 1280 per 720 con sfondo #0F172A.\n\
\n\
Crea un cerchio chiamato \"sole\" con raggio 60 e colore #F59E0B al centro.\n\
Crea un cerchio chiamato \"terra\" con raggio 18 e colore #38BDF8 a destra di \"sole\" di 200.\n\
Crea un gruppo chiamato \"sistema\" con \"sole\" e \"terra\", scala 0.8 e livello 1.\n\
Crea un titolo chiamato \"titolo\" con testo \"SISTEMA ORBITALE\", grandezza 36 e colore bianco in alto.\n\
Crea un quadrato chiamato \"stella\" con lato 24, colore bianco, scala 0 e rotazione 45 in basso a destra.\n\
\n\
Dal secondo 0 al secondo 1.5:\n\
\x20\x20\"sistema\" entra da sinistra con movimento scatto.\n\
\x20\x20\"titolo\" entra dall'alto con movimento morbido.\n\
\n\
Dal secondo 1.5 al secondo 7.5:\n\
\x20\x20\"terra\" orbita attorno a \"sole\" con raggio 200 per 2 giri in senso orario con movimento lineare.\n\
\x20\x20\"sole\" pulsa leggermente a ritmo lentamente e cambia colore in #FDE68A.\n\
\x20\x20\"stella\" si ingrandisce fino a 1.5 e ruota di 180 gradi con movimento elastico.\n\
\n\
Poi per 1.5 secondi:\n\
\x20\x20\"sistema\" esce verso destra con movimento morbido.\n\
\x20\x20\"titolo\" esce verso l'alto.\n\
\x20\x20\"stella\" scompare sfumando.\n";

#[test]
fn b8_compila_senza_avvisi() {
    let s = scena(ESEMPIO_B8);
    assert!(s.avvisi.is_empty(), "{:?}", s.avvisi);
    assert_vicino(s.durata, 9.0, "durata");
    let h = html(ESEMPIO_B8);
    assert!(div_bilanciati(&h));
}

#[test]
fn b8_pivot_del_sistema() {
    let s = scena(ESEMPIO_B8);
    let g = att(&s, "sistema");
    // AABB [580, 858] x [300, 420] -> P = (719, 360).
    assert_eq!(g.genitore, None);
    assert_vicino(g.x, 719.0, "P.x");
    assert_vicino(g.y, 360.0, "P.y");
    assert_vicino(g.scala_iniziale, 0.8, "scala del gruppo fin da t = 0");
    assert_eq!(g.stile.livello, 1);
    let (sole, terra) = (att(&s, "sole"), att(&s, "terra"));
    assert_eq!((sole.genitore, terra.genitore), (Some(0), Some(0)));
    assert_vicino(sole.x, 640.0 - 719.0, "sole.x");
    assert_vicino(sole.y, 0.0, "sole.y");
    assert_vicino(terra.x, 840.0 - 719.0, "terra.x");
    assert_vicino(terra.y, 0.0, "terra.y");
}

#[test]
fn b8_il_sistema_parte_fuori_campo_a_sinistra() {
    let s = scena(ESEMPIO_B8);
    // Estensione destra del gruppo: (858 - 719) * 0.8 = 111.2; il pivot parte da x = -113.2.
    let t = trasl(&s, "sistema");
    assert_vicino(t[0].1, -113.2 - 719.0, "traslazione iniziale del gruppo");
    assert_vicino(t[0].2, 0.0, "dy iniziale");
}

#[test]
fn b8_titolo_parte_con_il_centro_a_y_meno_29() {
    let s = scena(ESEMPIO_B8);
    let t = att(&s, "titolo");
    // "in alto" = (640, 72); stima: h = 54, ey = 27 -> yF = -27 - 2 = -29.
    assert_vicino(t.x, 640.0, "titolo.x");
    assert_vicino(t.y, 72.0, "titolo.y");
    let tr = trasl(&s, "titolo");
    assert_vicino(tr[0].1, 0.0, "dx");
    assert_vicino(tr[0].2, -29.0 - 72.0, "il centro parte a y = -29");
}

#[test]
fn b8_terra_e_sole_niente_salto() {
    let s = scena(ESEMPIO_B8);
    let t = trasl(&s, "terra");
    assert!(t.iter().all(|k| k.3 != Some(Curva::Gradino)), "distanza = raggio: nessun salto");
    // 2 giri orari: la terra torna al punto di partenza.
    let u = t.last().unwrap();
    assert_vicino(u.1, 0.0, "arrivo dx");
    assert_vicino(u.2, 0.0, "arrivo dy");
    // L'orbita non è calcolata attorno al sole sul pianeta: tutti i campioni stanno sul cerchio.
    for (_, dx, dy, _) in &t {
        let d = ((200.0 + dx).powi(2) + dy.powi(2)).sqrt();
        assert!((d - 200.0).abs() < 0.01, "campione a distanza {d}");
    }
    assert!(!ha_canale(&s, "sole", Canale::Traslazione));
    // La stella non è nel gruppo.
    assert_eq!(att(&s, "stella").genitore, None);
}

#[test]
fn b8_il_sistema_esce_oltre_il_bordo_destro() {
    let s = scena(ESEMPIO_B8);
    let t = trasl(&s, "sistema");
    let u = t.last().unwrap();
    // xF = L + sinistra + M, con sinistra = estensione sinistra scalata di 0.8 (include
    // l'intero cerchio dell'orbita, B.0.4) e M = 2. Il pivot finisce a destra di 1280 + 2.
    let pivot_finale = 719.0 + u.1;
    assert!(pivot_finale > 1280.0 + 2.0, "pivot finale {pivot_finale}");
    // Il sole (cerchio r60 con scala fino a 1.05): il bordo sinistro dell'intero gruppo, dopo
    // la traslazione, è oltre il bordo destro del palco. Estensione sinistra del gruppo:
    // terra a 840 con orbita fino a x = 440 (campione più vicino) e raggio 18.
    let sinistra = (719.0 - (840.0 - 399.5128 - 18.0)) * 0.8;
    assert_vicino(pivot_finale, 1280.0 + sinistra + 2.0, "pivot finale");
    // Il titolo esce verso l'alto: arriva a y = -29.
    assert_vicino(trasl(&s, "titolo").last().unwrap().2, -29.0 - 72.0, "titolo fuori");
}

#[test]
fn b8_stella_colore_e_stato_iniziale() {
    let s = scena(ESEMPIO_B8);
    let st = att(&s, "stella");
    assert_eq!((st.scala_iniziale, st.rotazione_iniziale), (0.0, 45.0));
    let rot = st.animazioni.iter().find(|a| a.canale == Canale::Rotazione).unwrap();
    assert_eq!(rot.keyframe.last().unwrap().valore, Valore::Rotazione(225.0));
    let sole = att(&s, "sole");
    let col = sole.animazioni.iter().find(|a| a.canale == Canale::Colore).unwrap();
    let Valore::Colore(fin) = col.keyframe.last().unwrap().valore else { panic!() };
    assert_eq!((fin.r, fin.g, fin.b), (0xFD, 0xE6, 0x8A));
    let h = html(ESEMPIO_B8);
    assert!(h.contains("#a1>.forma{"), "il sole è a1 (pre-ordine)");
    assert!(h.contains("@keyframes a1-colore"));
}

// ========================================== regressione: output della v0.1

/// FNV-1a a 64 bit, per confrontare l'HTML senza conservarlo.
fn fnv(testo: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in testo.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn esempio(nome: &str) -> String {
    let percorso = format!("{}/examples/{nome}.joy", env!("CARGO_MANIFEST_DIR"));
    let sorgente = std::fs::read_to_string(&percorso).unwrap_or_else(|e| panic!("{percorso}: {e}"));
    html(&sorgente)
}

#[test]
fn regressione_esempi_v01_senza_funzioni_v02_blocco_2b() {
    for nome in ["primo_test", "livello1_motion", "stato_iniziale"] {
        let h = esempio(nome);
        assert!(!h.contains("class=\"attore gruppo\""), "{nome}: gruppo inatteso");
        assert!(!h.contains("-colore"), "{nome}: canale colore inatteso");
        assert!(!h.contains("steps("), "{nome}: salto inatteso");
        assert!(!h.contains(">.forma{animation"), "{nome}: animazione sulla forma inattesa");
    }
}

/// (nome esempio, lunghezza in byte, hash FNV-1a) della parte dell'HTML compatto che precede
/// `<script>` (head, CSS, `#palco` e attori), prodotta con `--senza-player` dal compilatore
/// del commit d67175c (prima del blocco 2C) e identica a quella della v0.1 / blocco 2B.
///
/// Dal blocco 2C l'HTML completo non è più identico a quello della v0.1: lo script contiene
/// sempre `__joy_seek` (B.9.2) e, di default, il documento include il mini-player (B.5).
/// La regressione si restringe quindi alla parte che 2C non deve toccare. I riferimenti
/// sono stati ricavati compilando gli esempi con il binario di d67175c in una copia
/// temporanea fuori dal repository.
const ATTESI: [(&str, usize, u64); 3] = [
    ("primo_test", 4516, 18_401_684_762_150_092_797),
    ("livello1_motion", 19291, 4_295_816_693_622_175_511),
    ("stato_iniziale", 2637, 9_609_074_493_580_632_402),
];

#[test]
fn regressione_html_degli_esempi_v01_identico_byte_per_byte() {
    for (nome, lunghezza, hash) in ATTESI {
        let percorso = format!("{}/examples/{nome}.joy", env!("CARGO_MANIFEST_DIR"));
        let sorgente = std::fs::read_to_string(&percorso).unwrap();
        let opzioni = Opzioni { senza_player: true, ..Default::default() };
        let h = compila_sorgente(&sorgente, &opzioni).unwrap();
        let prima = h.split("<script>").next().unwrap();
        assert_eq!((prima.len(), fnv(prima)), (lunghezza, hash), "{nome}: head/CSS/palco cambiati");
        assert!(h.ends_with("</script></body></html>"), "{nome}: chiusura inattesa");
        // Senza player nessuna traccia del player nel documento.
        assert!(!h.contains("joy-player"), "{nome}");
    }
}

#[test]
fn regressione_senza_gruppi_attori_a_primo_livello() {
    for nome in ["primo_test", "livello1_motion", "stato_iniziale"] {
        let percorso = format!("{}/examples/{nome}.joy", env!("CARGO_MANIFEST_DIR"));
        let sorgente = std::fs::read_to_string(percorso).unwrap();
        let s = scena(&sorgente);
        assert!(s.attori.iter().all(|a| a.genitore.is_none()), "{nome}");
        assert!(s.attori.iter().enumerate().all(|(i, a)| a.id == format!("a{i}")), "{nome}");
        assert!(s.attori.iter().all(|a| !ha_canale_attore(a, Canale::Colore)), "{nome}");
    }
}

fn ha_canale_attore(a: &AttoreCompilato, canale: Canale) -> bool {
    a.animazioni.iter().any(|x| x.canale == canale)
}

// ============================================ esempio sistema_orbitale.joy

#[test]
fn esempio_sistema_orbitale_compila_senza_avvisi() {
    let percorso = format!("{}/examples/sistema_orbitale.joy", env!("CARGO_MANIFEST_DIR"));
    let sorgente = std::fs::read_to_string(percorso).unwrap();
    let s = scena(&sorgente);
    assert!(s.avvisi.is_empty(), "{:?}", s.avvisi);
    let g = att(&s, "sistema");
    assert!(matches!(g.forma, Forma::Gruppo { .. }));
    let (is, ip) = (indice(&s, "sistema"), indice(&s, "pianeta"));
    assert_eq!(att(&s, "sole").genitore, Some(is));
    assert_eq!(att(&s, "pianeta").genitore, Some(is));
    assert_eq!(att(&s, "terra").genitore, Some(ip));
    assert_eq!(att(&s, "luna").genitore, Some(ip));
    // Pivot di "pianeta": AABB x [822, 886] -> 854; distanza dal sole 214 = raggio: nessun salto.
    assert!(ha_canale(&s, "pianeta", Canale::Traslazione));
    assert!(ha_canale(&s, "luna", Canale::Traslazione));
    for n in ["pianeta", "luna", "sistema"] {
        assert!(trasl(&s, n).iter().all(|k| k.3 != Some(Curva::Gradino)), "{n}: salto inatteso");
    }
    assert!(ha_canale(&s, "titolo", Canale::Colore));
    assert!(ha_canale(&s, "sole", Canale::Colore));
    let h = html(&sorgente);
    assert!(div_bilanciati(&h));
    assert!(!h.contains("steps("), "nessun salto istantaneo");
}
