# `examples/` — Programmi Joy++ di esempio

Compilazione: `cargo run -- examples/primo_test.joy -o output.html`, poi apri `output.html`
nel browser.

- `primo_test.joy`: primo esempio della v0.1, con palco, attori e una timeline semplice.
- `livello1_motion.joy`: sigla di motion graphics "JOY++" di 6 s (19 attori), stress test creativo delle sole funzioni della v0.1.
- `stato_iniziale.joy`: stato iniziale dichiarativo della v0.2 (`scala` e `rotazione` alla creazione).
- `sfida_hyperframe_20s.joy`: sfida di 20 s a 60 fps (1280×720) con gruppi, `orbita` (sistema sole/terra/luna, il gruppo "pianeta_luna" orbita con raggio 121 = distanza del suo pivot dal sole), testi e `cambia colore`. Compila senza avvisi e senza salti (nessun `steps(1,end)`); nei testi non esistono sequenze di escape, quindi i nomi citati usano gli apici tipografici ‘ ’. Verificato con `joypp render examples/sfida_hyperframe_20s.joy` (1201 fotogrammi, h264/yuv420p, circa 82 s).
- `sistema_orbitale.joy`: scena v0.2 del blocco 2B con gruppi annidati ("sistema" contiene "sole" e il sottogruppo "pianeta" con terra e luna), `orbita`, `entra dall'alto` / `entra da sinistra`, `cambia colore` ed `esce verso destra`. Compila senza avvisi e senza salti. Il centro di un'orbita è statico (B.4.1): per questo la terra e la luna stanno nel sottogruppo "pianeta", che orbita attorno al sole (raggio 214 = distanza tra il pivot di "pianeta" e il sole), mentre la luna orbita attorno a una terra ferma dentro il sottogruppo. Verificato anche con il mini-player (presente di default, assente con `--senza-player`) e con `joypp render examples/sistema_orbitale.joy --fps 10` (91 fotogrammi, 1280×720, h264/yuv420p, circa 7 s).
