# `tests/repro/` — Riproduzioni minime di bug risolti

Ogni file `.joy` inizia con una riga che dichiara l'esito atteso, verificata da
`tests/end_to_end.rs::file_di_regressione_in_repro`:

- `# atteso: ok`: deve compilare;
- `# atteso: errore riga N`: deve fallire alla riga N.
