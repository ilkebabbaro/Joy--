# Changelog

Tutte le modifiche rilevanti a questo progetto sono documentate in questo file.
Il formato segue [Keep a Changelog](https://keepachangelog.com/it/1.1.0/) e il
progetto aderisce al [Semantic Versioning](https://semver.org/lang/it/).

## [Unreleased]

### Aggiunto
- Sicurezza (`src/security`): `escape_html`, `valida_nome_font` e limiti anti-DoS
  (dimensione sorgente, palco, attori, durata, keyframe, testi).
- Parser (`src/parser`): lexer con normalizzazione di accenti ed elisioni e rimozione
  degli articoli; frasi `Prepara un palco...`, `Crea [cerchio|rettangolo|quadrato|testo|titolo]...`
  con proprietà (`raggio`, `larghezza`, `altezza`, `lato`, `arrotondamento`, `testo`, `grandezza`,
  `font`, `peso`, `colore`, `opacità`, `livello`, `bordo`) e posizioni (assolute, ancore del palco,
  relative a un altro attore); blocchi `Dal secondo...`, `Al secondo...`, `Poi per...`, `Dopo...`;
  verbi `appare sfumando`, `scompare sfumando`, `cambia opacità a`, `si sposta a`, `scivola verso`,
  `ruota di`, `si ingrandisce/rimpicciolisce fino a`, `pulsa`; modificatore `con movimento`.
- Errori in italiano con numero di riga e suggerimenti ("forse intendevi «appare»?").
- `SPECIFICA.md`: Appendice A con i valori predefiniti della v0.1.
- AST (`src/ast`): `Programma`, `Palco`, `Attore` (cerchio, rettangolo/quadrato, testo/titolo),
  `Posizione`, `Timeline`, `Blocco`, `Azione`, `Verbo`, `Easing`; errore unico `Errore`
  con riga e suggerimento (`src/errore.rs`).
- Struttura iniziale del progetto Rust `joypp` (libreria + binario), repository Git
  e alberatura dei moduli (`parser`, `ast`, `compiler`, `codegen`, `security`, `exporter`)
  con relativi `README.md`.
