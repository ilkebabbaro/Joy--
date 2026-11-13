# `src/security` — Sanitizzazione e limiti (`@security`)

Funzioni pure usate da parser, compilatore e generatore.

## Anti-XSS
- `escape_html`: sostituisce `& < > " '` con entità HTML; usato per ogni stringa utente
  inserita nel documento (contenuto dei testi e attributo `data-nome`).
- Nessuna stringa utente entra nel blocco `<style>` o nel `<script>`, salvo il nome del font
  già validato: id e nomi `@keyframes` sono generati (`a0`, `a0-opacita`).
- Il lexer rifiuta i caratteri di controllo (es. `ESC`), che altrimenti verrebbero
  ristampati nei messaggi d'errore e potrebbero manipolare il terminale.
- `carattere_non_ammesso`: oltre ai caratteri di controllo, copre i caratteri bidirezionali
  (U+202A..202E, U+2066..2069, U+200E/F, U+061C) e i separatori di riga U+2028/2029, che
  `char::is_control` non rileva e che potrebbero falsare le righe mostrate nei messaggi
  e negli avvisi ("Trojan Source"). Il lexer deve usarla al posto di `is_control`.
- `valida_nome_font`: un nome di font finisce dentro il CSS, quindi sono ammessi solo
  lettere (anche accentate), cifre, spazi, `-` e `_`. Qualsiasi altro carattere
  (`"`, `;`, `<`, `{`, ...) è un errore di compilazione.
- I colori non vengono mai copiati come testo: il parser li converte in RGBA numerico.

## Anti-DoS (`limiti`)
Costanti che impediscono di generare file che bloccherebbero il browser o il compilatore:
dimensione massima del sorgente, del palco, numero di attori, durata della scena,
numero di keyframe per animazione, lunghezza di testi e nomi, scala, rotazione, BPM.
Dalla v0.2: `MAX_PROFONDITA_GRUPPI = 16` (annidamento dei gruppi; i gruppi contano in
`MAX_ATTORI`). I campioni di `orbita` sono limitati da `MAX_KEYFRAME` (10000), il raggio da
`MAX_COORDINATA`; il nome di gruppi e attori in `data-nome` passa da `escape_html`.
`MAX_KEYFRAME_TOTALI = 200000` è il tetto globale (somma su tutti i canali di attori e gruppi), previsto
dall'audit di sicurezza v0.2-2B (senza tetto: 4000 orbite da 7200 campioni = 1,3 GB di HTML
in 29 s). È applicato dal compilatore con `keyframe::Tetto` (`src/compiler/keyframe.rs`): un
contatore unico, la compilazione si ferma appena il totale supera il limite con un errore
globale. Inoltre ogni orbita verifica una sola volta che `|centro| + raggio` resti entro
`MAX_COORDINATA`.
Anche il calcolo dei suggerimenti "forse intendevi" è limitato, per evitare costi quadratici.

## Verifica
`tests/sicurezza.rs` contiene i payload XSS e di CSS injection, i test dei limiti e un
fuzzing deterministico leggero (3000 input casuali, nessun panic ammesso).

## Robustezza del compilatore
Il crate dichiara `#![forbid(unsafe_code)]` e nega `clippy::unwrap_used`, `expect_used`
e `panic` nel codice di produzione.
