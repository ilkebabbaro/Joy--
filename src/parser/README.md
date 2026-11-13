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
quando possibile, un **suggerimento** (parola chiave più simile tramite distanza di
Damerau-Levenshtein, che conta come un solo errore anche lo scambio di due lettere:
`apare` → "forse intendevi «appare»?", `sloe` → "forse intendevi \"sole\"?").
`suggerisci` è pubblica perché la usa anche il compilatore per i nomi degli attori; per
sicurezza ignora le parole più lunghe di 64 caratteri.
Il parser non usa mai `unwrap()`/`panic!`: qualsiasi input malformato produce un `Errore`.

## Proprietà di stile di `Crea` (v0.2)

- `scala N` imposta `stile.scala`; `rotazione GRADI [gradi|grado]` imposta `stile.rotazione`.
- Valgono per ogni tipo di attore. Se il numero manca l'errore è «mi aspettavo un numero dopo «scala»» (o «rotazione»), prodotto da `numero_dopo`.
- I limiti sui valori sono validati dal compilatore, non dal parser.

- Il lexer rifiuta i caratteri invisibili/bidirezionali ("Trojan Source") tramite `security::carattere_non_ammesso`, mostrando solo il codice escape (es. `\u{202e}`).
