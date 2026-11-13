# `examples/` — Programmi Joy++ di esempio

Compilazione: `cargo run -- examples/primo_test.joy -o output.html`, poi apri `output.html`
nel browser.

- `primo_test.joy`: primo esempio della v0.1, con palco, attori e una timeline semplice.
- `livello1_motion.joy`: sigla di motion graphics "JOY++" di 6 s (19 attori), stress test creativo delle sole funzioni della v0.1.
- `stato_iniziale.joy`: stato iniziale dichiarativo della v0.2 (`scala` e `rotazione` alla creazione).
- `sistema_orbitale.joy`: scena v0.2 del blocco 2B con gruppi annidati ("sistema" contiene "sole" e il sottogruppo "pianeta" con terra e luna), `orbita`, `entra dall'alto` / `entra da sinistra`, `cambia colore` ed `esce verso destra`. Compila senza avvisi e senza salti. Il centro di un'orbita è statico (B.4.1): per questo la terra e la luna stanno nel sottogruppo "pianeta", che orbita attorno al sole (raggio 214 = distanza tra il pivot di "pianeta" e il sole), mentre la luna orbita attorno a una terra ferma dentro il sottogruppo.
