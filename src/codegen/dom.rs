//! Backend DOM: attori come elementi HTML animati con CSS3 `@keyframes`.
//!
//! Struttura di ogni attore:
//!
//! ```html
//! <div class="attore" id="a0">          <!-- wrapper nel centro dell'attore: riceve le animazioni -->
//!   <div class="forma"></div>           <!-- forma visibile, centrata sul wrapper -->
//! </div>
//! ```
//!
//! Il wrapper ha `width:0;height:0;transform-origin:0 0`, quindi `rotate` e `scale`
//! agiscono attorno al centro dell'attore. Si animano solo proprietà composite
//! (`translate`, `rotate`, `scale`, `opacity`): niente reflow, accelerazione GPU.

use crate::ast::{Colore, Forma};
use crate::codegen::numero;
use crate::compiler::easing::Curva;
use crate::compiler::{Animazione, AttoreCompilato, Bersaglio, ScenaCompilata, Valore};
use crate::security::escape_html;

/// Emettitore di testo con modalità compatta (default) o leggibile (`--pretty`).
struct Emettitore {
    out: String,
    leggibile: bool,
}

impl Emettitore {
    fn riga(&mut self, rientro: usize, testo: &str) {
        if self.leggibile {
            for _ in 0..rientro {
                self.out.push_str("  ");
            }
            self.out.push_str(testo);
            self.out.push('\n');
        } else {
            self.out.push_str(testo);
        }
    }
}

/// Genera il documento HTML completo per una scena compilata.
///
/// `leggibile = true` produce output indentato; altrimenti l'output è compatto.
pub fn genera_html(scena: &ScenaCompilata, leggibile: bool) -> String {
    let mut e = Emettitore { out: String::with_capacity(4096), leggibile };
    let (l, h) = (numero(scena.larghezza), numero(scena.altezza));

    e.riga(0, "<!DOCTYPE html>");
    e.riga(0, "<html lang=\"it\">");
    e.riga(0, "<head>");
    e.riga(1, "<meta charset=\"utf-8\">");
    e.riga(1, "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    e.riga(1, &format!("<meta name=\"generator\" content=\"Joy++ {}\">", env!("CARGO_PKG_VERSION")));
    e.riga(1, "<title>Joy++</title>");
    e.riga(1, "<style>");
    e.riga(2, "*{box-sizing:border-box}");
    e.riga(2, "html,body{margin:0;height:100%}");
    e.riga(2, "body{display:flex;align-items:center;justify-content:center;background:#000;overflow:hidden}");
    e.riga(
        2,
        &format!(
            "#palco{{position:relative;flex:none;width:{l}px;height:{h}px;background:{};overflow:hidden;contain:strict}}",
            colore(scena.sfondo)
        ),
    );
    e.riga(2, ".attore{position:absolute;width:0;height:0;transform-origin:0 0}");
    e.riga(2, ".forma{position:absolute;left:0;top:0;transform:translate(-50%,-50%)}");
    e.riga(2, ".testo{white-space:pre;line-height:1.2}");
    for attore in &scena.attori {
        stile_attore(&mut e, attore, scena.durata);
    }
    e.riga(1, "</style>");
    e.riga(0, "</head>");
    e.riga(0, "<body>");
    let fps = scena.fps.map(|f| format!(" data-fps=\"{f}\"")).unwrap_or_default();
    e.riga(0, &format!("<div id=\"palco\"{fps}>"));
    // `attori` è in pre-ordine: una pila dei gruppi aperti ricostruisce l'annidamento (B.3.5).
    let mut aperti: Vec<usize> = Vec::new();
    for (i, attore) in scena.attori.iter().enumerate() {
        while aperti.last().copied() != attore.genitore {
            if aperti.pop().is_none() {
                break; // genitore incoerente: non si chiude oltre il palco
            }
            e.riga(1 + aperti.len(), "</div>");
        }
        let nome = escape_html(&attore.nome);
        let rientro = 1 + aperti.len();
        match &attore.forma {
            Forma::Gruppo { .. } => {
                e.riga(
                    rientro,
                    &format!("<div class=\"attore gruppo\" id=\"{}\" data-nome=\"{nome}\">", attore.id),
                );
                aperti.push(i);
            }
            forma => {
                let interno = match forma {
                    Forma::Testo { contenuto, .. } => {
                        format!("<span class=\"forma testo\">{}</span>", escape_html(contenuto))
                    }
                    _ => "<div class=\"forma\"></div>".to_string(),
                };
                e.riga(
                    rientro,
                    &format!("<div class=\"attore\" id=\"{}\" data-nome=\"{nome}\">{interno}</div>", attore.id),
                );
            }
        }
    }
    while aperti.pop().is_some() {
        e.riga(1 + aperti.len(), "</div>");
    }
    e.riga(0, "</div>");
    // Adatta il palco alla finestra senza toccare le coordinate interne della scena.
    e.riga(
        0,
        &format!(
            "<script>(function(){{var p=document.getElementById(\"palco\");function f(){{var s=Math.min(1,innerWidth/{l},innerHeight/{h});p.style.transform=\"scale(\"+s+\")\"}}addEventListener(\"resize\",f);f()}})();</script>"
        ),
    );
    e.riga(0, "</body>");
    e.riga(0, "</html>");
    e.out
}

/// Colore CSS riemesso dai valori numerici (mai dal testo utente).
fn colore(c: Colore) -> String {
    if c.a >= 1.0 {
        format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
    } else {
        format!("rgba({},{},{},{})", c.r, c.g, c.b, numero(c.a.clamp(0.0, 1.0)))
    }
}

fn stile_attore(e: &mut Emettitore, attore: &AttoreCompilato, durata: f64) {
    let id = &attore.id;
    let mut regole = format!(
        "left:{}px;top:{}px;z-index:{};opacity:{}",
        numero(attore.x),
        numero(attore.y),
        attore.stile.livello,
        numero(attore.opacita_iniziale)
    );
    // Stato iniziale statico (B.1): solo se diverso dal default, per non alterare l'output v0.1.
    if attore.scala_iniziale != 1.0 {
        regole.push_str(&format!(";scale:{}", numero(attore.scala_iniziale)));
    }
    if attore.rotazione_iniziale != 0.0 {
        regole.push_str(&format!(";rotate:{}deg", numero(attore.rotazione_iniziale)));
    }
    // Le animazioni si dividono per bersaglio: i canali compositi vanno sul wrapper e in
    // `will-change`; il canale `colore` va sulla `.forma` e mai in `will-change` (B.4.2).
    let anima = |bersaglio: Bersaglio| -> Vec<&Animazione> {
        if durata > 0.0 {
            attore.animazioni.iter().filter(|a| a.canale.bersaglio() == bersaglio).collect()
        } else {
            Vec::new()
        }
    };
    let su_wrapper = anima(Bersaglio::Wrapper);
    let su_forma = anima(Bersaglio::Forma);
    if !su_wrapper.is_empty() {
        let animazioni: Vec<String> = su_wrapper
            .iter()
            .map(|a| format!("{id}-{} {}s linear both", a.canale.suffisso(), numero(durata)))
            .collect();
        let proprieta: Vec<&str> =
            su_wrapper.iter().map(|a| a.canale.proprieta_css(&attore.forma)).collect();
        regole.push_str(&format!(";animation:{};will-change:{}", animazioni.join(","), proprieta.join(",")));
    }
    e.riga(2, &format!("#{id}{{{regole}}}"));

    // Un gruppo (B.3.5) non ha `.forma`: nessuna regola né animazione di forma.
    if matches!(attore.forma, Forma::Gruppo { .. }) {
        for animazione in &attore.animazioni {
            keyframes(e, id, &attore.forma, animazione);
        }
        return;
    }

    let mut forma = String::new();
    match &attore.forma {
        Forma::Cerchio { raggio } => {
            let d = numero(raggio * 2.0);
            forma.push_str(&format!("width:{d}px;height:{d}px;border-radius:50%;"));
            forma.push_str(&format!("background:{}", colore(attore.stile.colore)));
        }
        Forma::Rettangolo { larghezza, altezza, arrotondamento } => {
            forma.push_str(&format!("width:{}px;height:{}px;", numero(*larghezza), numero(*altezza)));
            if *arrotondamento > 0.0 {
                forma.push_str(&format!("border-radius:{}px;", numero(*arrotondamento)));
            }
            forma.push_str(&format!("background:{}", colore(attore.stile.colore)));
        }
        Forma::Testo { grandezza, font, grassetto, .. } => {
            forma.push_str(&format!("font-size:{}px;", numero(*grandezza)));
            forma.push_str(&format!("font-weight:{};", if *grassetto { 700 } else { 400 }));
            // Il nome del font è già stato validato da `security::valida_nome_font`.
            let famiglia = font
                .as_deref()
                .map(|f| format!("\"{f}\","))
                .unwrap_or_default();
            forma.push_str(&format!("font-family:{famiglia}system-ui,-apple-system,\"Segoe UI\",Roboto,sans-serif;"));
            forma.push_str(&format!("color:{}", colore(attore.stile.colore)));
        }
        // Gestito sopra (ritorno anticipato): il gruppo non ha `.forma`.
        Forma::Gruppo { .. } => {}
    }
    if let Some((spessore, c)) = attore.stile.bordo {
        forma.push_str(&format!(";border:{}px solid {}", numero(spessore), colore(c)));
    }
    if !su_forma.is_empty() {
        let animazioni: Vec<String> = su_forma
            .iter()
            .map(|a| format!("{id}-{} {}s linear both", a.canale.suffisso(), numero(durata)))
            .collect();
        forma.push_str(&format!(";animation:{}", animazioni.join(",")));
    }
    e.riga(2, &format!("#{id}>.forma{{{forma}}}"));

    for animazione in &attore.animazioni {
        keyframes(e, id, &attore.forma, animazione);
    }
}

fn keyframes(e: &mut Emettitore, id: &str, forma: &Forma, animazione: &Animazione) {
    let proprieta = animazione.canale.proprieta_css(forma);
    let mut corpo = String::new();
    for k in &animazione.keyframe {
        let valore = match k.valore {
            Valore::Opacita(v) => numero(v.clamp(0.0, 1.0)),
            Valore::Traslazione(x, y) => format!("{}px {}px", numero(x), numero(y)),
            Valore::Rotazione(g) => format!("{}deg", numero(g)),
            Valore::Scala(s) => numero(s),
            Valore::Colore(c) => colore(c),
        };
        corpo.push_str(&format!("{}%{{{proprieta}:{valore}", numero(k.offset * 100.0)));
        if let Some(c) = &k.curva {
            corpo.push_str(&format!(";animation-timing-function:{}", curva(c)));
        }
        corpo.push('}');
    }
    e.riga(2, &format!("@keyframes {id}-{}{{{corpo}}}", animazione.canale.suffisso()));
}

fn curva(c: &Curva) -> String {
    match c {
        Curva::Lineare => "linear".into(),
        Curva::CubicBezier([a, b, c, d]) => {
            format!("cubic-bezier({},{},{},{})", numero(*a), numero(*b), numero(*c), numero(*d))
        }
        Curva::Campionata(punti) => {
            let p: Vec<String> = punti.iter().map(|v| numero(*v)).collect();
            format!("linear({})", p.join(","))
        }
        Curva::Gradino => "steps(1,end)".into(),
    }
}

#[cfg(test)]
mod tests {
    use crate::codegen::genera_html;

    fn html(sorgente: &str, scala: f64, rotazione: f64) -> String {
        let mut p = crate::parser::analizza(sorgente).expect("parse");
        p.attori[0].stile.scala = scala;
        p.attori[0].stile.rotazione = rotazione;
        let s = crate::compiler::compila(&p).expect("compila");
        genera_html(&s, false)
    }

    #[test]
    fn scala_e_rotazione_iniziali_nella_regola_statica() {
        let h = html("Prepara un palco grande 800 per 600 con sfondo nero.\nCrea un quadrato chiamato \"q\" al centro.\n", 0.0, 45.0);
        assert!(h.contains("scale:0"), "{h}");
        assert!(h.contains("rotate:45deg"), "{h}");
    }

    use crate::ast::{Colore, Forma, Stile};
    use crate::compiler::easing::Curva;
    use crate::compiler::{Animazione, AttoreCompilato, Canale, Keyframe, ScenaCompilata, Valore};

    fn nodo(k: usize, nome: &str, forma: Forma, genitore: Option<usize>) -> AttoreCompilato {
        AttoreCompilato {
            id: format!("a{k}"),
            nome: nome.into(),
            forma,
            stile: Stile::default(),
            genitore,
            x: 10.0,
            y: 20.0,
            opacita_iniziale: 1.0,
            scala_iniziale: 1.0,
            rotazione_iniziale: 0.0,
            animazioni: Vec::new(),
        }
    }

    fn scena(attori: Vec<AttoreCompilato>) -> ScenaCompilata {
        ScenaCompilata {
            larghezza: 800.0,
            altezza: 600.0,
            sfondo: Colore::BIANCO,
            fps: None,
            durata: 2.0,
            attori,
            avvisi: Vec::new(),
        }
    }

    fn gruppo() -> Forma {
        Forma::Gruppo { membri: Vec::new() }
    }

    fn cerchio() -> Forma {
        Forma::Cerchio { raggio: 5.0 }
    }

    fn albero() -> ScenaCompilata {
        // g0 { c1, g2 { c3 }, c4 }, c5
        scena(vec![
            nodo(0, "esterno", gruppo(), None),
            nodo(1, "c1", cerchio(), Some(0)),
            nodo(2, "interno", gruppo(), Some(0)),
            nodo(3, "c3", cerchio(), Some(2)),
            nodo(4, "c4", cerchio(), Some(0)),
            nodo(5, "c5", cerchio(), None),
        ])
    }

    #[test]
    fn gruppi_annidati_compatto() {
        let h = genera_html(&albero(), false);
        let corpo = h.split("<div id=\"palco\">").nth(1).expect("palco");
        let corpo = corpo.split("<script>").next().expect("script");
        let f = "<div class=\"forma\"></div>";
        let atteso = format!(
            "<div class=\"attore gruppo\" id=\"a0\" data-nome=\"esterno\">\
             <div class=\"attore\" id=\"a1\" data-nome=\"c1\">{f}</div>\
             <div class=\"attore gruppo\" id=\"a2\" data-nome=\"interno\">\
             <div class=\"attore\" id=\"a3\" data-nome=\"c3\">{f}</div></div>\
             <div class=\"attore\" id=\"a4\" data-nome=\"c4\">{f}</div></div>\
             <div class=\"attore\" id=\"a5\" data-nome=\"c5\">{f}</div></div>"
        );
        // Il `</div>` finale chiude il palco.
        assert_eq!(corpo, atteso, "{h}");
        assert!(!h.contains("#a0>.forma") && !h.contains("#a2>.forma"), "{h}");
    }

    #[test]
    fn gruppi_annidati_leggibile_indenta_per_profondita() {
        let h = genera_html(&albero(), true);
        assert!(h.contains("\n  <div class=\"attore gruppo\" id=\"a0\""), "{h}");
        assert!(h.contains("\n    <div class=\"attore gruppo\" id=\"a2\""), "{h}");
        assert!(h.contains("\n      <div class=\"attore\" id=\"a3\""), "{h}");
        assert!(h.contains("\n    </div>\n    <div class=\"attore\" id=\"a4\""), "{h}");
        assert!(h.contains("\n  </div>\n  <div class=\"attore\" id=\"a5\""), "{h}");
    }

    #[test]
    fn gruppo_con_animazioni_sul_wrapper_e_nome_escapato() {
        let mut g = nodo(0, "<b>\"g\"", gruppo(), None);
        g.stile.livello = 3;
        g.scala_iniziale = 2.0;
        g.animazioni.push(Animazione {
            canale: Canale::Rotazione,
            keyframe: vec![
                Keyframe { offset: 0.0, valore: Valore::Rotazione(0.0), curva: None },
                Keyframe { offset: 1.0, valore: Valore::Rotazione(90.0), curva: None },
            ],
        });
        let h = genera_html(&scena(vec![g]), false);
        assert!(h.contains("#a0{left:10px;top:20px;z-index:3;opacity:1;scale:2;animation:a0-rotazione 2s linear both;will-change:rotate}"), "{h}");
        assert!(h.contains("@keyframes a0-rotazione{0%{rotate:0deg}100%{rotate:90deg}}"), "{h}");
        assert!(!h.contains("<b>"), "{h}");
        assert!(h.contains("data-nome=\"&lt;b&gt;"), "{h}");
    }

    fn colore_anim() -> Animazione {
        Animazione {
            canale: Canale::Colore,
            keyframe: vec![
                Keyframe { offset: 0.0, valore: Valore::Colore(Colore::BIANCO), curva: None },
                Keyframe {
                    offset: 1.0,
                    valore: Valore::Colore(Colore { r: 255, g: 0, b: 0, a: 1.0 }),
                    curva: None,
                },
            ],
        }
    }

    #[test]
    fn canale_colore_su_cerchio_e_su_testo() {
        let mut c = nodo(0, "c", cerchio(), None);
        c.animazioni.push(colore_anim());
        let mut t = nodo(
            1,
            "t",
            Forma::Testo { contenuto: "ciao".into(), grandezza: 20.0, font: None, grassetto: false },
            None,
        );
        t.animazioni.push(colore_anim());
        let h = genera_html(&scena(vec![c, t]), false);
        // Il wrapper non riceve animazione né will-change.
        assert!(h.contains("#a0{left:10px;top:20px;z-index:0;opacity:1}"), "{h}");
        assert!(h.contains("#a1{left:10px;top:20px;z-index:0;opacity:1}"), "{h}");
        assert!(!h.contains("will-change"), "{h}");
        // L'animazione sta sulla `.forma`, accanto alle proprietà statiche.
        assert!(h.contains("#a0>.forma{width:10px;height:10px;border-radius:50%;background:#ffffff;animation:a0-colore 2s linear both}"), "{h}");
        assert!(h.contains(";animation:a1-colore 2s linear both}"), "{h}");
        assert!(h.contains("@keyframes a0-colore{0%{background-color:#ffffff}100%{background-color:#ff0000}}"), "{h}");
        assert!(h.contains("@keyframes a1-colore{0%{color:#ffffff}100%{color:#ff0000}}"), "{h}");
    }

    #[test]
    fn colore_e_traslazione_insieme_will_change_solo_composito() {
        let mut c = nodo(0, "c", cerchio(), None);
        c.animazioni.push(Animazione {
            canale: Canale::Traslazione,
            keyframe: vec![Keyframe { offset: 0.0, valore: Valore::Traslazione(0.0, 0.0), curva: None }],
        });
        c.animazioni.push(colore_anim());
        let h = genera_html(&scena(vec![c]), false);
        assert!(h.contains("animation:a0-traslazione 2s linear both;will-change:translate}"), "{h}");
        assert!(h.contains("animation:a0-colore 2s linear both}"), "{h}");
    }

    #[test]
    fn keyframe_gradino_emesso_come_steps() {
        let mut c = nodo(0, "c", cerchio(), None);
        c.animazioni.push(Animazione {
            canale: Canale::Traslazione,
            keyframe: vec![
                Keyframe { offset: 0.0, valore: Valore::Traslazione(0.0, 0.0), curva: Some(Curva::Gradino) },
                Keyframe { offset: 1.0, valore: Valore::Traslazione(5.0, 0.0), curva: None },
            ],
        });
        let h = genera_html(&scena(vec![c]), false);
        assert!(h.contains("0%{translate:0px 0px;animation-timing-function:steps(1,end)}"), "{h}");
    }

    #[test]
    fn gradino_emesso_come_steps() {
        assert_eq!(super::curva(&crate::compiler::easing::Curva::Gradino), "steps(1,end)");
    }

    #[test]
    fn valori_di_default_non_emettono_nulla() {
        let h = html("Prepara un palco grande 800 per 600 con sfondo nero.\nCrea un quadrato chiamato \"q\" al centro.\n", 1.0, 0.0);
        assert!(!h.contains("scale:") && !h.contains("rotate:"), "{h}");
    }
}
