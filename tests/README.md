# `tests/` — Test di integrazione (`@debugger`)

- `parser.rs`: almeno una prova per ogni parola chiave del parser e per i messaggi d'errore.
- `compiler.rs`: risoluzione posizioni/tempi, keyframe, conflitti tra canali.
- `sicurezza.rs`: tentativi di XSS/CSS injection e limiti anti-DoS, anche per i gruppi (nomi con payload o caratteri bidirezionali, annidamento profondo), `orbita` con giri enormi e due fuzzer deterministici (frammenti generici e frammenti v0.2).
- `end_to_end.rs`: compilazione completa degli esempi in `examples/` e della CLI.
- `v02_stato_iniziale.rs`: stato iniziale `scala`/`rotazione` (B.1), avvisi non bloccanti (B.6) e regressioni diagnostiche della v0.2.
- `v02_blocco2b.rs`: blocco v0.2 "2B" dalla frase `.joy` all'HTML: gruppi (B.3: sintassi, errori, pivot, pre-ordine, annidamento, profondità 16/17), `entra da`/`esce verso` (B.2, salto `steps(1,end)`, fuori campo prudente), `orbita` (B.4.1: verso, giri frazionari, campioni, salto, errori, `MAX_KEYFRAME`), `cambia colore` (B.4.2), l'esempio B.8 con i valori numerici attesi e la regressione byte per byte degli esempi della v0.1 (lunghezza e hash FNV-1a dell'HTML: se un'evoluzione voluta del generatore cambia l'output, aggiornare `ATTESI`).
- `repro/`: file `.joy` minimali che riproducono bug risolti (test di regressione).

Comandi: `cargo test` e `cargo clippy --all-targets -- -D warnings`.
