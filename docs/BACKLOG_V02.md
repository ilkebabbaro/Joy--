# Backlog v0.2: esito dello stress test creativo `examples/livello1_motion.joy`

**Data:** 2026-11-13

## Contesto

Sigla di motion graphics "JOY++" di 6 s (1280x720, 19 attori, 37 animazioni `@keyframes`), scritta usando **solo** le funzioni della v0.1.

- Compila senza errori.
- Output da 19,5 KB, nessuna risorsa esterna, solo proprietà composite animate.
- 83/83 test superati, clippy pulito.
- Report scritti da `@debugger` (Sonnet) e `@renderer` (Sonnet).

## Cosa ha funzionato

- La sigla si legge come un copione: 19 attori in 5 atti; i commenti `#` e un blocco per atto bastano come struttura (`@debugger`).
- Il parallelismo per blocco è naturale: più attori nello stesso blocco senza codice aggiuntivo (`@debugger`).
- Gli easing semantici (scatto, elastico, rimbalzo) e `pulsa` con aggettivi danno risultati prevedibili senza numeri magici (`@debugger`).
- Errori chiari in italiano con riga e suggerimento. Le sovrapposizioni sulla stessa grandezza vengono rifiutate citando le righe di entrambe le azioni, il che evita bug visivi silenziosi (`@debugger`).
- I canali separati con proprietà CSS individuali (translate/rotate/scale/opacity) si compongono senza matrici (`@renderer`).
- Il wrapper `.attore` (width:0, height:0, transform-origin 0 0) con la forma centrata fa ruotare e scalare sempre attorno al centro (`@renderer`).
- Gli easing `linear()` campionati sono fedeli per elastico e rimbalzo; `will-change` mirato per canale; `contain:strict` sul palco (`@renderer`).

## Limiti emersi

Workaround usati nella sigla:

| Problema | Workaround usato nella sigla |
|----------|------------------------------|
| Nessuna scala/rotazione iniziale | Blocco "finto" da 0 a 0,01 s con `si rimpicciolisce fino a 0` (righe 45-52 della sigla). Nota: nessun flash al primo fotogramma, verificato dall'orchestratore: quegli attori partono con `opacity:0` grazie ad `appare sfumando`. |
| Le posizioni relative usano la posizione iniziale del riferimento, non quella animata | Coordinate assolute per il badge. |
| Gli spostamenti fuori campo si calcolano a mano | `-750` e `2780` scritti a mano per i pannelli. |
| Nessun gruppo | Logo, badge e versione animati riga per riga, con righe duplicate. |
| Nessun `orbita` | Percorso a quadrato con 3 blocchi `Poi per`. |
| Nessuna scala in una sola direzione | "Wipe" ottenuto spostando pannelli 1500x800 (3 layer GPU molto grandi per 6 s). |
| Nessun loop/replay | Ricaricare la pagina. |

Altri limiti:

- `ruota` su un cerchio è invisibile e accettato senza avviso.
- `"Arial Black"` non ha un'alternativa garantita su macOS e Linux.
- Le curve `linear()` compaiono 19 volte ma solo 2 sono distinte (circa 4 KB sprecati); i keyframe di `pulsa` sono verbosi (`a7-scala` circa 1,1 KB).
- `will-change` resta attivo su tutti i 19 attori anche dopo la fine delle loro animazioni.
- Lo scale di `#palco` usa `Math.min(1, …)`, quindi non ingrandisce mai su schermi grandi.

## Difetti veri da correggere nella v0.2

- Il suggerimento «forse intendevi "a"?» può proporre l'attore che si sta creando (posizione relativa che riferisce un nome sconosciuto).
- Concordanza: i messaggi dicono «1 secondi».

## Priorità v0.2

1. **Stato iniziale dichiarativo:** `con scala [N]`, `con rotazione [GRADI]`. Elimina il blocco di setup.
   → Specifica in `SPECIFICA.md`, Appendice B.
2. **Mini-player nell'HTML** (Play, Pausa, Replay, scrubber a scomparsa) basato sulla Web Animations API.
   → `SPECIFICA.md`, Appendice B.
3. **Gruppi:** `Crea un gruppo chiamato "[NOME]" con "[A]", "[B]"...`, animabili come un unico attore.
   → `SPECIFICA.md`, Appendice B.
4. **Ingressi e uscite dai bordi del palco** (`entra da …`, `esce verso …`), più i verbi `orbita attorno a` e `cambia colore in`, già presenti nella `SPECIFICA.md` §5.
   → `SPECIFICA.md`, Appendice B.
5. **Rendering più leggero:** `scaleX`/`scaleY` con origine ancorabile per i wipe; dedup delle curve `linear()` (variabili CSS) e keyframe di `pulsa` più compatti; `will-change` solo quando serve.
   → Solo backlog, da specificare.
6. **Diagnostica:** opzione `--timeline` che stampa la tabella attore/tempo/grandezza; avviso non bloccante su `ruota` applicato a un cerchio; correzione dei due difetti sopra.
   → Parzialmente in `SPECIFICA.md`, Appendice B.

---

> La sintassi delle priorità 1-4 (e dell'avviso sui cerchi) è definita in `SPECIFICA.md`, Appendice B, scritta da `@compiler` (Opus).

## Stato al rilascio 0.2.0

Rilascio `v0.2.0` (2026-11-13).

| # | Priorità | Stato | Dove |
|---|----------|-------|------|
| 1 | Stato iniziale dichiarativo (`scala`, `rotazione`) | Fatta | B.1, CHANGELOG 0.2.0 |
| 2 | Mini-player nell'HTML (Play, Pausa, Replay, scrubber) | Fatta (con `--senza-player` per disattivarlo) | B.5 |
| 3 | Gruppi annidati | Fatta (fino a 16 livelli) | B.3 |
| 4 | `entra da` / `esce verso`, `orbita attorno a`, `cambia colore in` | Fatta | B.2, B.4.1, B.4.2 |
| 5 | Rendering più leggero (`scaleX`/`scaleY`, dedup di `linear()`, `pulsa` più compatto, `will-change` mirato) | Rimandata | solo backlog |
| 6 | Diagnostica: `--timeline`, avviso su `ruota` su cerchio, correzione dei difetti | Avviso sui cerchi e difetti diagnostici fatti; `--timeline` rimandata | B.6 |

Oltre alle priorità del backlog, la 0.2.0 include l'esportazione video MP4 con `joypp render` (B.9), non prevista in questo backlog.
