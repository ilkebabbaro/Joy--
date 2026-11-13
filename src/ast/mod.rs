//! Albero Sintattico Astratto (AST) di Joy++.
//!
//! Strutture dati pure prodotte da [`crate::parser`] e consumate da [`crate::compiler`].
//! Non contengono logica di I/O né stringhe "grezze" destinate all'HTML:
//! i colori, ad esempio, sono già convertiti in valori numerici.

/// Radice di un programma Joy++.
#[derive(Debug, Clone, PartialEq)]
pub struct Programma {
    /// Configurazione della scena (obbligatoria, prima frase del file).
    pub palco: Palco,
    /// Attori nell'ordine in cui sono stati creati.
    pub attori: Vec<Attore>,
    /// Blocchi temporali nell'ordine in cui compaiono nel file.
    pub timeline: Timeline,
}

/// Area di lavoro: il contenitore HTML principale.
#[derive(Debug, Clone, PartialEq)]
pub struct Palco {
    /// Larghezza in pixel.
    pub larghezza: f64,
    /// Altezza in pixel.
    pub altezza: f64,
    /// Colore di sfondo.
    pub sfondo: Colore,
    /// Fotogrammi al secondo dichiarati (informativo: il browser segue il refresh del display).
    pub fps: Option<u32>,
    /// Riga del sorgente.
    pub riga: usize,
}

/// Colore RGBA già validato.
///
/// Il testo scritto dall'utente non viene mai conservato: in output il colore
/// viene riemesso a partire da questi valori numerici.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colore {
    /// Rosso (0-255).
    pub r: u8,
    /// Verde (0-255).
    pub g: u8,
    /// Blu (0-255).
    pub b: u8,
    /// Opacità del colore (0.0-1.0).
    pub a: f64,
}

impl Colore {
    /// Colore opaco a partire dalle tre componenti.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Bianco opaco, colore predefinito degli attori.
    pub const BIANCO: Colore = Colore::rgb(255, 255, 255);
}

/// Un elemento visivo della scena.
#[derive(Debug, Clone, PartialEq)]
pub struct Attore {
    /// Nome univoco scritto tra virgolette (`chiamato "sole"`).
    pub nome: String,
    /// Forma geometrica o testo.
    pub forma: Forma,
    /// Proprietà visive comuni.
    pub stile: Stile,
    /// Posizione del **centro** dell'attore.
    pub posizione: Posizione,
    /// Riga del sorgente.
    pub riga: usize,
}

/// Tipi di attore supportati nella v0.1.
#[derive(Debug, Clone, PartialEq)]
pub enum Forma {
    /// `un cerchio` con `raggio [N]`.
    Cerchio {
        /// Raggio in pixel.
        raggio: f64,
    },
    /// `un rettangolo` (`larghezza`, `altezza`) o `un quadrato` (`lato`).
    Rettangolo {
        /// Larghezza in pixel.
        larghezza: f64,
        /// Altezza in pixel.
        altezza: f64,
        /// Raggio degli angoli in pixel (`arrotondamento`).
        arrotondamento: f64,
    },
    /// `un testo` o `un titolo`.
    Testo {
        /// Contenuto testuale (non ancora sanificato: lo fa il generatore).
        contenuto: String,
        /// Dimensione del carattere in pixel (`grandezza`).
        grandezza: f64,
        /// Nome del font (`font "..."`), validato dal compilatore.
        font: Option<String>,
        /// `true` per `peso grassetto`.
        grassetto: bool,
    },
}

/// Proprietà visive comuni a tutti gli attori.
#[derive(Debug, Clone, PartialEq)]
pub struct Stile {
    /// Colore di riempimento (o del testo).
    pub colore: Colore,
    /// Opacità iniziale (0.0-1.0).
    pub opacita: f64,
    /// Ordine di sovrapposizione (`z-index`).
    pub livello: i32,
    /// Bordo opzionale: spessore in pixel e colore.
    pub bordo: Option<(f64, Colore)>,
    /// Scala iniziale `s0` (`scala [N]`, SPECIFICA B.1): fattore ≥ 0, predefinito 1.0.
    /// È il valore del canale `scala` a `t = 0`.
    pub scala: f64,
    /// Rotazione iniziale `r0` in gradi (`rotazione [GRADI]`, SPECIFICA B.1), predefinita 0.0.
    /// Non viene normalizzata: è il valore del canale `rotazione` a `t = 0`.
    pub rotazione: f64,
}

impl Default for Stile {
    fn default() -> Self {
        Self {
            colore: Colore::BIANCO,
            opacita: 1.0,
            livello: 0,
            bordo: None,
            scala: 1.0,
            rotazione: 0.0,
        }
    }
}

/// Posizione del centro di un attore.
#[derive(Debug, Clone, PartialEq)]
pub enum Posizione {
    /// `a X, Y`: coordinate assolute in pixel.
    Assoluta {
        /// Ascissa.
        x: f64,
        /// Ordinata.
        y: f64,
    },
    /// `al centro`, `in alto a sinistra`, ...
    Palco(Ancora),
    /// `a destra di "sole" di 200`, `sopra a "x" di 50`, ...
    Relativa {
        /// Direzione dello spostamento rispetto al riferimento.
        direzione: Direzione,
        /// Nome dell'attore di riferimento.
        riferimento: String,
        /// Distanza tra i centri, in pixel.
        distanza: f64,
    },
}

/// Ancoraggi relativi al palco.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ancora {
    /// `al centro`
    Centro,
    /// `in alto`
    Alto,
    /// `in basso`
    Basso,
    /// `a sinistra`
    Sinistra,
    /// `a destra`
    Destra,
    /// `in alto a sinistra`
    AltoSinistra,
    /// `in alto a destra`
    AltoDestra,
    /// `in basso a sinistra`
    BassoSinistra,
    /// `in basso a destra`
    BassoDestra,
}

/// Direzioni cardinali (posizioni relative e verbo `scivola verso`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direzione {
    /// Verso l'alto (y decrescente).
    Alto,
    /// Verso il basso (y crescente).
    Basso,
    /// Verso sinistra (x decrescente).
    Sinistra,
    /// Verso destra (x crescente).
    Destra,
}

impl Direzione {
    /// Vettore unitario (dx, dy) nel sistema di coordinate dello schermo.
    pub fn vettore(self) -> (f64, f64) {
        match self {
            Direzione::Alto => (0.0, -1.0),
            Direzione::Basso => (0.0, 1.0),
            Direzione::Sinistra => (-1.0, 0.0),
            Direzione::Destra => (1.0, 0.0),
        }
    }
}

/// Sequenza dei blocchi temporali.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Timeline {
    /// Blocchi nell'ordine del sorgente.
    pub blocchi: Vec<Blocco>,
}

/// Un blocco temporale con le azioni al suo interno.
#[derive(Debug, Clone, PartialEq)]
pub struct Blocco {
    /// Quando si svolge il blocco.
    pub tempo: Tempo,
    /// Azioni del blocco (righe indentate).
    pub azioni: Vec<Azione>,
    /// Riga dell'intestazione.
    pub riga: usize,
}

/// Collocazione temporale di un blocco (in secondi).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tempo {
    /// `Dal secondo A al secondo B:` (e `Al secondo T:` con durata predefinita).
    Assoluto {
        /// Istante iniziale.
        inizio: f64,
        /// Istante finale.
        fine: f64,
    },
    /// `Poi per D secondi:` (attesa 0) o `Dopo A secondi per D secondi:`,
    /// relativo alla fine del blocco precedente.
    Relativo {
        /// Attesa dopo la fine del blocco precedente.
        attesa: f64,
        /// Durata del blocco.
        durata: f64,
    },
}

/// Una singola azione di un attore.
#[derive(Debug, Clone, PartialEq)]
pub struct Azione {
    /// Nome dell'attore che agisce.
    pub attore: String,
    /// Cosa fa.
    pub verbo: Verbo,
    /// Curva di movimento.
    pub easing: Easing,
    /// Riga del sorgente.
    pub riga: usize,
}

/// Verbi supportati nella v0.1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verbo {
    /// `appare sfumando`: l'opacità sale fino a quella dichiarata dall'attore.
    AppareSfumando,
    /// `scompare sfumando`: l'opacità scende a 0.
    ScompareSfumando,
    /// `cambia opacità a V`.
    CambiaOpacita(f64),
    /// `si sposta a X, Y`: il centro raggiunge le coordinate assolute.
    SiSpostaA {
        /// Ascissa di arrivo.
        x: f64,
        /// Ordinata di arrivo.
        y: f64,
    },
    /// `scivola verso DIREZIONE di N`.
    Scivola {
        /// Direzione.
        direzione: Direzione,
        /// Distanza in pixel.
        distanza: f64,
    },
    /// `ruota di N gradi` (relativo alla rotazione attuale).
    Ruota(f64),
    /// `si ingrandisce fino a F` / `si rimpicciolisce fino a F` / scala assoluta.
    ScalaFinoA(f64),
    /// `pulsa [con intensità I] [a ritmo R]`.
    Pulsa {
        /// Intensità (costanti SPECIFICA §6: 0.2, 0.5, 1.0, 2.0 o valore numerico).
        intensita: f64,
        /// Battiti al minuto.
        bpm: f64,
    },
}

/// Curve di movimento (SPECIFICA §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// `lineare`
    Lineare,
    /// `morbido` (predefinito)
    #[default]
    Morbido,
    /// `scatto`
    Scatto,
    /// `elastico`
    Elastico,
    /// `rimbalzo`
    Rimbalzo,
}
