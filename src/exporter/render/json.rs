//! JSON minimale: parser per le risposte del browser e funzione di escape per i comandi.
//!
//! Il parser è completo ma piccolo (profondità massima [`PROFONDITA_MASSIMA`]) e restituisce
//! solo errori, mai `panic!`.

use crate::security::limiti::PROFONDITA_MASSIMA_JSON as PROFONDITA_MASSIMA;

/// Valore JSON.
#[derive(Debug, Clone, PartialEq)]
pub enum Valore {
    /// `null`.
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// Numero (sempre `f64`).
    Numero(f64),
    /// Stringa decodificata.
    Testo(String),
    /// Array.
    Lista(Vec<Valore>),
    /// Oggetto, nell'ordine di comparsa delle chiavi.
    Oggetto(Vec<(String, Valore)>),
}

impl Valore {
    /// Valore associato a `chiave` se `self` è un oggetto (prima occorrenza).
    pub fn campo(&self, chiave: &str) -> Option<&Valore> {
        match self {
            Valore::Oggetto(v) => v.iter().find(|(k, _)| k == chiave).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Il testo, se `self` è una stringa.
    pub fn testo(&self) -> Option<&str> {
        match self {
            Valore::Testo(s) => Some(s),
            _ => None,
        }
    }

    /// Il numero, se `self` è un numero.
    pub fn numero(&self) -> Option<f64> {
        match self {
            Valore::Numero(n) => Some(*n),
            _ => None,
        }
    }
}

/// Racchiude `s` tra virgolette con gli escape JSON necessari.
pub fn stringa(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Interpreta un documento JSON completo.
///
/// # Errori
/// Descrizione testuale del primo problema (sintassi, profondità, testo residuo).
pub fn analizza(testo: &str) -> Result<Valore, String> {
    let mut p = Parser { d: testo.as_bytes(), i: 0 };
    p.spazi();
    let v = p.valore(0)?;
    p.spazi();
    if p.i != p.d.len() {
        return Err(format!("testo residuo alla posizione {}", p.i));
    }
    Ok(v)
}

struct Parser<'a> {
    d: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn spazi(&mut self) {
        while matches!(self.d.get(self.i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.i += 1;
        }
    }

    fn atteso(&mut self, c: u8) -> Result<(), String> {
        if self.d.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("atteso «{}» alla posizione {}", c as char, self.i))
        }
    }

    fn parola(&mut self, p: &str, v: Valore) -> Result<Valore, String> {
        if self.d[self.i..].starts_with(p.as_bytes()) {
            self.i += p.len();
            Ok(v)
        } else {
            Err(format!("valore non valido alla posizione {}", self.i))
        }
    }

    fn valore(&mut self, profondita: usize) -> Result<Valore, String> {
        if profondita > PROFONDITA_MASSIMA {
            return Err("annidamento troppo profondo".into());
        }
        match self.d.get(self.i) {
            None => Err("fine inattesa del documento".into()),
            Some(b'{') => self.oggetto(profondita),
            Some(b'[') => self.lista(profondita),
            Some(b'"') => Ok(Valore::Testo(self.stringa()?)),
            Some(b't') => self.parola("true", Valore::Bool(true)),
            Some(b'f') => self.parola("false", Valore::Bool(false)),
            Some(b'n') => self.parola("null", Valore::Null),
            Some(b'-' | b'0'..=b'9') => self.numero(),
            Some(_) => Err(format!("carattere inatteso alla posizione {}", self.i)),
        }
    }

    fn numero(&mut self) -> Result<Valore, String> {
        let inizio = self.i;
        while matches!(self.d.get(self.i), Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')) {
            self.i += 1;
        }
        let t = std::str::from_utf8(&self.d[inizio..self.i]).map_err(|e| e.to_string())?;
        match t.parse::<f64>() {
            Ok(n) if n.is_finite() => Ok(Valore::Numero(n)),
            _ => Err(format!("numero non valido alla posizione {inizio}")),
        }
    }

    fn esadecimale4(&mut self) -> Result<u32, String> {
        let s = self
            .d
            .get(self.i..self.i + 4)
            .and_then(|b| std::str::from_utf8(b).ok())
            .ok_or("escape \\u troncato")?;
        let v = u32::from_str_radix(s, 16).map_err(|_| "escape \\u non valido".to_string())?;
        self.i += 4;
        Ok(v)
    }

    fn stringa(&mut self) -> Result<String, String> {
        self.atteso(b'"')?;
        let mut o: Vec<u8> = Vec::new();
        loop {
            let inizio = self.i;
            while let Some(&c) = self.d.get(self.i) {
                if c == b'"' || c == b'\\' || c < 0x20 {
                    break;
                }
                self.i += 1;
            }
            o.extend_from_slice(&self.d[inizio..self.i]);
            match self.d.get(self.i) {
                None => return Err("stringa non terminata".into()),
                Some(b'"') => {
                    self.i += 1;
                    return String::from_utf8(o).map_err(|_| "UTF-8 non valido".to_string());
                }
                Some(b'\\') => {
                    self.i += 1;
                    let e = *self.d.get(self.i).ok_or("escape troncato")?;
                    self.i += 1;
                    let c = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let a = self.esadecimale4()?;
                            if (0xD800..0xDC00).contains(&a) {
                                if self.d.get(self.i..self.i + 2) != Some(b"\\u") {
                                    return Err("surrogato alto senza basso".into());
                                }
                                self.i += 2;
                                let b = self.esadecimale4()?;
                                if !(0xDC00..0xE000).contains(&b) {
                                    return Err("surrogato basso non valido".into());
                                }
                                char::from_u32(0x10000 + ((a - 0xD800) << 10) + (b - 0xDC00))
                                    .ok_or("carattere non valido")?
                            } else {
                                char::from_u32(a).ok_or("surrogato basso isolato")?
                            }
                        }
                        _ => return Err("escape sconosciuto".into()),
                    };
                    let mut buf = [0u8; 4];
                    o.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                }
                Some(_) => return Err("carattere di controllo in una stringa".into()),
            }
        }
    }

    fn lista(&mut self, profondita: usize) -> Result<Valore, String> {
        self.atteso(b'[')?;
        let mut v = Vec::new();
        self.spazi();
        if self.d.get(self.i) == Some(&b']') {
            self.i += 1;
            return Ok(Valore::Lista(v));
        }
        loop {
            self.spazi();
            v.push(self.valore(profondita + 1)?);
            self.spazi();
            match self.d.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Valore::Lista(v));
                }
                _ => return Err(format!("atteso «,» o «]» alla posizione {}", self.i)),
            }
        }
    }

    fn oggetto(&mut self, profondita: usize) -> Result<Valore, String> {
        self.atteso(b'{')?;
        let mut v = Vec::new();
        self.spazi();
        if self.d.get(self.i) == Some(&b'}') {
            self.i += 1;
            return Ok(Valore::Oggetto(v));
        }
        loop {
            self.spazi();
            let k = self.stringa()?;
            self.spazi();
            self.atteso(b':')?;
            self.spazi();
            v.push((k, self.valore(profondita + 1)?));
            self.spazi();
            match self.d.get(self.i) {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Valore::Oggetto(v));
                }
                _ => return Err(format!("atteso «,» o «}}» alla posizione {}", self.i)),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valori_semplici() {
        assert_eq!(analizza("null").unwrap(), Valore::Null);
        assert_eq!(analizza(" true ").unwrap(), Valore::Bool(true));
        assert_eq!(analizza("-1.5e2").unwrap(), Valore::Numero(-150.0));
        assert_eq!(analizza("[1, 2]").unwrap(), Valore::Lista(vec![Valore::Numero(1.0), Valore::Numero(2.0)]));
    }

    #[test]
    fn oggetto_e_accessori() {
        let v = analizza(r#"{"id":3,"result":{"data":"abc"},"x":[]}"#).unwrap();
        assert_eq!(v.campo("id").and_then(Valore::numero), Some(3.0));
        assert_eq!(v.campo("result").and_then(|r| r.campo("data")).and_then(Valore::testo), Some("abc"));
        assert!(v.campo("manca").is_none());
    }

    #[test]
    fn escape_e_surrogati() {
        let v = analizza(r#""a\n\t\"\\\/\u00e8\ud83d\ude00""#).unwrap();
        assert_eq!(v, Valore::Testo("a\n\t\"\\/è😀".into()));
    }

    #[test]
    fn errori_senza_panic() {
        for t in [
            "", "{", "[1,", "\"abc", "\"\\ud83d\"", "\"\\ude00\"", "\"\\u12\"", "tru", "{\"a\"}", "[1 2]",
            "1 2", "\"\\q\"", "\"a\u{1}b\"", "-", "{\"a\":1,}", "nul", "\"\\", "{1:2}",
        ] {
            assert!(analizza(t).is_err(), "doveva fallire: {t:?}");
        }
    }

    #[test]
    fn profondita_limitata() {
        let ok = format!("{}{}", "[".repeat(60), "]".repeat(60));
        assert!(analizza(&ok).is_ok());
        let troppo = format!("{}{}", "[".repeat(100), "]".repeat(100));
        assert!(analizza(&troppo).is_err());
    }

    #[test]
    fn escape_in_scrittura() {
        assert_eq!(stringa("a\"b\\c\n\u{1}è"), "\"a\\\"b\\\\c\\n\\u0001è\"");
        assert_eq!(analizza(&stringa("x\"\\\n\u{7}")).unwrap(), Valore::Testo("x\"\\\n\u{7}".into()));
    }
}
