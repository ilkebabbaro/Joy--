//! Test del parser: una prova per ogni parola chiave e per i principali errori.

use joypp::ast::{Ancora, Colore, Direzione, Easing, Forma, Posizione, Tempo, Verbo};
use joypp::parser::analizza;

const PALCO: &str = "Prepara un palco grande 800 per 600 con sfondo nero.\n";

fn con_palco(resto: &str) -> String {
    format!("{PALCO}{resto}")
}

fn errore(sorgente: &str) -> joypp::Errore {
    analizza(sorgente).expect_err("doveva fallire")
}

// ------------------------------------------------------------------ palco

#[test]
fn palco_completo_con_fps() {
    let p = analizza("Prepara un palco grande 1920 per 1080 con sfondo #0B0C10 a 60 fotogrammi.").unwrap();
    assert_eq!(p.palco.larghezza, 1920.0);
    assert_eq!(p.palco.altezza, 1080.0);
    assert_eq!(p.palco.sfondo, Colore::rgb(0x0B, 0x0C, 0x10));
    assert_eq!(p.palco.fps, Some(60));
}

#[test]
fn palco_semplice_e_maiuscole_ignorate() {
    let p = analizza("PREPARA UN PALCO GRANDE 800 PER 600 CON SFONDO NERO.").unwrap();
    assert_eq!(p.palco.sfondo, Colore::rgb(0, 0, 0));
    assert_eq!(p.palco.fps, None);
}

#[test]
fn palco_mancante() {
    let e = errore("# solo un commento\n");
    assert!(e.messaggio.contains("palco"));
    assert!(e.suggerimento.is_some());
}

#[test]
fn palco_deve_essere_il_primo() {
    let e = errore("Crea un cerchio chiamato \"a\".\nPrepara un palco grande 10 per 10.");
    assert_eq!(e.riga, Some(1));
}

#[test]
fn palco_duplicato() {
    let e = errore(&con_palco("Prepara un palco grande 10 per 10."));
    assert_eq!(e.riga, Some(2));
}

// ----------------------------------------------------------------- colori

#[test]
fn colori_nominali_esadecimali_e_rgb() {
    let src = con_palco(
        "Crea un cerchio chiamato \"a\" con colore azzurro.\n\
         Crea un cerchio chiamato \"b\" con colore #F05.\n\
         Crea un cerchio chiamato \"c\" con colore rgb(1, 2, 3).\n\
         Crea un cerchio chiamato \"d\" con colore #11223380.\n\
         Crea un cerchio chiamato \"e\" con colore trasparente.\n",
    );
    let p = analizza(&src).unwrap();
    assert_eq!(p.attori[0].stile.colore, Colore::rgb(0, 191, 255));
    assert_eq!(p.attori[1].stile.colore, Colore::rgb(255, 0, 85));
    assert_eq!(p.attori[2].stile.colore, Colore::rgb(1, 2, 3));
    assert!((p.attori[3].stile.colore.a - 128.0 / 255.0).abs() < 1e-9);
    assert_eq!(p.attori[4].stile.colore.a, 0.0);
}

#[test]
fn colore_sconosciuto_con_suggerimento() {
    let e = errore(&con_palco("Crea un cerchio chiamato \"a\" con colore rosa.\n"));
    assert_eq!(e.riga, Some(2));
    let e = errore(&con_palco("Crea un cerchio chiamato \"a\" con colore biancho.\n"));
    assert_eq!(e.suggerimento.as_deref(), Some("forse intendevi «bianco»?"));
}

#[test]
fn colori_esadecimali_e_rgb_non_validi() {
    assert!(analizza(&con_palco("Crea un cerchio chiamato \"a\" con colore #12345.\n")).is_err());
    assert!(analizza(&con_palco("Crea un cerchio chiamato \"a\" con colore #GG0000.\n")).is_err());
    assert!(analizza(&con_palco("Crea un cerchio chiamato \"a\" con colore rgb(300, 0, 0).\n")).is_err());
}

// ----------------------------------------------------------------- attori

#[test]
fn cerchio_con_raggio_e_posizione_al_centro() {
    let p = analizza(&con_palco("Crea un cerchio chiamato \"sole\" con raggio 60 e colore #F59E0B al centro.\n")).unwrap();
    let a = &p.attori[0];
    assert_eq!(a.nome, "sole");
    assert_eq!(a.forma, Forma::Cerchio { raggio: 60.0 });
    assert_eq!(a.posizione, Posizione::Palco(Ancora::Centro));
    assert_eq!(a.riga, 2);
}

#[test]
fn rettangolo_e_quadrato() {
    let p = analizza(&con_palco(
        "Crea un rettangolo chiamato \"r\" con larghezza 200, altezza 50 e arrotondamento 8.\n\
         Crea un quadrato chiamato \"q\" con lato 30.\n",
    ))
    .unwrap();
    assert_eq!(p.attori[0].forma, Forma::Rettangolo { larghezza: 200.0, altezza: 50.0, arrotondamento: 8.0 });
    assert_eq!(p.attori[1].forma, Forma::Rettangolo { larghezza: 30.0, altezza: 30.0, arrotondamento: 0.0 });
}

#[test]
fn titolo_e_testo_con_predefiniti() {
    let p = analizza(&con_palco(
        "Crea un titolo chiamato \"t\" con testo \"SISTEMA ORBITALE\", grandezza 36 e colore bianco in alto.\n\
         Crea un testo chiamato \"s\" con testo “Ciao, mondo.”, font \"Open Sans\" e peso grassetto.\n",
    ))
    .unwrap();
    assert_eq!(
        p.attori[0].forma,
        Forma::Testo { contenuto: "SISTEMA ORBITALE".into(), grandezza: 36.0, font: None, grassetto: true }
    );
    assert_eq!(p.attori[0].posizione, Posizione::Palco(Ancora::Alto));
    assert_eq!(
        p.attori[1].forma,
        Forma::Testo { contenuto: "Ciao, mondo.".into(), grandezza: 24.0, font: Some("Open Sans".into()), grassetto: true }
    );
}

#[test]
fn proprieta_comuni() {
    let p = analizza(&con_palco(
        "Crea un cerchio chiamato \"a\" con opacità 0.5, livello 3 e bordo 2 rosso.\n",
    ))
    .unwrap();
    let s = &p.attori[0].stile;
    assert_eq!(s.opacita, 0.5);
    assert_eq!(s.livello, 3);
    assert_eq!(s.bordo, Some((2.0, Colore::rgb(255, 0, 0))));
}

#[test]
fn testo_senza_contenuto() {
    let e = errore(&con_palco("Crea un titolo chiamato \"t\" con grandezza 30.\n"));
    assert!(e.suggerimento.unwrap_or_default().contains("con testo"));
}

#[test]
fn proprieta_non_applicabile() {
    let e = errore(&con_palco("Crea un cerchio chiamato \"a\" con larghezza 30.\n"));
    assert!(e.messaggio.contains("non si applica"));
}

#[test]
fn tipo_sconosciuto_con_suggerimento() {
    let e = errore(&con_palco("Crea un cercho chiamato \"a\".\n"));
    assert_eq!(e.suggerimento.as_deref(), Some("forse intendevi «cerchio»?"));
}

#[test]
fn tipi_futuri_non_supportati() {
    for frase in [
        "Crea una linea chiamata \"l\".\n",
        "Crea un'immagine chiamata \"i\".\n",
        "Crea 5000 particelle chiamate \"neve\".\n",
    ] {
        let e = errore(&con_palco(frase));
        assert!(e.messaggio.contains("non è ancora supportato"), "{frase}: {e}");
    }
}

// --------------------------------------------------------------- posizioni

#[test]
fn tutte_le_posizioni() {
    let p = analizza(&con_palco(
        "Crea un cerchio chiamato \"a\" a 100, 250.\n\
         Crea un cerchio chiamato \"b\" in alto a sinistra.\n\
         Crea un cerchio chiamato \"c\" in basso a destra.\n\
         Crea un cerchio chiamato \"d\" a destra.\n\
         Crea un cerchio chiamato \"e\" a destra di \"a\" di 200.\n\
         Crea un cerchio chiamato \"f\" sopra a \"a\" di 50.\n\
         Crea un cerchio chiamato \"g\" sotto \"a\" di 10.\n\
         Crea un cerchio chiamato \"h\" a sinistra di \"a\" di 5.\n\
         Crea un cerchio chiamato \"i\" in basso.\n",
    ))
    .unwrap();
    let pos: Vec<&Posizione> = p.attori.iter().map(|a| &a.posizione).collect();
    assert_eq!(pos[0], &Posizione::Assoluta { x: 100.0, y: 250.0 });
    assert_eq!(pos[1], &Posizione::Palco(Ancora::AltoSinistra));
    assert_eq!(pos[2], &Posizione::Palco(Ancora::BassoDestra));
    assert_eq!(pos[3], &Posizione::Palco(Ancora::Destra));
    assert_eq!(pos[4], &Posizione::Relativa { direzione: Direzione::Destra, riferimento: "a".into(), distanza: 200.0 });
    assert_eq!(pos[5], &Posizione::Relativa { direzione: Direzione::Alto, riferimento: "a".into(), distanza: 50.0 });
    assert_eq!(pos[6], &Posizione::Relativa { direzione: Direzione::Basso, riferimento: "a".into(), distanza: 10.0 });
    assert_eq!(pos[7], &Posizione::Relativa { direzione: Direzione::Sinistra, riferimento: "a".into(), distanza: 5.0 });
    assert_eq!(pos[8], &Posizione::Palco(Ancora::Basso));
}

// ---------------------------------------------------------------- timeline

fn azioni(blocco: &str) -> Vec<joypp::ast::Azione> {
    let src = con_palco(&format!("Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  {blocco}\n"));
    analizza(&src).unwrap().timeline.blocchi.remove(0).azioni
}

#[test]
fn intestazioni_dei_blocchi() {
    let src = con_palco(
        "Dal secondo 0 al secondo 2:\nAl secondo 3:\nPoi per 2 secondi:\nDopo 1 secondi per 0.5 secondi:\n",
    );
    let p = analizza(&src).unwrap();
    let tempi: Vec<Tempo> = p.timeline.blocchi.iter().map(|b| b.tempo).collect();
    assert_eq!(tempi[0], Tempo::Assoluto { inizio: 0.0, fine: 2.0 });
    assert_eq!(tempi[1], Tempo::Assoluto { inizio: 3.0, fine: 3.3 });
    assert_eq!(tempi[2], Tempo::Relativo { attesa: 0.0, durata: 2.0 });
    assert_eq!(tempi[3], Tempo::Relativo { attesa: 1.0, durata: 0.5 });
}

#[test]
fn blocco_al_contrario() {
    let e = errore(&con_palco("Dal secondo 3 al secondo 1:\n"));
    assert_eq!(e.riga, Some(2));
}

#[test]
fn blocchi_futuri_non_supportati() {
    assert!(errore(&con_palco("Per tutta la durata:\n")).messaggio.contains("non è ancora supportato"));
    assert!(errore(&con_palco("In loop continuo:\n")).messaggio.contains("non è ancora supportato"));
}

#[test]
fn verbo_appare_sfumando() {
    assert_eq!(azioni("\"a\" appare sfumando.")[0].verbo, Verbo::AppareSfumando);
}

#[test]
fn verbo_scompare_sfumando() {
    assert_eq!(azioni("\"a\" scompare sfumando.")[0].verbo, Verbo::ScompareSfumando);
}

#[test]
fn verbo_cambia_opacita() {
    assert_eq!(azioni("\"a\" cambia opacità a 0.3.")[0].verbo, Verbo::CambiaOpacita(0.3));
}

#[test]
fn verbo_si_sposta_a() {
    assert_eq!(azioni("\"a\" si sposta a 640, -160.")[0].verbo, Verbo::SiSpostaA { x: 640.0, y: -160.0 });
}

#[test]
fn verbo_scivola() {
    assert_eq!(
        azioni("\"a\" scivola verso basso di 40.")[0].verbo,
        Verbo::Scivola { direzione: Direzione::Basso, distanza: 40.0 }
    );
}

#[test]
fn verbo_ruota() {
    assert_eq!(azioni("\"a\" ruota di -90 gradi.")[0].verbo, Verbo::Ruota(-90.0));
}

#[test]
fn verbi_di_scala() {
    assert_eq!(azioni("\"a\" si ingrandisce fino a 1.5.")[0].verbo, Verbo::ScalaFinoA(1.5));
    assert_eq!(azioni("\"a\" si rimpicciolisce fino a 0.")[0].verbo, Verbo::ScalaFinoA(0.0));
}

#[test]
fn verbo_pulsa_aggettivi_numeri_e_predefiniti() {
    assert_eq!(
        azioni("\"a\" pulsa leggermente a ritmo lentamente.")[0].verbo,
        Verbo::Pulsa { intensita: 0.2, bpm: 30.0 }
    );
    assert_eq!(
        azioni("\"a\" pulsa con intensità estremamente a ritmo 120.")[0].verbo,
        Verbo::Pulsa { intensita: 2.0, bpm: 120.0 }
    );
    assert_eq!(azioni("\"a\" pulsa.")[0].verbo, Verbo::Pulsa { intensita: 0.5, bpm: 60.0 });
    assert_eq!(
        azioni("\"a\" pulsa con movimento lineare.")[0],
        joypp::ast::Azione {
            attore: "a".into(),
            verbo: Verbo::Pulsa { intensita: 0.5, bpm: 60.0 },
            easing: Easing::Lineare,
            riga: 4
        }
    );
}

#[test]
fn azioni_concatenate_ereditano_easing_finale() {
    let a = azioni("\"a\" appare sfumando e scivola verso basso di 40 con movimento scatto.");
    assert_eq!(a.len(), 2);
    assert!(a.iter().all(|x| x.easing == Easing::Scatto));
    let a = azioni("\"a\" ruota di 90 gradi con movimento elastico e appare sfumando.");
    assert_eq!(a[0].easing, Easing::Elastico);
    assert_eq!(a[1].easing, Easing::Morbido);
}

#[test]
fn tutti_gli_easing() {
    for (nome, atteso) in [
        ("lineare", Easing::Lineare),
        ("morbido", Easing::Morbido),
        ("scatto", Easing::Scatto),
        ("elastico", Easing::Elastico),
        ("rimbalzo", Easing::Rimbalzo),
    ] {
        let a = azioni(&format!("\"a\" appare sfumando con movimento {nome}."));
        assert_eq!(a[0].easing, atteso);
    }
}

#[test]
fn verbo_sconosciuto_con_suggerimento() {
    let src = con_palco("Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" apare sfumando.\n");
    let e = errore(&src);
    assert_eq!(e.riga, Some(4));
    assert_eq!(e.suggerimento.as_deref(), Some("forse intendevi «appare»?"));
}

#[test]
fn verbi_futuri_non_supportati() {
    // `orbita` e `cambia colore` sono ora supportati (v0.2, B.4): restano futuri gli altri verbi.
    for verbo in ["trema", "fluttua", "gira", "lascia una scia"] {
        let src = con_palco(&format!(
            "Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" {verbo}.\n"
        ));
        let e = errore(&src);
        assert!(e.messaggio.contains("non è ancora supportato"), "{verbo}: {e}");
    }
}

// ------------------------------------------------------------ struttura file

#[test]
fn punto_finale_mancante() {
    let e = errore("Prepara un palco grande 800 per 600");
    assert_eq!(e.riga, Some(1));
    assert!(e.messaggio.contains("punto"));
}

#[test]
fn azione_fuori_blocco() {
    let e = errore(&con_palco("Crea un cerchio chiamato \"a\".\n\"a\" appare sfumando.\n"));
    assert_eq!(e.riga, Some(3));
    assert!(e.messaggio.contains("blocco"));
}

#[test]
fn riga_indentata_senza_blocco() {
    let e = errore(&con_palco("  \"a\" appare sfumando.\n"));
    assert_eq!(e.riga, Some(2));
}

#[test]
fn commenti_e_righe_vuote_ignorati() {
    let p = analizza("# titolo\n\n   \n# altro\nPrepara un palco grande 10 per 10.\n").unwrap();
    assert_eq!(p.palco.riga, 5);
}

#[test]
fn messaggio_errore_formattato_in_italiano() {
    let e = errore(&con_palco("Crea un cerchio chiamato \"a\" con colore biancho.\n"));
    assert_eq!(
        e.to_string(),
        "Errore alla riga 2: colore sconosciuto «biancho»\n  Suggerimento: forse intendevi «bianco»?"
    );
}
