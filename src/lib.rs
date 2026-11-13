//! # Joy++
//!
//! Compilatore del linguaggio dichiarativo **Joy++** (file `.joy`): frasi in
//! italiano naturale vengono tradotte in un singolo file HTML5 auto-contenuto
//! con animazioni CSS3 accelerate via GPU.
//!
//! La grammatica ufficiale è definita in `SPECIFICA.md`.

#![forbid(unsafe_code)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]

pub mod ast;
pub mod codegen;
pub mod compiler;
pub mod errore;
pub mod exporter;
pub mod parser;
pub mod security;

pub use errore::{Avviso, Errore, Risultato};
pub use exporter::{Opzioni, compila_sorgente};
