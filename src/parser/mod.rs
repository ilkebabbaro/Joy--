//! Parser di Joy++: dal sorgente `.joy` all'AST ([`crate::ast::Programma`]).
//!
//! Il parser è **orientato alle righe**: ogni riga non vuota (e non commento) è una
//! frase che termina con `.`, oppure un'intestazione di blocco temporale che termina con `:`.
//! Le righe indentate appartengono all'ultimo blocco temporale aperto.

pub mod lexer;

use crate::ast::{
    Ancora, Attore, Azione, Blocco, Colore, Direzione, Easing, Forma, Palco, Posizione, Programma,
    Stile, Tempo, Timeline, Verbo,
};
use crate::errore::{Errore, Risultato};
use lexer::{Token, tokenizza};

/// Durata predefinita di un blocco `Al secondo T:` (SPECIFICA §4).
pub const DURATA_ISTANTE: f64 = 0.3;

/// Battiti al minuto corrispondenti al ritmo `normalmente` (1.0x).
pub const BPM_BASE: f64 = 60.0;

/// Aggettivi di velocità/ritmo e relativi moltiplicatori (SPECIFICA §6).
const RITMI: &[(&str, f64)] = &[
    ("lentamente", 0.5),
    ("normalmente", 1.0),
    ("velocemente", 2.0),
    ("freneticamente", 4.0),
];

/// Aggettivi di intensità e relativi valori (SPECIFICA §6).
const INTENSITA: &[(&str, f64)] = &[
    ("leggermente", 0.2),
    ("moderatamente", 0.5),
    ("intensamente", 1.0),
    ("estremamente", 2.0),
];

/// Nomi di colore italiani riconosciuti (SPECIFICA §3).
const COLORI: &[(&str, Colore)] = &[
    ("bianco", Colore::rgb(255, 255, 255)),
    ("nero", Colore::rgb(0, 0, 0)),
    ("rosso", Colore::rgb(255, 0, 0)),
    ("blu", Colore::rgb(0, 0, 255)),
    ("verde", Colore::rgb(0, 128, 0)),
    ("giallo", Colore::rgb(255, 255, 0)),
    ("azzurro", Colore::rgb(0, 191, 255)),
    ("viola", Colore::rgb(128, 0, 128)),
    ("grigio", Colore::rgb(128, 128, 128)),
    ("trasparente", Colore { r: 0, g: 0, b: 0, a: 0.0 }),
];

const EASING: &[(&str, Easing)] = &[
    ("lineare", Easing::Lineare),
    ("morbido", Easing::Morbido),
    ("scatto", Easing::Scatto),
    ("elastico", Easing::Elastico),
    ("rimbalzo", Easing::Rimbalzo),
];

const VERBI_SUPPORTATI: &[&str] = &[
    "appare", "scompare", "si", "scivola", "ruota", "pulsa", "cambia",
];
const VERBI_FUTURI: &[&str] = &[
    "orbita", "trema", "fluttua", "lascia", "gira", "raggiunge", "passa", "esplode", "viene",
    "cade",
];
const PROPRIETA: &[&str] = &[
    "raggio", "larghezza", "altezza", "lato", "arrotondamento", "testo", "grandezza", "font",
    "peso", "colore", "opacita", "livello", "bordo",
];

/// Analizza un sorgente Joy++ completo e restituisce l'AST.
///
/// # Errori
/// Restituisce il primo [`Errore`] incontrato, con numero di riga e suggerimento.
pub fn analizza(sorgente: &str) -> Risultato<Programma> {
    let mut palco: Option<Palco> = None;
    let mut attori: Vec<Attore> = Vec::new();
    let mut blocchi: Vec<Blocco> = Vec::new();
    let mut blocco_aperto = false;

    for (indice, linea) in sorgente.lines().enumerate() {
        let riga = indice + 1;
        let linea = linea.trim_start_matches('\u{feff}');
        let contenuto = linea.trim();
        if contenuto.is_empty() || contenuto.starts_with('#') {
            continue;
        }
        let indentata = linea.starts_with([' ', '\t']);

        let mut token = tokenizza(contenuto, riga)?;
        let terminatore = token.pop();
        if token.is_empty() {
            return Err(Errore::alla_riga(riga, "frase vuota"));
        }
        if let Some(pos) = token.iter().position(|t| matches!(t, Token::Punto | Token::DuePunti)) {
            return Err(Errore::alla_riga(
                riga,
                format!("punteggiatura inattesa {} a metà frase", token[pos].descrivi()),
            )
            .con_suggerimento("scrivi una sola frase per riga"));
        }
        let mut frase = Frase::new(&token, riga);

        match terminatore {
            Some(Token::DuePunti) => {
                if indentata {
                    return Err(Errore::alla_riga(
                        riga,
                        "un'intestazione di blocco temporale non può essere indentata",
                    )
                    .con_suggerimento("togli gli spazi a inizio riga"));
                }
                richiedi_palco(&palco, riga)?;
                let tempo = frase.intestazione_blocco()?;
                blocchi.push(Blocco { tempo, azioni: Vec::new(), riga });
                blocco_aperto = true;
            }
            Some(Token::Punto) => {
                if indentata {
                    let blocco = match blocchi.last_mut() {
                        Some(b) if blocco_aperto => b,
                        _ => {
                            return Err(Errore::alla_riga(
                                riga,
                                "riga indentata fuori da un blocco temporale",
                            )
                            .con_suggerimento(
                                "apri prima un blocco, es. «Dal secondo 0 al secondo 2:»",
                            ));
                        }
                    };
                    blocco.azioni.extend(frase.riga_azioni()?);
                    continue;
                }
                blocco_aperto = false;
                match frase.peek() {
                    Some(Token::Parola(p)) if p == "prepara" => {
                        if palco.is_some() {
                            return Err(Errore::alla_riga(riga, "il palco è già stato preparato")
                                .con_suggerimento("usa una sola frase «Prepara un palco ...»"));
                        }
                        palco = Some(frase.palco()?);
                    }
                    Some(Token::Parola(p)) if p == "crea" => {
                        richiedi_palco(&palco, riga)?;
                        attori.push(frase.attore()?);
                    }
                    Some(Token::Stringa(_)) => {
                        return Err(Errore::alla_riga(
                            riga,
                            "le azioni devono stare dentro un blocco temporale",
                        )
                        .con_suggerimento(
                            "indenta la riga di 2 spazi sotto un'intestazione come «Dal secondo 0 al secondo 2:»",
                        ));
                    }
                    Some(t) => {
                        let mut e = Errore::alla_riga(
                            riga,
                            format!("non so come iniziare una frase con {}", t.descrivi()),
                        );
                        if let Token::Parola(p) = t
                            && let Some(s) = suggerisci(p, &["prepara", "crea"])
                        {
                            e = e.con_suggerimento(format!("forse intendevi «{s}»?"));
                        } else {
                            e = e.con_suggerimento(
                                "le frasi iniziano con «Prepara», «Crea», «Dal secondo», «Poi per»...",
                            );
                        }
                        return Err(e);
                    }
                    None => return Err(Errore::alla_riga(riga, "frase vuota")),
                }
            }
            _ => {
                return Err(Errore::alla_riga(riga, "la frase non termina con un punto")
                    .con_suggerimento(
                        "aggiungi «.» alla fine della frase (o «:» per aprire un blocco temporale)",
                    ));
            }
        }
    }

    let palco = palco.ok_or_else(|| {
        Errore::globale("manca la configurazione del palco")
            .con_suggerimento("inizia il file con «Prepara un palco grande 800 per 600 con sfondo nero.»")
    })?;

    Ok(Programma { palco, attori, timeline: Timeline { blocchi } })
}

fn richiedi_palco(palco: &Option<Palco>, riga: usize) -> Risultato<()> {
    if palco.is_none() {
        return Err(Errore::alla_riga(riga, "il palco deve essere preparato prima di tutto il resto")
            .con_suggerimento("metti «Prepara un palco grande 800 per 600 con sfondo nero.» come prima frase"));
    }
    Ok(())
}

/// Distanza di Levenshtein tra due stringhe (per i suggerimenti).
fn distanza(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut precedente: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut corrente = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let costo = usize::from(ca != *cb);
            corrente[j + 1] = (precedente[j] + costo)
                .min(precedente[j + 1] + 1)
                .min(corrente[j] + 1);
        }
        precedente = corrente;
    }
    precedente.last().copied().unwrap_or(usize::MAX)
}

/// Restituisce il candidato più simile a `parola`, se abbastanza vicino.
pub fn suggerisci<'a>(parola: &str, candidati: &[&'a str]) -> Option<&'a str> {
    let soglia = (parola.chars().count() / 3).clamp(1, 3);
    candidati
        .iter()
        .map(|c| (distanza(parola, c), *c))
        .filter(|(d, _)| *d <= soglia)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// Cursore sui token di una frase.
struct Frase<'a> {
    token: &'a [Token],
    pos: usize,
    riga: usize,
}

impl<'a> Frase<'a> {
    fn new(token: &'a [Token], riga: usize) -> Self {
        Self { token, pos: 0, riga }
    }

    fn peek(&self) -> Option<&'a Token> {
        self.token.get(self.pos)
    }

    fn peek_parola_a(&self, offset: usize) -> Option<&'a str> {
        match self.token.get(self.pos + offset) {
            Some(Token::Parola(p)) => Some(p.as_str()),
            _ => None,
        }
    }

    fn avanza(&mut self) -> Option<&'a Token> {
        let t = self.token.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn finita(&self) -> bool {
        self.pos >= self.token.len()
    }

    fn errore(&self, messaggio: impl Into<String>) -> Errore {
        Errore::alla_riga(self.riga, messaggio)
    }

    /// Descrive cosa c'è alla posizione corrente (per gli errori).
    fn trovato(&self) -> String {
        self.peek().map_or_else(|| "la fine della frase".to_string(), Token::descrivi)
    }

    /// Consuma la parola `p` se è la prossima.
    fn accetta(&mut self, p: &str) -> bool {
        if self.peek_parola_a(0) == Some(p) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consuma la prima delle parole `scelte` presente; restituisce quella trovata.
    fn accetta_una(&mut self, scelte: &[&'static str]) -> Option<&'static str> {
        let p = self.peek_parola_a(0)?;
        let trovata = scelte.iter().find(|s| **s == p).copied()?;
        self.pos += 1;
        Some(trovata)
    }

    /// Richiede la parola `p`, altrimenti errore che cita il contesto.
    fn richiedi(&mut self, p: &str, contesto: &str) -> Risultato<()> {
        if self.accetta(p) {
            return Ok(());
        }
        Err(self
            .errore(format!("mi aspettavo «{p}» {contesto}, ma ho trovato {}", self.trovato()))
            .con_suggerimento(format!("controlla la sintassi: {contesto}")))
    }

    fn numero(&mut self, cosa: &str) -> Risultato<f64> {
        match self.peek() {
            Some(Token::Numero(n)) => {
                self.pos += 1;
                Ok(*n)
            }
            _ => Err(self
                .errore(format!("mi aspettavo un numero per {cosa}, ma ho trovato {}", self.trovato()))),
        }
    }

    fn stringa(&mut self, cosa: &str) -> Risultato<String> {
        match self.peek() {
            Some(Token::Stringa(s)) => {
                self.pos += 1;
                Ok(s.clone())
            }
            _ => Err(self
                .errore(format!(
                    "mi aspettavo un testo tra virgolette per {cosa}, ma ho trovato {}",
                    self.trovato()
                ))
                .con_suggerimento("scrivi il testo tra virgolette doppie, es. \"sole\"")),
        }
    }

    fn virgola_opzionale(&mut self) {
        if matches!(self.peek(), Some(Token::Virgola)) {
            self.pos += 1;
        }
    }

    /// Errore se restano token non consumati.
    fn termina(&self) -> Risultato<()> {
        if self.finita() {
            Ok(())
        } else {
            Err(self.errore(format!("non capisco la parte finale della frase a partire da {}", self.trovato())))
        }
    }

    // ------------------------------------------------------------------ palco

    /// `Prepara un palco grande L per H [con sfondo COLORE] [a N fotogrammi].`
    fn palco(&mut self) -> Risultato<Palco> {
        self.richiedi("prepara", "all'inizio della frase")?;
        self.richiedi("palco", "dopo «Prepara un»")?;
        self.accetta("grande");
        let larghezza = self.numero("la larghezza del palco")?;
        self.richiedi("per", "tra larghezza e altezza (es. «800 per 600»)")?;
        let altezza = self.numero("l'altezza del palco")?;
        let mut sfondo = Colore::rgb(0, 0, 0);
        let mut fps = None;
        loop {
            if self.finita() {
                break;
            }
            if matches!(self.peek(), Some(Token::Virgola)) || self.accetta("e") {
                self.virgola_opzionale();
                continue;
            }
            if self.accetta("con") {
                self.richiedi("sfondo", "dopo «con» (es. «con sfondo nero»)")?;
                sfondo = self.colore()?;
            } else if self.accetta("a") {
                let n = self.numero("i fotogrammi al secondo")?;
                if !(1.0..=240.0).contains(&n) || n.fract() != 0.0 {
                    return Err(self.errore("i fotogrammi al secondo devono essere un intero tra 1 e 240"));
                }
                fps = Some(n as u32);
                self.accetta_una(&["fotogrammi", "fps"]);
            } else {
                return Err(self
                    .errore(format!("non capisco {} nella frase del palco", self.trovato()))
                    .con_suggerimento("es. «Prepara un palco grande 800 per 600 con sfondo nero a 60 fotogrammi.»"));
            }
        }
        Ok(Palco { larghezza, altezza, sfondo, fps, riga: self.riga })
    }

    // ----------------------------------------------------------------- colori

    /// Nome italiano, `#RGB`/`#RRGGBB`/`#RRGGBBAA` o `rgb(R, G, B)`.
    fn colore(&mut self) -> Risultato<Colore> {
        match self.avanza() {
            Some(Token::Esadecimale(h)) => self.esadecimale(h),
            Some(Token::Parola(p)) if p == "rgb" => self.rgb(),
            Some(Token::Parola(p)) => {
                if let Some((_, c)) = COLORI.iter().find(|(n, _)| n == p) {
                    return Ok(*c);
                }
                let nomi: Vec<&str> = COLORI.iter().map(|(n, _)| *n).collect();
                let mut e = self.errore(format!("colore sconosciuto «{p}»"));
                e = match suggerisci(p, &nomi) {
                    Some(s) => e.con_suggerimento(format!("forse intendevi «{s}»?")),
                    None => e.con_suggerimento(format!(
                        "usa un nome ({}), un codice come #FF0055 oppure rgb(255, 0, 85)",
                        nomi.join(", ")
                    )),
                };
                Err(e)
            }
            altro => Err(self.errore(format!(
                "mi aspettavo un colore, ma ho trovato {}",
                altro.map_or_else(|| "la fine della frase".into(), Token::descrivi)
            ))),
        }
    }

    fn esadecimale(&self, h: &str) -> Risultato<Colore> {
        let errore = || {
            self.errore(format!("codice colore non valido «#{h}»"))
                .con_suggerimento("usa 3, 4, 6 oppure 8 cifre esadecimali, es. #F59E0B")
        };
        if !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(errore());
        }
        let cifre: Vec<u8> = h
            .chars()
            .filter_map(|c| c.to_digit(16).and_then(|d| u8::try_from(d).ok()))
            .collect();
        let (r, g, b, a) = match cifre.as_slice() {
            [r, g, b] => (r * 17, g * 17, b * 17, 255),
            [r, g, b, a] => (r * 17, g * 17, b * 17, a * 17),
            [r1, r2, g1, g2, b1, b2] => (r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2, 255),
            [r1, r2, g1, g2, b1, b2, a1, a2] => (r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2, a1 * 16 + a2),
            _ => return Err(errore()),
        };
        Ok(Colore { r, g, b, a: f64::from(a) / 255.0 })
    }

    fn rgb(&mut self) -> Risultato<Colore> {
        let sintassi = "nel formato rgb(R, G, B)";
        if !matches!(self.avanza(), Some(Token::ParentesiAperta)) {
            return Err(self.errore(format!("mi aspettavo «(» {sintassi}")));
        }
        let mut componenti = [0u8; 3];
        for (i, comp) in componenti.iter_mut().enumerate() {
            if i > 0 && !matches!(self.avanza(), Some(Token::Virgola)) {
                return Err(self.errore(format!("mi aspettavo «,» {sintassi}")));
            }
            let n = self.numero("una componente rgb")?;
            if !(0.0..=255.0).contains(&n) || n.fract() != 0.0 {
                return Err(self.errore("le componenti rgb devono essere interi tra 0 e 255"));
            }
            *comp = n as u8;
        }
        if !matches!(self.avanza(), Some(Token::ParentesiChiusa)) {
            return Err(self.errore(format!("mi aspettavo «)» {sintassi}")));
        }
        let [r, g, b] = componenti;
        Ok(Colore::rgb(r, g, b))
    }

    // ----------------------------------------------------------------- attori

    /// `Crea TIPO chiamato "NOME" [con PROPRIETÀ] [POSIZIONE].`
    fn attore(&mut self) -> Risultato<Attore> {
        self.richiedi("crea", "all'inizio della frase")?;
        let tipo = match self.avanza() {
            Some(Token::Parola(p)) => p.as_str(),
            Some(Token::Numero(_)) if self.peek_parola_a(0) == Some("particelle") => {
                return Err(self.non_supportato("le particelle"));
            }
            altro => {
                return Err(self.errore(format!(
                    "mi aspettavo il tipo di attore dopo «Crea», ma ho trovato {}",
                    altro.map_or_else(|| "la fine della frase".into(), Token::descrivi)
                )));
            }
        };
        const TIPI: &[&str] = &["cerchio", "rettangolo", "quadrato", "testo", "titolo"];
        if !TIPI.contains(&tipo) {
            if ["linea", "immagine", "gruppo", "particelle"].contains(&tipo) {
                return Err(self.non_supportato(&format!("il tipo «{tipo}»")));
            }
            let mut e = self.errore(format!("tipo di attore sconosciuto «{tipo}»"));
            e = match suggerisci(tipo, TIPI) {
                Some(s) => e.con_suggerimento(format!("forse intendevi «{s}»?")),
                None => e.con_suggerimento(format!("tipi disponibili: {}", TIPI.join(", "))),
            };
            return Err(e);
        }
        if self.accetta_una(&["chiamato", "chiamata", "chiamati", "chiamate"]).is_none() {
            return Err(self
                .errore(format!("mi aspettavo «chiamato» dopo il tipo, ma ho trovato {}", self.trovato()))
                .con_suggerimento(format!("es. «Crea un {tipo} chiamato \"nome\" ...»")));
        }
        let nome = self.stringa("il nome dell'attore")?.trim().to_string();
        if nome.is_empty() {
            return Err(self.errore("il nome dell'attore non può essere vuoto"));
        }

        let mut p = Proprieta::default();
        let mut stile = Stile::default();
        let mut posizione: Option<Posizione> = None;

        while !self.finita() {
            if matches!(self.peek(), Some(Token::Virgola)) {
                self.pos += 1;
                continue;
            }
            if self.accetta_una(&["e", "ed", "con"]).is_some() {
                continue;
            }
            if matches!(self.peek_parola_a(0), Some("a" | "al" | "in" | "sopra" | "sotto")) {
                if posizione.is_some() {
                    return Err(self.errore("la posizione dell'attore è indicata due volte"));
                }
                posizione = Some(self.posizione()?);
                continue;
            }
            let parola = match self.avanza() {
                Some(Token::Parola(w)) => w.as_str(),
                _ => {
                    self.pos -= 1;
                    return Err(self.errore(format!("non capisco {} in questa frase", self.trovato())));
                }
            };
            match parola {
                "raggio" => p.raggio = Some(self.numero("il raggio")?),
                "larghezza" => p.larghezza = Some(self.numero("la larghezza")?),
                "altezza" => p.altezza = Some(self.numero("l'altezza")?),
                "lato" => p.lato = Some(self.numero("il lato")?),
                "arrotondamento" => p.arrotondamento = Some(self.numero("l'arrotondamento")?),
                "testo" => p.testo = Some(self.stringa("il testo")?),
                "grandezza" => p.grandezza = Some(self.numero("la grandezza del testo")?),
                "font" => p.font = Some(self.stringa("il nome del font")?),
                "peso" => {
                    p.grassetto = Some(match self.accetta_una(&["normale", "grassetto"]) {
                        Some("grassetto") => true,
                        Some(_) => false,
                        None => {
                            return Err(self
                                .errore(format!("peso non valido: {}", self.trovato()))
                                .con_suggerimento("usa «peso normale» oppure «peso grassetto»"));
                        }
                    });
                }
                "colore" => stile.colore = self.colore()?,
                "opacita" => stile.opacita = self.numero("l'opacità")?,
                "livello" => {
                    let n = self.numero("il livello")?;
                    if n.fract() != 0.0 || n.abs() > 1_000_000.0 {
                        return Err(self.errore("il livello deve essere un numero intero"));
                    }
                    stile.livello = n as i32;
                }
                "bordo" => {
                    let spessore = self.numero("lo spessore del bordo")?;
                    stile.bordo = Some((spessore, self.colore()?));
                }
                "ombra" | "sfocatura" => {
                    return Err(self.non_supportato(&format!("la proprietà «{parola}»")));
                }
                altro => {
                    let mut e = self.errore(format!("proprietà sconosciuta «{altro}»"));
                    if let Some(s) = suggerisci(altro, PROPRIETA) {
                        e = e.con_suggerimento(format!("forse intendevi «{s}»?"));
                    }
                    return Err(e);
                }
            }
        }

        let forma = p.in_forma(tipo, self)?;
        Ok(Attore {
            nome,
            forma,
            stile,
            posizione: posizione.unwrap_or(Posizione::Palco(Ancora::Centro)),
            riga: self.riga,
        })
    }

    fn non_supportato(&self, cosa: &str) -> Errore {
        self.errore(format!("{cosa} non è ancora supportato nella versione 0.1"))
            .con_suggerimento("consulta CHANGELOG.md per l'elenco delle funzioni disponibili")
    }

    /// Posizioni: `a X, Y`, ancore del palco, relative a un altro attore.
    fn posizione(&mut self) -> Risultato<Posizione> {
        match self.avanza() {
            Some(Token::Parola(p)) if p == "a" => {
                if matches!(self.peek(), Some(Token::Numero(_))) {
                    let x = self.numero("la coordinata X")?;
                    self.virgola_opzionale();
                    let y = self.numero("la coordinata Y")?;
                    return Ok(Posizione::Assoluta { x, y });
                }
                let direzione = match self.accetta_una(&["destra", "sinistra"]) {
                    Some("destra") => Direzione::Destra,
                    Some(_) => Direzione::Sinistra,
                    None => {
                        return Err(self
                            .errore(format!("posizione non valida dopo «a»: {}", self.trovato()))
                            .con_suggerimento("usa «a 100, 200», «a destra», «a sinistra» o «a destra di \"nome\" di 50»"));
                    }
                };
                if self.accetta("di") {
                    return self.relativa(direzione);
                }
                Ok(Posizione::Palco(match direzione {
                    Direzione::Destra => Ancora::Destra,
                    _ => Ancora::Sinistra,
                }))
            }
            Some(Token::Parola(p)) if p == "al" => {
                self.richiedi("centro", "dopo «al» (es. «al centro»)")?;
                Ok(Posizione::Palco(Ancora::Centro))
            }
            Some(Token::Parola(p)) if p == "in" => {
                let verticale = self.accetta_una(&["alto", "basso", "centro"]).ok_or_else(|| {
                    self.errore(format!("posizione non valida dopo «in»: {}", self.trovato()))
                        .con_suggerimento("usa «in alto», «in basso», «in alto a sinistra», ...")
                })?;
                if verticale == "centro" {
                    return Ok(Posizione::Palco(Ancora::Centro));
                }
                let orizzontale = if self.peek_parola_a(0) == Some("a")
                    && matches!(self.peek_parola_a(1), Some("sinistra" | "destra"))
                {
                    self.pos += 1;
                    self.accetta_una(&["sinistra", "destra"])
                } else {
                    None
                };
                Ok(Posizione::Palco(match (verticale, orizzontale) {
                    ("alto", Some("sinistra")) => Ancora::AltoSinistra,
                    ("alto", Some(_)) => Ancora::AltoDestra,
                    ("alto", None) => Ancora::Alto,
                    (_, Some("sinistra")) => Ancora::BassoSinistra,
                    (_, Some(_)) => Ancora::BassoDestra,
                    (_, None) => Ancora::Basso,
                }))
            }
            Some(Token::Parola(p)) if p == "sopra" || p == "sotto" => {
                let direzione = if p == "sopra" { Direzione::Alto } else { Direzione::Basso };
                self.accetta_una(&["a", "di"]);
                self.relativa(direzione)
            }
            _ => Err(self.errore("posizione non valida")),
        }
    }

    /// Parte finale di una posizione relativa: `"ALTRO" di N`.
    fn relativa(&mut self, direzione: Direzione) -> Risultato<Posizione> {
        let riferimento = self.stringa("l'attore di riferimento")?;
        self.richiedi("di", "prima della distanza (es. «a destra di \"sole\" di 200»)")?;
        let distanza = self.numero("la distanza")?;
        Ok(Posizione::Relativa { direzione, riferimento, distanza })
    }

    // --------------------------------------------------------------- timeline

    /// Intestazioni di blocco temporale (senza i due punti finali).
    fn intestazione_blocco(&mut self) -> Risultato<Tempo> {
        let tempo = match self.peek_parola_a(0) {
            Some("dal") => {
                self.pos += 1;
                self.accetta_una(&["secondo", "secondi"]);
                let inizio = self.numero("l'istante iniziale")?;
                self.richiedi("al", "tra i due istanti (es. «Dal secondo 0 al secondo 2:»)")?;
                self.accetta_una(&["secondo", "secondi"]);
                let fine = self.numero("l'istante finale")?;
                if inizio < 0.0 {
                    return Err(self.errore("il tempo non può essere negativo"));
                }
                if fine <= inizio {
                    return Err(self
                        .errore(format!("il blocco finisce (secondo {fine}) prima di iniziare (secondo {inizio})"))
                        .con_suggerimento("il secondo finale deve essere maggiore di quello iniziale"));
                }
                Tempo::Assoluto { inizio, fine }
            }
            Some("al") => {
                self.pos += 1;
                self.accetta_una(&["secondo", "secondi"]);
                let inizio = self.numero("l'istante")?;
                if inizio < 0.0 {
                    return Err(self.errore("il tempo non può essere negativo"));
                }
                Tempo::Assoluto { inizio, fine: inizio + DURATA_ISTANTE }
            }
            Some("poi") => {
                self.pos += 1;
                self.richiedi("per", "dopo «Poi» (es. «Poi per 2 secondi:»)")?;
                let durata = self.durata("la durata")?;
                Tempo::Relativo { attesa: 0.0, durata }
            }
            Some("dopo") => {
                self.pos += 1;
                let attesa = self.numero("l'attesa")?;
                if attesa < 0.0 {
                    return Err(self.errore("l'attesa non può essere negativa"));
                }
                self.accetta_una(&["secondi", "secondo"]);
                self.richiedi("per", "dopo l'attesa (es. «Dopo 1 secondi per 2 secondi:»)")?;
                let durata = self.durata("la durata")?;
                Tempo::Relativo { attesa, durata }
            }
            Some("per") if self.peek_parola_a(1) == Some("tutta") => {
                return Err(self.non_supportato("il blocco «Per tutta la durata»"));
            }
            Some("in") if self.peek_parola_a(1) == Some("loop") => {
                return Err(self.non_supportato("il blocco «In loop continuo»"));
            }
            _ => {
                return Err(self
                    .errore(format!("intestazione di blocco non riconosciuta: inizia con {}", self.trovato()))
                    .con_suggerimento("usa «Dal secondo 0 al secondo 2:», «Al secondo 3:», «Poi per 2 secondi:» o «Dopo 1 secondi per 2 secondi:»"));
            }
        };
        self.termina()?;
        Ok(tempo)
    }

    fn durata(&mut self, cosa: &str) -> Risultato<f64> {
        let d = self.numero(cosa)?;
        self.accetta_una(&["secondi", "secondo"]);
        if d <= 0.0 {
            return Err(self.errore("la durata di un blocco deve essere maggiore di zero"));
        }
        Ok(d)
    }

    /// `"NOME" VERBO ... [con movimento E] [e VERBO ...].`
    fn riga_azioni(&mut self) -> Risultato<Vec<Azione>> {
        let attore = match self.avanza() {
            Some(Token::Stringa(s)) => s.trim().to_string(),
            altro => {
                return Err(self
                    .errore(format!(
                        "un'azione deve iniziare con il nome dell'attore tra virgolette, ma ho trovato {}",
                        altro.map_or_else(|| "la fine della frase".into(), Token::descrivi)
                    ))
                    .con_suggerimento("es. «\"sole\" appare sfumando.»"));
            }
        };
        let mut clausole: Vec<(Verbo, Option<Easing>)> = Vec::new();
        loop {
            let verbo = self.verbo()?;
            let easing = self.easing_opzionale()?;
            clausole.push((verbo, easing));
            if self.finita() {
                break;
            }
            let separatore = matches!(self.peek(), Some(Token::Virgola));
            if separatore {
                self.pos += 1;
            }
            if self.accetta_una(&["e", "ed"]).is_none() && !separatore {
                return Err(self
                    .errore(format!("non capisco {} dopo l'azione", self.trovato()))
                    .con_suggerimento("collega più azioni con «e», es. «appare sfumando e ruota di 90 gradi»"));
            }
        }
        // Un easing finale vale anche per le azioni precedenti che non ne hanno uno.
        let finale = clausole.last().and_then(|(_, e)| *e).unwrap_or_default();
        Ok(clausole
            .into_iter()
            .map(|(verbo, easing)| Azione {
                attore: attore.clone(),
                verbo,
                easing: easing.unwrap_or(finale),
                riga: self.riga,
            })
            .collect())
    }

    fn easing_opzionale(&mut self) -> Risultato<Option<Easing>> {
        if !(self.peek_parola_a(0) == Some("con") && self.peek_parola_a(1) == Some("movimento")) {
            return Ok(None);
        }
        self.pos += 2;
        let nome = self.peek_parola_a(0).unwrap_or("");
        match EASING.iter().find(|(n, _)| *n == nome) {
            Some((_, e)) => {
                self.pos += 1;
                Ok(Some(*e))
            }
            None => {
                let nomi: Vec<&str> = EASING.iter().map(|(n, _)| *n).collect();
                let e = self.errore(format!("movimento sconosciuto {}", self.trovato()));
                Err(match suggerisci(nome, &nomi) {
                    Some(s) => e.con_suggerimento(format!("forse intendevi «{s}»?")),
                    None => e.con_suggerimento(format!("movimenti disponibili: {}", nomi.join(", "))),
                })
            }
        }
    }

    fn verbo(&mut self) -> Risultato<Verbo> {
        let parola = match self.avanza() {
            Some(Token::Parola(p)) => p.as_str(),
            altro => {
                return Err(self.errore(format!(
                    "mi aspettavo un verbo, ma ho trovato {}",
                    altro.map_or_else(|| "la fine della frase".into(), Token::descrivi)
                )));
            }
        };
        match parola {
            "appare" => {
                self.richiedi("sfumando", "dopo «appare»")?;
                Ok(Verbo::AppareSfumando)
            }
            "scompare" => {
                self.richiedi("sfumando", "dopo «scompare»")?;
                Ok(Verbo::ScompareSfumando)
            }
            "si" => match self.accetta_una(&["sposta", "ingrandisce", "rimpicciolisce", "disperde"]) {
                Some("sposta") => {
                    self.richiedi("a", "dopo «si sposta» (es. «si sposta a 100, 200»)")?;
                    let x = self.numero("la coordinata X")?;
                    self.virgola_opzionale();
                    let y = self.numero("la coordinata Y")?;
                    Ok(Verbo::SiSpostaA { x, y })
                }
                Some(v @ ("ingrandisce" | "rimpicciolisce")) => {
                    self.richiedi("fino", &format!("dopo «si {v}» (es. «si {v} fino a 1.5»)"))?;
                    self.richiedi("a", "dopo «fino»")?;
                    let fattore = self.numero("il fattore di scala")?;
                    if fattore < 0.0 {
                        return Err(self.errore("il fattore di scala non può essere negativo"));
                    }
                    Ok(Verbo::ScalaFinoA(fattore))
                }
                Some(_) => Err(self.non_supportato("il verbo «si disperde»")),
                None => Err(self
                    .errore(format!("verbo incompleto dopo «si»: {}", self.trovato()))
                    .con_suggerimento("usa «si sposta a», «si ingrandisce fino a» o «si rimpicciolisce fino a»")),
            },
            "scivola" => {
                self.richiedi("verso", "dopo «scivola» (es. «scivola verso basso di 40»)")?;
                let direzione = match self.accetta_una(&["alto", "basso", "destra", "sinistra"]) {
                    Some("alto") => Direzione::Alto,
                    Some("basso") => Direzione::Basso,
                    Some("destra") => Direzione::Destra,
                    Some(_) => Direzione::Sinistra,
                    None => {
                        return Err(self
                            .errore(format!("direzione non valida: {}", self.trovato()))
                            .con_suggerimento("usa alto, basso, destra o sinistra"));
                    }
                };
                self.richiedi("di", "prima della distanza (es. «verso basso di 40»)")?;
                let distanza = self.numero("la distanza")?;
                Ok(Verbo::Scivola { direzione, distanza })
            }
            "ruota" => {
                self.richiedi("di", "dopo «ruota» (es. «ruota di 90 gradi»)")?;
                let gradi = self.numero("i gradi di rotazione")?;
                self.accetta_una(&["gradi", "grado"]);
                Ok(Verbo::Ruota(gradi))
            }
            "pulsa" => self.pulsa(),
            "cambia" => match self.accetta_una(&["opacita", "colore"]) {
                Some("opacita") => {
                    self.richiedi("a", "dopo «cambia opacità» (es. «cambia opacità a 0.5»)")?;
                    Ok(Verbo::CambiaOpacita(self.numero("l'opacità")?))
                }
                Some(_) => Err(self.non_supportato("il verbo «cambia colore»")),
                None => Err(self
                    .errore(format!("non so cosa cambiare: {}", self.trovato()))
                    .con_suggerimento("usa «cambia opacità a 0.5»")),
            },
            altro if VERBI_FUTURI.contains(&altro) => {
                Err(self.non_supportato(&format!("il verbo «{altro}»")))
            }
            altro => {
                let e = self.errore(format!("verbo sconosciuto «{altro}»"));
                let mut candidati: Vec<&str> = VERBI_SUPPORTATI.to_vec();
                candidati.extend(["sposta", "ingrandisce", "rimpicciolisce"]);
                Err(match suggerisci(altro, &candidati) {
                    Some(s) => e.con_suggerimento(format!("forse intendevi «{s}»?")),
                    None => e.con_suggerimento(
                        "verbi disponibili: appare sfumando, scompare sfumando, si sposta a, scivola verso, ruota di, si ingrandisce, si rimpicciolisce, pulsa, cambia opacità",
                    ),
                })
            }
        }
    }

    /// `pulsa [con intensità I | I] [a ritmo R]`
    fn pulsa(&mut self) -> Risultato<Verbo> {
        let mut intensita = 0.5;
        let mut bpm = BPM_BASE;
        if self.peek_parola_a(0) == Some("con") && self.peek_parola_a(1) == Some("intensita") {
            self.pos += 2;
            intensita = self.valore_intensita()?;
        } else if matches!(self.peek(), Some(Token::Numero(_)))
            || self.peek_parola_a(0).is_some_and(|p| INTENSITA.iter().any(|(n, _)| *n == p))
        {
            intensita = self.valore_intensita()?;
        }
        if self.peek_parola_a(0) == Some("a") && self.peek_parola_a(1) == Some("ritmo") {
            self.pos += 2;
            bpm = match self.avanza() {
                Some(Token::Numero(n)) => {
                    self.accetta("bpm");
                    *n
                }
                Some(Token::Parola(p)) => match RITMI.iter().find(|(n, _)| n == p) {
                    Some((_, moltiplicatore)) => BPM_BASE * moltiplicatore,
                    None => {
                        return Err(self
                            .errore(format!("ritmo sconosciuto «{p}»"))
                            .con_suggerimento("usa lentamente, normalmente, velocemente, freneticamente o un numero di BPM"));
                    }
                },
                _ => return Err(self.errore("mi aspettavo un ritmo dopo «a ritmo»")),
            };
        }
        if intensita < 0.0 {
            return Err(self.errore("l'intensità non può essere negativa"));
        }
        Ok(Verbo::Pulsa { intensita, bpm })
    }

    fn valore_intensita(&mut self) -> Risultato<f64> {
        match self.avanza() {
            Some(Token::Numero(n)) => Ok(*n),
            Some(Token::Parola(p)) => INTENSITA
                .iter()
                .find(|(n, _)| n == p)
                .map(|(_, v)| *v)
                .ok_or_else(|| {
                    self.errore(format!("intensità sconosciuta «{p}»"))
                        .con_suggerimento("usa leggermente, moderatamente, intensamente, estremamente o un numero")
                }),
            _ => Err(self.errore("mi aspettavo un'intensità")),
        }
    }
}

/// Valori grezzi delle proprietà di un attore, prima di sapere a quale forma appartengono.
#[derive(Default)]
struct Proprieta {
    raggio: Option<f64>,
    larghezza: Option<f64>,
    altezza: Option<f64>,
    lato: Option<f64>,
    arrotondamento: Option<f64>,
    testo: Option<String>,
    grandezza: Option<f64>,
    font: Option<String>,
    grassetto: Option<bool>,
}

impl Proprieta {
    /// Costruisce la [`Forma`] verificando che le proprietà siano coerenti con il tipo.
    fn in_forma(self, tipo: &str, frase: &Frase<'_>) -> Risultato<Forma> {
        let estranea = |nome: &str| {
            frase.errore(format!("la proprietà «{nome}» non si applica a un {tipo}"))
        };
        let geometriche = [
            ("raggio", self.raggio.is_some()),
            ("larghezza", self.larghezza.is_some()),
            ("altezza", self.altezza.is_some()),
            ("lato", self.lato.is_some()),
            ("arrotondamento", self.arrotondamento.is_some()),
        ];
        let testuali = [
            ("testo", self.testo.is_some()),
            ("grandezza", self.grandezza.is_some()),
            ("font", self.font.is_some()),
            ("peso", self.grassetto.is_some()),
        ];
        let controlla = |ammesse: &[&str]| -> Risultato<()> {
            for (nome, presente) in geometriche.iter().chain(testuali.iter()) {
                if *presente && !ammesse.contains(nome) {
                    return Err(estranea(nome));
                }
            }
            Ok(())
        };
        match tipo {
            "cerchio" => {
                controlla(&["raggio"])?;
                Ok(Forma::Cerchio { raggio: self.raggio.unwrap_or(50.0) })
            }
            "quadrato" => {
                controlla(&["lato", "arrotondamento"])?;
                let lato = self.lato.unwrap_or(100.0);
                Ok(Forma::Rettangolo {
                    larghezza: lato,
                    altezza: lato,
                    arrotondamento: self.arrotondamento.unwrap_or(0.0),
                })
            }
            "rettangolo" => {
                controlla(&["larghezza", "altezza", "lato", "arrotondamento"])?;
                Ok(Forma::Rettangolo {
                    larghezza: self.larghezza.or(self.lato).unwrap_or(160.0),
                    altezza: self.altezza.or(self.lato).unwrap_or(100.0),
                    arrotondamento: self.arrotondamento.unwrap_or(0.0),
                })
            }
            _ => {
                controlla(&["testo", "grandezza", "font", "peso"])?;
                let titolo = tipo == "titolo";
                let contenuto = self.testo.ok_or_else(|| {
                    frase
                        .errore(format!("un {tipo} ha bisogno del suo testo"))
                        .con_suggerimento(format!("aggiungi ad esempio «con testo \"Ciao\"» dopo il nome del {tipo}"))
                })?;
                Ok(Forma::Testo {
                    contenuto,
                    grandezza: self.grandezza.unwrap_or(if titolo { 48.0 } else { 24.0 }),
                    font: self.font,
                    grassetto: self.grassetto.unwrap_or(titolo),
                })
            }
        }
    }
}
