# `src/ast` — Albero Sintattico Astratto (`@compiler`)

Strutture dati **pure** (nessuna logica di I/O) prodotte dal parser e consumate dal compilatore.

## Tipi principali

- `Programma` — radice: un `Palco`, la lista ordinata di `Attore` e la `Timeline`.
- `Palco` — dimensioni in px, colore di sfondo, FPS opzionale.
- `Attore` — nome univoco, `Forma` (`Cerchio`, `Rettangolo`, `Testo`), `Stile` comune
  e `Posizione` (assoluta, ancorata al palco o relativa a un altro attore).
- `Stile` — proprietà comuni: `colore`, `opacita` (predefinita 1.0), `livello` (`z-index`),
  `bordo` facoltativo e lo stato iniziale di SPECIFICA B.1: `scala: f64` (`s0`, predefinita
  1.0) e `rotazione: f64` (`r0` in gradi, predefinita 0.0, non normalizzata). I limiti
  li valida il compilatore. `Stile::default()` riporta tutti i valori predefiniti.
- `Colore` — sempre memorizzato come RGBA **numerico**: il testo originale dell'utente
  non arriva mai al generatore HTML (difesa anti-injection).
- `Timeline` → `Blocco` (`Tempo` assoluto o relativo al blocco precedente) → `Azione`
  (attore + `Verbo` + `Easing`).

Ogni nodo che può generare errori semantici conserva il proprio numero di `riga`.
