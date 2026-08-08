//! Filesystem helpers shared by subcommands.
//!
//! Thin wrappers over `std::fs` that attach the failing path and a description
//! of what the file was for, so every I/O failure surfaces as an actionable
//! [`CliError::Io`]. The shape is `crates/lys/src/commands/files.rs`'s; see
//! [`output`](super::output) for why the two cannot be one file.

use std::path::Path;

use serde::Serialize;

use crate::commands::error::{CliError, CliResult};

/// Read a file fully into memory, describing the file's role (`what`, e.g.
/// "genesis leaf file") and its path in any error.
pub fn read_file(path: &Path, what: &str) -> CliResult<Vec<u8>> {
    std::fs::read(path).map_err(|source| CliError::Io {
        context: format!("failed to read {what} {}", path.display()),
        source,
    })
}

/// Write bytes to a file, describing the file's role (`what`, e.g. "receipt
/// file") and its path in any error.
pub fn write_file(path: &Path, contents: &[u8], what: &str) -> CliResult<()> {
    std::fs::write(path, contents).map_err(|source| CliError::Io {
        context: format!("failed to write {what} {}", path.display()),
        source,
    })
}

/// Writes an artifact as pretty JSON with a trailing newline.
///
/// The same emit-side convention `lys log prove` uses, and cosmetic for the
/// same reason: the artifact carries no signature over its own JSON bytes, so
/// all of its integrity flows through the embedded signed note and the root
/// recomputation a reader performs. Pretty-printing therefore changes how it
/// reads and not whether it verifies.
pub fn write_artifact<T: Serialize>(
    path: &Path,
    artifact: &T,
    what: &'static str,
) -> CliResult<()> {
    let mut json = serde_json::to_string_pretty(artifact)
        .map_err(|source| CliError::JsonSerialize { what, source })?;
    json.push('\n');
    write_file(path, json.as_bytes(), what)
}
