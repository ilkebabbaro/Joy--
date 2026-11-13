//! Ricerca e avvio del browser headless (Chrome, Chromium o Edge), B.9.5.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::errore::Errore;

/// Sistema operativo per la scelta dei percorsi standard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sistema {
    /// Windows.
    Windows,
    /// macOS.
    MacOs,
    /// Linux e altri Unix: nessun percorso fisso.
    Altro,
}

impl Sistema {
    /// Il sistema su cui gira il programma.
    pub fn corrente() -> Self {
        if cfg!(windows) {
            Sistema::Windows
        } else if cfg!(target_os = "macos") {
            Sistema::MacOs
        } else {
            Sistema::Altro
        }
    }
}

/// Nomi cercati nel `PATH`, in ordine.
const NOMI_PATH: [&str; 8] = [
    "google-chrome-stable",
    "google-chrome",
    "chromium",
    "chromium-browser",
    "microsoft-edge-stable",
    "microsoft-edge",
    "chrome",
    "msedge",
];

/// Risolve una variabile `JOYPP_*`: `None` se non definita (o vuota), `Some(Err)` se punta a un
/// file inesistente, `Some(Ok)` altrimenti. Il valore è un percorso, mai una riga di comando.
pub fn da_variabile(
    nome: &str,
    var: &dyn Fn(&str) -> Option<String>,
    esiste: &dyn Fn(&Path) -> bool,
) -> Option<Result<PathBuf, Errore>> {
    let valore = var(nome).filter(|v| !v.is_empty())?;
    let p = PathBuf::from(&valore);
    if esiste(&p) {
        Some(Ok(p))
    } else {
        Some(Err(Errore::globale(format!("{nome} indica «{valore}», che non esiste"))))
    }
}

/// Cerca `nomi` (con `.exe` su Windows) nelle cartelle di `PATH`, senza shell né `which`.
pub fn cerca_nel_path(
    nomi: &[&str],
    sistema: Sistema,
    var: &dyn Fn(&str) -> Option<String>,
    esiste: &dyn Fn(&Path) -> bool,
) -> Option<PathBuf> {
    let path = var("PATH").or_else(|| var("Path"))?;
    let cartelle: Vec<PathBuf> = std::env::split_paths(&OsString::from(path)).collect();
    for nome in nomi {
        let file = if sistema == Sistema::Windows { format!("{nome}.exe") } else { (*nome).to_string() };
        for c in &cartelle {
            if c.as_os_str().is_empty() {
                continue;
            }
            let candidato = c.join(&file);
            if esiste(&candidato) {
                return Some(candidato);
            }
        }
    }
    None
}

/// Percorsi standard del sistema, in ordine di preferenza (B.9.5, passo 2).
pub fn percorsi_standard(sistema: Sistema, var: &dyn Fn(&str) -> Option<String>) -> Vec<PathBuf> {
    let mut v = Vec::new();
    match sistema {
        Sistema::Windows => {
            let pf = var("ProgramFiles");
            let pf86 = var("ProgramFiles(x86)");
            let locale = var("LOCALAPPDATA");
            let mut aggiungi = |base: &Option<String>, resto: &str| {
                if let Some(b) = base {
                    v.push(PathBuf::from(b).join(resto));
                }
            };
            aggiungi(&pf, r"Google\Chrome\Application\chrome.exe");
            aggiungi(&pf86, r"Google\Chrome\Application\chrome.exe");
            aggiungi(&locale, r"Google\Chrome\Application\chrome.exe");
            aggiungi(&locale, r"Chromium\Application\chrome.exe");
            aggiungi(&pf86, r"Microsoft\Edge\Application\msedge.exe");
            aggiungi(&pf, r"Microsoft\Edge\Application\msedge.exe");
        }
        Sistema::MacOs => {
            let relativi = [
                "Google Chrome.app/Contents/MacOS/Google Chrome",
                "Chromium.app/Contents/MacOS/Chromium",
                "Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
            ];
            for r in relativi {
                v.push(Path::new("/Applications").join(r));
            }
            if let Some(h) = var("HOME") {
                for r in relativi {
                    v.push(Path::new(&h).join("Applications").join(r));
                }
            }
        }
        Sistema::Altro => {}
    }
    v
}

/// Trova il browser (funzione pura: variabili d'ambiente e controllo di esistenza iniettati).
///
/// # Errori
/// `JOYPP_BROWSER` che non esiste (senza ripiego) o nessun browser trovato.
pub fn cerca_browser(
    sistema: Sistema,
    var: &dyn Fn(&str) -> Option<String>,
    esiste: &dyn Fn(&Path) -> bool,
) -> Result<PathBuf, Errore> {
    if let Some(r) = da_variabile("JOYPP_BROWSER", var, esiste) {
        return r;
    }
    if let Some(p) = percorsi_standard(sistema, var).into_iter().find(|p| esiste(p)) {
        return Ok(p);
    }
    cerca_nel_path(&NOMI_PATH, sistema, var, esiste).ok_or_else(|| {
        Errore::globale("nessun browser compatibile trovato (Chrome, Chromium o Edge)").con_suggerimento(
            "installa Google Chrome o Microsoft Edge, oppure indica il percorso con la variabile d'ambiente JOYPP_BROWSER",
        )
    })
}

/// Lato minimo della finestra del browser. Con finestre più piccole (es. un palco 64×36) il
/// browser headless smette di rispondere a `Page.captureScreenshot` dopo pochi fotogrammi
/// (provato con Edge): la viewport resta comunque quella del palco grazie a
/// `Emulation.setDeviceMetricsOverride`.
pub const LATO_MINIMO_FINESTRA: u32 = 500;

/// Dimensioni della finestra: quelle del palco, ma mai sotto [`LATO_MINIMO_FINESTRA`].
pub fn dimensioni_finestra(larghezza: u32, altezza: u32) -> (u32, u32) {
    (larghezza.max(LATO_MINIMO_FINESTRA), altezza.max(LATO_MINIMO_FINESTRA))
}

/// Argomenti di avvio del browser (B.9.5). Mai `--no-sandbox`.
pub fn argomenti_browser(profilo: &Path, larghezza: u32, altezza: u32) -> Vec<OsString> {
    let mut a: Vec<OsString> = vec!["--headless=new".into(), "--remote-debugging-port=0".into()];
    let mut dati = OsString::from("--user-data-dir=");
    dati.push(profilo.as_os_str());
    a.push(dati);
    for fisso in [
        "--no-first-run",
        "--no-default-browser-check",
        "--disable-extensions",
        "--disable-sync",
        "--disable-background-networking",
        "--disable-component-update",
        "--disable-default-apps",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-renderer-backgrounding",
        "--mute-audio",
        "--hide-scrollbars",
        "--force-device-scale-factor=1",
        "--force-color-profile=srgb",
    ] {
        a.push(fisso.into());
    }
    let (lf, af) = dimensioni_finestra(larghezza, altezza);
    a.push(format!("--window-size={lf},{af}").into());
    a.push("about:blank".into());
    a
}

/// Sostituisce i caratteri di controllo (tranne la tabulazione) con `?` (B.9.8).
pub fn ripulisci_riga(riga: &str) -> String {
    riga.chars().map(|c| if c.is_control() && c != '\t' { '?' } else { c }).collect()
}

/// Interpreta `DevToolsActivePort`: riga 1 = porta, riga 2 = percorso `/devtools/browser/…`.
/// `Ok(None)` se il file non è ancora completo.
///
/// # Errori
/// Contenuto completo ma non valido.
pub fn analizza_porta(contenuto: &str) -> Result<Option<(u16, String)>, String> {
    let mut righe = contenuto.lines();
    let (Some(p), Some(percorso)) = (righe.next(), righe.next()) else {
        return Ok(None);
    };
    let porta: u16 = p.trim().parse().map_err(|_| format!("porta non valida «{}»", ripulisci_riga(p)))?;
    if porta == 0 {
        return Err("porta non valida «0»".into());
    }
    let percorso = percorso.trim();
    if !percorso.starts_with("/devtools/browser/") {
        return Err(format!("percorso non valido «{}»", ripulisci_riga(percorso)));
    }
    Ok(Some((porta, percorso.to_string())))
}

/// Processo del browser. Il `Drop` lo termina sempre (kill + wait).
pub struct Browser {
    figlio: Child,
    stderr: Arc<Mutex<VecDeque<String>>>,
    eseguibile: PathBuf,
    terminato: bool,
}

impl Browser {
    /// Avvia il browser con gli argomenti di B.9.5.
    ///
    /// # Errori
    /// `impossibile avviare il browser («X»): <errore di sistema>`.
    pub fn avvia(eseguibile: &Path, profilo: &Path, larghezza: u32, altezza: u32) -> Result<Self, Errore> {
        let mut figlio = Command::new(eseguibile)
            .args(argomenti_browser(profilo, larghezza, altezza))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Errore::globale(format!(
                    "impossibile avviare il browser («{}»): {e}",
                    eseguibile.display()
                ))
            })?;
        let righe: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        if let Some(e) = figlio.stderr.take() {
            let r = Arc::clone(&righe);
            // Il thread non viene atteso: eventuali processi figli potrebbero tenere aperta la pipe.
            std::thread::spawn(move || {
                let mut lettore = BufReader::new(e);
                let mut buf = Vec::new();
                while let Ok(n) = lettore.read_until(b'\n', &mut buf) {
                    if n == 0 {
                        break;
                    }
                    let riga = ripulisci_riga(String::from_utf8_lossy(&buf).trim_end());
                    buf.clear();
                    if let Ok(mut c) = r.lock() {
                        if c.len() >= 50 {
                            c.pop_front();
                        }
                        c.push_back(riga);
                    }
                }
            });
        }
        Ok(Self { figlio, stderr: righe, eseguibile: eseguibile.to_path_buf(), terminato: false })
    }

    /// Percorso dell'eseguibile (per i messaggi).
    pub fn eseguibile(&self) -> &Path {
        &self.eseguibile
    }

    /// Stato di uscita se il processo è già terminato.
    pub fn uscito(&mut self) -> Option<ExitStatus> {
        match self.figlio.try_wait() {
            Ok(Some(s)) => {
                self.terminato = true;
                Some(s)
            }
            _ => None,
        }
    }

    /// Ultime (al massimo 20) righe di stderr, ciascuna preceduta da `  browser: `.
    pub fn righe_stderr(&self) -> String {
        let Ok(c) = self.stderr.lock() else { return String::new() };
        let inizio = c.len().saturating_sub(20);
        c.iter().skip(inizio).filter(|r| !r.is_empty()).map(|r| format!("\n  browser: {r}")).collect()
    }

    /// Errore «il browser si è chiuso inaspettatamente», con l'eventuale stderr.
    pub fn errore_chiusura(&mut self) -> Errore {
        let codice = match self.uscito().and_then(|s| s.code()) {
            Some(c) => format!("codice {c}"),
            None => "terminato da un segnale".to_string(),
        };
        Errore::globale(format!("il browser si è chiuso inaspettatamente ({codice}){}", self.righe_stderr()))
    }

    /// Attende `DevToolsActivePort` nel profilo (ogni 50 ms, fino a `timeout`).
    ///
    /// # Errori
    /// Timeout, browser terminato prima, file non valido.
    pub fn attendi_porta(&mut self, profilo: &Path, timeout: Duration) -> Result<(u16, String), Errore> {
        let file = profilo.join("DevToolsActivePort");
        let fine = Instant::now() + timeout;
        let mut precedente: Option<String> = None;
        loop {
            if self.uscito().is_some() {
                return Err(self.errore_chiusura());
            }
            if let Ok(c) = std::fs::read_to_string(&file) {
                // Si accetta il contenuto solo se identico a quello del controllo precedente:
                // evita di leggere un file scritto a metà.
                if precedente.as_deref() == Some(c.as_str()) {
                    match analizza_porta(&c) {
                        Ok(Some(r)) => return Ok(r),
                        Ok(None) => {}
                        Err(m) => {
                            return Err(Errore::globale(format!(
                                "il browser ha scritto un file DevToolsActivePort non valido: {m}"
                            )));
                        }
                    }
                }
                precedente = Some(c);
            }
            if Instant::now() >= fine {
                return Err(Errore::globale("il browser non ha aperto la porta di debug entro 30 secondi")
                    .con_suggerimento(format!(
                        "verifica che «{}» sia Chrome, Chromium o Edge in una versione recente",
                        self.eseguibile.display()
                    )));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Attende l'uscita fino a `timeout`, poi la forza (kill + wait).
    pub fn chiudi(&mut self, timeout: Duration) {
        let fine = Instant::now() + timeout;
        while !self.terminato && Instant::now() < fine {
            if self.uscito().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        self.termina();
    }

    fn termina(&mut self) {
        if !self.terminato {
            let _ = self.figlio.kill();
            let _ = self.figlio.wait();
            self.terminato = true;
        }
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        self.termina();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    fn ambiente(coppie: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let m: HashMap<String, String> =
            coppie.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        move |k: &str| m.get(k).cloned()
    }

    #[test]
    fn variabile_vince_su_tutto() {
        let var = ambiente(&[("JOYPP_BROWSER", "/x/mio")]);
        let r = cerca_browser(Sistema::Altro, &var, &|p| p == Path::new("/x/mio"));
        assert_eq!(r.unwrap(), PathBuf::from("/x/mio"));
    }

    #[test]
    fn variabile_inesistente_e_errore_senza_ripiego() {
        let var = ambiente(&[("JOYPP_BROWSER", "/x/no"), ("PATH", "/bin")]);
        let e = cerca_browser(Sistema::Altro, &var, &|p| p.starts_with("/bin")).unwrap_err();
        assert_eq!(e.messaggio, "JOYPP_BROWSER indica «/x/no», che non esiste");
    }

    #[test]
    fn variabile_vuota_e_ignorata() {
        let var = ambiente(&[("JOYPP_BROWSER", ""), ("PATH", "/bin")]);
        let r = cerca_browser(Sistema::Altro, &var, &|p| p == Path::new("/bin").join("chromium"));
        assert_eq!(r.unwrap(), Path::new("/bin").join("chromium"));
    }

    #[test]
    fn ordine_dei_percorsi_windows() {
        let var = ambiente(&[
            ("ProgramFiles", r"C:\PF"),
            ("ProgramFiles(x86)", r"C:\PF86"),
            ("LOCALAPPDATA", r"C:\L"),
        ]);
        let v = percorsi_standard(Sistema::Windows, &var);
        assert_eq!(v.len(), 6);
        assert!(v[0].starts_with(r"C:\PF") && v[0].ends_with("chrome.exe"));
        assert!(v[4].to_string_lossy().contains("PF86") && v[4].ends_with("msedge.exe"));
        assert!(v[5].to_string_lossy().contains(r"C:\PF\Microsoft"));
        // Senza variabili: nessun percorso.
        assert!(percorsi_standard(Sistema::Windows, &ambiente(&[])).is_empty());
    }

    #[test]
    fn edge_trovato_se_manca_chrome() {
        let var = ambiente(&[("ProgramFiles(x86)", r"C:\PF86"), ("ProgramFiles", r"C:\PF")]);
        let edge = PathBuf::from(r"C:\PF86").join(r"Microsoft\Edge\Application\msedge.exe");
        let e2 = edge.clone();
        let r = cerca_browser(Sistema::Windows, &var, &move |p| p == e2);
        assert_eq!(r.unwrap(), edge);
    }

    #[test]
    fn macos_e_linux() {
        let var = ambiente(&[("HOME", "/Users/m")]);
        let v = percorsi_standard(Sistema::MacOs, &var);
        assert_eq!(v.len(), 6);
        assert!(v[3].starts_with("/Users/m/Applications"));
        assert!(percorsi_standard(Sistema::Altro, &var).is_empty());
    }

    #[test]
    fn path_in_ordine_di_nome() {
        let var = ambiente(&[("PATH", if cfg!(windows) { r"C:\a;C:\b" } else { "/a:/b" })]);
        let (a, b) = if cfg!(windows) { (r"C:\a", r"C:\b") } else { ("/a", "/b") };
        let chromium = Path::new(b).join("chromium");
        let chrome = Path::new(a).join("chrome");
        let (c1, c2) = (chromium.clone(), chrome.clone());
        let r = cerca_browser(Sistema::Altro, &var, &move |p| p == c1 || p == c2);
        assert_eq!(r.unwrap(), chromium);
        // Su Windows ogni nome riceve .exe.
        let nome = Path::new(a).join("msedge.exe");
        let n2 = nome.clone();
        assert_eq!(cerca_nel_path(&["msedge"], Sistema::Windows, &var, &move |p| p == n2), Some(nome));
    }

    #[test]
    fn nessun_browser() {
        let e = cerca_browser(Sistema::Altro, &ambiente(&[]), &|_| false).unwrap_err();
        assert_eq!(e.messaggio, "nessun browser compatibile trovato (Chrome, Chromium o Edge)");
        assert!(e.suggerimento.is_some());
    }

    #[test]
    fn argomenti_senza_no_sandbox() {
        let a = argomenti_browser(Path::new("/tmp/p"), 640, 360);
        let t: Vec<String> = a.iter().map(|x| x.to_string_lossy().into_owned()).collect();
        assert_eq!(t[0], "--headless=new");
        assert_eq!(t[1], "--remote-debugging-port=0");
        assert!(t[2].starts_with("--user-data-dir="));
        assert!(t.contains(&"--window-size=640,500".to_string()));
        assert_eq!(dimensioni_finestra(1280, 720), (1280, 720));
        assert_eq!(dimensioni_finestra(64, 36), (500, 500));
        assert_eq!(t.last().map(String::as_str), Some("about:blank"));
        assert!(!t.iter().any(|x| x.contains("no-sandbox") || x.contains("remote-allow-origins")));
    }

    #[test]
    fn file_porta() {
        assert_eq!(analizza_porta("9222"), Ok(None));
        assert_eq!(analizza_porta("9222\n/devtools/browser/abc"), Ok(Some((9222, "/devtools/browser/abc".into()))));
        assert!(analizza_porta("0\n/devtools/browser/a").is_err());
        assert!(analizza_porta("70000\n/devtools/browser/a").is_err());
        assert!(analizza_porta("x\n/devtools/browser/a").is_err());
        assert!(analizza_porta("9222\n/altro").is_err());
    }

    #[test]
    fn pulizia_stderr() {
        assert_eq!(ripulisci_riga("a\u{1b}[31mb\tc"), "a?[31mb\tc");
    }
}
