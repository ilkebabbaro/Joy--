//! Decodifica base64 (RFC 4648, alfabeto standard) per i PNG di `Page.captureScreenshot`.

/// Valore a 6 bit di un carattere dell'alfabeto base64, `None` se non appartiene all'alfabeto.
fn valore(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some(u32::from(c - b'A')),
        b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// Decodifica una stringa base64 standard con padding `=` obbligatorio.
///
/// # Errori
/// Lunghezza non multipla di 4, caratteri fuori alfabeto, padding in posizione errata.
pub fn decodifica(testo: &str) -> Result<Vec<u8>, String> {
    let dati = testo.as_bytes();
    if !dati.len().is_multiple_of(4) {
        return Err("lunghezza base64 non valida".into());
    }
    let mut uscita = Vec::with_capacity(dati.len() / 4 * 3);
    let (blocchi, _) = dati.as_chunks::<4>();
    for (i, blocco) in blocchi.iter().enumerate() {
        let ultimo = i + 1 == blocchi.len();
        let riempimento = blocco.iter().rev().take_while(|&&c| c == b'=').count();
        if riempimento > 2 || (riempimento > 0 && !ultimo) {
            return Err("padding base64 non valido".into());
        }
        let mut acc: u32 = 0;
        for &c in &blocco[..4 - riempimento] {
            let v = valore(c).ok_or_else(|| "carattere base64 non valido".to_string())?;
            acc = (acc << 6) | v;
        }
        acc <<= 6 * riempimento as u32;
        let byte = [(acc >> 16) as u8, (acc >> 8) as u8, acc as u8];
        uscita.extend_from_slice(&byte[..3 - riempimento]);
    }
    Ok(uscita)
}

#[cfg(test)]
mod test {
    use super::decodifica;

    #[test]
    fn vettori_rfc4648() {
        assert_eq!(decodifica("").unwrap(), b"");
        assert_eq!(decodifica("Zg==").unwrap(), b"f");
        assert_eq!(decodifica("Zm8=").unwrap(), b"fo");
        assert_eq!(decodifica("Zm9v").unwrap(), b"foo");
        assert_eq!(decodifica("Zm9vYg==").unwrap(), b"foob");
        assert_eq!(decodifica("Zm9vYmE=").unwrap(), b"fooba");
        assert_eq!(decodifica("Zm9vYmFy").unwrap(), b"foobar");
    }

    #[test]
    fn alfabeto_completo() {
        assert_eq!(decodifica("+/+/").unwrap(), vec![0xfb, 0xff, 0xbf]);
    }

    #[test]
    fn input_non_valido() {
        assert!(decodifica("Zg=").is_err());
        assert!(decodifica("Zg==Zg==").is_err());
        assert!(decodifica("Z===").is_err());
        assert!(decodifica("Zm9!").is_err());
        assert!(decodifica("Zm 9").is_err());
        assert!(decodifica("Zm9-").is_err());
    }
}
