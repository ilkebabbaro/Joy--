//! Ricerca e pilotaggio di `ffmpeg` (B.9.6): PNG in pipe su stdin, H.264 `yuv420p` in MP4.
//! Sempre argomenti separati, mai una shell.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use super::browser::{Sistema, cerca_nel_path, da_variabile, ripulisci_riga};
use crate::errore::Errore;

/// Fotogrammi in coda tra il ciclo di cattura e il thread di scrittura (B.9.5, «Pipeline»).
const CODA_FOTOGRAMMI: usize = 4;
/// Quantità di stderr conservata (B.9.6).
const MAX_STDERR: usize = 64 * 1024;

/// Trova `ffmpeg`: `JOYPP_FFMPEG` (senza ripiego se non esiste), poi `ffmpeg` nel `PATH`.
///
/// # Errori
/// `JOYPP_FFMPEG indica «X», che non esiste` oppure `ffmpeg non trovato`.
pub fn cerca_ffmpeg(
    sistema: Sistema,
    var: &dyn Fn(&str) -> Option<String>,
    esiste: &dyn Fn(&Path) -> bool,
) -> Result<PathBuf, Errore> {
    if let Some(r) = da_variabile("JOYPP_FFMPEG", var, esiste) {
        return r;
    }
    cerca_nel_path(&["ffmpeg"], sistema, var, esiste).ok_or_else(|| {
        Errore::globale("ffmpeg non trovato").con_suggerimento(
            "installalo da https://ffmpeg.org e aggiungilo al PATH, oppure indica il percorso con la variabile d'ambiente JOYPP_FFMPEG",
        )
    })
}

/// Verifica che l'eseguibile funzioni (`-hide_banner -version` con codice 0).
///
/// # Errori
/// `impossibile avviare ffmpeg («X»): …` o codice d'uscita diverso da 0.
pub fn verifica(exe: &Path) -> Result<(), Errore> {
    let stato = Command::new(exe)
        .args(["-hide_banner", "-version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| {
            Errore::globale(format!("impossibile avviare ffmpeg («{}»): {e}", exe.display()))
        })?;
    if stato.success() {
        Ok(())
    } else {
        Err(Errore::globale(format!(
            "ffmpeg («{}») non funziona: `-version` è terminato con un errore",
            exe.display()
        )))
    }
}

/// Filtro video: converte con matrice BT.709 e, per dimensioni dispari, aggiunge un bordo a
/// destra e in basso del colore `sfondo` (B.9.6).
pub fn filtro_video(larghezza: u32, altezza: u32, sfondo: [u8; 3]) -> String {
    let conv = "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p";
    if larghezza.is_multiple_of(2) && altezza.is_multiple_of(2) {
        conv.to_string()
    } else {
        format!(
            "pad={}:{}:0:0:color=0x{:02x}{:02x}{:02x},{conv}",
            larghezza + larghezza % 2,
            altezza + altezza % 2,
            sfondo[0],
            sfondo[1],
            sfondo[2]
        )
    }
}

/// Argomenti di `ffmpeg` (senza l'eseguibile). `larghezza` e `altezza` sono quelle del palco.
/// L'uscita è `file:<percorso assoluto del parziale>`.
pub fn argomenti_ffmpeg(
    larghezza: u32,
    altezza: u32,
    fps: u32,
    crf: u8,
    sfondo: [u8; 3],
    parziale: &Path,
) -> Vec<OsString> {
    let mut a: Vec<OsString> = Vec::new();
    for s in [
        "-hide_banner", "-loglevel", "error", "-nostats", "-y", "-f", "image2pipe", "-c:v", "png",
        "-framerate",
    ] {
        a.push(s.into());
    }
    a.push(fps.to_string().into());
    for s in ["-i", "-", "-vf"] {
        a.push(s.into());
    }
    a.push(filtro_video(larghezza, altezza, sfondo).into());
    for s in ["-an", "-c:v", "libx264", "-preset", "medium", "-crf"] {
        a.push(s.into());
    }
    a.push(crf.to_string().into());
    for s in [
        "-pix_fmt", "yuv420p", "-color_range", "tv", "-colorspace", "bt709", "-color_primaries",
        "bt709", "-color_trc", "bt709", "-movflags", "+faststart", "-f", "mp4",
    ] {
        a.push(s.into());
    }
    let mut uscita = OsString::from("file:");
    uscita.push(parziale.as_os_str());
    a.push(uscita);
    a
}

/// Processo `ffmpeg` in esecuzione con il suo thread di scrittura. Il `Drop` lo termina.
pub struct Ffmpeg {
    figlio: Child,
    coda: Option<SyncSender<Vec<u8>>>,
    scrittore: Option<JoinHandle<Option<String>>>,
    lettore: Option<JoinHandle<()>>,
    stderr: Arc<Mutex<Vec<u8>>>,
    concluso: bool,
}

impl Ffmpeg {
    /// Avvia `exe` con `argomenti`.
    ///
    /// # Errori
    /// `impossibile avviare ffmpeg («X»): …`.
    pub fn avvia(exe: &Path, argomenti: &[OsString]) -> Result<Self, Errore> {
        let mut figlio = Command::new(exe)
            .args(argomenti)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Errore::globale(format!("impossibile avviare ffmpeg («{}»): {e}", exe.display()))
            })?;
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let lettore = figlio.stderr.take().map(|mut e| {
            let acc = Arc::clone(&stderr);
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                while let Ok(n) = e.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    if let Ok(mut v) = acc.lock() {
                        v.extend_from_slice(&buf[..n]);
                        if v.len() > MAX_STDERR {
                            let eccesso = v.len() - MAX_STDERR;
                            v.drain(..eccesso);
                        }
                    }
                }
            })
        });
        let (coda, ricevitore) = sync_channel::<Vec<u8>>(CODA_FOTOGRAMMI);
        let scrittore = figlio.stdin.take().map(|mut stdin| {
            std::thread::spawn(move || {
                for png in ricevitore {
                    if let Err(e) = stdin.write_all(&png) {
                        return Some(e.to_string());
                    }
                }
                None
            })
        });
        Ok(Self {
            figlio,
            coda: Some(coda),
            scrittore,
            lettore,
            stderr,
            concluso: false,
        })
    }

    /// Accoda un PNG (l'ordine è quello delle chiamate). `false` se ffmpeg ha chiuso la pipe:
    /// chiamare allora [`Ffmpeg::finisci`] per ottenere la causa.
    pub fn invia(&mut self, png: Vec<u8>) -> bool {
        self.coda.as_ref().is_some_and(|c| c.send(png).is_ok())
    }

    /// Chiude lo stdin, attende ffmpeg (senza timeout) e ne valuta l'esito.
    ///
    /// # Errori
    /// Codice d'uscita diverso da 0, con le ultime 20 righe di stderr; oppure errore di scrittura.
    pub fn finisci(mut self) -> Result<(), Errore> {
        self.coda = None;
        let errore_scrittura = self.scrittore.take().and_then(|t| t.join().ok().flatten());
        let stato = self.figlio.wait();
        self.concluso = true;
        if let Some(l) = self.lettore.take() {
            let _ = l.join();
        }
        let testo = {
            let v = self.stderr.lock().map(|g| g.clone()).unwrap_or_default();
            String::from_utf8_lossy(&v).into_owned()
        };
        let stato = stato.map_err(|e| Errore::globale(format!("attesa di ffmpeg fallita: {e}")))?;
        if stato.success() && errore_scrittura.is_none() {
            return Ok(());
        }
        let codice = match stato.code() {
            Some(c) => format!("codice {c}"),
            None => "terminato da un segnale".to_string(),
        };
        let righe: Vec<&str> = testo.lines().collect();
        let inizio = righe.len().saturating_sub(20);
        let mut m = if stato.success() {
            format!(
                "impossibile scrivere i fotogrammi a ffmpeg: {}",
                errore_scrittura.unwrap_or_default()
            )
        } else {
            format!("ffmpeg è terminato con un errore ({codice})")
        };
        for r in &righe[inizio..] {
            m.push_str(&format!("\n  ffmpeg: {}", ripulisci_riga(r)));
        }
        let mut e = Errore::globale(m);
        if testo.contains("libx264") {
            e = e.con_suggerimento("installa una versione di ffmpeg che includa libx264 (es. una build GPL)");
        }
        Err(e)
    }
}

impl Drop for Ffmpeg {
    fn drop(&mut self) {
        if !self.concluso {
            self.coda = None;
            let _ = self.figlio.kill();
            let _ = self.figlio.wait();
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    fn s(a: &[OsString]) -> Vec<String> {
        a.iter().map(|x| x.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn argomenti_1280x720() {
        let a = s(&argomenti_ffmpeg(1280, 720, 60, 18, [0, 0, 0], Path::new("/o/v.mp4.joypp-parziale")));
        let atteso = "-hide_banner -loglevel error -nostats -y -f image2pipe -c:v png -framerate 60 -i - \
            -vf scale=out_color_matrix=bt709:out_range=tv,format=yuv420p -an -c:v libx264 -preset medium \
            -crf 18 -pix_fmt yuv420p -color_range tv -colorspace bt709 -color_primaries bt709 \
            -color_trc bt709 -movflags +faststart -f mp4 file:/o/v.mp4.joypp-parziale";
        assert_eq!(a, atteso.split(' ').collect::<Vec<_>>());
    }

    #[test]
    fn pad_per_dimensioni_dispari() {
        let f = filtro_video(1281, 721, [0x0f, 0x17, 0x2a]);
        assert!(f.starts_with("pad=1282:722:0:0:color=0x0f172a,scale="));
        assert_eq!(filtro_video(1280, 721, [0, 0, 0]).split(',').next(), Some("pad=1280:722:0:0:color=0x000000"));
        assert_eq!(filtro_video(63, 36, [255, 0, 128]).split(',').next(), Some("pad=64:36:0:0:color=0xff0080"));
        assert!(!filtro_video(64, 36, [1, 2, 3]).contains("pad="));
    }

    #[test]
    fn uscita_con_prefisso_file() {
        let a = s(&argomenti_ffmpeg(2, 2, 1, 0, [0; 3], Path::new("-strano")));
        assert_eq!(a.last().map(String::as_str), Some("file:-strano"));
        assert!(a.contains(&"-crf".to_string()) && a.contains(&"0".to_string()));
    }

    fn var(c: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let m: HashMap<String, String> = c.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect();
        move |k: &str| m.get(k).cloned()
    }

    #[test]
    fn ricerca_ffmpeg() {
        let v = var(&[("JOYPP_FFMPEG", "/x/f")]);
        assert_eq!(cerca_ffmpeg(Sistema::Altro, &v, &|p| p == Path::new("/x/f")).unwrap(), PathBuf::from("/x/f"));
        let e = cerca_ffmpeg(Sistema::Altro, &v, &|_| false).unwrap_err();
        assert_eq!(e.messaggio, "JOYPP_FFMPEG indica «/x/f», che non esiste");
        let e = cerca_ffmpeg(Sistema::Altro, &var(&[("PATH", "/bin")]), &|_| false).unwrap_err();
        assert_eq!(e.messaggio, "ffmpeg non trovato");
        assert!(e.suggerimento.is_some());
        let p = Path::new("/bin").join("ffmpeg");
        let p2 = p.clone();
        assert_eq!(cerca_ffmpeg(Sistema::Altro, &var(&[("PATH", "/bin")]), &move |q| q == p2).unwrap(), p);
    }
}
