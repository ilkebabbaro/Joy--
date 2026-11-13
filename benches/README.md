# Baseline di prestazioni di `joypp render` (v0.2.1 + diagnostica C.1)

Misure, **nessuna ottimizzazione**. Servono da riferimento per i confronti futuri. I numeri vengono
dal JSON `joypp-profilo/1` prodotto da `--profilo-json` (SPECIFICA C.1).

## Ambiente

| voce | valore |
|---|---|
| OS | Windows 11 Pro |
| CPU | AMD Ryzen 9 7900X, 24 thread logici |
| RAM | 31,6 GB |
| Browser | Edge 157.0.4341.46 (headless, CDP) |
| ffmpeg | build gyan.dev 2026-04-09 essentials |
| Rust | rustc 1.100, `cargo build --release` (`lto = true`, `codegen-units = 1`) |
| Codifica | libx264, preset `medium`, `crf 18` |

## Protocollo

Una variabile per volta: stesso binario release, stessa macchina, render eseguiti in sequenza, file
generati fuori dal repository.

```text
cargo build --release
target/release/joypp render examples/livello1_motion.joy    -o <tmp>/l1_N.mp4  --profilo-json <tmp>/l1_N.json   # N = 1..3
target/release/joypp render examples/sistema_orbitale.joy   -o <tmp>/so_N.mp4  --profilo-json <tmp>/so_N.json   # N = 1..3
target/release/joypp render examples/cattedrale.joy --fps 6 -o <tmp>/cat_1.mp4 --profilo-json <tmp>/cat_1.json
target/release/joypp render examples/JOYPP_VOXEL_OVERDRIVE_v021.joy --fps 4 -o <tmp>/vox_1.mp4 --profilo-json <tmp>/vox_1.json
```

Gli esempi 4K sono ridotti con `--fps` (181 e 193 fotogrammi) e campionano l'intera durata della
scena (30 s e 48 s), quindi i costi per fotogramma sono mediati su tutto lo spettacolo. Non sono
stati lanciati render 4K completi a 60 fps. I due esempi 720p sono render completi a 60 fps.

## Risultati

Media su 3 ripetizioni per i 720p (min-max tra parentesi dove utile); singola misura per i 4K. Tempi
in ms, tranne dove indicato.

| | livello1_motion | sistema_orbitale | cattedrale (`--fps 6`) | voxel overdrive (`--fps 4`) |
|---|---|---|---|---|
| Risoluzione | 1280x720 | 1280x720 | 3840x2160 | 3840x2160 |
| Fotogrammi | 361 | 541 | 181 | 193 |
| Attori / animazioni | 19 / 37 | 4 / 6 | 203 / 332 | 381 / 579 |
| Totale (s) | 25,39 (25,30-25,51) | 37,19 (37,16-37,23) | 32,37 | 35,93 |
| fps_ciclo | 14,64 (14,59-14,67) | 14,81 (14,79-14,83) | 5,85 | 5,61 |
| Ciclo per fotogramma | 68,30 (68,16-68,54) | 67,55 (67,43-67,61) | 170,88 | 178,13 |
| seek media / p95 | 6,26 / 13,46 | 7,93 / 14,25 | 10,71 / 17,54 | 11,16 / 15,94 |
| screenshot media / p95 | 61,78 / 65,85 | 59,53 / 64,13 | 158,32 / 200,88 | 164,66 / 206,01 |
| base64 media | 0,243 | 0,070 | 1,817 | 2,256 |
| attesa_coda media | 0,006 | 0,006 | 0,006 | 0,006 |
| scrittura_pipe per fotogramma | 0,41 | 0,020 | 0,60 | 0,52 |
| finalizzazione_ffmpeg | 153,9 | 73,8 | 786,2 | 830,5 |
| PNG medio (KiB) | 36,5 | 13,0 | 343,1 | 416,3 |
| PNG max (KiB) | 62,3 | 15,8 | 691,2 | 895,0 |
| Quota seek | 9,2% | 11,7% | 6,3% | 6,3% |
| Quota screenshot | 90,5% | 88,1% | 92,6% | 92,4% |
| Quota base64 | 0,36% | 0,10% | 1,06% | 1,27% |
| Quota attesa_coda | 0,009% | 0,009% | 0,003% | 0,003% |
| Altro (verifica PNG, ecc.) | < 0,03% | < 0,03% | < 0,03% | < 0,03% |
| rapporto_tempo_reale | 0,237 | 0,242 | 0,932 (a 6 fps) | 1,343 (a 4 fps) |
| `fotogrammi_con_coda_piena` | 0 | 0 | 0 | 0 |

Le quote sono `totale della misura / ciclo_fotogrammi`. Costi fissi per render (ordine di
grandezza): avvio_browser ~210 ms, connessione_cdp 30-46 ms, caricamento_pagina 80-160 ms,
chiusura_browser 180-260 ms, preparazione ~20 ms; insieme circa 0,6 s.

### Stima di un render 4K completo a 60 fps (stima, non misura)

Formula: `T ~ N x ciclo_per_fotogramma + costi_fissi + finalizzazione_ffmpeg`, con
`N = durata x 60 + 1`. Si assume che il costo per fotogramma misurato a 6 o 4 fps resti invariato a
60 fps: il ritmo di ingresso verso ffmpeg è lo stesso (limitato dalla cattura) e `attesa_coda` è ~0.
Costi fissi (~0,6 s) e finalizzazione (~0,8 s) sono quelli misurati.

| scena | N | ciclo per fotogramma | N x ciclo | + fissi e finalizzazione | stima | riferimento storico dell'utente |
|---|---|---|---|---|---|---|
| cattedrale (30 s) | 1801 | 170,88 ms | 307,8 s | ~1,4 s | ~309 s = **~5 min 9 s** | ~4 min |
| voxel (48 s) | 2881 | 178,13 ms | 513,2 s | ~1,4 s | ~515 s = **~8 min 35 s** | 8 min 22 s (502 s) |

I riferimenti dell'utente provengono da una macchina diversa o ignota e forse da una versione
precedente: non sono direttamente comparabili. Incertezza: il campione a 4-6 fps contiene solo 181-193
istanti, quindi tratti di complessità maggiore tra due campioni potrebbero sfuggire; il PNG medio
dipende dal contenuto.

## Osservazioni

### Fatti misurati
* Lo **screenshot** (`Page.captureScreenshot`, misura combinata) occupa l'88-93% del ciclo in tutti i
  casi. Seek 6-12%, base64 0,1-1,3%, tutto il resto sotto lo 0,03%.
* `attesa_coda` vale ~6 us e `fotogrammi_con_coda_piena` è 0 in tutti i render: ffmpeg consuma più
  veloce di quanto il browser produca (4-6 fps in ingresso su 4K, ~15 fps su 720p). Anche
  `scrittura_pipe` è di 0,02-0,6 ms per fotogramma.
* La `finalizzazione_ffmpeg` costa 0,07-0,83 s in totale: trascurabile rispetto al ciclo.
* Le ripetizioni sono stabili: ciclo per fotogramma entro ~0,6% sui 720p.
* **Il costo dello screenshot non segue la dimensione del PNG nei 720p.** `sistema_orbitale` produce
  PNG da 13 KiB (4 attori) e `livello1_motion` da 36,5 KiB (19 attori), ma lo screenshot costa quasi
  uguale (59,5 contro 61,8 ms: 2,3 ms per ~23 KiB in più).
* Il costo cresce con i pixel ma molto meno che proporzionalmente: da 720p a 4K (9x i pixel) lo
  screenshot passa da ~60 a 158-165 ms (2,7x) e il seek da 6-8 a ~11 ms.
* Il base64 è trascurabile (1,8-2,3 ms su PNG da 340-420 KiB).
* I PNG 4K sono 343-416 KiB in media (max 0,7-0,9 MB): il buffer in memoria è di pochi MB.
* A 720p il render dura ~4,2 volte il video (`rapporto_tempo_reale` 0,24 a 60 fps).

### Ipotesi (non verificate da queste misure)
1. **Attesa del fotogramma del compositor.** Lo screenshot a 720p costa ~60 ms quasi indipendenti da
   contenuto e dimensione del PNG, e con p95 vicino (~65 ms). Ciò è coerente con una latenza quasi
   fissa di alcuni periodi a 60 Hz (~3,6 x 16,7 ms) per `captureScreenshot`, più che con la sola
   codifica PNG.
2. **Codifica PNG nel browser.** Sul 4K una parte maggiore dei 158-165 ms può essere la lettura dei
   pixel e la codifica PNG di 8,3 Mpixel; la crescita di soli 2,7x con 9x i pixel indica però che
   una componente fissa (ipotesi 1) pesa molto anche lì.
3. **Trasferimento WebSocket e parsing JSON** (400 KiB di base64 per risposta): probabilmente minore
   della codifica; il parsing è incluso nella misura combinata e non separato.
4. Con 24 thread, joypp e ffmpeg non sono limitanti; il collo di bottiglia è la sequenza seriale
   nel browser (un fotogramma alla volta: seek, poi screenshot).

### Come separare le ipotesi (non eseguito)
* Ripetere lo screenshot con `format: jpeg`: se il tempo scende poco, domina l'attesa del compositor.
* Misurare `captureScreenshot` su una pagina vuota alla stessa risoluzione: costo fisso di base.
* Separare nella diagnostica il tempo della risposta CDP dall'analisi JSON (oggi combinati, C.1.3).
* Pagine/target in parallelo sarebbero un cambiamento d'architettura da valutare con
  `@compiler`/`@renderer`.
