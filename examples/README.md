# `examples/` — Programmi Joy++ di esempio

Compilazione: `cargo run -- examples/primo_test.joy -o output.html`, poi apri `output.html`
nel browser.

- `primo_test.joy`: primo esempio della v0.1, con palco, attori e una timeline semplice.
- `livello1_motion.joy`: sigla di motion graphics "JOY++" di 6 s (19 attori), stress test creativo delle sole funzioni della v0.1.
- `stato_iniziale.joy`: stato iniziale dichiarativo della v0.2 (`scala` e `rotazione` alla creazione).
- `sfida_hyperframe_20s.joy`: sfida "Astrolabio" 4K (3840×2160 a 60 fps) di circa 14 s, con cornice HUD, ghiera di cerchi e tacche, gruppi annidati, `orbita`, testi e `cambia colore`; chiusura finale tra 13 s e 14,2 s. Il nome del file è rimasto invariato (non riflette più la durata).
- `cattedrale.joy`: stress test 4K (3840×2160 a 60 fps) di 30 s con 164 attori e 5 atti: griglia architettonica, gruppi, `orbita`, `cambia colore` e testi. Limite noto: l'orbita di "sistema_orbitale" fa uscire dal centro l'anello dei satelliti.
- `JOYPP_VOXEL_OVERDRIVE_v021.joy`: stress test narrativo 4K (3840×2160 a 60 fps) di 48 s, "La cittadella del tempo", con geometria pseudo-voxel costruita da rettangoli, gruppi, `orbita`, HUD e `cambia colore` applicato alle facce `_front`. Nessuna mesh 3D o particella.
- `sistema_orbitale.joy`: scena v0.2 del blocco 2B con gruppi annidati ("sistema" contiene "sole" e il sottogruppo "pianeta" con terra e luna), `orbita`, `entra dall'alto` / `entra da sinistra`, `cambia colore` ed `esce verso destra`. Compila senza avvisi e senza salti. Il centro di un'orbita è statico (B.4.1): per questo la terra e la luna stanno nel sottogruppo "pianeta", che orbita attorno al sole (raggio 214 = distanza tra il pivot di "pianeta" e il sole), mentre la luna orbita attorno a una terra ferma dentro il sottogruppo. Verificato anche con il mini-player (presente di default, assente con `--senza-player`) e con `joypp render examples/sistema_orbitale.joy --fps 10` (91 fotogrammi, 1280×720, h264/yuv420p, circa 7 s).
