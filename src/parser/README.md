# `src/parser` — Analisi lessicale e sintattica (`@parser`)

Traduce il sorgente `.joy` (italiano naturale) nell'AST definito in `src/ast`.

## Architettura

| File        | Ruolo |
|-------------|-------|
| `lexer.rs`  | Scompone **una riga** in token: `Parola`, `Numero`, `Stringa`, `Esadecimale`, punteggiatura. Normalizza le parole (minuscolo, accenti rimossi: `opacità` → `opacita`), separa le elisioni (`un'immagine` → `un` + `immagine`) e scarta le **parole di cortesia** (articoli). |
| `mod.rs`    | Parser a discesa ricorsiva **orientato alle righe**: ogni riga non vuota è una frase che termina con `.` oppure un'intestazione di blocco che termina con `:`. Le righe indentate appartengono all'ultimo blocco temporale aperto. |

## Flusso

1. Righe vuote e commenti (`# ...` a inizio riga) vengono ignorati.
2. Ogni riga viene tokenizzata da `lexer::tokenizza`.
3. Il parser riconosce: intestazione `Prepara un palco...`, creazione `Crea ...`,
   intestazioni di blocco (`Dal secondo...`, `Al secondo...`, `Poi per...`, `Dopo...`)
   e righe d'azione indentate (`"nome" verbo ...`).
4. Gli aggettivi semantici (`lentamente`, `leggermente`, ...) vengono convertiti nelle
   costanti numeriche della SPECIFICA §6 già in fase di parsing.

## Errori

Ogni errore è un `crate::errore::Errore` con **numero di riga**, messaggio in italiano e,
quando possibile, un **suggerimento** (es. parola chiave più simile tramite distanza di
Levenshtein: `apare` → "forse intendevi `appare`?").
Il parser non usa mai `unwrap()`/`panic!`: qualsiasi input malformato produce un `Errore`.
