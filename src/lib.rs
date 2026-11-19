// src/lib.rs
// Library crate root.
//
// The application logic lives in the library so that the binary (`main.rs`)
// and the benchmarks (`benches/`) can both depend on it.

pub mod ai;
pub mod commands;
pub mod core;
pub mod db;
pub mod settings;
pub mod state;
