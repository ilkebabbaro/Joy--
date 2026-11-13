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

## Stato iniziale statico: `scale` e `rotate` (v0.2, B.1)

`stile_attore` aggiunge alla regola statica `#aN{...}` del wrapper `scale:<s0>` se
`scala_iniziale != 1` e `rotate:<r0>deg` se `rotazione_iniziale != 0`. Con i valori
di default non viene emesso nulla (output v0.1 invariato byte per byte). `rotate` e
`scale` sono proprietà CSS individuali con `transform-origin:0 0` nel centro
dell'attore, quindi la regola statica si compone con i keyframe (il primo keyframe
riporta lo stesso valore iniziale). Per i valori statici non serve `will-change`.
