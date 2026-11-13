# `src/codegen` — Generazione HTML/CSS (`@renderer`)

Emette un **singolo documento HTML5 auto-contenuto** (CSS/JS inline, nessun CDN) a partire
dalla `ScenaCompilata`.

| File        | Stato v0.1 | Ruolo |
|-------------|------------|-------|
| `dom.rs`    | ✅ | Attori come elementi DOM (`div`/`span`) animati con CSS3 `@keyframes`. |
| `svg.rs`    | ⏳ futuro | Forme vettoriali complesse (linee, tracciati). |
| `canvas.rs` | ⏳ futuro | Sistemi particellari ad alta densità. |

## Regole di performance
- Si animano **solo** proprietà composite (`translate`, `rotate`, `scale`, `opacity`):
  nessun *layout thrashing*, accelerazione hardware, 60/120 FPS secondo il display.
- Ogni attore è un *wrapper* posizionato nel proprio centro
  (`width:0;height:0;transform-origin:0 0`) con la forma interna centrata: rotazioni e
  scale avvengono attorno al centro dell'attore.
- `will-change` viene dichiarato solo per i canali effettivamente animati.
- Un micro-script inline adatta il palco alla finestra (`scale`), senza toccare le animazioni.

## Regole di sicurezza
- Il testo utente passa **sempre** da `security::escape_html`.
- Gli `id` e i nomi `@keyframes` sono generati (`a0`, `a0-opacita`), mai derivati dall'input.
- I colori sono riemessi da valori numerici (`rgba(...)`).
- Modalità compatta (default) o leggibile (`--pretty`) tramite un emettitore interno.
