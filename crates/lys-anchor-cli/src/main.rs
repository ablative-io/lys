//! The `lys-anchor` executable: a shim over [`lys_anchor_cli::dispatch::run`].
//!
//! Deliberately empty of logic, and documented as `doc = false` in
//! `Cargo.toml`. `lys_anchor_cli`'s crate docs say why: a documented binary
//! target named `lys-anchor` writes to the same rustdoc directory as the
//! `lys-anchor` library, and the doc gate runs at zero warnings.

use std::process::ExitCode;

fn main() -> ExitCode {
    lys_anchor_cli::dispatch::run()
}
