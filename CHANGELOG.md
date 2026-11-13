# Changelog

Tutte le modifiche rilevanti a questo progetto sono documentate in questo file.
Il formato segue [Keep a Changelog](https://keepachangelog.com/it/1.1.0/) e il
progetto aderisce al [Semantic Versioning](https://semver.org/lang/it/).

## [Unreleased]

### Aggiunto
- Esempio `examples/livello1_motion.joy`: sigla di motion graphics "JOY++" di 6 secondi (19 attori), stress test creativo delle sole funzioni della v0.1.
- Roadmap v0.2: `docs/BACKLOG_V02.md` (esito dello stress test) e `SPECIFICA.md` Appendice B (stato iniziale, entra da/esce verso, gruppi, orbita, cambia colore, mini-player, avvisi, esportazione video MP4 `joypp render` (B.9)).

## [0.1.0] - 2026-11-13

Prima versione (MVP) del compilatore Joy++: `joypp file.joy -o file.html`.

### Aggiunto
- **Progetto**: crate Rust `joypp` (libreria + binario) senza dipendenze esterne, con moduli
  `parser`, `ast`, `compiler`, `codegen`, `security` ed `exporter`, ciascuno con il proprio `README.md`.
- **Linguaggio** (vedi `SPECIFICA.md`, inclusa l'Appendice A con i valori predefiniti):
  - palco: `Prepara un palco grande L per H [con sfondo COLORE] [a N fotogrammi].`;
  - attori: `cerchio`, `rettangolo`, `quadrato`, `testo`, `titolo` con le proprietà `raggio`,
    `larghezza`, `altezza`, `lato`, `arrotondamento`, `testo`, `grandezza`, `font`, `peso`,
    `colore`, `opacità`, `livello`, `bordo`;
  - colori: nomi italiani, `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`, `rgb(R, G, B)`;
  - posizioni: `a X, Y`, ancore del palco (`al centro`, `in alto a sinistra`, ...) e relative
    (`a destra di "sole" di 200`, `sopra a`, `sotto a`, `a sinistra di`);
  - blocchi: `Dal secondo A al secondo B:`, `Al secondo T:`, `Poi per D secondi:`,
    `Dopo A secondi per D secondi:`;
  - verbi: `appare sfumando`, `scompare sfumando`, `cambia opacità a`, `si sposta a`,
    `scivola verso`, `ruota di`, `si ingrandisce fino a`, `si rimpicciolisce fino a`, `pulsa`;
  - modificatore `con movimento lineare|morbido|scatto|elastico|rimbalzo` e aggettivi
    semantici di intensità e ritmo.
- **Parser**: articoli scartati, accenti ed elisioni normalizzati, virgolette tipografiche;
  errori in italiano con numero di riga e suggerimenti ("forse intendevi «appare»?").
- **Compilatore**: risoluzione iterativa delle posizioni relative con rilevamento dei cicli,
  ancoraggio dei blocchi relativi, canali di animazione indipendenti (opacità, traslazione,
  rotazione, scala) con errore sulle sovrapposizioni, keyframe pre-calcolati (inclusa
  l'espansione di `pulsa`), curve `elastico`/`rimbalzo` campionate in `linear(...)`.
- **Generatore DOM**: un singolo HTML5 auto-contenuto; CSS3 `@keyframes` solo su proprietà
  composite (`translate`, `rotate`, `scale`, `opacity`) per 60/120 FPS; `will-change` mirato;
  palco adattato alla finestra; output compatto o leggibile (`--pretty`).
- **CLI**: `joypp <file.joy> [-o file.html] [--pretty] [--watch]`; API `joypp::compila_sorgente`.
- **Test**: 83 test (unitari, parser, compilatore, sicurezza, end-to-end, CLI) e riproduzioni
  di regressione in `tests/repro/`.

### Sicurezza
- Escape HTML di tutti i testi e nomi utente; colori riemessi da valori numerici; id CSS generati;
  nomi di font validati con una lista di caratteri ammessi.
- Limiti anti-DoS: dimensione del sorgente (1 MiB), palco, numero di attori, durata della scena,
  keyframe per animazione, lunghezza di testi e nomi, scala, rotazione, BPM.
- Rifiuto dei caratteri di controllo (iniezione di sequenze ANSI nei messaggi d'errore).
- `#![forbid(unsafe_code)]`; `unwrap`, `expect` e `panic` vietati nel codice di produzione.
- La CLI non sovrascrive il file sorgente se `-o` punta allo stesso file.

### Corretto
- I suggerimenti usano la distanza di Damerau-Levenshtein: le lettere scambiate (`sloe` → `sole`)
  ora vengono riconosciute.
- `SPECIFICA.md`: chiuso il blocco di codice dell'esempio finale, che era rimasto aperto.
