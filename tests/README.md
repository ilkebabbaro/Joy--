# `tests/` — Test di integrazione (`@debugger`)

- `parser.rs`: almeno una prova per ogni parola chiave del parser e per i messaggi d'errore.
- `compiler.rs`: risoluzione posizioni/tempi, keyframe, conflitti tra canali.
- `sicurezza.rs`: tentativi di XSS/CSS injection e limiti anti-DoS.
- `end_to_end.rs`: compilazione completa degli esempi in `examples/` e della CLI.
- `v02_stato_iniziale.rs`: stato iniziale `scala`/`rotazione` (B.1), avvisi non bloccanti (B.6) e regressioni diagnostiche della v0.2.
- `repro/`: file `.joy` minimali che riproducono bug risolti (test di regressione).

Comandi: `cargo test` e `cargo clippy --all-targets -- -D warnings`.
