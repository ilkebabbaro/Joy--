//! Client WebSocket minimale (RFC 6455, lato client) per parlare con il browser su loopback.
//!
//! Gestisce handshake HTTP/1.1, frame di testo mascherati, lunghezze a 7/16/64 bit,
//! frammentazione, ping/pong e close. I frame binari sono rifiutati. L'intestazione
//! `Sec-WebSocket-Accept` non viene verificata (D-B9-1: servirebbe SHA-1 e il server è il
//! processo figlio appena avviato).

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::security::limiti::MAX_MESSAGGIO_CDP as MAX_MESSAGGIO;

const OP_CONTINUA: u8 = 0x0;
const OP_TESTO: u8 = 0x1;
const OP_BINARIO: u8 = 0x2;
const OP_CHIUDI: u8 = 0x8;
const OP_PING: u8 = 0x9;
const OP_PONG: u8 = 0xA;

/// Errori del livello WebSocket.
#[derive(Debug)]
pub enum ErroreWs {
    /// Errore di I/O (il timeout è `ErrorKind::TimedOut` o `WouldBlock`).
    Io(io::Error),
    /// Violazione del protocollo, con descrizione.
    Protocollo(String),
    /// Messaggio oltre [`MAX_MESSAGGIO`].
    TroppoGrande,
    /// Il server ha chiuso la connessione.
    Chiuso,
}

impl ErroreWs {
    /// `true` se l'errore è un timeout di lettura/scrittura.
    pub fn e_timeout(&self) -> bool {
        matches!(self, ErroreWs::Io(e) if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock))
    }
}

impl From<io::Error> for ErroreWs {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            ErroreWs::Chiuso
        } else {
            ErroreWs::Io(e)
        }
    }
}

/// Un frame decodificato.
#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    /// Bit FIN: ultimo frammento del messaggio.
    pub fin: bool,
    /// Codice operativo.
    pub opcode: u8,
    /// Contenuto (già smascherato).
    pub dati: Vec<u8>,
}

/// Numero pseudo-casuale a 64 bit (`RandomState`, tempo, pid, contatore). L'entropia non conta su
/// loopback: serve solo a non ripetere maschere e chiavi.
pub fn casuale() -> u64 {
    static CONTATORE: AtomicU64 = AtomicU64::new(0);
    let mut h = RandomState::new().build_hasher();
    h.write_u64(CONTATORE.fetch_add(1, Ordering::Relaxed));
    h.write_u32(std::process::id());
    if let Ok(t) = SystemTime::now().duration_since(UNIX_EPOCH) {
        h.write_u128(t.as_nanos());
    }
    h.finish()
}

/// Codifica un frame client (FIN=1) con la maschera data.
pub fn codifica_frame(opcode: u8, dati: &[u8], maschera: [u8; 4]) -> Vec<u8> {
    let mut o = Vec::with_capacity(dati.len() + 14);
    o.push(0x80 | opcode);
    let n = dati.len();
    if n < 126 {
        o.push(0x80 | n as u8);
    } else if n <= 0xFFFF {
        o.push(0x80 | 126);
        o.extend_from_slice(&(n as u16).to_be_bytes());
    } else {
        o.push(0x80 | 127);
        o.extend_from_slice(&(n as u64).to_be_bytes());
    }
    o.extend_from_slice(&maschera);
    o.extend(dati.iter().enumerate().map(|(i, b)| b ^ maschera[i % 4]));
    o
}

/// Legge un frame dal flusso. Accetta anche frame mascherati (li smaschera).
///
/// # Errori
/// I/O, lunghezza oltre [`MAX_MESSAGGIO`], bit riservati impostati.
pub fn leggi_frame(r: &mut impl Read) -> Result<Frame, ErroreWs> {
    let mut intest = [0u8; 2];
    r.read_exact(&mut intest)?;
    if intest[0] & 0x70 != 0 {
        return Err(ErroreWs::Protocollo("bit riservati impostati".into()));
    }
    let fin = intest[0] & 0x80 != 0;
    let opcode = intest[0] & 0x0F;
    let mascherato = intest[1] & 0x80 != 0;
    let lunghezza = match intest[1] & 0x7F {
        126 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
            u64::from(u16::from_be_bytes(b))
        }
        127 => {
            let mut b = [0u8; 8];
            r.read_exact(&mut b)?;
            u64::from_be_bytes(b)
        }
        n => u64::from(n),
    };
    if lunghezza > MAX_MESSAGGIO as u64 {
        return Err(ErroreWs::TroppoGrande);
    }
    let mut maschera = [0u8; 4];
    if mascherato {
        r.read_exact(&mut maschera)?;
    }
    let mut dati = vec![0u8; lunghezza as usize];
    r.read_exact(&mut dati)?;
    if mascherato {
        for (i, b) in dati.iter_mut().enumerate() {
            *b ^= maschera[i % 4];
        }
    }
    Ok(Frame { fin, opcode, dati })
}

/// Connessione WebSocket già negoziata, su un flusso qualsiasi (TCP in produzione).
pub struct Connessione<S: Read + Write> {
    flusso: S,
}

impl<S: Read + Write> Connessione<S> {
    /// Avvolge un flusso su cui l'handshake è già avvenuto.
    pub fn nuova(flusso: S) -> Self {
        Self { flusso }
    }

    /// Accesso al flusso sottostante (per impostare i timeout).
    pub fn flusso(&self) -> &S {
        &self.flusso
    }

    /// Invia un messaggio di testo.
    ///
    /// # Errori
    /// I/O.
    pub fn invia_testo(&mut self, testo: &str) -> Result<(), ErroreWs> {
        let m = casuale().to_le_bytes();
        let frame = codifica_frame(OP_TESTO, testo.as_bytes(), [m[0], m[1], m[2], m[3]]);
        self.flusso.write_all(&frame)?;
        self.flusso.flush()?;
        Ok(())
    }

    /// Riceve il prossimo messaggio di testo completo, gestendo frammentazione e ping/pong.
    ///
    /// # Errori
    /// I/O, close del server, frame binari, UTF-8 non valido, messaggio troppo grande.
    pub fn ricevi(&mut self) -> Result<String, ErroreWs> {
        let mut acc: Vec<u8> = Vec::new();
        let mut in_corso = false;
        loop {
            let f = leggi_frame(&mut self.flusso)?;
            match f.opcode {
                OP_PING => {
                    let m = casuale().to_le_bytes();
                    let pong = codifica_frame(OP_PONG, &f.dati, [m[0], m[1], m[2], m[3]]);
                    self.flusso.write_all(&pong)?;
                    self.flusso.flush()?;
                }
                OP_PONG => {}
                OP_CHIUDI => return Err(ErroreWs::Chiuso),
                OP_BINARIO => return Err(ErroreWs::Protocollo("frame binario non supportato".into())),
                OP_TESTO | OP_CONTINUA => {
                    if (f.opcode == OP_TESTO) == in_corso {
                        return Err(ErroreWs::Protocollo("sequenza di frammenti non valida".into()));
                    }
                    if acc.len() + f.dati.len() > MAX_MESSAGGIO {
                        return Err(ErroreWs::TroppoGrande);
                    }
                    acc.extend_from_slice(&f.dati);
                    in_corso = true;
                    if f.fin {
                        return String::from_utf8(acc)
                            .map_err(|_| ErroreWs::Protocollo("testo non UTF-8".into()));
                    }
                }
                _ => return Err(ErroreWs::Protocollo(format!("opcode sconosciuto {}", f.opcode))),
            }
        }
    }
}

/// Esegue l'handshake HTTP di upgrade su `flusso`.
///
/// # Errori
/// I/O, risposta senza stato `101`, intestazioni troppo lunghe.
pub fn handshake(flusso: &mut (impl Read + Write), host: &str, percorso: &str) -> Result<(), ErroreWs> {
    let mut chiave_grezza = [0u8; 16];
    chiave_grezza[..8].copy_from_slice(&casuale().to_le_bytes());
    chiave_grezza[8..].copy_from_slice(&casuale().to_le_bytes());
    let chiave = codifica_base64(&chiave_grezza);
    let richiesta = format!(
        "GET {percorso} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {chiave}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    flusso.write_all(richiesta.as_bytes())?;
    flusso.flush()?;
    // Lettura byte per byte: non si deve consumare nulla oltre la fine delle intestazioni.
    let mut risposta: Vec<u8> = Vec::new();
    let mut b = [0u8; 1];
    while !risposta.ends_with(b"\r\n\r\n") {
        if risposta.len() > 16 * 1024 {
            return Err(ErroreWs::Protocollo("intestazioni HTTP troppo lunghe".into()));
        }
        flusso.read_exact(&mut b)?;
        risposta.push(b[0]);
    }
    let testo = String::from_utf8_lossy(&risposta);
    let riga = testo.lines().next().unwrap_or("");
    if riga.split_whitespace().nth(1) != Some("101") {
        return Err(ErroreWs::Protocollo(format!("handshake rifiutato («{riga}»)")));
    }
    Ok(())
}

/// Codifica base64 (solo per la chiave dell'handshake).
fn codifica_base64(dati: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::new();
    for blocco in dati.chunks(3) {
        let mut v = 0u32;
        for (i, &b) in blocco.iter().enumerate() {
            v |= u32::from(b) << (16 - 8 * i);
        }
        for i in 0..4 {
            if i <= blocco.len() {
                o.push(A[((v >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;

    /// Flusso di prova: legge da un buffer e raccoglie ciò che viene scritto.
    struct Finto {
        da_leggere: Cursor<Vec<u8>>,
        scritto: Vec<u8>,
    }
    impl Read for Finto {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            self.da_leggere.read(b)
        }
    }
    impl Write for Finto {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.scritto.extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    fn finto(dati: Vec<u8>) -> Connessione<Finto> {
        Connessione::nuova(Finto { da_leggere: Cursor::new(dati), scritto: Vec::new() })
    }

    /// Frame del server (non mascherato).
    fn server(fin: bool, op: u8, dati: &[u8]) -> Vec<u8> {
        let mut o = vec![(if fin { 0x80 } else { 0 }) | op];
        match dati.len() {
            n if n < 126 => o.push(n as u8),
            n if n <= 0xFFFF => {
                o.push(126);
                o.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                o.push(127);
                o.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }
        o.extend_from_slice(dati);
        o
    }

    #[test]
    fn maschera_e_lunghezza_7_bit() {
        let f = codifica_frame(OP_TESTO, b"Hello", [1, 2, 3, 4]);
        assert_eq!(f[0], 0x81);
        assert_eq!(f[1], 0x80 | 5);
        assert_eq!(&f[2..6], &[1, 2, 3, 4]);
        let esp: Vec<u8> = b"Hello".iter().enumerate().map(|(i, b)| b ^ [1, 2, 3, 4][i % 4]).collect();
        assert_eq!(&f[6..], &esp[..]);
    }

    #[test]
    fn lunghezze_16_e_64_bit() {
        let d16 = vec![7u8; 300];
        let f = codifica_frame(OP_TESTO, &d16, [0; 4]);
        assert_eq!(f[1], 0x80 | 126);
        assert_eq!(&f[2..4], &300u16.to_be_bytes());
        assert_eq!(f.len(), 2 + 2 + 4 + 300);
        let d64 = vec![9u8; 70_000];
        let f = codifica_frame(OP_TESTO, &d64, [0; 4]);
        assert_eq!(f[1], 0x80 | 127);
        assert_eq!(&f[2..10], &70_000u64.to_be_bytes());
        // Andata e ritorno: la decodifica smaschera.
        let r = leggi_frame(&mut Cursor::new(f)).unwrap();
        assert_eq!(r.dati, d64);
        assert!(r.fin);
    }

    #[test]
    fn decodifica_lunghezze() {
        for n in [0usize, 125, 126, 65_535, 65_536] {
            let d = vec![b'x'; n];
            let r = leggi_frame(&mut Cursor::new(server(true, OP_TESTO, &d))).unwrap();
            assert_eq!(r.dati.len(), n);
        }
    }

    #[test]
    fn frammentazione_con_ping_in_mezzo() {
        let mut s = server(false, OP_TESTO, b"ab");
        s.extend(server(true, OP_PING, b"p"));
        s.extend(server(false, OP_CONTINUA, b"cd"));
        s.extend(server(true, OP_CONTINUA, b"ef"));
        let mut c = finto(s);
        assert_eq!(c.ricevi().unwrap(), "abcdef");
        // Ha risposto con un pong mascherato.
        assert_eq!(c.flusso.scritto[0], 0x80 | OP_PONG);
        assert_eq!(c.flusso.scritto[1] & 0x80, 0x80);
    }

    #[test]
    fn close_binario_e_sequenze_errate() {
        assert!(matches!(finto(server(true, OP_CHIUDI, b"")).ricevi(), Err(ErroreWs::Chiuso)));
        assert!(matches!(finto(server(true, OP_BINARIO, b"x")).ricevi(), Err(ErroreWs::Protocollo(_))));
        assert!(matches!(finto(server(true, OP_CONTINUA, b"x")).ricevi(), Err(ErroreWs::Protocollo(_))));
        let mut s = server(false, OP_TESTO, b"a");
        s.extend(server(true, OP_TESTO, b"b"));
        assert!(matches!(finto(s).ricevi(), Err(ErroreWs::Protocollo(_))));
        assert!(matches!(finto(vec![0x81]).ricevi(), Err(ErroreWs::Chiuso)));
        assert!(matches!(finto(server(true, OP_TESTO, &[0xff])).ricevi(), Err(ErroreWs::Protocollo(_))));
    }

    #[test]
    fn lunghezza_enorme_rifiutata_senza_allocare() {
        let mut f = vec![0x81, 127];
        f.extend_from_slice(&u64::MAX.to_be_bytes());
        assert!(matches!(leggi_frame(&mut Cursor::new(f)), Err(ErroreWs::TroppoGrande)));
    }

    #[test]
    fn invio_testo_e_mascheramento() {
        let mut c = finto(Vec::new());
        c.invia_testo("{\"id\":1}").unwrap();
        let s = &c.flusso.scritto;
        assert_eq!(s[0], 0x81);
        let m = [s[2], s[3], s[4], s[5]];
        let chiaro: Vec<u8> = s[6..].iter().enumerate().map(|(i, b)| b ^ m[i % 4]).collect();
        assert_eq!(chiaro, b"{\"id\":1}");
    }

    #[test]
    fn handshake_accetta_solo_101() {
        let ok = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\r\n\x81\x01x".to_vec();
        let mut f = Finto { da_leggere: Cursor::new(ok), scritto: Vec::new() };
        handshake(&mut f, "127.0.0.1:9222", "/devtools/browser/abc").unwrap();
        let richiesta = String::from_utf8(f.scritto.clone()).unwrap();
        assert!(richiesta.starts_with("GET /devtools/browser/abc HTTP/1.1\r\n"));
        assert!(richiesta.contains("Sec-WebSocket-Key: "));
        assert!(!richiesta.contains("Origin"));
        // Il frame dopo le intestazioni non è stato consumato.
        assert_eq!(f.da_leggere.position() as usize, f.da_leggere.get_ref().len() - 3);
        let ko = b"HTTP/1.1 403 Forbidden\r\n\r\n".to_vec();
        let mut f = Finto { da_leggere: Cursor::new(ko), scritto: Vec::new() };
        assert!(matches!(handshake(&mut f, "h", "/"), Err(ErroreWs::Protocollo(_))));
    }

    #[test]
    fn base64_della_chiave() {
        assert_eq!(codifica_base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(codifica_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(codifica_base64(b"f"), "Zg==");
        assert_eq!(codifica_base64(&[0u8; 16]).len(), 24);
    }
}
