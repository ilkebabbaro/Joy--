# `src/exporter` — Packaging, I/O e CLI (`@exporter`)

Collega la pipeline al file system.

- `compila_file`: legge un `.joy` (con limite di dimensione), invoca
  `parser → compiler → codegen` e scrive il `.html` risultante.
- Opzioni: percorso di output (`-o`), output leggibile (`--pretty`, default: compatto),
  modalità *watch* (`--watch`: ricompila quando il file sorgente cambia, tramite polling
  dell'orario di modifica, senza dipendenze esterne).

La CLI vera e propria (`src/main.rs`) si limita a interpretare gli argomenti e a
delegare a questo modulo.
