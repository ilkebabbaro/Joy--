//! Punto di ingresso della CLI `joypp` (implementazione completa a cura di `@exporter`).

#![forbid(unsafe_code)]

fn main() {
    println!("joypp {}", env!("CARGO_PKG_VERSION"));
}
