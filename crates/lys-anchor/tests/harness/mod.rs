#![cfg(test)]
//! `lys-anchor`'s Go-toolchain harness; its contract, code and checks are in `go.rs`.

mod go;

pub use go::{GoScaffold, build_go_tool, go_or_skip, run_built_tool};
