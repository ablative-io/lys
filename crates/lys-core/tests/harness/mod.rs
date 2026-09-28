#![cfg(test)]
//! The shared Go-toolchain harness; its contract and code are in `go.rs`.

mod go;

pub use go::{build_go_tool, go_or_skip, run_built_tool};
