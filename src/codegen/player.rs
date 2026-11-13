//! Script inline dell'HTML: adattamento del palco, `window.__joy_seek` (B.9.2) e
//! mini-player (B.5).
//!
//! Tutto il JS e il CSS qui sono **testi costanti del compilatore**: gli unici valori
//! variabili sono numeri (larghezza, altezza, durata in ms) formattati da [`numero`].
//! Nessuna stringa dell'utente entra mai nello script né nel CSS del player.
//!
//! Ogni riga di JS è un'istruzione completa (termina con `;`, `{` o `}`): in modalità
//! compatta le righe sono concatenate senza separatore, in `--pretty` sono indentate.

use crate::codegen::numero;

/// Regole CSS del mini-player (solo con il player attivo).
pub(super) const CSS_PLAYER: &[&str] = &[
    "#joy-player{position:fixed;left:50%;bottom:16px;transform:translateX(-50%);display:flex;align-items:center;gap:8px;max-width:calc(100vw - 16px);padding:6px 12px;border-radius:999px;background:rgba(20,20,20,.78);color:#fff;font:13px system-ui,-apple-system,\"Segoe UI\",Roboto,sans-serif;opacity:0;transition:opacity .2s;z-index:2147483647}",
    "#joy-player.v{opacity:1}",
    "#joy-player button{width:32px;height:32px;padding:0;border:0;border-radius:50%;background:rgba(255,255,255,.16);color:#fff;font-size:14px;line-height:1;cursor:pointer}",
    "#joy-player button:hover{background:rgba(255,255,255,.3)}",
    "#joy-player button:focus-visible,#joy-player input:focus-visible{outline:2px solid #fff;outline-offset:2px}",
    "#joy-player input{width:min(40vw,260px);margin:0;cursor:pointer}",
    "#joy-t{min-width:7.5em;text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}",
    "html[data-joy-render] #joy-player{display:none!important}",
];

/// Markup del mini-player: una barra fuori da `#palco`. `durata` in secondi (> 0).
pub(super) fn righe_markup(durata: f64) -> Vec<String> {
    let max = numero(durata * 1000.0);
    let etichetta = format!("0.0 / {:.1} s", durata);
    vec![
        "<div id=\"joy-player\" role=\"group\" aria-label=\"Controlli di riproduzione\">".to_string(),
        "<button type=\"button\" id=\"joy-pp\" aria-label=\"Pausa\">\u{275a}\u{275a}</button>".to_string(),
        "<button type=\"button\" id=\"joy-rp\" aria-label=\"Ricomincia\">\u{21ba}</button>".to_string(),
        format!(
            "<input type=\"range\" id=\"joy-sc\" min=\"0\" max=\"{max}\" step=\"10\" value=\"0\" aria-label=\"Posizione nel tempo\">"
        ),
        format!("<span id=\"joy-t\">{etichetta}</span>"),
        "</div>".to_string(),
    ]
}

/// Parte costante del JS: adattamento del palco e `__joy_seek` (B.9.2).
const JS_BASE: &[&str] = &[
    r#"var p=document.getElementById("palco"),e0=document.documentElement,q=Promise.resolve();"#,
    r#"function f(){if(e0.hasAttribute("data-joy-render"))return;var s=Math.min(1,innerWidth/W,innerHeight/H);p.style.transform="scale("+s+")"}"#,
    r#"addEventListener("resize",f);"#,
    r#"f();"#,
    r#"function passo(t){"#,
    r#"if(!e0.hasAttribute("data-joy-render")){e0.setAttribute("data-joy-render","");p.style.transform="none"}"#,
    r#"return(document.fonts?document.fonts.ready:Promise.resolve()).then(function(){"#,
    r#"if(!p.getAnimations)throw new Error("__joy_seek: Web Animations API non disponibile");"#,
    r#"var a=p.getAnimations({subtree:true});"#,
    r#"a.forEach(function(x){x.pause();x.currentTime=t});"#,
    r#"return Promise.all(a.map(function(x){return x.ready}))"#,
    r#"}).then(function(){return new Promise(function(r){"#,
    r#"requestAnimationFrame(function(){requestAnimationFrame(function(){r(t)})})})})}"#,
    r#"Object.defineProperty(window,"__joy_seek",{value:function(ms){"#,
    r#"if(typeof ms!=="number"||!isFinite(ms))return Promise.reject(new TypeError("__joy_seek: ms deve essere un numero finito"));"#,
    r#"var t=Math.min(Math.max(ms,0),D)||0,r=q.then(function(){return passo(t)});"#,
    r#"q=r.catch(function(){});return r}});"#,
];

/// Parte costante del JS del mini-player (B.5), con le guardie `data-joy-render` (B.9.2).
const JS_PLAYER: &[&str] = &[
    r#"function player(b){"#,
    r#"if(!document.getAnimations){b.parentNode.removeChild(b);return}"#,
    r#"var pp=document.getElementById("joy-pp"),rp=document.getElementById("joy-rp"),sc=document.getElementById("joy-sc"),tm=document.getElementById("joy-t"),vis=false,over=false,tmr=0,raf=0,ico="p";"#,
    r#"function R(){return e0.hasAttribute("data-joy-render")}"#,
    r#"function A(){return document.getAnimations().filter(function(a){var t=a.effect&&a.effect.target;return!!t&&p.contains(t)})}"#,
    r#"function cur(){var a=A()[0],t=a&&a.currentTime!=null?Number(a.currentTime):0;return Math.min(Math.max(t,0),D)}"#,
    r#"function run(){return A().some(function(a){return a.playState==="running"})}"#,
    r#"function upd(){var t=cur(),s=run()?"p":"r";sc.value=String(t);tm.textContent=(t/1000).toFixed(1)+" / "+(D/1000).toFixed(1)+" s";"#,
    r#"if(s!==ico){ico=s;pp.textContent=s==="p"?"❚❚":"▶";pp.setAttribute("aria-label",s==="p"?"Pausa":"Riproduci")}}"#,
    r#"function tick(){raf=0;if(R()||!vis)return;upd();raf=requestAnimationFrame(tick)}"#,
    r#"function arm(){clearTimeout(tmr);tmr=setTimeout(hide,2500)}"#,
    r#"function hide(){tmr=0;if(R())return;if(over||b.contains(document.activeElement)){arm();return}vis=false;b.classList.remove("v")}"#,
    r#"function show(){if(R())return;vis=true;b.classList.add("v");arm();if(!raf)raf=requestAnimationFrame(tick)}"#,
    r#"function replay(){if(R())return;A().forEach(function(a){a.currentTime=0;a.play()})}"#,
    r#"function play(){if(R())return;if(cur()>=D){replay();return}A().forEach(function(a){a.play()})}"#,
    r#"function pause(){if(R())return;A().forEach(function(a){a.pause()})}"#,
    r#"function toggle(){if(run())pause();else play()}"#,
    r#"function go(d){if(R())return;var r=run(),c=Math.min(Math.max(cur()+d,0),D);A().forEach(function(a){a.currentTime=c;if(!r)a.pause()})}"#,
    r#"pp.addEventListener("click",function(){toggle();upd()});"#,
    r#"rp.addEventListener("click",function(){replay();upd()});"#,
    r#"sc.addEventListener("input",function(){if(R())return;var v=Number(sc.value);A().forEach(function(a){a.pause();a.currentTime=v});upd()});"#,
    r#"document.addEventListener("pointermove",show);"#,
    r#"document.addEventListener("touchstart",show,{passive:true});"#,
    r#"b.addEventListener("pointerenter",function(){over=true;show()});"#,
    r#"b.addEventListener("pointerleave",function(){over=false;arm()});"#,
    r#"b.addEventListener("focusin",show);"#,
    r#"document.addEventListener("keydown",function(ev){"#,
    r#"if(R()||ev.ctrlKey||ev.metaKey||ev.altKey)return;"#,
    r#"var n=ev.target&&ev.target.tagName;if(n==="BUTTON"||n==="INPUT"||n==="TEXTAREA"||n==="SELECT"||(ev.target&&ev.target.isContentEditable))return;"#,
    r#"var k=ev.key;"#,
    r#"if(k===" "||k==="Spacebar"){ev.preventDefault();toggle()}else if(k==="r"||k==="R")replay();else if(k==="ArrowLeft")go(-1000);else if(k==="ArrowRight")go(1000);else return;"#,
    r#"upd();show()});"#,
    r#"upd()}"#,
    r#"var b0=document.getElementById("joy-player");"#,
    r#"if(b0)player(b0);"#,
];

/// Righe del blocco `<script>` unico: adattamento, `__joy_seek` ed eventuale player.
///
/// `larghezza`/`altezza` in px e `durata` in secondi sono numeri della scena compilata.
pub(super) fn righe_script(larghezza: f64, altezza: f64, durata: f64, con_player: bool) -> Vec<String> {
    let mut righe = vec![
        "(function(){".to_string(),
        format!(
            "var W={},H={},D={};",
            numero(larghezza),
            numero(altezza),
            numero(durata * 1000.0)
        ),
    ];
    righe.extend(JS_BASE.iter().map(|r| (*r).to_string()));
    if con_player {
        righe.extend(JS_PLAYER.iter().map(|r| (*r).to_string()));
    }
    righe.push("})();".to_string());
    righe
}
