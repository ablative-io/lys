//! Filesystem helpers shared by subcommands.
//!
//! Thin wrappers over `std::fs` that attach the failing path and a
//! description of what the file was for, so every I/O failure surfaces as an
//! actionable [`CliError::Io`](crate::commands::error::CliError::Io).

use std::path::Path;

use crate::commands::error::{CliError, CliResult};

/// Read a file fully into memory, describing the file's role (`what`, e.g.
/// "payload file") and its path in any error.
pub fn read_file(path: &Path, what: &str) -> CliResult<Vec<u8>> {
    std::fs::read(path).map_err(|source| CliError::Io {
        context: format!("failed to read {what} {}", path.display()),
        source,
    })
}

/// Write bytes to a file, describing the file's role (`what`, e.g.
/// "attestation file") and its path in any error.
pub fn write_file(path: &Path, contents: &[u8], what: &str) -> CliResult<()> {
    std::fs::write(path, contents).map_err(|source| CliError::Io {
        context: format!("failed to write {what} {}", path.display()),
        source,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn read_file_round_trips_written_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.bin");
        write_file(&path, b"payload bytes", "test file").unwrap();
        assert_eq!(read_file(&path, "test file").unwrap(), b"payload bytes");
    }

    #[test]
    fn read_file_error_names_role_and_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.bin");
        let err = read_file(&path, "payload file").unwrap_err();
        let display = err.to_string();
        assert!(display.contains("payload file"), "got: {display}");
        assert!(display.contains("missing.bin"), "got: {display}");
    }

    #[test]
    fn write_file_error_names_role_and_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-such-dir").join("out.json");
        let err = write_file(&path, b"x", "attestation file").unwrap_err();
        let display = err.to_string();
        assert!(display.contains("attestation file"), "got: {display}");
        assert!(display.contains("out.json"), "got: {display}");
    }
}
