# `tests/repro/` — Riproduzioni minime di bug risolti

Ogni file `.joy` inizia con una riga che dichiara l'esito atteso, verificata da
`tests/end_to_end.rs::file_di_regressione_in_repro`:

- `# atteso: ok`: deve compilare;
- `# atteso: errore riga N`: deve fallire alla riga N.

## Render (non verificati da `file_di_regressione_in_repro`)

- `profilo_json_uguale_al_video.joy`: regressione B1 di `--profilo-json`. Comando:
  `joypp render tests/repro/profilo_json_uguale_al_video.joy -o out.mp4 --profilo-json ./out.mp4`
  (atteso exit 1; vedi `tests/profilo.rs::binario_b1_*`).
