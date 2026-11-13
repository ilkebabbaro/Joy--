# `src/security` — Sanitizzazione e limiti (`@security`)

Funzioni pure usate da parser, compilatore e generatore.

## Anti-XSS
- `escape_html`: sostituisce `& < > " '` con entità HTML; usato per ogni stringa utente
  inserita nel documento (contenuto dei testi e attributo `data-nome`).
- Nessuna stringa utente entra nel blocco `<style>` o nel `<script>`, salvo il nome del font
  già validato: id e nomi `@keyframes` sono generati (`a0`, `a0-opacita`).
- Il lexer rifiuta i caratteri di controllo (es. `ESC`), che altrimenti verrebbero
  ristampati nei messaggi d'errore e potrebbero manipolare il terminale.
- `valida_nome_font`: un nome di font finisce dentro il CSS, quindi sono ammessi solo
  lettere (anche accentate), cifre, spazi, `-` e `_`. Qualsiasi altro carattere
  (`"`, `;`, `<`, `{`, ...) è un errore di compilazione.
- I colori non vengono mai copiati come testo: il parser li converte in RGBA numerico.

## Anti-DoS (`limiti`)
Costanti che impediscono di generare file che bloccherebbero il browser o il compilatore:
dimensione massima del sorgente, del palco, numero di attori, durata della scena,
numero di keyframe per animazione, lunghezza di testi e nomi, scala, rotazione, BPM.
Anche il calcolo dei suggerimenti "forse intendevi" è limitato, per evitare costi quadratici.

## Verifica
`tests/sicurezza.rs` contiene i payload XSS e di CSS injection, i test dei limiti e un
fuzzing deterministico leggero (3000 input casuali, nessun panic ammesso).

## Robustezza del compilatore
Il crate dichiara `#![forbid(unsafe_code)]` e nega `clippy::unwrap_used`, `expect_used`
e `panic` nel codice di produzione.
