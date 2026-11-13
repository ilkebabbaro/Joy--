# Changelog

Tutte le modifiche rilevanti a questo progetto sono documentate in questo file.
Il formato segue [Keep a Changelog](https://keepachangelog.com/it/1.1.0/) e il
progetto aderisce al [Semantic Versioning](https://semver.org/lang/it/).

## [Unreleased]

### Aggiunto
- AST (`src/ast`): `Programma`, `Palco`, `Attore` (cerchio, rettangolo/quadrato, testo/titolo),
  `Posizione`, `Timeline`, `Blocco`, `Azione`, `Verbo`, `Easing`; errore unico `Errore`
  con riga e suggerimento (`src/errore.rs`).
- Struttura iniziale del progetto Rust `joypp` (libreria + binario), repository Git
  e alberatura dei moduli (`parser`, `ast`, `compiler`, `codegen`, `security`, `exporter`)
  con relativi `README.md`.
