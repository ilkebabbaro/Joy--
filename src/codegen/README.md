# `src/codegen` — Generazione HTML/CSS (`@renderer`)

Emette un **singolo documento HTML5 auto-contenuto** (CSS/JS inline, nessun CDN) a partire
dalla `ScenaCompilata`.

| File        | Stato v0.1 | Ruolo |
|-------------|------------|-------|
| `dom.rs`    | ✅ | Attori come elementi DOM (`div`/`span`) animati con CSS3 `@keyframes`. |
| `svg.rs`    | ⏳ futuro | Forme vettoriali complesse (linee, tracciati). |
| `canvas.rs` | ⏳ futuro | Sistemi particellari ad alta densità. |

## Contratto con `@exporter` (v0.2 blocco 2C)

`pub fn genera_html(scena: &ScenaCompilata, opzioni: &OpzioniHtml) -> String`, con
`OpzioniHtml { leggibile: bool, player: bool }` (definita in `mod.rs`; `Default`:
`leggibile: false`, `player: true`). `player` abilita il mini-player (B.5); con durata 0
il player non viene mai emesso.

## Script inline, `__joy_seek` e mini-player (v0.2 blocco 2C, `player.rs`)

- Un **solo** `<script>` (IIFE) dopo `#palco` e dopo il markup del player. Contiene, in ordine:
  adattamento del palco alla finestra, `window.__joy_seek` (sempre, anche con
  `--senza-player` e durata 0) e, se `opzioni.player`, il mini-player.
- Gli unici valori variabili sono numeri: `W`, `H`, `D` (durata in ms, `numero(durata*1000)`) e
  l'attributo `max` dello scrubber. Il resto è testo costante (`JS_BASE`, `JS_PLAYER`,
  `CSS_PLAYER`): nessuna stringa utente, nessun `eval`/`innerHTML`.
- Ogni riga di JS è un'istruzione completa: compatto = righe concatenate, `--pretty` = indentate.
- **`__joy_seek(ms)`** (B.9.2): `Promise<number>`, `Object.defineProperty` non riscrivibile;
  valida `ms` (numero finito, altrimenti `TypeError`), clamp a `[0, D]`, coda serializzata,
  alla prima chiamata imposta `data-joy-render` su `<html>` e `#palco.style.transform="none"`,
  attende `document.fonts.ready`, mette in pausa `getAnimations({subtree:true})` con
  `currentTime=t`, attende `ready` e un doppio `requestAnimationFrame`; risolve con `t`.
- `data-joy-render`: la funzione di adattamento esce subito se presente (nessuna riscalatura al
  `resize`); il CSS `html[data-joy-render] #joy-player{display:none!important}` nasconde il
  player, e tutti i suoi gestori (puntatore, tocco, focus, tastiera, ciclo rAF, play/pausa/replay)
  non fanno nulla.
- **Mini-player** (B.5): `#joy-player` (`position:fixed`, fratello di `#palco`), pulsanti
  `<button>` con `aria-label` italiani, scrubber `range` (step 10), etichetta tempo; a scomparsa
  (2,5 s), scorciatoie Spazio/R/←/→ ignorate su `button`/`input`; Web Animations API
  (`document.getAnimations()` filtrate su `#palco`). Senza `document.getAnimations` la barra viene
  rimossa.

## Regole di performance
- Si animano **solo** proprietà composite (`translate`, `rotate`, `scale`, `opacity`):
  nessun *layout thrashing*, accelerazione hardware, 60/120 FPS secondo il display.
- Ogni attore è un *wrapper* posizionato nel proprio centro
  (`width:0;height:0;transform-origin:0 0`) con la forma interna centrata: rotazioni e
  scale avvengono attorno al centro dell'attore.
- `will-change` viene dichiarato solo per i canali effettivamente animati.
- Uno script inline adatta il palco alla finestra (`scale`), senza toccare le animazioni.

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

## Gruppi e canale colore (v0.2 blocco 2B, B.3.5 e B.4.2)

- **Gruppi:** `ScenaCompilata.attori` è in pre-ordine. `genera_html` usa una pila dei gruppi
  aperti: prima di ogni attore chiude i `</div>` finché la cima coincide con `genitore`, poi
  emette il wrapper. Un `Forma::Gruppo` diventa
  `<div class="attore gruppo" id="aK" data-nome="…">` con i figli dentro e nessuna `.forma`
  (e nessuna regola `#aK>.forma`); `data-nome` passa da `escape_html`. `left/top` sono già
  relativi al genitore. Con `--pretty` il rientro cresce di un livello per profondità. Le
  animazioni di un gruppo stanno sul suo wrapper come per un attore. Il `z-index` è per
  wrapper (contesto di impilamento): l'ordine tra fratelli è quello del DOM.
- **Canale colore:** `stile_attore` divide le animazioni per `Canale::bersaglio()`. I canali
  `Wrapper` vanno in `#aK{animation;will-change}`; il canale `colore` (`Bersaglio::Forma`)
  si aggiunge come `;animation:aK-colore Ds linear both` alla regola `#aK>.forma{…}` dopo le
  proprietà statiche e il bordo, e non entra mai in `will-change` (non è composito: solo
  repaint). Se il wrapper ha solo il canale colore non emette né `animation` né `will-change`.
  Le keyframe usano `background-color` (cerchi, rettangoli) o `color` (testi).
- **Salto istantaneo (B.0.3):** `Curva::Gradino` → `animation-timing-function:steps(1,end)`.
- L'output senza gruppi e senza canale colore è identico byte per byte alla v0.1/v0.2 precedente.
