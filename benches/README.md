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

## Fase 3 — Cattura screenshot

Esperimento isolato (`tests/esperimento_cattura.rs`, tutti i test `#[ignore]`) per capire dove va il
tempo di `Page.captureScreenshot` e se `optimizeForSpeed` lo riduce. Nessuna modifica a `src/`: il
seek (`__joy_seek`, B.9.2), il preset ffmpeg e i parametri di produzione restano invariati. Il client
CDP dell'harness ha un proprio `TcpStream` e riproduce i passi 4-9 di B.9.5 (stessi argomenti del
browser via `Browser::avvia`, stessa viewport, `setDefaultBackgroundColorOverride`, `navigate`,
`loadEventFired`, HTML compatto senza player come `render_file`).

### Protocollo e comando

```text
set JOYPP_ESPERIMENTO_DIR=C:\Users\xhoni\AppData\Local\Temp\claude\b2eb6e1a7a6a7f6e\2a22d48b-fdb6-4b78-b9cf-0ed1238f7383\scratchpad\cattura
cargo test --release --test esperimento_cattura -- --ignored --nocapture --test-threads=1
```

(`esperimento_completo` dura circa 10 minuti; `esperimento_rapido` è una prova da ~1 minuto;
`esperimento_riepilogo` rielabora CSV e immagini già presenti. Senza la variabile, o senza browser o
ffmpeg, stampano «esperimento saltato: ...» e passano. File generati solo in quella cartella.)

* Scene: pagina vuota e `livello1_motion` a 1280x720 (M = 60 istanti); pagina vuota, `cattedrale` e
  `JOYPP_VOXEL_OVERDRIVE_v021` a 3840x2160 (M = 30). La «pagina vuota» è un `.joy` con un quadrato
  4x4 e un blocco temporale (stessa durata: 6 s a 720p, 30 s a 4K). Più due sonde `about:blank`
  (`blank720`, `blank4k`: stessa viewport, nessuna pagina né seek).
* Istanti equispaziati in `[0, D]`, identici per tutte le varianti. I primi 5 fotogrammi sono
  warm-up, esclusi dalle statistiche «a regime» e riportati a parte.
* Varianti, una variabile per volta: **V0** baseline `{"format":"png","fromSurface":true,
  "captureBeyondViewport":false}`; **V1** = V0 + `"optimizeForSpeed":true` (Edge 157 lo accetta: nessun
  `error`); sonde solo di misura, non candidate: **C** secondo screenshot consecutivo senza nuovo seek
  (si registra il secondo), **JPEG** `quality 100`, **WEBP** `quality 100` (accettato da Edge).
* 3 ripetizioni per variante, ognuna con un browser nuovo e profilo nuovo; ordine alternato (rep 0 e 2:
  V0, V1, C, JPEG, WEBP; rep 1: inverso). 87 run, 0 errori, 0 timeout. Nessun render MP4.
* Componenti (µs con `Instant`): `invio` (serializzazione + scrittura), `primo_byte` (dall'invio al
  `peek` bloccante: lavoro lato browser), `ricezione` (dal primo byte al messaggio riassemblato),
  `json` (`json::analizza`), `copie` (il `cloned()` di `result` come in `Sessione::comando`, più il
  rilascio), `base64`. `shot` = invio + primo_byte + ricezione. `ciclo` = seek + shot + json + copie +
  base64. Gli eventi CDP sono ignorati e contati.
* Pixel: ogni PNG decodificato con `ffmpeg -f rawvideo -pix_fmt rgba` e confrontato byte per byte.

### Ambiente

| voce | valore |
|---|---|
| OS | Windows 11 Pro |
| CPU / RAM | AMD Ryzen 9 7900X, 24 thread / 31,6 GB |
| Browser | Edge 157.0.4341.46 (headless=new) |
| ffmpeg | gyan 2026-04-09 (solo per decodificare i PNG) |
| Rust | rustc 1.100, build release |

### Scomposizione della latenza (ms, a regime, 3 ripetizioni accorpate)

| scena | var | seek | invio | primo_byte | ricezione | json | copie | base64 | **ciclo** | shot p95 | shot max | KiB PNG |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| vuota 720p | V0 | 8,27 | 0,03 | 58,77 | 0,02 | 0,01 | 0,00 | 0,01 | **67,11** | 61,9 | 68,0 | 4 |
| vuota 720p | V1 | 10,84 | 0,03 | 56,01 | 0,02 | 0,02 | 0,00 | 0,07 | **66,99** | 59,0 | 59,3 | 16 |
| livello1 720p | V0 | 5,92 | 0,04 | 61,14 | 0,24 | 0,04 | 0,01 | 0,28 | **67,67** | 65,3 | 87,2 | 39 |
| livello1 720p | V1 | 9,74 | 0,04 | 56,83 | 0,18 | 0,05 | 0,02 | 0,37 | **67,23** | 59,8 | 81,1 | 59 |
| vuota 4K | V0 | 11,18 | 0,03 | 117,66 | 0,03 | 0,03 | 0,01 | 0,07 | **129,01** | 137,2 | 146,2 | 29 |
| vuota 4K | V1 | 11,05 | 0,04 | 89,58 | 0,10 | 0,10 | 0,05 | 0,40 | **101,32** | 94,3 | 98,9 | 124 |
| cattedrale 4K | V0 | 10,09 | 0,04 | 151,17 | 0,51 | 0,27 | 0,11 | 1,96 | **164,16** | 182,8 | 196,6 | 369 |
| cattedrale 4K | V1 | 9,41 | 0,04 | 97,54 | 1,44 | 0,43 | 0,16 | 2,62 | **111,64** | 112,6 | 119,6 | 509 |
| voxel 4K | V0 | 10,11 | 0,04 | 159,39 | 0,86 | 0,38 | 0,12 | 2,52 | **173,43** | 195,6 | 206,0 | 456 |
| voxel 4K | V1 | 9,31 | 0,04 | 99,08 | 1,28 | 0,49 | 0,16 | 3,23 | **113,60** | 115,3 | 122,7 | 621 |
| about:blank 720p | V0 / V1 | - | 0,04 | 45,68 / 44,55 | 0,02 | 0,01 | 0,00 | 0,01 / 0,07 | 45,75 / 44,70 | 54,4 / 53,8 | 81,3 / 57,4 | 4 / 16 |
| about:blank 4K | V0 / V1 | - | 0,04 | 102,74 / 77,67 | 0,02-0,06 | 0,03-0,12 | 0,00-0,01 | 0,07 / 0,41 | 102,90 / 78,30 | 109,8 / 85,3 | 141,3 / 87,8 | 29 / 124 |

Altre sonde (ciclo medio, ms): sulla scena `cattedrale` JPEG 114,55 (come V1), WEBP 230,00
(peggiore di V0), C 153,95; su `voxel` JPEG 122,45, WEBP 216,92, C 166,16. A 720p JPEG ≈ V0 (67,35 e
67,19), C circa -23% e -18% (51,25 e 55,28), WEBP ≈ V0 (vuota) o 11% peggio (livello1). Tabelle complete
(media/p50/p95/max/min per ogni componente) nel `riepilogo.md` prodotto dall'harness.

### A freddo (ms, media sulle 3 ripetizioni, shot = invio + primo_byte + ricezione)

| scena | var | k=0 seek | k=0 shot | k=1..4 shot | regime shot |
|---|---|---|---|---|---|
| vuota 720p | V0 / V1 | 21,7 / 36,9 | 57,5 / 51,1 | 63,4 / 57,0 | 58,8 / 56,1 |
| livello1 720p | V0 / V1 | 21,8 / 21,7 | 68,5 / 53,1 | 58,5 / 60,0 | 61,4 / 57,1 |
| vuota 4K | V0 / V1 | 24,4 / 15,5 | 117,7 / 89,2 | 118,6 / 93,2 | 117,7 / 89,7 |
| cattedrale 4K | V0 / V1 | 31,0 / 29,8 | 130,7 / 109,3 | 135,0 / 95,6 | 151,7 / 99,0 |
| voxel 4K | V0 / V1 | 17,3 / 18,3 | 143,9 / 92,6 | 142,5 / 102,3 | 160,3 / 100,4 |

Il primo screenshot dopo il caricamento non è in generale più lento dei successivi (il seek del
fotogramma 0 invece sì: 15-37 ms contro 6-11): niente costo di avvio significativo da ammortizzare.

### Miglioramento di V1 su V0 (medie per ripetizione appaiate; + = più veloce; min-max sulle 3 rip.)

| scena | shot | ciclo |
|---|---|---|
| vuota 720p | +4,7% (+4,4 / +5,1) | +0,2% (-0,0 / +0,3) |
| livello1 720p | +7,1% (+6,1 / +8,7) | +0,6% (+0,2 / +1,3) |
| about:blank 720p | +2,5% (-1,7 / +7,0) | +2,3% (-1,8 / +6,8) |
| vuota 4K | +23,8% (+22,8 / +25,6) | +21,5% (+20,0 / +22,6) |
| cattedrale 4K | +34,7% (+34,6 / +34,9) | +32,0% (+31,4 / +32,4) |
| voxel 4K | +37,4% (+36,2 / +38,8) | +34,5% (+33,3 / +35,8) |
| about:blank 4K | +24,4% (+24,2 / +24,5) | +23,9% (+23,7 / +24,0) |

Il «ciclo» a 720p è invariato perché in V1 il seek costa mediamente 2-4 ms in più (10,8 contro 8,3 e
9,7 contro 5,9) e annulla il guadagno sullo screenshot; il seek non è stato modificato e questa
differenza non è stata indagata. Effetto collaterale: il PNG cresce (+38% su cattedrale e voxel, 4x
su pagina vuota 4K, 29 → 124 KiB, +50% su livello1); base64, ricezione e JSON restano comunque sotto
il 3% del ciclo.

### Riferimenti

* Andata e ritorno di `Runtime.evaluate` di `1`: p50 0,16-0,20 ms in tutti i run (la pipe CDP non è un
  collo di bottiglia).
* Intervallo tra fotogrammi del browser (30 `requestAnimationFrame` concatenati): 13,40-13,41 ms
  (~74,6 Hz) in tutti i run. Andata e ritorno di una Promise su rAF: p50 13,4 ms.
* Lo screenshot di V0 a 720p (59-61 ms) vale circa 4,4-4,6 intervalli; about:blank 720p 45,7 ms
  (3,4 intervalli); a 4K 117-159 ms.

### Memoria (misura approssimata)

Working set sommato dei processi `msedge` con il profilo del run (PowerShell/`Win32_Process`), prima e
dopo il ciclo: ~700-800 MiB prima, ~770-840 MiB dopo a 720p, ~830-930 MiB a 4K. Nessuna differenza
percepibile tra V0 e V1 (variazioni entro ~30-40 MiB, comparabili con quelle tra ripetizioni).

### Confronto dei pixel

* **Pagina vuota (720p e 4K) e livello1 (720p)**: V1 identico a V0 sui pixel in 60/60 e 30/30
  fotogrammi per tutte le 3 ripetizioni (i PNG non sono identici byte per byte perché la compressione
  è diversa). V0 e V1 sono ripetibili tra istanze del browser (rep 0 contro rep 1 e rep 2: 100% identici).
* **cattedrale**: V0 vs V1 identici in 24/30 fotogrammi; nei fotogrammi 18-23 differiscono 562-719
  pixel di 8,3 M, con differenza massima 1 per canale, sparsi lungo i bordi sfumati degli anelli
  (immagine diff x32: `...\cattura\completo\pixel\diff_cattedrale_V0_vs_V1_(rep_0)_018.png`, quasi
  nera con archi tenui). **Ma V0 non è deterministico**: V0 rep 0 e V0 rep 1 differiscono negli stessi 6
  fotogrammi con gli stessi conteggi (719, 569, 696, 562, 619, 656); V0 rep 0 = V0 rep 2 (30/30). Gli
  hash dei pixel (ffmpeg `-f hash`) mostrano due classi di output per fotogramma, indipendenti dalla
  variante: V1 rep 0 = V0 rep 1 = C rep 0 e V0 rep 0 = V1 rep 1 = V0 rep 2. Per tutti i 90 fotogrammi
  V1 di cattedrale, i pixel coincidono con quelli di almeno un run V0/C: nessuna differenza
  attribuibile a V1.
* **voxel**: V0 vs V1 identici in 28, 25 e 26 fotogrammi su 30; differenze di 1-107 pixel, massimo 2
  per canale (fotogrammi 8, 9, 24, 25, 26, 29). Anche V0 contro V0 differisce (rep 0 vs rep 2:
  fotogrammi 9 e 25, 107 e 1 pixel). Su 90 fotogrammi V1, 7 hanno pixel mai visti nei 9 run V0/C
  (max 2 per canale, al più 15 pixel su 8,3 M): con 3 ripetizioni di V0 non si può escludere né provare
  che V1 aggiunga variazioni; l'ordine di grandezza è quello della variabilità di V0.
* Causa plausibile (ipotesi): arrotondamenti non deterministici del raster/composizione (GPU o
  multithread) su bordi sfumati e gradienti, dipendenti dall'istanza del browser, non dalla
  codifica PNG (V1 comprime con un altro compressore ma i pixel sono gli stessi).
* Il criterio «identico bit a bit» non è quindi applicabile a V0 stessa sulle scene 4K dense: il
  criterio corretto è «entro la variabilità tra istanze» (massimo 1-2 livelli su pochi pixel).

### Conclusioni

**Fatti misurati**
* A 4K `Page.captureScreenshot` resta oltre l'88% del ciclo (`primo_byte` 117-159 ms); invio,
  ricezione, JSON, copie e base64 sommano 0,1-5 ms (meno del 3%). Il trasferimento WebSocket e le
  copie non sono il problema. La pipe CDP costa 0,16 ms andata e ritorno.
* `optimizeForSpeed:true` è accettato da Edge 157 e riduce il ciclo a 4K di 21-35% (cattedrale
  164,2 → 111,6 ms, voxel 173,4 → 113,6 ms, pagina vuota 129,0 → 101,3 ms), stabile tra ripetizioni
  (min-max entro ~3 punti); il guadagno sta tutto in `primo_byte`.
* A 720p il ciclo non cambia (+0,2% e +0,6%): lo screenshot guadagna 2-4 ms ma il seek ne perde altrettanti.
* `format:"jpeg"` a 4K dà lo stesso guadagno di V1 (cattedrale 114,6 contro 111,6 ms): il costo
  eliminato da V1 a 4K è la compressione PNG. WEBP è più lento di V0 su scene reali.
* Il pavimento a 720p è alto e indipendente da contenuto e codifica: about:blank 45,7 ms, pagina vuota
  58,8, scena reale 61,1; JPEG e V1 non lo abbassano. Uno screenshot consecutivo senza nuovo seek
  (C) è circa 17 ms più veloce (41,9 contro 58,8 ms su pagina vuota), circa 1,3 intervalli di
  13,4 ms in meno.
* Il PNG di V1 è più grande (+38% a 4K con scene reali, fino a 4x su pagina vuota): irrilevante per
  il costo (meno di 1 ms), ma allarga i buffer verso ffmpeg.
* La memoria del browser non cambia in modo percepibile. 0 errori e 0 timeout in 87 run.
* Il pixel output non è bit a bit riproducibile nemmeno con V0 su cattedrale e voxel (±1-2 livelli su
  al più 719 pixel in alcuni fotogrammi).

**Ipotesi (non provate)**
* A 720p il costo (~45-60 ms, 3-4,5 intervalli di 13,4 ms) è un'attesa di fotogrammi del
  compositor/BeginFrame: coerente con l'about:blank a 45 ms e con C più veloce dopo un seek, non con
  codifica o trasferimento. Non verificato con una traccia del browser.
* La variabilità dei pixel nasce da arrotondamenti del raster dipendenti dall'istanza.
* La maggiore latenza del seek con V1 (+2-4 ms a 720p) potrebbe essere rumore di ordinamento o un
  effetto di sincronizzazione; non indagata.

**Raccomandazione**: adottare V1 per il render: beneficio solo sulle scene 4K (-21/-35% di ciclo), neutro
a 720p. La decisione spetta ad `@auditor`/`@exporter`, dopo aver accettato la tolleranza di ±1-2 livelli
su pochi pixel (V0 stessa non è deterministica) e dopo un render completo di verifica per confrontare gli
MP4. Il codice di produzione non è stato toccato.

**Prossimo esperimento** (il pavimento di 45-60 ms a 720p non è toccato da V1): controllare il
rendering a fotogrammi in headless (`HeadlessExperimental.beginFrame` / `--enable-begin-frame-control`)
per catturare senza attendere il successivo fotogramma del compositor; misurare con lo stesso harness
(V0 contro nuova variante, stessi istanti, stessi confronti di pixel). In subordine, screenshot in
parallelo su più target come cambiamento d'architettura.

### Esperimento S — stessa superficie (risolutivo sui pixel di V1)

Test `esperimento_stessa_superficie` (stesso comando, 3 ripetizioni con browser nuovo; ~4 minuti).
Per ogni istante: un solo `__joy_seek(t)`, poi tre screenshot consecutivi senza altri seek, V0, V1, V0
(ripetizioni 0 e 2) oppure V1, V0, V1 (ripetizione 1). PNG decodificati con ffmpeg in RGBA e confrontati
byte per byte. 0 errori.

| scena | terne (V0,V1,V0 / V1,V0,V1) | stesse varianti 1a == 3a (superficie stabile) | coppie V0-V1 identiche | pixel diversi / max canale |
|---|---|---|---|---|
| voxel 4K (30 istanti) | 60 / 30 | 60/60 e 30/30 | 120/120 e 60/60 | 0 / 0 |
| cattedrale 4K (30 istanti) | 60 / 30 | 60/60 e 30/30 | 120/120 e 60/60 | 0 / 0 |
| livello1 720p (60 istanti) | 120 / 60 | 120/120 e 60/60 | 240/240 e 120/120 | 0 / 0 |

Tempi dello screenshot (ms, media; il primo dopo il seek, i successivi sulla stessa superficie):

| scena | V0,V1,V0 | V1,V0,V1 |
|---|---|---|
| voxel 4K | 172,7 / 99,0 / 158,1 | 104,3 / 155,9 / 93,7 |
| cattedrale 4K | 173,2 / 103,0 / 163,3 | 110,1 / 155,6 / 100,3 |
| livello1 720p | 63,7 / 42,0 / 50,8 | 57,5 / 50,0 / 42,9 |

**Fatti misurati**
* Sulla stessa superficie V0a == V0b e V1 == V0 in 100% delle terne (330 terne, 660 coppie V0-V1, 0 pixel
  diversi): `optimizeForSpeed` non cambia i pixel decodificati; cambia solo il compressore PNG.
* Sulla stessa superficie la superficie è stabile (V0a == V0b sempre): le differenze tra run
  della Fase 3 vengono dalla superficie prodotta dal seek/raster in istanze diverse, non dalla cattura.
* V1 resta più veloce anche in seconda e terza posizione (4K: circa 94-110 ms contro 156-173 ms di V0;
  a 720p 42-57 contro 50-64 ms): il guadagno non dipende dall'ordine.
* A 720p lo screenshot ripetuto sulla stessa superficie costa meno (V0: 63,7 → 50,8 ms; vedi anche la
  sonda C).

### Non determinazione di V0 tra istanze (voxel 4K, 5 istanze)

* In questa serie: 5 istanze nuove, V0 sugli stessi 30 istanti: **1 sola classe di pixel per ogni
  fotogramma** (30/30), differenza massima 0. Nessuna variazione osservata.
* Nel run principale della Fase 3 (stesse scene, istanze avviate dopo altre varianti) V0 aveva invece
  prodotto più classi: cattedrale 2 classi in 6 fotogrammi su 30 (massimo 1 livello, 562-719 pixel),
  voxel fino a 107 pixel e massimo 2 livelli. Quindi la non determinazione c'è, ma è **intermittente**:
  non si presenta in ogni serie di istanze.
* Ipotesi (non verificate): la variazione dipende dallo stato del sistema o della GPU al momento
  del raster (carico, ordine e tipo dei run precedenti, cache di shader), non dal contenuto della
  pagina né dal codec. Un'istanza per volta in serie «pulita» non l'ha mostrata; il run con 5 varianti
  consecutive (incluse JPEG/WEBP) sì.
* Conseguenza per la documentazione: la frase sul determinismo di B.9.5 va ridimensionata («stesso
  browser, stessa macchina: i pixel possono differire di 1-2 livelli su pochi pixel tra istanze»).

### Conclusione sulla correttezza di V1

**V1 è identico ai pixel sulla stessa superficie: sì** (0 pixel diversi in 660 coppie, su voxel,
cattedrale e livello1). Le differenze osservate tra V0 e V1 nei run separati sono la non
determinazione intermittente tra istanze, già presente in V0 contro V0.

### Validazione end-to-end (prima/dopo)

Binario PRIMA: worktree di HEAD `7294965` (rimosso a fine prova), `cargo build --release` con
`CARGO_TARGET_DIR` separato. DOPO: albero di lavoro con `PARAMETRI_SCREENSHOT` (`optimizeForSpeed:true`).
Render reali con `--profilo-json`, 3 ripetizioni per binario, ordine alternato (P,D / D,P / P,D), tre
scene: `livello1_motion` (361 fotogrammi), `cattedrale --fps 6` (181), `JOYPP_VOXEL_OVERDRIVE_v021 --fps 4`
(193). Nessun render 4K/60 completo. 18 render, tutti con exit 0: **0 errori, 0 timeout**. File in
`...\scratchpad\e2e`.

| scena | bin | totale s | fps_ciclo | screenshot media ms | screenshot p95 ms | screenshot max ms | PNG medio KiB | MP4 byte | finalizz. ffmpeg ms |
|---|---|---|---|---|---|---|---|---|---|
| livello1 720p | prima | 25,87 (25,26-26,81) | 14,35 | 62,49 | 68,05 | 106,7 (76,6-154,8) | 36,5 | 634841 | 138,2 |
| livello1 720p | dopo | 25,10 (25,06-25,16) | 14,84 | 57,40 | 61,46 | 81,5 (68,0-91,7) | 57,3 | 623097 (599610-634841) | 149,3 |
| cattedrale 4K | prima | 34,56 (32,02-36,30) | 5,50 | 169,66 | 220,18 | 239,5 (205,5-264,2) | 355,0 | 4348206 | 850,3 |
| cattedrale 4K | dopo | 23,24 (21,88-24,16) | 8,31 | 107,82 | 129,45 | 146,7 (145,1-149,4) | 479,0 | 4339662 | 715,4 |
| voxel 4K | prima | 38,21 (36,29-39,40) | 5,27 | 176,62 | 221,13 | 262,7 (251,9-270,3) | 418,8 | 4085024 | 863,0 |
| voxel 4K | dopo | 25,78 (23,90-26,81) | 7,99 | 110,86 | 130,67 | 144,7 (135,2-155,9) | 578,1 | 4095321 | 860,3 |

Miglioramento di DOPO su PRIMA (appaiato per ripetizione, media con min / max; + = più veloce):

| scena | totale | ciclo per fotogramma | screenshot medio | fps_ciclo |
|---|---|---|---|---|
| livello1 720p | +2,9% (+0,8 / +6,1) | +3,3% (+1,0 / +6,6) | +8,1% (+7,0 / +10,2) | +3,5% (+1,0 / +7,0) |
| cattedrale 4K | +32,7% (+31,7 / +33,4) | +33,8% (+32,5 / +34,6) | +36,4% (+35,3 / +37,4) | +51,1% (+48,2 / +53,0) |
| voxel 4K | +32,6% (+31,2 / +34,1) | +34,0% (+32,5 / +35,7) | +37,3% (+35,7 / +39,1) | +51,6% (+48,2 / +55,5) |

**Fatti misurati (prestazioni)**
* Il guadagno reale end-to-end a 4K è del 32-34% sul totale (fps_ciclo +51%), in linea con la stima
  della Fase 3 (-21/-35% del ciclo). A 720p è di +2,9% (min +0,8%).
* p95 e massimo dello screenshot scendono a 4K (p95 220 → 129 ms e 221 → 131 ms; max 240-263 → 145-147 ms).
  Il PNG medio cresce (36,5 → 57,3 KiB a 720p, +57%; +35% e +38% a 4K) ma l'MP4 resta
  praticamente uguale (±0,2% a 4K; a 720p -1,9% in media, intervallo 599610-634841 byte).
* La finalizzazione di ffmpeg non cambia in modo significativo (138 → 149 ms, 850 → 715, 863 → 860).

**Equivalenza dei video: NON bit a bit, e non per colpa di V1**

Confronto con `ffmpeg -f framemd5` dei fotogrammi decodificati, stessa ripetizione P/D:

| scena | rep 0 | rep 1 | rep 2 |
|---|---|---|---|
| livello1 | 361/361 identici | 32/361 | 361/361 |
| cattedrale | 132/181 | 25/181 | 32/181 |
| voxel | 193/193 | 7/193 | 193/193 |

Dove i video differiscono la differenza è grande (centinaia di migliaia di pixel diversi nei 4K, canale
fino a 68), perché una piccola differenza nei fotogrammi di ingresso viene propagata e amplificata
dal codificatore H.264 (differenze P/D iniziano a un fotogramma preciso e restano fino alla fine).
I fotogrammi diversi cadono sempre dove anche P contro P (o D contro D) differisce tra ripetizioni:

* Classi di video identici (hash complessivo dei fotogrammi) in 6 video per scena:
  livello1 [P0 P1 P2 D0 D2] [D1]; cattedrale [P0] [P1] [P2 D0 D1] [D2]; voxel [P0 D0] [P1] [P2 D1 D2].
  Ogni video D tranne livello1 D1 e cattedrale D2 è identico a qualche video P; le classi non seguono il binario.
* P contro P: cattedrale P0 vs P1 diversi in 156/181 fotogrammi (dal fotogramma 2); voxel P0 vs P1 in 190/193
  (dal 3). D contro D: cattedrale D0 vs D2 in 149/181; voxel D0 vs D1 in 190/193. Stessa entità
  (fino a centinaia di migliaia di pixel, canale 49-68 dopo la codifica) delle differenze P/D.
* Per ogni coppia di video diversi, i fotogrammi P/D non spiegati dalla non determinazione tra
  istanze della stessa variante: nessuno (lista vuota) in tutte e tre le scene.

**Test causale a monte del codificatore** (`esperimento_istanze_livello1`: 4 istanze per variante, 361 istanti
come in produzione, PNG grezzi senza ffmpeg): in una prova V0 aveva 254 fotogrammi su 361 con più classi
di PNG tra istanze e V1 nessuno; ripetendo la prova **entrambe** le varianti hanno 254 fotogrammi con
più classi (dal fotogramma 55), con le **stesse** istanze «minoritarie» (0 e 3) e gli stessi conteggi
di pixel (954, 1264, 3240, 5408, 7563, 8399 ... pixel, max canale 22-95) in V0 e V1. Quindi:
* La non determinazione è nel rendering della pagina (due classi di superfici per gli stessi istanti)
  e colpisce V0 e V1 allo stesso modo, in modo intermittente (varia tra prove).
* Non è uno scarto di un solo fotogramma: la classe minoritaria differisce dalla maggioritaria
  di k più poco che da k-1 o k+1 (es. fotogramma 55: 954 px contro 10178 e 12766), cioè è una
  differenza di tempo molto inferiore a un intervallo di 16,7 ms oppure di una proprietà non legata al tempo.
* Magnitudine: fino a 95 livelli per canale su circa 8400 pixel a 720p, molto più degli
  ±1-2 livelli visti in precedenza a 4K: il caso va indagato da `@renderer`/`@compiler` (non dall'esportatore).

**Ipotesi (non provate)**: lo stato dell'animazione sul compositor non coincide sempre con
quello impostato da `__joy_seek` al momento della cattura (race tra commit e raster), oppure le
animazioni con iterazioni/riempimento hanno uno stato dipendente dalla storia dei seek e dal tempo di
parete; il codice di `__joy_seek` è congelato e non è stato modificato.

**Conclusioni**
* V1 migliora il tempo di render del 32-34% a 4K (totale) e del 2,9% a 720p, senza errori né timeout,
  e riduce p95 e massimo dello screenshot a 4K.
* L'equivalenza bit a bit dei video P/D NON è dimostrata e non è raggiungibile: nemmeno P contro P lo è.
  Tutte le differenze osservate P/D sono comprese nella non determinazione tra istanze già presente
  nel binario PRIMA; a parità di superficie V1 è identico ai pixel (esperimento S).
* La frase sul determinismo di B.9.5 va corretta, e la non determinazione del rendering (più grande
  di quanto stimato prima) va aperta come problema separato.
