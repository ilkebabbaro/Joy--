//! Interpretazione della riga di comando di `joypp` (forma normale e sottocomando `render`,
//! SPECIFICA B.9.1). Funzioni pure, testabili senza avviare processi.

use std::path::{Path, PathBuf};

use super::Opzioni;
use super::render::{CRF_PREDEFINITO, OpzioniVideo};
use crate::errore::Errore;
use crate::security::limiti;

/// Argomenti della forma normale (compilazione in HTML).
#[derive(Debug, Clone, PartialEq)]
pub struct ArgomentiCompila {
    /// File sorgente.
    pub ingresso: PathBuf,
    /// `-o`, se presente.
    pub uscita: Option<PathBuf>,
    /// `--pretty` e `--senza-player`.
    pub opzioni: Opzioni,
    /// `--watch`.
    pub osserva: bool,
}

/// Argomenti di `joypp render`.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgomentiRender {
    /// File sorgente.
    pub ingresso: PathBuf,
    /// `-o`, se presente.
    pub uscita: Option<PathBuf>,
    /// `--fps` e `--crf`.
    pub video: OpzioniVideo,
    /// `--profilo`: stampa il rapporto di diagnostica su stderr.
    pub profilo_testo: bool,
    /// `--profilo-json <file>`: scrive il rapporto JSON nel file indicato, se presente.
    pub profilo_json: Option<PathBuf>,
}

/// Cosa deve fare il programma.
#[derive(Debug, Clone, PartialEq)]
pub enum Comando {
    /// Compila in HTML (anche con `--watch`).
    Compila(ArgomentiCompila),
    /// Esporta in MP4.
    Render(ArgomentiRender),
    /// Mostra l'aiuto.
    Aiuto,
    /// Mostra la versione.
    Versione,
}

/// Percorso del video predefinito: il sorgente con estensione `.mp4`.
pub fn uscita_video_predefinita(sorgente: &Path) -> PathBuf {
    sorgente.with_extension("mp4")
}

/// Valida un valore intero di `--fps`/`--crf`: solo cifre decimali, nell'intervallo dato.
fn intero(testo: &str, massimo: u32, minimo: u32) -> Option<u32> {
    if testo.is_empty() || !testo.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let v: u64 = testo.parse().ok()?;
    u32::try_from(v).ok().filter(|v| (minimo..=massimo).contains(v))
}

/// Interpreta gli argomenti (senza il nome del programma).
///
/// `render` è un sottocomando solo se è il **primo** argomento ed è esattamente quella parola.
///
/// # Errori
/// Errori d'uso (exit code 2), con messaggi e suggerimenti di B.9.7.
pub fn interpreta(argomenti: impl IntoIterator<Item = String>) -> Result<Comando, Errore> {
    let mut it = argomenti.into_iter().peekable();
    let render = it.peek().map(String::as_str) == Some("render");
    if render {
        it.next();
    }
    let mut ingresso: Option<PathBuf> = None;
    let mut uscita = None;
    let mut opzioni = Opzioni::default();
    let mut osserva = false;
    let mut fps: Option<u32> = None;
    let mut crf = CRF_PREDEFINITO;
    let mut profilo_testo = false;
    let mut profilo_json: Option<PathBuf> = None;
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => return Ok(Comando::Aiuto),
            "-V" | "--version" => return Ok(Comando::Versione),
            "-o" | "--output" => {
                let p = it
                    .next()
                    .ok_or_else(|| Errore::globale(format!("l'opzione {a} richiede un valore")))?;
                uscita = Some(PathBuf::from(p));
            }
            "--fps" | "--crf" => {
                if !render {
                    return Err(Errore::globale(format!("l'opzione «{a}» vale solo con «joypp render»"))
                        .con_suggerimento("es. «joypp render scena.joy --fps 30»"));
                }
                let v = it
                    .next()
                    .ok_or_else(|| Errore::globale(format!("l'opzione {a} richiede un valore")))?;
                if a == "--fps" {
                    fps = Some(intero(&v, limiti::MAX_FPS_VIDEO, 1).ok_or_else(|| {
                        Errore::globale(format!(
                            "il valore di --fps deve essere un intero tra 1 e {} (trovato «{v}»)",
                            limiti::MAX_FPS_VIDEO
                        ))
                    })?);
                } else {
                    let c = intero(&v, u32::from(limiti::MAX_CRF), 0).ok_or_else(|| {
                        Errore::globale(format!(
                            "il valore di --crf deve essere un intero tra 0 e {} (trovato «{v}»)",
                            limiti::MAX_CRF
                        ))
                        .con_suggerimento(
                            "18 è quasi indistinguibile dall'originale; valori più alti riducono il file",
                        )
                    })?;
                    crf = c as u8;
                }
            }
            "--profilo" => {
                if !render {
                    return Err(Errore::globale(format!("l'opzione «{a}» vale solo con «joypp render»"))
                        .con_suggerimento("es. «joypp render scena.joy --profilo»"));
                }
                profilo_testo = true;
            }
            "--profilo-json" => {
                if !render {
                    return Err(Errore::globale(format!("l'opzione «{a}» vale solo con «joypp render»"))
                        .con_suggerimento("es. «joypp render scena.joy --profilo-json profilo.json»"));
                }
                let p = it
                    .next()
                    .ok_or_else(|| Errore::globale(format!("l'opzione {a} richiede un valore")))?;
                profilo_json = Some(PathBuf::from(p));
            }
            "--watch" | "--pretty" | "--senza-player" if render => {
                let e = Errore::globale(format!("l'opzione «{a}» non è ammessa con «joypp render»"));
                return Err(if a == "--watch" {
                    e.con_suggerimento("il video si genera una volta sola: rilancia «joypp render» dopo le modifiche")
                } else {
                    e.con_suggerimento("il video non dipende da questa opzione: toglila")
                });
            }
            "--pretty" => opzioni.leggibile = true,
            "--senza-player" => opzioni.senza_player = true,
            "--watch" => osserva = true,
            opzione if opzione.starts_with('-') && opzione.len() > 1 => {
                return Err(Errore::globale(format!("opzione sconosciuta «{opzione}»")));
            }
            percorso => {
                if ingresso.is_some() {
                    return Err(Errore::globale(format!(
                        "è ammesso un solo file sorgente (trovato anche «{percorso}»)"
                    )));
                }
                ingresso = Some(PathBuf::from(percorso));
            }
        }
    }
    let ingresso = ingresso.ok_or_else(|| Errore::globale("manca il file sorgente .joy"))?;
    if render {
        let profilo = profilo_testo || profilo_json.is_some();
        Ok(Comando::Render(ArgomentiRender {
            ingresso,
            uscita,
            video: OpzioniVideo { fps, crf, profilo },
            profilo_testo,
            profilo_json,
        }))
    } else {
        Ok(Comando::Compila(ArgomentiCompila { ingresso, uscita, opzioni, osserva }))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn p(s: &str) -> Result<Comando, Errore> {
        interpreta(s.split_whitespace().map(String::from))
    }
    fn render(s: &str) -> ArgomentiRender {
        match p(s) {
            Ok(Comando::Render(r)) => r,
            altro => panic!("atteso Render, trovato {altro:?}"),
        }
    }
    fn compila(s: &str) -> ArgomentiCompila {
        match p(s) {
            Ok(Comando::Compila(r)) => r,
            altro => panic!("atteso Compila, trovato {altro:?}"),
        }
    }

    #[test]
    fn render_come_primo_argomento() {
        let r = render("render scena.joy");
        assert_eq!(r.ingresso, PathBuf::from("scena.joy"));
        assert_eq!(r.uscita, None);
        assert_eq!(r.video, OpzioniVideo { fps: None, crf: 18, profilo: false });
        assert!(!r.profilo_testo && r.profilo_json.is_none());
        assert_eq!(uscita_video_predefinita(&r.ingresso), PathBuf::from("scena.mp4"));
    }

    #[test]
    fn forma_normale_invariata() {
        let c = compila("scena.joy -o x.html --pretty --senza-player --watch");
        assert_eq!(c.uscita, Some(PathBuf::from("x.html")));
        assert!(c.opzioni.leggibile && c.opzioni.senza_player && c.osserva);
    }

    #[test]
    fn file_chiamato_render() {
        assert_eq!(compila("./render").ingresso, PathBuf::from("./render"));
        assert_eq!(compila(r".\render").ingresso, PathBuf::from(r".\render"));
        assert_eq!(render("render ./render").ingresso, PathBuf::from("./render"));
        assert_eq!(compila("render.joy").ingresso, PathBuf::from("render.joy"));
        assert_eq!(compila("--pretty render").ingresso, PathBuf::from("render"));
        assert!(compila("--pretty render").opzioni.leggibile);
    }

    #[test]
    fn render_senza_file() {
        assert_eq!(p("render").unwrap_err().messaggio, "manca il file sorgente .joy");
        assert_eq!(p("").unwrap_err().messaggio, "manca il file sorgente .joy");
    }

    #[test]
    fn opzioni_prima_e_dopo_il_file() {
        let r = render("render --fps 30 -o v.mp4 s.joy --crf 23");
        assert_eq!(r.video, OpzioniVideo { fps: Some(30), crf: 23, profilo: false });
        assert_eq!(r.uscita, Some(PathBuf::from("v.mp4")));
    }

    #[test]
    fn limiti_di_fps() {
        assert_eq!(render("render s.joy --fps 1").video.fps, Some(1));
        assert_eq!(render("render s.joy --fps 240").video.fps, Some(240));
        assert_eq!(render("render s.joy --fps 0060").video.fps, Some(60));
        for v in ["0", "241", "-1", "29.97", "+30", "abc", "", "1e1", "99999999999999999999"] {
            let e = interpreta(["render", "s.joy", "--fps", v].map(String::from)).unwrap_err();
            assert_eq!(
                e.messaggio,
                format!("il valore di --fps deve essere un intero tra 1 e 240 (trovato «{v}»)")
            );
        }
    }

    #[test]
    fn limiti_di_crf() {
        assert_eq!(render("render s.joy --crf 0").video.crf, 0);
        assert_eq!(render("render s.joy --crf 51").video.crf, 51);
        for v in ["52", "-1", "1e1", "+5", "x"] {
            let e = interpreta(["render", "s.joy", "--crf", v].map(String::from)).unwrap_err();
            assert_eq!(
                e.messaggio,
                format!("il valore di --crf deve essere un intero tra 0 e 51 (trovato «{v}»)")
            );
            assert!(e.suggerimento.is_some());
        }
    }

    #[test]
    fn forma_con_uguale_non_ammessa() {
        assert_eq!(p("render s.joy --fps=60").unwrap_err().messaggio, "opzione sconosciuta «--fps=60»");
    }

    #[test]
    fn opzione_ripetuta_vale_l_ultima() {
        let r = render("render s.joy --fps 10 --fps 20 --crf 1 --crf 2 -o a.mp4 -o b.mp4");
        assert_eq!(r.video, OpzioniVideo { fps: Some(20), crf: 2, profilo: false });
        assert_eq!(r.uscita, Some(PathBuf::from("b.mp4")));
    }

    #[test]
    fn profilo_testo_da_solo() {
        let r = render("render s.joy --profilo");
        assert!(r.profilo_testo);
        assert_eq!(r.profilo_json, None);
        assert!(r.video.profilo);
    }

    #[test]
    fn profilo_json_richiede_il_valore_successivo() {
        let r = render("render s.joy --profilo-json x.json");
        assert!(!r.profilo_testo);
        assert_eq!(r.profilo_json, Some(PathBuf::from("x.json")));
        assert!(r.video.profilo);
        assert_eq!(
            p("render s.joy --profilo-json").unwrap_err().messaggio,
            "l'opzione --profilo-json richiede un valore"
        );
        assert_eq!(
            p("render s.joy --profilo-json=x.json").unwrap_err().messaggio,
            "opzione sconosciuta «--profilo-json=x.json»"
        );
    }

    #[test]
    fn profilo_combinati_e_ripetuti() {
        let r = render("render s.joy --profilo --profilo-json x.json");
        assert!(r.profilo_testo);
        assert_eq!(r.profilo_json, Some(PathBuf::from("x.json")));
        assert!(r.video.profilo);
        let r = render("render s.joy --profilo-json a.json --profilo-json b.json");
        assert_eq!(r.profilo_json, Some(PathBuf::from("b.json")));
    }

    #[test]
    fn profilo_senza_render() {
        let e = interpreta(["s.joy", "--profilo"].map(String::from)).unwrap_err();
        assert_eq!(e.messaggio, "l'opzione «--profilo» vale solo con «joypp render»");
        assert_eq!(e.suggerimento.as_deref(), Some("es. «joypp render scena.joy --profilo»"));
        let e = interpreta(["s.joy", "--profilo-json", "x.json"].map(String::from)).unwrap_err();
        assert_eq!(e.messaggio, "l'opzione «--profilo-json» vale solo con «joypp render»");
        assert_eq!(
            e.suggerimento.as_deref(),
            Some("es. «joypp render scena.joy --profilo-json profilo.json»")
        );
    }

    #[test]
    fn valore_mancante() {
        assert_eq!(p("render s.joy --fps").unwrap_err().messaggio, "l'opzione --fps richiede un valore");
        assert_eq!(p("render s.joy --crf").unwrap_err().messaggio, "l'opzione --crf richiede un valore");
        assert_eq!(p("render s.joy -o").unwrap_err().messaggio, "l'opzione -o richiede un valore");
    }

    #[test]
    fn opzioni_vietate_con_render() {
        for o in ["--watch", "--pretty", "--senza-player"] {
            let e = interpreta(["render", "s.joy", o].map(String::from)).unwrap_err();
            assert_eq!(e.messaggio, format!("l'opzione «{o}» non è ammessa con «joypp render»"));
            assert!(e.suggerimento.is_some());
        }
        let e = p("render --watch s.joy").unwrap_err();
        assert!(e.suggerimento.unwrap().contains("il video si genera una volta sola"));
    }

    #[test]
    fn opzioni_vietate_senza_render() {
        for o in ["--fps", "--crf"] {
            let e = interpreta(["s.joy", o, "30"].map(String::from)).unwrap_err();
            assert_eq!(e.messaggio, format!("l'opzione «{o}» vale solo con «joypp render»"));
            assert_eq!(e.suggerimento.as_deref(), Some("es. «joypp render scena.joy --fps 30»"));
        }
    }

    #[test]
    fn errori_comuni() {
        assert_eq!(p("render a.joy b.joy").unwrap_err().messaggio, "è ammesso un solo file sorgente (trovato anche «b.joy»)");
        assert_eq!(p("render a.joy --x").unwrap_err().messaggio, "opzione sconosciuta «--x»");
    }

    #[test]
    fn aiuto_e_versione() {
        assert_eq!(p("render -h").unwrap(), Comando::Aiuto);
        assert_eq!(p("--version").unwrap(), Comando::Versione);
    }
}
