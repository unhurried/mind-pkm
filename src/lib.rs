//! Mind PKM: a personal knowledge management system around a Freeplane mind
//! map. The library holds everything; `main.rs` only starts the application,
//! so that integration tests and benchmarks can use the modules too.

pub mod app;
pub mod mindmap;
pub mod paths;
pub mod pkm;
pub mod search;
pub mod xml;
