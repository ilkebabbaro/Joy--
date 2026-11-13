//! Test del compilatore: posizioni, tempi, canali e keyframe.

use joypp::compiler::easing::Curva;
use joypp::compiler::{Canale, ScenaCompilata, Valore, compila};
use joypp::parser::analizza;

fn scena(src: &str) -> ScenaCompilata {
    compila(&analizza(src).unwrap()).unwrap()
}

fn errore(src: &str) -> joypp::Errore {
    compila(&analizza(src).unwrap()).expect_err("doveva fallire")
}

const PALCO: &str = "Prepara un palco grande 1000 per 500.\n";

#[test]
fn posizioni_ancorate_e_relative_in_catena() {
    let s = scena(&format!(
        "{PALCO}\
         Crea un cerchio chiamato \"c\" a destra di \"b\" di 10.\n\
         Crea un cerchio chiamato \"b\" sotto a \"a\" di 20.\n\
         Crea un cerchio chiamato \"a\" in alto a sinistra.\n\
         Crea un cerchio chiamato \"d\" al centro.\n"
    ));
    let p: Vec<(f64, f64)> = s.attori.iter().map(|a| (a.x, a.y)).collect();
    assert_eq!(p[2], (100.0, 50.0));
    assert_eq!(p[1], (100.0, 70.0));
    assert_eq!(p[0], (110.0, 70.0));
    assert_eq!(p[3], (500.0, 250.0));
}

#[test]
fn posizioni_circolari() {
    let e = errore(&format!(
        "{PALCO}\
         Crea un cerchio chiamato \"a\" a destra di \"b\" di 10.\n\
         Crea un cerchio chiamato \"b\" a destra di \"a\" di 10.\n"
    ));
    assert!(e.messaggio.contains("circolari"), "{e}");
}

#[test]
fn riferimento_sconosciuto_con_suggerimento() {
    let e = errore(&format!(
        "{PALCO}Crea un cerchio chiamato \"sole\".\nCrea un cerchio chiamato \"b\" a destra di \"sloe\" di 10.\n"
    ));
    assert_eq!(e.riga, Some(3));
    assert_eq!(e.suggerimento.as_deref(), Some("forse intendevi \"sole\"?"));
}

#[test]
fn nomi_duplicati() {
    let e = errore(&format!("{PALCO}Crea un cerchio chiamato \"a\".\nCrea un quadrato chiamato \"a\".\n"));
    assert_eq!(e.riga, Some(3));
}

#[test]
fn attore_sconosciuto_in_azione() {
    let e = errore(&format!(
        "{PALCO}Crea un cerchio chiamato \"sole\".\nDal secondo 0 al secondo 1:\n  \"sol\" appare sfumando.\n"
    ));
    assert_eq!(e.riga, Some(4));
    assert_eq!(e.suggerimento.as_deref(), Some("forse intendevi \"sole\"?"));
}

#[test]
fn blocchi_relativi_si_agganciano_al_precedente() {
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\n\
         Dal secondo 1 al secondo 2:\n  \"a\" ruota di 90 gradi.\n\
         Poi per 2 secondi:\n  \"a\" ruota di 90 gradi.\n\
         Dopo 1 secondi per 1 secondi:\n  \"a\" ruota di 90 gradi.\n"
    ));
    // 1-2, 2-4, 5-6 → durata 6
    assert_eq!(s.durata, 6.0);
    let anim = &s.attori[0].animazioni[0];
    assert_eq!(anim.canale, Canale::Rotazione);
    let finale = anim.keyframe.last().unwrap();
    assert_eq!(finale.offset, 1.0);
    assert_eq!(finale.valore, Valore::Rotazione(270.0));
}

#[test]
fn appare_parte_invisibile_e_raggiunge_opacita_dichiarata() {
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\" con opacità 0.6.\nDal secondo 0 al secondo 2:\n  \"a\" appare sfumando.\n"
    ));
    let a = &s.attori[0];
    assert_eq!(a.opacita_iniziale, 0.0);
    let k = &a.animazioni[0].keyframe;
    assert_eq!(k.first().unwrap().valore, Valore::Opacita(0.0));
    assert_eq!(k.last().unwrap().valore, Valore::Opacita(0.6));
}

#[test]
fn si_sposta_a_e_relativo_alla_posizione_iniziale() {
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\" a 100, 100.\n\
         Dal secondo 0 al secondo 1:\n  \"a\" si sposta a 300, 50 con movimento lineare.\n\
         Poi per 1 secondi:\n  \"a\" scivola verso destra di 10.\n"
    ));
    let k = &s.attori[0].animazioni[0].keyframe;
    assert_eq!(k[0].valore, Valore::Traslazione(0.0, 0.0));
    assert_eq!(k[0].curva, Some(Curva::Lineare));
    assert_eq!(k[1].offset, 0.5);
    assert_eq!(k[1].valore, Valore::Traslazione(200.0, -50.0));
    assert_eq!(k.last().unwrap().valore, Valore::Traslazione(210.0, -50.0));
}

#[test]
fn canali_diversi_non_confliggono() {
    let s = scena(&format!(
        "{PALCO}Crea un quadrato chiamato \"q\".\nDal secondo 0 al secondo 1:\n  \"q\" appare sfumando e ruota di 90 gradi e si ingrandisce fino a 2 e scivola verso alto di 5.\n"
    ));
    let canali: Vec<Canale> = s.attori[0].animazioni.iter().map(|a| a.canale).collect();
    assert_eq!(canali, vec![Canale::Opacita, Canale::Traslazione, Canale::Rotazione, Canale::Scala]);
}

#[test]
fn sovrapposizione_sullo_stesso_canale() {
    let e = errore(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\n\
         Dal secondo 0 al secondo 2:\n  \"a\" si sposta a 1, 1.\n\
         Dal secondo 1 al secondo 3:\n  \"a\" scivola verso alto di 5.\n"
    ));
    assert_eq!(e.riga, Some(6));
    assert!(e.messaggio.contains("righe 4 e 6"), "{e}");
}

#[test]
fn pulsa_genera_battiti_esatti() {
    // 4 secondi a 60 BPM = 4 battiti = 8 mezzi periodi.
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 4:\n  \"a\" pulsa intensamente a ritmo 60.\n"
    ));
    let k = &s.attori[0].animazioni[0].keyframe;
    assert_eq!(k.len(), 9);
    assert_eq!(k[1].valore, Valore::Scala(1.25));
    assert_eq!(k[2].valore, Valore::Scala(1.0));
    assert_eq!(k[8].valore, Valore::Scala(1.0));
    assert_eq!(k[8].offset, 1.0);
}

#[test]
fn easing_fisici_campionati() {
    let s = scena(&format!(
        "{PALCO}Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" ruota di 90 gradi con movimento rimbalzo.\n"
    ));
    let curva = s.attori[0].animazioni[0].keyframe[0].curva.clone();
    assert!(matches!(curva, Some(Curva::Campionata(p)) if p.len() == 41));
}

#[test]
fn scena_senza_timeline() {
    let s = scena(&format!("{PALCO}Crea un cerchio chiamato \"a\".\n"));
    assert_eq!(s.durata, 0.0);
    assert!(s.attori[0].animazioni.is_empty());
}

#[test]
fn opacita_fuori_intervallo() {
    let e = errore(&format!("{PALCO}Crea un cerchio chiamato \"a\" con opacità 2.\n"));
    assert_eq!(e.riga, Some(2));
}
