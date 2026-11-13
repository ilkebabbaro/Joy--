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
use crate::compiler::{Animazione, AttoreCompilato, ScenaCompilata, Valore};
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
    for attore in &scena.attori {
        let nome = escape_html(&attore.nome);
        let interno = match &attore.forma {
            Forma::Testo { contenuto, .. } => {
                format!("<span class=\"forma testo\">{}</span>", escape_html(contenuto))
            }
            _ => "<div class=\"forma\"></div>".to_string(),
        };
        e.riga(
            1,
            &format!("<div class=\"attore\" id=\"{}\" data-nome=\"{nome}\">{interno}</div>", attore.id),
        );
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
    if !attore.animazioni.is_empty() && durata > 0.0 {
        let animazioni: Vec<String> = attore
            .animazioni
            .iter()
            .map(|a| format!("{id}-{} {}s linear both", a.canale.suffisso(), numero(durata)))
            .collect();
        let proprieta: Vec<&str> = attore.animazioni.iter().map(|a| a.canale.proprieta_css()).collect();
        regole.push_str(&format!(";animation:{};will-change:{}", animazioni.join(","), proprieta.join(",")));
    }
    e.riga(2, &format!("#{id}{{{regole}}}"));

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
    }
    if let Some((spessore, c)) = attore.stile.bordo {
        forma.push_str(&format!(";border:{}px solid {}", numero(spessore), colore(c)));
    }
    e.riga(2, &format!("#{id}>.forma{{{forma}}}"));

    for animazione in &attore.animazioni {
        keyframes(e, id, animazione);
    }
}

fn keyframes(e: &mut Emettitore, id: &str, animazione: &Animazione) {
    let proprieta = animazione.canale.proprieta_css();
    let mut corpo = String::new();
    for k in &animazione.keyframe {
        let valore = match k.valore {
            Valore::Opacita(v) => numero(v.clamp(0.0, 1.0)),
            Valore::Traslazione(x, y) => format!("{}px {}px", numero(x), numero(y)),
            Valore::Rotazione(g) => format!("{}deg", numero(g)),
            Valore::Scala(s) => numero(s),
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
    }
}
