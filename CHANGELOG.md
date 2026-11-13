# Changelog

Tutte le modifiche rilevanti a questo progetto sono documentate in questo file.
Il formato segue [Keep a Changelog](https://keepachangelog.com/it/1.1.0/) e il
progetto aderisce al [Semantic Versioning](https://semver.org/lang/it/).

## [Unreleased]

### Aggiunto
- Esempio `examples/cattedrale.joy`: stress test 4K (3840×2160, 60 fps) di 30 s con 164 attori e 5 atti.
- Esempio `examples/JOYPP_VOXEL_OVERDRIVE_v021.joy`: stress test 4K (3840×2160, 60 fps) di 48 s, geometria pseudo-voxel con gruppi, orbite e HUD.
- Nuova versione di `examples/sfida_hyperframe_20s.joy`: sfida "Astrolabio" 4K (3840×2160, 60 fps) di circa 14 s.
- `joypp render --profilo` (rapporto di diagnostica su stderr) e `joypp render --profilo-json <file>` (rapporto JSON `joypp-profilo/1`): misurano i costi della pipeline MP4 (fasi, fotogrammi, ffmpeg, memoria stimata). Senza le opzioni il render è invariato. Specifica in SPECIFICA C.1.
- Baseline delle prestazioni del render in `benches/README.md`.

### Sicurezza
- `--profilo-json` confronta il percorso con sorgente e video in modo lessicale e canonico: il JSON non può sovrascrivere il video né il sorgente.
- Il JSON è scritto in modo atomico (`<file>.joypp-parziale`, poi rinomina); il parziale viene rimosso in caso di errore.
- Il profilo non contiene percorsi di file né di eseguibili.

## [0.2.1] - 2026-11-13

Patch: `appare sfumando` su un attore con `opacità 0` ora lo rende visibile; nuovo avviso sulle opacità che non cambiano nulla.

### Corretto
- `appare sfumando` su un attore o gruppo con `opacità 0` ora porta l'opacità a 1. Prima restava 0 e l'attore non compariva mai (trovato dallo stress test `examples/sfida_hyperframe_20s.joy`).

### Aggiunto
- Avviso non bloccante «opacità invariata» per `appare`, `scompare` e `cambia opacità` che non cambiano nulla.
- Esempio `examples/sfida_hyperframe_20s.joy`: stress test di 20 s con gruppi e orbite.

### Modificato
- SPECIFICA A.3 e B.3.5: nuova destinazione di `appare sfumando` (con opacità 0 dichiarata si arriva a 1).
- `.gitignore`: i video `*.mp4` generati non vengono tracciati.

## [0.2.0] - 2026-11-13

Release v0.2: stato iniziale (`scala`, `rotazione`), avvisi non bloccanti, gruppi annidati, `entra da` / `esce verso`, `orbita`, `cambia colore`, mini-player nell'HTML ed esportazione video MP4 con `joypp render`.

### Aggiunto
- Esempio `examples/livello1_motion.joy`: sigla di motion graphics "JOY++" di 6 secondi (19 attori), stress test creativo delle sole funzioni della v0.1.
- Roadmap v0.2: `docs/BACKLOG_V02.md` (esito dello stress test) e `SPECIFICA.md` Appendice B (stato iniziale, entra da/esce verso, gruppi, orbita, cambia colore, mini-player, avvisi, esportazione video MP4 `joypp render` (B.9)).
- Stato iniziale dichiarativo v0.2 (SPECIFICA B.1): proprietà `scala N` e `rotazione G gradi` nella frase `Crea`, valide per ogni tipo di attore; il compilatore le valida e il renderer le emette come `scale`/`rotate` statici sul wrapper dell'attore.
- Avvisi non bloccanti v0.2 (SPECIFICA B.6): primo avviso, «rotazione invisibile su un cerchio», stampato su stderr senza cambiare l'exit code; nuova API `compila_con_avvisi` che restituisce `Compilazione { html, avvisi }`.
- Esempio `examples/stato_iniziale.joy`: stato iniziale con scala e rotazione dichiarate alla creazione.
- Gruppi annidati v0.2 (SPECIFICA B.3): `Crea un gruppo chiamato "G" con "A", "B"...`, animabili come un unico attore; fino a 16 livelli di annidamento; pivot al centro dell'AABB degli ingombri dei membri.
- Ingressi e uscite dai bordi v0.2 (SPECIFICA B.2): `entra da …` ed `esce verso …`, con ingombro prudente e salto istantaneo.
- Verbo `orbita attorno a` (SPECIFICA B.4.1): centro statico, verso antiorario predefinito (`in senso orario` per l'inverso), campioni calcolati con la sagitta (≤ 0,5 px).
- Verbo `cambia colore in` (SPECIFICA B.4.2): animazione del colore della forma, fuori da `will-change`.
- Esempio `examples/sistema_orbitale.joy`: sottogruppo "pianeta" che orbita il sole.
- Specifica: il `--fps` predefinito di `joypp render` segue `a N fotogrammi` dell'intestazione, con fallback 60 (SPECIFICA B.9.1).
- Mini-player a scomparsa nell'HTML generato (SPECIFICA B.5): Play/Pausa, Replay, scrubber, scorciatoie Spazio, R e frecce; disattivabile con `--senza-player`.
- `window.__joy_seek(ms)` nell'HTML generato: posiziona la scena su un istante con la Web Animations API, usato dal render video (B.9.2).
- `joypp render <file.joy> [-o file.mp4] [--fps N] [--crf N]` (SPECIFICA B.9): esporta un video H.264 yuv420p con fps predefinito da `a N fotogrammi` o 60 e crf 18. Usa un client CDP senza dipendenze esterne e un browser headless di sistema (Chrome, Chromium o Edge); i PNG sono passati a ffmpeg.

### Corretto
- Il suggerimento «forse intendevi "…"?» non propone più l'attore che si sta creando; i candidati sono ordinati per somiglianza.
- Messaggi al singolare quando il valore è 1: «Poi per 1 secondo», «Dopo 1 secondo» (prima «1 secondi»).

### Sicurezza
- Il lexer rifiuta i caratteri bidirezionali (U+202A..U+202E, U+2066..U+2069, U+200E, U+200F, U+061C) e i separatori di riga Unicode (U+2028, U+2029), che potrebbero falsare il testo mostrato nei messaggi e negli avvisi ("Trojan Source"). Il controllo è `security::carattere_non_ammesso`.
- Tetto globale di 200000 keyframe per scena (`MAX_KEYFRAME_TOTALI`, anti-DoS): senza tetto un'animazione di gruppi e orbite poteva produrre un HTML di oltre 1 GB.
- Limite di profondità dei gruppi (`MAX_PROFONDITA_GRUPPI = 16`); i gruppi contano in `MAX_ATTORI`.
- I caratteri bidirezionali e i separatori di riga Unicode sono bloccati anche nei nomi dei membri dei gruppi.
- Limiti del render video in `security::limiti`, verificati prima dell'avvio del browser.
- Output passato a ffmpeg come percorso assoluto con prefisso `file:`, senza possibilità di option injection.
- Limite noto: la terminazione forzata del browser non è garantita in ogni caso (SPECIFICA B.9.4).

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
