//! Test del blocco v0.2 "2C" (SPECIFICA, Appendice B): mini-player (B.5), `window.__joy_seek`
//! (B.9.2) e interfaccia di `joypp render` (B.9.1, B.9.3, B.9.6, B.9.7).
//!
//! Nessun test di questo file avvia un browser: il render reale sta in `tests/render.rs`.
//! La CLI è provata solo sui percorsi che falliscono prima di avviare browser e ffmpeg.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use joypp::ast::Colore;
use joypp::exporter::cli::{Comando, interpreta, uscita_video_predefinita};
use joypp::exporter::compila_con_avvisi;
use joypp::exporter::render::ffmpeg::{argomenti_ffmpeg, filtro_video};
use joypp::exporter::render::{
    CRF_PREDEFINITO, FPS_PREDEFINITO, fps_effettivo, istante, numero_fotogrammi, pianifica, sfondo_su_nero,
    url_file,
};
use joypp::{Opzioni, compila_sorgente};

const PALCO: &str = "Prepara un palco grande 1000 per 500.\n";

/// Tre blocchi temporali consecutivi: 0-1 s, poi 1.25 s, poi 0.25 s = 2.5 s (2500 ms).
const TRE_SCENE: &str = "Prepara un palco grande 320 per 180.\nCrea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" appare sfumando.\nPoi per 1.25 secondi:\n  \"a\" entra da sinistra.\nPoi per 0.25 secondi:\n  \"a\" esce verso destra.\n";

// ------------------------------------------------------------------ helper

fn compatto() -> Opzioni {
    Opzioni::default()
}
fn leggibile() -> Opzioni {
    Opzioni { leggibile: true, ..Default::default() }
}
fn senza_player() -> Opzioni {
    Opzioni { senza_player: true, ..Default::default() }
}
fn html_con(src: &str, o: &Opzioni) -> String {
    compila_sorgente(src, o).unwrap_or_else(|e| panic!("{e}\n--\n{src}"))
}

/// Contenuto del (solo) blocco `<script>`.
fn script(h: &str) -> &str {
    assert_eq!(h.matches("<script>").count(), 1, "un solo <script>");
    assert_eq!(h.matches("</script>").count(), 1, "un solo </script>");
    h.split("<script>").nth(1).and_then(|s| s.split("</script>").next()).unwrap()
}

/// Contenuto del (solo) blocco `<style>`.
fn stile(h: &str) -> &str {
    h.split("<style>").nth(1).and_then(|s| s.split("</style>").next()).unwrap()
}

fn senza_spazi(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn radice() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Cartella temporanea di sistema, unica per test.
struct Cartella(PathBuf);
impl Cartella {
    fn nuova(nome: &str) -> Self {
        let p = std::env::temp_dir().join(format!("joypp-test-2c-{nome}-{}", std::process::id()));
        fs::create_dir_all(&p).unwrap();
        Cartella(p)
    }
    fn file(&self, nome: &str) -> PathBuf {
        self.0.join(nome)
    }
}
impl Drop for Cartella {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

/// Esegue `joypp` con le variabili d'ambiente date **solo sul processo figlio**.
fn joypp(argomenti: &[&str], ambiente: &[(&str, &Path)], cwd: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_joypp"));
    c.args(argomenti);
    for (k, v) in ambiente {
        c.env(k, v);
    }
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    c.output().unwrap()
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

// ===================================================== HTML: mini-player (B.5)

#[test]
fn player_presente_di_default_con_controlli_ed_etichette() {
    for o in [compatto(), leggibile()] {
        let h = html_con(TRE_SCENE, &o);
        assert!(h.contains("id=\"joy-player\""));
        assert!(h.contains("role=\"group\" aria-label=\"Controlli di riproduzione\""));
        for id in ["joy-pp", "joy-rp", "joy-sc", "joy-t"] {
            assert_eq!(h.matches(&format!("id=\"{id}\"")).count(), 1, "{id}");
        }
        // Ogni controllo interattivo ha un'etichetta accessibile.
        assert!(h.contains("id=\"joy-pp\" aria-label=\"Pausa\""));
        assert!(h.contains("id=\"joy-rp\" aria-label=\"Ricomincia\""));
        assert!(h.contains("aria-label=\"Posizione nel tempo\""));
        assert_eq!(h.matches("<button").count(), 2);
        assert_eq!(h.matches("type=\"button\"").count(), 2);
        // Lo scrubber copre la durata in ms (2500) con passo di 10 ms.
        assert!(h.contains("type=\"range\" id=\"joy-sc\" min=\"0\" max=\"2500\" step=\"10\" value=\"0\""));
        assert!(h.contains("0.0 / 2.5 s"));
        // Il player sta fuori da #palco: viene dopo la chiusura del palco e prima dello script.
        let (i_palco, i_player, i_script) =
            (h.find("id=\"palco\"").unwrap(), h.find("id=\"joy-player\"").unwrap(), h.find("<script>").unwrap());
        assert!(i_palco < i_player && i_player < i_script);
        // Il CSS del player è presente.
        assert!(stile(&h).contains("#joy-player{position:fixed"));
    }
}

#[test]
fn player_script_gestisce_i_tasti_previsti() {
    let h = html_con(TRE_SCENE, &compatto());
    let s = script(&h);
    for frammento in [
        "k===\" \"",             // Spazio
        "k===\"r\"",             // R
        "k===\"ArrowLeft\"",     // freccia sinistra
        "k===\"ArrowRight\"",    // freccia destra
        "go(-1000)",
        "go(1000)",
        "function replay()",
        "function toggle()",
    ] {
        assert!(s.contains(frammento), "manca {frammento}");
    }
    // Nessuna dipendenza esterna (CLAUDE.md: HTML auto-contenuto).
    assert!(!h.contains("http://") && !h.contains("https://"));
}

#[test]
fn senza_player_niente_player_ma_sempre_joy_seek() {
    for (src, nome) in [(TRE_SCENE, "tre scene"), (PALCO, "senza timeline")] {
        for pretty in [false, true] {
            let o = Opzioni { leggibile: pretty, senza_player: true };
            let h = html_con(src, &o);
            assert!(!h.contains("joy-player"), "{nome}");
            assert!(!h.contains("joy-pp") && !h.contains("joy-sc") && !h.contains("joy-rp"), "{nome}");
            assert!(!h.contains("function player("), "{nome}");
            assert!(!h.contains("Controlli di riproduzione"), "{nome}");
            let s = script(&h);
            assert!(s.contains("Object.defineProperty(window,\"__joy_seek\""), "{nome}");
            // Con il player attivo, invece, lo stesso sorgente lo contiene, tranne con durata 0
            // (B.5: nessun blocco temporale, il player non viene emesso).
            let con = html_con(src, &Opzioni { leggibile: pretty, senza_player: false });
            assert_eq!(con.contains("joy-player"), src == TRE_SCENE, "{nome}");
            assert!(script(&con).contains("Object.defineProperty(window,\"__joy_seek\""), "{nome}");
        }
    }
}

#[test]
fn un_solo_script_in_ogni_combinazione() {
    for src in [TRE_SCENE, PALCO] {
        for o in [
            compatto(),
            leggibile(),
            senza_player(),
            Opzioni { leggibile: true, senza_player: true },
        ] {
            let h = html_con(src, &o);
            assert_eq!(h.matches("<script").count(), 1, "{o:?}");
            assert_eq!(h.matches("</script>").count(), 1, "{o:?}");
        }
    }
}

#[test]
fn nessun_nome_o_testo_utente_nello_script() {
    let payload = "</script><script>alert(1)//";
    let con = format!(
        "{PALCO}Crea un cerchio chiamato \"{payload}\".\nCrea un testo chiamato \"t{payload}\" con testo \"{payload}Z\".\nDal secondo 0 al secondo 1:\n  \"{payload}\" appare sfumando.\n"
    );
    let benigno = format!(
        "{PALCO}Crea un cerchio chiamato \"c\".\nCrea un testo chiamato \"t\" con testo \"Z\".\nDal secondo 0 al secondo 1:\n  \"c\" appare sfumando.\n"
    );
    for o in [compatto(), leggibile(), senza_player()] {
        let h = html_con(&con, &o);
        // Il payload non rompe la struttura: un solo script, che non lo contiene.
        let s = script(&h);
        assert!(!s.contains("alert"), "{o:?}");
        assert!(!s.contains("script"), "{o:?}");
        assert!(!s.contains("Z\""), "{o:?}");
        assert!(!h.contains("<script>alert"), "{o:?}");
        assert!(!h.contains("</script><script>"), "{o:?}");
        // Né il player né il CSS portano il testo utente.
        let tutto_tranne_palco = h.split("<div id=\"palco\"").next().unwrap().to_string()
            + h.split("</div>").last().unwrap_or("");
        assert!(!tutto_tranne_palco.contains("alert"), "{o:?}");
        // Lo script è identico a quello di una scena con nomi innocui e stessa durata/dimensioni.
        let rif = html_con(&benigno, &o);
        assert_eq!(s, script(&rif), "{o:?}: lo script dipende dal testo utente");
    }
}

#[test]
fn durata_in_ms_nello_script_per_piu_scene() {
    let h = html_con(TRE_SCENE, &compatto());
    assert!(script(&h).contains("var W=320,H=180,D=2500;"), "{}", script(&h));
    let h = html_con(TRE_SCENE, &leggibile());
    assert!(script(&h).contains("var W=320,H=180,D=2500;"));
    let h = html_con(TRE_SCENE, &senza_player());
    assert!(script(&h).contains("var W=320,H=180,D=2500;"));
    // Una sola scena frazionaria: 0.5 s -> 500 ms.
    let una = "Prepara un palco grande 64 per 36.\nCrea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 0.5:\n  \"a\" appare sfumando.\n";
    assert!(script(&html_con(una, &compatto())).contains("var W=64,H=36,D=500;"));
    // Senza timeline la durata è 0, ma __joy_seek c'è (B.9.2).
    let h0 = html_con("Prepara un palco grande 64 per 36.\n", &compatto());
    assert!(script(&h0).contains("var W=64,H=36,D=0;"));
    assert!(script(&h0).contains("__joy_seek"));
}

#[test]
fn data_joy_render_e_regola_che_nasconde_il_player() {
    let h = html_con(TRE_SCENE, &compatto());
    assert!(stile(&h).contains("html[data-joy-render] #joy-player{display:none!important}"));
    let s = script(&h);
    // `__joy_seek` imposta l'attributo e azzera la scala; l'adattamento alla finestra lo rispetta.
    assert!(s.contains("e0.setAttribute(\"data-joy-render\",\"\")"));
    assert!(s.contains("p.style.transform=\"none\""));
    assert!(s.contains("function f(){if(e0.hasAttribute(\"data-joy-render\"))return;"));
    // Il player ha le guardie in modalità render su tutte le azioni.
    assert!(s.contains("function R(){return e0.hasAttribute(\"data-joy-render\")}"));
    assert!(s.matches("if(R())return").count() >= 6);
    // Senza player la regola CSS non c'è, ma la guardia dell'adattamento sì.
    let h2 = html_con(TRE_SCENE, &senza_player());
    assert!(!stile(&h2).contains("joy-player"));
    assert!(script(&h2).contains("data-joy-render"));
}

#[test]
fn joy_seek_valida_clampa_e_restituisce_promise() {
    for o in [compatto(), senza_player(), leggibile()] {
        let h = html_con(TRE_SCENE, &o);
        let s = senza_spazi(script(&h));
        assert!(s.contains("Object.defineProperty(window,\"__joy_seek\""));
        // Validazione: solo numeri finiti, altrimenti Promise rifiutata (TypeError).
        assert!(s.contains("typeofms!==\"number\"||!isFinite(ms)"));
        assert!(s.contains("Promise.reject(newTypeError("));
        // Clamp a [0, D].
        assert!(s.contains("Math.min(Math.max(ms,0),D)"));
        // Attende i font e l'avvio delle animazioni e due frame.
        assert!(s.contains("document.fonts.ready"));
        assert!(s.contains("requestAnimationFrame(function(){requestAnimationFrame("));
    }
}

#[test]
fn pretty_e_compatto_identici_a_meno_degli_spazi() {
    for src in [TRE_SCENE, PALCO] {
        for senza in [false, true] {
            let c = html_con(src, &Opzioni { leggibile: false, senza_player: senza });
            let l = html_con(src, &Opzioni { leggibile: true, senza_player: senza });
            assert_ne!(c, l, "il leggibile deve differire dal compatto");
            assert!(l.len() > c.len());
            assert_eq!(senza_spazi(&c), senza_spazi(&l), "senza_player={senza}");
        }
    }
    // Anche sugli esempi reali (compresi gruppi e orbita).
    for nome in ["primo_test", "sistema_orbitale"] {
        let s = fs::read_to_string(radice().join(format!("examples/{nome}.joy"))).unwrap();
        let c = html_con(&s, &compatto());
        let l = html_con(&s, &leggibile());
        assert_eq!(senza_spazi(&c), senza_spazi(&l), "{nome}");
    }
}

// ===================================================== node --check

fn node_disponibile() -> bool {
    Command::new("node").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn script_con_sintassi_javascript_valida_node_check() {
    if !node_disponibile() {
        eprintln!("test saltato: node non è nel PATH");
        return;
    }
    let dir = Cartella::nuova("node");
    let mut sorgenti = vec![TRE_SCENE.to_string(), PALCO.to_string()];
    sorgenti.push(fs::read_to_string(radice().join("examples/sistema_orbitale.joy")).unwrap());
    let mut n = 0;
    for src in &sorgenti {
        for o in [
            compatto(),
            leggibile(),
            senza_player(),
            Opzioni { leggibile: true, senza_player: true },
        ] {
            let h = html_con(src, &o);
            let js = dir.file(&format!("script-{n}.js"));
            n += 1;
            fs::write(&js, script(&h)).unwrap();
            let out = Command::new("node").arg("--check").arg(&js).output().unwrap();
            assert!(
                out.status.success(),
                "node --check ha fallito per {o:?}:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
}

// ===================================================== CLI: errori di uso (exit 2)

#[test]
fn cli_render_errori_di_uso_exit_2() {
    let dir = Cartella::nuova("uso");
    let s = dir.file("s.joy");
    fs::write(&s, TRE_SCENE).unwrap();
    let sp = s.to_str().unwrap();
    let casi: [(&[&str], &str); 8] = [
        (&["render", sp, "--fps", "0"], "il valore di --fps deve essere un intero tra 1 e 240 (trovato «0»)"),
        (&["render", sp, "--fps", "241"], "il valore di --fps deve essere un intero tra 1 e 240 (trovato «241»)"),
        (&["render", sp, "--crf", "52"], "il valore di --crf deve essere un intero tra 0 e 51 (trovato «52»)"),
        (&["render", sp, "--watch"], "l'opzione «--watch» non è ammessa con «joypp render»"),
        (&["render", sp, "--pretty"], "l'opzione «--pretty» non è ammessa con «joypp render»"),
        (&["render", sp, "--senza-player"], "l'opzione «--senza-player» non è ammessa con «joypp render»"),
        (&[sp, "--fps", "30"], "l'opzione «--fps» vale solo con «joypp render»"),
        (&["render"], "manca il file sorgente .joy"),
    ];
    for (args, atteso) in casi {
        let out = joypp(args, &[], None);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let e = stderr(&out);
        assert!(e.contains(atteso), "{args:?}: {e}");
        assert!(e.contains("Uso:"), "{args:?}: deve seguire l'aiuto");
        assert!(out.stdout.is_empty(), "{args:?}");
    }
    // Niente video creato da un uso errato.
    assert!(!dir.file("s.mp4").exists());
}

#[test]
fn cli_aiuto_descrive_render_e_variabili() {
    let out = joypp(&["--help"], &[], None);
    assert!(out.status.success());
    let t = String::from_utf8_lossy(&out.stdout);
    for parola in ["joypp render", "--fps", "--crf", "JOYPP_BROWSER", "JOYPP_FFMPEG", "--senza-player"] {
        assert!(t.contains(parola), "aiuto senza {parola}");
    }
}

// ===================================================== CLI: file chiamato render, errori di render

#[test]
fn cli_file_chiamato_render_si_compila_in_html() {
    let dir = Cartella::nuova("render-file");
    fs::write(dir.file("render"), TRE_SCENE).unwrap();
    // `./render` è il file sorgente: compila in HTML (con l'avviso sull'estensione).
    let out = joypp(&["./render", "-o", "out.html"], &[], Some(&dir.0));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).contains("non ha estensione .joy"));
    let h = fs::read_to_string(dir.file("out.html")).unwrap();
    assert!(h.contains("__joy_seek") && h.contains("joy-player"));
    // `render ./render` è il sottocomando applicato al file `render`: percorso del video, non HTML.
    // Con ffmpeg inesistente il render fallisce dopo la pianificazione, quindi il sottocomando
    // è stato riconosciuto.
    let inesistente = dir.file("non-esiste-ffmpeg");
    let out = joypp(&["render", "./render", "-o", "v.mp4"], &[("JOYPP_FFMPEG", &inesistente)], Some(&dir.0));
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("JOYPP_FFMPEG indica «"), "{}", stderr(&out));
    assert!(!dir.file("v.mp4").exists());
}

#[test]
fn cli_render_output_uguale_al_sorgente_esce_1_senza_toccarlo() {
    let dir = Cartella::nuova("render-uguale");
    let s = dir.file("scena.joy");
    fs::write(&s, TRE_SCENE).unwrap();
    let sp = s.to_str().unwrap();
    let out = joypp(&["render", sp, "-o", sp], &[], None);
    assert_eq!(out.status.code(), Some(1));
    let e = stderr(&out);
    assert!(e.contains("Errore"), "{e}");
    assert!(e.contains("scegli un percorso diverso con -o, es. -o video.mp4"), "{e}");
    assert_eq!(fs::read_to_string(&s).unwrap(), TRE_SCENE);
}

#[test]
fn cli_render_scena_senza_timeline_esce_1() {
    let dir = Cartella::nuova("render-senza-timeline");
    let s = dir.file("vuota.joy");
    fs::write(&s, "Prepara un palco grande 64 per 36.\nCrea un cerchio chiamato \"a\".\n").unwrap();
    let out = joypp(&["render", s.to_str().unwrap()], &[], None);
    assert_eq!(out.status.code(), Some(1));
    let e = stderr(&out);
    assert!(e.contains("la scena non ha una timeline: il video sarebbe un solo fotogramma"), "{e}");
    assert!(e.contains("Dal secondo 0 al secondo 3:"), "{e}");
    // Il video predefinito (sorgente con estensione .mp4) non viene creato.
    assert!(!dir.file("vuota.mp4").exists());
    assert!(!dir.file("vuota.mp4.joypp-parziale").exists());
}

#[test]
fn cli_render_palco_non_intero_esce_1() {
    let dir = Cartella::nuova("render-frazionario");
    let s = dir.file("f.joy");
    fs::write(&s, "Prepara un palco grande 800.5 per 600.\nCrea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" appare sfumando.\n").unwrap();
    let out = joypp(&["render", s.to_str().unwrap()], &[], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("per il video le dimensioni del palco devono essere intere (trovato 800.5 × 600)"));
}

#[test]
fn cli_render_ffmpeg_inesistente_messaggio_italiano_exit_1() {
    let dir = Cartella::nuova("ffmpeg-assente");
    let s = dir.file("scena.joy");
    fs::write(&s, TRE_SCENE).unwrap();
    let falso = dir.file("ffmpeg-che-non-esiste.exe");
    let out = joypp(&["render", s.to_str().unwrap()], &[("JOYPP_FFMPEG", &falso)], None);
    assert_eq!(out.status.code(), Some(1));
    let e = stderr(&out);
    assert!(e.starts_with("Errore: JOYPP_FFMPEG indica «"), "{e}");
    assert!(e.contains("», che non esiste"), "{e}");
    // Nessun file creato accanto al sorgente.
    assert!(!dir.file("scena.mp4").exists());
    assert!(!dir.file("scena.mp4.joypp-parziale").exists());
}

fn ffmpeg_nel_path() -> bool {
    Command::new("ffmpeg").arg("-version").output().map(|o| o.status.success()).unwrap_or(false)
}

#[test]
fn cli_render_browser_inesistente_messaggio_italiano_exit_1() {
    // L'ordine di controllo di B.9.7 cerca ffmpeg prima del browser: serve un ffmpeg funzionante.
    if !ffmpeg_nel_path() && std::env::var_os("JOYPP_FFMPEG").is_none() {
        eprintln!("test saltato: ffmpeg non trovato (serve per arrivare al controllo del browser)");
        return;
    }
    let dir = Cartella::nuova("browser-assente");
    let s = dir.file("scena.joy");
    fs::write(&s, TRE_SCENE).unwrap();
    let falso = dir.file("browser-che-non-esiste.exe");
    let out = joypp(&["render", s.to_str().unwrap()], &[("JOYPP_BROWSER", &falso)], None);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let e = stderr(&out);
    assert!(e.starts_with("Errore: JOYPP_BROWSER indica «"), "{e}");
    assert!(e.contains("», che non esiste"), "{e}");
    assert!(!dir.file("scena.mp4").exists());
    assert!(!dir.file("scena.mp4.joypp-parziale").exists());
}

// ===================================================== funzioni pure

fn comando(args: &[&str]) -> Comando {
    interpreta(args.iter().map(|s| s.to_string())).unwrap()
}

#[test]
fn cli_interpreta_predefiniti_di_render() {
    match comando(&["render", "video.joy"]) {
        Comando::Render(r) => {
            assert_eq!(r.uscita, None);
            assert_eq!(r.video.fps, None);
            assert_eq!(r.video.crf, CRF_PREDEFINITO);
            assert_eq!(uscita_video_predefinita(&r.ingresso), PathBuf::from("video.mp4"));
        }
        altro => panic!("{altro:?}"),
    }
}

#[test]
fn fps_predefinito_dall_intestazione_poi_60_sovrascritto_da_opzione() {
    assert_eq!(FPS_PREDEFINITO, 60);
    let senza = compila_con_avvisi("Prepara un palco grande 640 per 360.\n", &compatto()).unwrap();
    let a30 = compila_con_avvisi("Prepara un palco grande 640 per 360 a 30 fotogrammi.\n", &compatto()).unwrap();
    assert_eq!(senza.fps, None);
    assert_eq!(a30.fps, Some(30));
    assert_eq!(fps_effettivo(None, senza.fps), 60);
    assert_eq!(fps_effettivo(None, a30.fps), 30);
    assert_eq!(fps_effettivo(Some(60), a30.fps), 60);
    assert_eq!(fps_effettivo(Some(1), None), 1);
    assert_eq!(fps_effettivo(Some(240), a30.fps), 240);
}

#[test]
fn pianifica_usa_la_precedenza_degli_fps_e_calcola_n() {
    let con_timeline = |intestazione: &str| {
        format!("Prepara un palco grande 640 per 360{intestazione}.\nCrea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" appare sfumando.\n")
    };
    let c = compila_con_avvisi(&con_timeline(" a 30 fotogrammi"), &compatto()).unwrap();
    let p = pianifica(&c, None).unwrap();
    assert_eq!((p.fps, p.n, p.d_ms), (30, 31, 1000.0));
    let p = pianifica(&c, Some(60)).unwrap();
    assert_eq!((p.fps, p.n), (60, 61));
    let c = compila_con_avvisi(&con_timeline(""), &compatto()).unwrap();
    assert_eq!(pianifica(&c, None).unwrap().fps, 60);
    assert_eq!(pianifica(&c, None).unwrap().dimensioni_video(), (640, 360));
}

#[test]
fn pianifica_errori_prima_di_avviare_processi() {
    let piano = |src: &str| compila_con_avvisi(src, &compatto()).unwrap();
    let e = pianifica(&piano("Prepara un palco grande 64 per 36.\n"), None).unwrap_err();
    assert!(e.messaggio.contains("la scena non ha una timeline"));
    let anim = "Crea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 1:\n  \"a\" appare sfumando.\n";
    let e = pianifica(&piano(&format!("Prepara un palco grande 800.5 per 600.\n{anim}")), None).unwrap_err();
    assert!(e.messaggio.contains("devono essere intere (trovato 800.5 × 600)"));
    let e = pianifica(&piano(&format!("Prepara un palco grande 8193 per 100.\n{anim}")), None).unwrap_err();
    assert!(e.messaggio.contains("troppo grande"));
    // Un palco 8191 dispari diventa 8192: ancora ammesso; 8192×8192 supera i pixel massimi.
    assert!(pianifica(&piano(&format!("Prepara un palco grande 8191 per 100.\n{anim}")), None).is_ok());
    let e = pianifica(&piano(&format!("Prepara un palco grande 8192 per 8192.\n{anim}")), None).unwrap_err();
    assert!(e.messaggio.contains("troppo grande"));
    // Un'ora a 240 fps: troppi fotogrammi.
    let lunga = "Prepara un palco grande 64 per 36 a 240 fotogrammi.\nCrea un cerchio chiamato \"a\".\nDal secondo 0 al secondo 3600:\n  \"a\" appare sfumando.\n";
    let e = pianifica(&piano(lunga), None).unwrap_err();
    assert!(e.messaggio.contains("il video avrebbe troppi fotogrammi: 864001 (massimo 216001)"), "{e}");
    assert_eq!(e.suggerimento.as_deref(), Some("riduci --fps o la durata della scena"));
    // Lo stesso con 60 fps è esattamente al limite.
    let a60 = lunga.replace(" a 240 fotogrammi", "");
    assert_eq!(pianifica(&piano(&a60), None).unwrap().n, 216_001);
}

fn vicino(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn campionamento_tabella_di_b93() {
    // (D ms, fps, N)
    for (d, fps, n) in [(8500.0, 60, 511u64), (1000.0, 30, 31), (1010.0, 60, 62), (300.0, 1, 2)] {
        assert_eq!(numero_fotogrammi(d, fps), n, "D={d} fps={fps}");
    }
    assert!(vicino(istante(509, 60, 8500.0), 8483.33));
    assert_eq!(istante(510, 60, 8500.0), 8500.0);
    assert_eq!(istante(30, 30, 1000.0), 1000.0);
    assert_eq!(istante(60, 60, 1010.0), 1000.0);
    assert_eq!(istante(61, 60, 1010.0), 1010.0);
    assert_eq!(istante(0, 1, 300.0), 0.0);
    assert_eq!(istante(1, 1, 300.0), 300.0);
}

#[test]
fn campionamento_casi_limite_fps_1_e_240() {
    // Durata non multipla del passo e fps estremi.
    assert_eq!(numero_fotogrammi(1000.0, 1), 2);
    assert_eq!(numero_fotogrammi(1000.0, 240), 241);
    assert_eq!(numero_fotogrammi(1.0, 240), 2); // 0.24 fotogrammi: serve comunque l'ultimo stato
    assert_eq!(numero_fotogrammi(1.0, 1), 2);
    assert_eq!(numero_fotogrammi(0.0, 60), 1);
    assert_eq!(numero_fotogrammi(1001.0, 1), 3); // 1.001 -> ceil = 2, + 1
    assert_eq!(numero_fotogrammi(3_600_000.0, 60), 216_001);
    assert_eq!(numero_fotogrammi(3_600_000.0, 240), 864_001);
    // Durata esattamente multipla in virgola mobile: nessun fotogramma in più.
    assert_eq!(numero_fotogrammi(100.0, 30), 4); // 3.0000000000000004? deve restare 3 + 1
    assert_eq!(numero_fotogrammi(700.0, 30), 22); // 21 esatto + 1
    assert_eq!(numero_fotogrammi(10_000.0, 240), 2401);
    // Durata non multipla: l'ultimo istante è limitato a D.
    assert_eq!(istante(numero_fotogrammi(1.0, 240) - 1, 240, 1.0), 1.0);
    assert_eq!(istante(1, 1, 1.0), 1.0);
}

#[test]
fn campionamento_proprieta_ultimo_istante_e_monotonia() {
    for fps in [1u32, 2, 7, 24, 25, 29, 30, 59, 60, 120, 240] {
        for d in [1.0, 33.3333, 300.0, 999.9, 1000.0, 1010.0, 2500.0, 8500.0, 12345.6789] {
            let n = numero_fotogrammi(d, fps);
            assert!(n >= 2, "D={d} fps={fps}");
            assert_eq!(istante(0, fps, d), 0.0);
            assert_eq!(istante(n - 1, fps, d), d, "D={d} fps={fps}: l'ultimo fotogramma deve essere D");
            let mut prec = -1.0;
            for k in 0..n {
                let t = istante(k, fps, d);
                assert!(t > prec, "D={d} fps={fps} k={k}: non strettamente crescente ({prec} -> {t})");
                assert!(t <= d);
                prec = t;
            }
            // Il fotogramma precedente all'ultimo è strettamente prima di D.
            assert!(istante(n - 2, fps, d) < d, "D={d} fps={fps}");
        }
    }
}

#[test]
fn argomenti_ffmpeg_1280x720_vettore_esatto() {
    let parziale = Path::new("/tmp/out.mp4.joypp-parziale");
    let a = argomenti_ffmpeg(1280, 720, 60, 18, [0, 0, 0], parziale);
    let atteso: Vec<OsString> = [
        "-hide_banner", "-loglevel", "error", "-nostats", "-y", "-f", "image2pipe", "-c:v", "png",
        "-framerate", "60", "-i", "-", "-vf",
        "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
        "-an", "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p",
        "-color_range", "tv", "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc",
        "bt709", "-movflags", "+faststart", "-f", "mp4",
    ]
    .iter()
    .map(OsString::from)
    .chain([OsString::from("file:/tmp/out.mp4.joypp-parziale")])
    .collect();
    assert_eq!(a, atteso);
}

#[test]
fn argomenti_ffmpeg_dimensioni_dispari_con_pad() {
    let parziale = Path::new("x.mp4.joypp-parziale");
    let a = argomenti_ffmpeg(1281, 721, 60, 18, [0x0f, 0x17, 0x2a], parziale);
    let vf = a.iter().position(|x| x == "-vf").map(|i| a[i + 1].to_string_lossy().into_owned()).unwrap();
    assert_eq!(vf, "pad=1282:722:0:0:color=0x0f172a,scale=out_color_matrix=bt709:out_range=tv,format=yuv420p");
    // Solo una dimensione dispari: si aggiunge solo quella.
    assert!(filtro_video(64, 35, [0, 0, 0]).starts_with("pad=64:36:0:0:color=0x000000,"));
    assert!(filtro_video(63, 36, [255, 255, 255]).starts_with("pad=64:36:0:0:color=0xffffff,"));
    // Dimensioni pari: nessun pad.
    assert!(!filtro_video(64, 36, [1, 2, 3]).contains("pad="));
    // Il valore di -crf e -framerate segue i parametri.
    let b = argomenti_ffmpeg(64, 36, 7, 51, [0, 0, 0], parziale);
    let dopo = |chiave: &str| b.iter().position(|x| x == chiave).map(|i| b[i + 1].to_string_lossy().into_owned());
    assert_eq!(dopo("-crf").as_deref(), Some("51"));
    assert_eq!(dopo("-framerate").as_deref(), Some("7"));
    // L'uscita è sempre un `file:`: un nome che inizia con `-` o `pipe:1` non diventa un'opzione.
    let c = argomenti_ffmpeg(64, 36, 7, 0, [0, 0, 0], Path::new("-pipe:1"));
    assert_eq!(c.last().unwrap(), &OsString::from("file:-pipe:1"));
}

#[test]
fn sfondo_su_nero_e_url_file() {
    assert_eq!(sfondo_su_nero(Colore { r: 255, g: 0, b: 0, a: 0.5 }), [128, 0, 0]);
    assert_eq!(sfondo_su_nero(Colore { r: 15, g: 23, b: 42, a: 1.0 }), [15, 23, 42]);
    assert_eq!(sfondo_su_nero(Colore { r: 200, g: 200, b: 200, a: 0.0 }), [0, 0, 0]);
    assert_eq!(
        url_file(r"C:\Users\Mario Rossi\AppData\Local\Temp\joypp-render-1-0\scena.html").unwrap(),
        "file:///C:/Users/Mario%20Rossi/AppData/Local/Temp/joypp-render-1-0/scena.html"
    );
    assert_eq!(
        url_file(r"\\?\C:\Temp\joypp-render-1-0\scena.html").unwrap(),
        "file:///C:/Temp/joypp-render-1-0/scena.html"
    );
    assert!(url_file(r"\\server\share\scena.html").is_err());
    assert!(url_file(r"\\?\UNC\server\share\scena.html").is_err());
    assert_eq!(url_file("/tmp/é à/scena.html").unwrap(), "file:///tmp/%C3%A9%20%C3%A0/scena.html");
}

// ===================================================== esempio sistema_orbitale con la CLI

#[test]
fn cli_sistema_orbitale_senza_avvisi_con_e_senza_player() {
    let dir = Cartella::nuova("orbitale");
    let esempio = radice().join("examples/sistema_orbitale.joy");
    let con = dir.file("con.html");
    let senza = dir.file("senza.html");
    let out = joypp(&[esempio.to_str().unwrap(), "-o", con.to_str().unwrap()], &[], None);
    assert!(out.status.success());
    assert!(stderr(&out).is_empty(), "avvisi inattesi: {}", stderr(&out));
    let out = joypp(&[esempio.to_str().unwrap(), "-o", senza.to_str().unwrap(), "--senza-player"], &[], None);
    assert!(out.status.success());
    assert!(stderr(&out).is_empty(), "avvisi inattesi: {}", stderr(&out));
    let (c, s) = (fs::read_to_string(&con).unwrap(), fs::read_to_string(&senza).unwrap());
    assert!(c.contains("id=\"joy-player\"") && !s.contains("joy-player"));
    assert!(c.contains("var W=1280,H=720,D=9000;") && s.contains("var W=1280,H=720,D=9000;"));
    assert!(c.len() > s.len());
}
