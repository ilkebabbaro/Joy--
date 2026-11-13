# `src/exporter` — Packaging, I/O e CLI (`@exporter`)

Collega la pipeline al file system.

- `compila_con_avvisi`: punto unico della pipeline (limite di dimensione, `parser → compiler →
  codegen`). Restituisce una `Compilazione { html, avvisi }`.
- `compila_sorgente`: wrapper che delega a `compila_con_avvisi` e restituisce solo l'HTML
  (firma stabile, usata dai test; gli avvisi vengono scartati).
- `compila_file`: legge un `.joy`, compila con `compila_con_avvisi` e scrive il `.html`.
  Restituisce `Risultato<Esito>`, dove `Esito { byte, avvisi }` contiene i byte scritti e gli avvisi.
- `osserva`: modalità *watch*, ricompila quando il sorgente cambia (polling dell'orario di
  modifica, senza dipendenze esterne). La callback riceve `Risultato<Esito>` a ogni compilazione.
- Opzioni: percorso di output (`-o`), output leggibile (`--pretty`, default: compatto),
  `--watch`, `--senza-player` (B.5).

**Contratto (v0.2 blocco 2C).**
- `Opzioni { leggibile: bool, senza_player: bool }` (entrambi `false` di default). Si traduce
  in `codegen::OpzioniHtml { leggibile, player: !senza_player }`.
- `Compilazione { html, avvisi, durata: f64 (s), larghezza: f64, altezza: f64, fps: Option<u32> }`:
  i metadati servono a `joypp render` (B.9). `fps` è quello dell'intestazione del palco.
- `compila_sorgente` ha firma invariata.

**Avvisi (SPECIFICA B.6).** Gli avvisi non bloccanti del compilatore non cambiano l'esito:
la compilazione riuscita scrive l'HTML e termina con exit code 0. La CLI li stampa su
**stderr**, in ordine di riga, prima del messaggio di successo `✔ …` su stdout. Se la
compilazione fallisce si stampa solo l'errore. In `--watch` gli avvisi sono ristampati a
ogni ricompilazione riuscita.

La CLI vera e propria (`src/main.rs`) si limita a stampare aiuto, avvisi, progresso ed esito e
a delegare a questo modulo. L'interpretazione degli argomenti è in `cli.rs`.

## `cli.rs` — riga di comando (B.9.1)
Funzione pura `interpreta(args) -> Result<Comando, Errore>` con `Comando::{Compila, Render, Aiuto,
Versione}`. `render` è un sottocomando solo se è il **primo** argomento ed è esattamente quella
parola (`./render` e `render.joy` sono file). `--fps` (1-240) e `--crf` (0-51) accettano solo cifre
decimali; `--watch`, `--pretty`, `--senza-player` sono vietati con `render`, `--fps`/`--crf` senza.
Ogni errore è d'uso (exit code 2, seguito dall'aiuto).

## `render/` — `joypp render` (SPECIFICA B.9)
Pipeline: sorgente → HTML compatto senza player → browser headless via CDP → un PNG per
fotogramma → `ffmpeg` (H.264 `yuv420p`, BT.709) → `.mp4`. Solo libreria standard (D-B9-1).

| Modulo | Compito |
|--------|---------|
| `mod.rs` | API (`OpzioniVideo`, `EsitoVideo`, `Evento`, `render_file`); funzioni pure: `fps_effettivo`, `numero_fotogrammi`, `istante`, `pianifica` (controlli e limiti di B.9.7), `url_file`, `dimensioni_png`, `sfondo_su_nero`; guardie `Drop` per cartella temporanea e file parziale; costanti in `render::limiti` |
| `browser.rs` | ricerca (`JOYPP_BROWSER`, percorsi standard, `PATH`; funzione pura con ambiente iniettato), argomenti di avvio, attesa di `DevToolsActivePort`, stderr ripulito, `Drop` = kill + wait |
| `cdp.rs` | sessione CDP: comandi con `id`, risposte, coda di eventi, timeout per operazione |
| `websocket.rs` | handshake HTTP, frame mascherati, lunghezze 7/16/64 bit, frammentazione, ping/pong/close |
| `json.rs`, `base64.rs` | parser JSON (profondità ≤ 64) e escape; decodifica base64 |
| `ffmpeg.rs` | ricerca (`JOYPP_FFMPEG`, `PATH`), `argomenti_ffmpeg` (funzione pura), processo con thread di scrittura (coda di 4 fotogrammi) e di lettura dello stderr (ultimi 64 KiB) |

Ordine dei controlli: lettura e compilazione, output ≠ sorgente, durata > 0, dimensioni intere,
limiti, ffmpeg, browser, cartella temporanea. Poi: browser → pagina → `__joy_seek(t_k)` +
`Page.captureScreenshot` per `k = 0…N-1` → `Browser.close` → fine di ffmpeg → rinomina del
`.joypp-parziale`. Su ogni errore browser, ffmpeg, parziale e cartella `joypp-render-*` sono
ripuliti e un vecchio output resta intatto. Nessuna shell: solo `Command` con argomenti separati.

**Scostamento da B.9.5 (da recepire in SPECIFICA).** `--window-size` è `max(W, 500) × max(H, 500)`
(`dimensioni_finestra`): con finestre molto piccole (es. un palco 64×36) Edge headless smette di
rispondere a `Page.captureScreenshot` dopo pochi fotogrammi. La viewport resta esattamente W×H
grazie a `Emulation.setDeviceMetricsOverride`, verificata con l'`IHDR` di ogni PNG.

I limiti di B.9.7 (fps, crf, fotogrammi, lato e pixel, timeout, messaggio CDP, profondità JSON)
non sono definiti qui: si usano le costanti di `crate::security::limiti`.
