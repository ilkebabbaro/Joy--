//! Sanitizzazione dell'output e limiti anti-DoS.
//!
//! Regola d'oro: **nessuna stringa utente** arriva nell'HTML/CSS generato senza
//! passare da questo modulo. I colori non sono stringhe (vedi [`crate::ast::Colore`]);
//! gli identificatori HTML/CSS sono generati dal compilatore.

/// Limiti che impediscono di produrre file in grado di bloccare il browser
/// (o il compilatore stesso) a partire da input malevoli o malformati.
pub mod limiti {
    /// Dimensione massima del sorgente `.joy` (1 MiB).
    pub const MAX_SORGENTE_BYTE: u64 = 1024 * 1024;
    /// Lato massimo del palco in pixel.
    pub const MAX_LATO_PALCO: f64 = 16_384.0;
    /// Numero massimo di attori in una scena.
    pub const MAX_ATTORI: usize = 5_000;
    /// Dimensione massima (raggio, larghezza, spessore, grandezza testo...) in pixel.
    pub const MAX_DIMENSIONE: f64 = 100_000.0;
    /// Valore assoluto massimo di una coordinata o di uno spostamento in pixel.
    pub const MAX_COORDINATA: f64 = 1_000_000.0;
    /// Durata massima della scena in secondi (1 ora).
    pub const MAX_DURATA_SCENA: f64 = 3_600.0;
    /// Numero massimo di keyframe per singola animazione.
    pub const MAX_KEYFRAME: usize = 10_000;
    /// Lunghezza massima di un testo, in caratteri.
    pub const MAX_LUNGHEZZA_TESTO: usize = 10_000;
    /// Lunghezza massima del nome di un attore, in caratteri.
    pub const MAX_LUNGHEZZA_NOME: usize = 100;
    /// Lunghezza massima del nome di un font, in caratteri.
    pub const MAX_LUNGHEZZA_FONT: usize = 64;
    /// Fattore di scala massimo.
    pub const MAX_SCALA: f64 = 1_000.0;
    /// Valore assoluto massimo di una rotazione in gradi.
    pub const MAX_GRADI: f64 = 360_000.0;
    /// Ritmo massimo di una pulsazione (battiti al minuto).
    pub const MAX_BPM: f64 = 1_200.0;
    /// Intensità massima di una pulsazione.
    pub const MAX_INTENSITA: f64 = 20.0;
}

/// Esegue l'escape di una stringa per l'inserimento sicuro in HTML
/// (sia come contenuto di un elemento sia come valore di attributo tra virgolette).
///
/// Sostituisce `& < > " '` con le rispettive entità e rimuove i caratteri di controllo
/// (eccetto tabulazione e a capo).
pub fn escape_html(testo: &str) -> String {
    let mut out = String::with_capacity(testo.len());
    for c in testo.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() && c != '\t' && c != '\n' => {}
            c => out.push(c),
        }
    }
    out
}

/// Indica se `c` è un carattere che non deve mai entrare nei sorgenti né nei messaggi
/// mostrati sul terminale: i caratteri di controllo (tranne la tabulazione, quindi anche
/// `ESC`) e i caratteri Unicode invisibili che riordinano o falsano il testo mostrato
/// ("Trojan Source": `U+202A..U+202E`, `U+2066..U+2069`, `U+200E`, `U+200F`, `U+061C`)
/// o spezzano la riga (`U+2028`, `U+2029`). `char::is_control` da solo non li copre.
pub fn carattere_non_ammesso(c: char) -> bool {
    (c.is_control() && c != '\t')
        || matches!(
            c,
            '\u{061C}'
                | '\u{200E}'
                | '\u{200F}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202A}'..='\u{202E}'
                | '\u{2066}'..='\u{2069}'
        )
}

/// Verifica che un nome di font possa essere inserito in sicurezza in una
/// dichiarazione CSS `font-family: "..."`.
///
/// Sono ammessi solo lettere (anche accentate), cifre, spazi, `-` e `_`.
///
/// # Errori
/// Restituisce un messaggio in italiano che descrive il problema.
pub fn valida_nome_font(nome: &str) -> Result<(), String> {
    if nome.trim().is_empty() {
        return Err("il nome del font non può essere vuoto".into());
    }
    if nome.chars().count() > limiti::MAX_LUNGHEZZA_FONT {
        return Err(format!(
            "il nome del font è troppo lungo (massimo {} caratteri)",
            limiti::MAX_LUNGHEZZA_FONT
        ));
    }
    if let Some(c) = nome
        .chars()
        .find(|c| !(c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')))
    {
        return Err(format!(
            "il nome del font contiene il carattere non ammesso «{c}» (ammessi: lettere, cifre, spazi, - e _)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn escape_caratteri_pericolosi() {
        assert_eq!(
            escape_html("<script>alert('x' & \"y\")</script>"),
            "&lt;script&gt;alert(&#39;x&#39; &amp; &quot;y&quot;)&lt;/script&gt;"
        );
    }

    #[test]
    fn escape_rimuove_controlli() {
        assert_eq!(escape_html("a\u{0}b\u{1b}c"), "abc");
    }

    #[test]
    fn caratteri_non_ammessi() {
        for c in ['\u{1b}', '\0', '\u{85}', '\u{202e}', '\u{2066}', '\u{2069}', '\u{200f}', '\u{2028}'] {
            assert!(carattere_non_ammesso(c), "{c:?}");
        }
        for c in ['a', 'è', ' ', '\t', '«', '😀'] {
            assert!(!carattere_non_ammesso(c), "{c:?}");
        }
    }

    #[test]
    fn font_validi_e_non_validi() {
        assert!(valida_nome_font("Open Sans").is_ok());
        assert!(valida_nome_font("Noto_Serif-Display").is_ok());
        assert!(valida_nome_font("Arial\";}body{display:none").is_err());
        assert!(valida_nome_font("a<b").is_err());
        assert!(valida_nome_font("  ").is_err());
    }
}
