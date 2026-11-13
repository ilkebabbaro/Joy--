//! Analisi lessicale di una singola riga Joy++.
//!
//! Il lexer produce token già normalizzati:
//! - le parole sono in minuscolo e senza accenti (`opacità` → `opacita`);
//! - le elisioni sono separate (`un'immagine` → `un`, `immagine`);
//! - le **parole di cortesia** (articoli) vengono scartate.

use crate::errore::{Errore, Risultato};

/// Un token di una riga Joy++.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Parola normalizzata (minuscolo, senza accenti).
    Parola(String),
    /// Numero (intero o decimale con il punto, eventualmente negativo).
    Numero(f64),
    /// Testo tra virgolette (dritte `"` o tipografiche `“ ”`), senza le virgolette.
    Stringa(String),
    /// Codice esadecimale di colore, senza il `#` (non ancora validato).
    Esadecimale(String),
    /// `,`
    Virgola,
    /// `:`
    DuePunti,
    /// `.` di fine frase.
    Punto,
    /// `(`
    ParentesiAperta,
    /// `)`
    ParentesiChiusa,
}

impl Token {
    /// Rappresentazione leggibile del token, per i messaggi d'errore.
    pub fn descrivi(&self) -> String {
        match self {
            Token::Parola(p) => format!("«{p}»"),
            Token::Numero(n) => format!("«{n}»"),
            Token::Stringa(s) => format!("«\"{s}\"»"),
            Token::Esadecimale(h) => format!("«#{h}»"),
            Token::Virgola => "«,»".into(),
            Token::DuePunti => "«:»".into(),
            Token::Punto => "«.»".into(),
            Token::ParentesiAperta => "«(»".into(),
            Token::ParentesiChiusa => "«)»".into(),
        }
    }
}

/// Articoli scartati dal parser (SPECIFICA §1, "parole di cortesia").
///
/// Le congiunzioni `e`/`ed` **non** sono scartate qui perché separano proprietà e azioni:
/// è il parser a trattarle come separatori.
const ARTICOLI: &[&str] = &["il", "lo", "la", "i", "gli", "le", "un", "una", "uno", "l"];

/// Rimuove gli accenti dalle vocali italiane più comuni.
fn normalizza_carattere(c: char) -> char {
    match c {
        'à' | 'á' => 'a',
        'è' | 'é' => 'e',
        'ì' | 'í' => 'i',
        'ò' | 'ó' => 'o',
        'ù' | 'ú' => 'u',
        altro => altro,
    }
}

fn e_apostrofo(c: char) -> bool {
    matches!(c, '\'' | '’' | '‘')
}

/// Scompone una riga (già privata dell'indentazione) in token.
///
/// `riga` è il numero di riga usato nei messaggi d'errore.
pub fn tokenizza(testo: &str, riga: usize) -> Risultato<Vec<Token>> {
    // Sicurezza: i caratteri di controllo (es. ESC) verrebbero ristampati nei messaggi
    // d'errore e potrebbero manipolare il terminale. Si rifiutano subito, mostrandone il codice.
    if let Some(c) = testo.chars().find(|c| c.is_control() && *c != '\t') {
        return Err(Errore::alla_riga(
            riga,
            format!("carattere di controllo non ammesso ({})", c.escape_unicode()),
        )
        .con_suggerimento("rimuovi il carattere invisibile dalla riga"));
    }
    let caratteri: Vec<char> = testo.chars().collect();
    let mut token = Vec::new();
    let mut i = 0;

    while let Some(&c) = caratteri.get(i) {
        if c.is_whitespace() || e_apostrofo(c) {
            i += 1;
            continue;
        }

        // Stringhe tra virgolette.
        if matches!(c, '"' | '“' | '”') {
            let inizio = i + 1;
            let mut fine = inizio;
            while let Some(&d) = caratteri.get(fine) {
                if matches!(d, '"' | '“' | '”') {
                    break;
                }
                fine += 1;
            }
            if fine >= caratteri.len() {
                return Err(Errore::alla_riga(riga, "virgolette non chiuse")
                    .con_suggerimento("chiudi il testo con un'altra virgoletta doppia, es. \"sole\""));
            }
            token.push(Token::Stringa(caratteri[inizio..fine].iter().collect()));
            i = fine + 1;
            continue;
        }

        // Numeri (eventualmente negativi): il punto è decimale solo se seguito da una cifra.
        let segno_meno = c == '-' && caratteri.get(i + 1).is_some_and(|d| d.is_ascii_digit());
        if c.is_ascii_digit() || segno_meno {
            let inizio = i;
            i += 1;
            while caratteri.get(i).is_some_and(|d| d.is_ascii_digit()) {
                i += 1;
            }
            if caratteri.get(i) == Some(&'.') && caratteri.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
                i += 1;
                while caratteri.get(i).is_some_and(|d| d.is_ascii_digit()) {
                    i += 1;
                }
            }
            let letterale: String = caratteri[inizio..i].iter().collect();
            let valore: f64 = letterale.parse().map_err(|_| {
                Errore::alla_riga(riga, format!("numero non valido «{letterale}»"))
            })?;
            if !valore.is_finite() {
                return Err(Errore::alla_riga(riga, format!("numero troppo grande «{letterale}»")));
            }
            token.push(Token::Numero(valore));
            continue;
        }

        // Codici colore esadecimali.
        if c == '#' {
            let inizio = i + 1;
            i = inizio;
            while caratteri.get(i).is_some_and(|d| d.is_ascii_alphanumeric()) {
                i += 1;
            }
            token.push(Token::Esadecimale(caratteri[inizio..i].iter().collect()));
            continue;
        }

        // Parole.
        if c.is_alphabetic() {
            let mut parola = String::new();
            while let Some(&d) = caratteri.get(i) {
                if !d.is_alphabetic() {
                    break;
                }
                parola.extend(d.to_lowercase().map(normalizza_carattere));
                i += 1;
            }
            if !ARTICOLI.contains(&parola.as_str()) {
                token.push(Token::Parola(parola));
            }
            continue;
        }

        let semplice = match c {
            ',' => Some(Token::Virgola),
            ':' => Some(Token::DuePunti),
            '.' => Some(Token::Punto),
            '(' => Some(Token::ParentesiAperta),
            ')' => Some(Token::ParentesiChiusa),
            _ => None,
        };
        match semplice {
            Some(t) => {
                token.push(t);
                i += 1;
            }
            None => {
                return Err(Errore::alla_riga(riga, format!("carattere non riconosciuto «{c}»"))
                    .con_suggerimento("i testi con simboli speciali vanno scritti tra virgolette doppie"));
            }
        }
    }

    Ok(token)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn scarta_articoli_e_normalizza_accenti() {
        let t = tokenizza("Crea un cerchio con opacità 0.5", 1).unwrap();
        assert_eq!(
            t,
            vec![
                Token::Parola("crea".into()),
                Token::Parola("cerchio".into()),
                Token::Parola("con".into()),
                Token::Parola("opacita".into()),
                Token::Numero(0.5),
            ]
        );
    }

    #[test]
    fn separa_elisioni() {
        let t = tokenizza("un'immagine", 1).unwrap();
        assert_eq!(t, vec![Token::Parola("immagine".into())]);
    }

    #[test]
    fn punto_finale_non_e_decimale() {
        let t = tokenizza("ruota di 90.", 1).unwrap();
        assert_eq!(t.last(), Some(&Token::Punto));
        assert!(t.contains(&Token::Numero(90.0)));
    }

    #[test]
    fn numeri_negativi_e_stringhe() {
        let t = tokenizza("\"sole\" ruota di -90", 1).unwrap();
        assert_eq!(t[0], Token::Stringa("sole".into()));
        assert_eq!(t.last(), Some(&Token::Numero(-90.0)));
    }

    #[test]
    fn virgolette_non_chiuse() {
        let e = tokenizza("\"sole", 7).unwrap_err();
        assert_eq!(e.riga, Some(7));
    }
}
