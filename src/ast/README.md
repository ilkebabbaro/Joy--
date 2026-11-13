# `src/ast` — Albero Sintattico Astratto (`@compiler`)

Strutture dati **pure** (nessuna logica di I/O) prodotte dal parser e consumate dal compilatore.

## Tipi principali

- `Programma` — radice: un `Palco`, la lista ordinata di `Attore` e la `Timeline`.
- `Palco` — dimensioni in px, colore di sfondo, FPS opzionale.
- `Attore` — nome univoco, `Forma` (`Cerchio`, `Rettangolo`, `Testo`), `Stile` comune
  (colore, opacità, livello, bordo) e `Posizione` (assoluta, ancorata al palco o relativa
  a un altro attore).
- `Colore` — sempre memorizzato come RGBA **numerico**: il testo originale dell'utente
  non arriva mai al generatore HTML (difesa anti-injection).
- `Timeline` → `Blocco` (`Tempo` assoluto o relativo al blocco precedente) → `Azione`
  (attore + `Verbo` + `Easing`).

Ogni nodo che può generare errori semantici conserva il proprio numero di `riga`.
