//! Filesystem helpers shared by subcommands.
//!
//! Thin wrappers over `std::fs` that attach the failing path and a
//! description of what the file was for, so every I/O failure surfaces as an
//! actionable [`CliError::Io`].

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

/// Write bytes to a file created owner-readable only (mode `0600` on Unix),
/// for content that was confidential enough to arrive encrypted — e.g. the
/// plaintext recovered by `lys open`. On non-Unix platforms this is a plain
/// [`write_file`].
///
/// The mode is enforced unconditionally: `OpenOptions::mode` covers the
/// creation case, and permissions are set again after writing so that a
/// pre-existing file at `path` cannot donate looser permissions to the
/// plaintext. This mirrors `lys-core`'s identity-key write path, which
/// force-tightens the same way. (`lys-core` warns rather than tightens when
/// *loading* an over-permissive key — that is a different path, and the
/// earlier version of this comment cited it as precedent for not tightening
/// here, which was the wrong analog.)
pub fn write_file_private(path: &Path, contents: &[u8], what: &str) -> CliResult<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|source| CliError::Io {
                context: format!("failed to create {what} {}", path.display()),
                source,
            })?;
        file.write_all(contents).map_err(|source| CliError::Io {
            context: format!("failed to write {what} {}", path.display()),
            source,
        })?;
        // Creation-time mode does not apply to a file that already existed,
        // so tighten explicitly. Failing loudly here is deliberate: silently
        // leaving decrypted plaintext readable is the outcome this exists to
        // prevent, so it must not degrade into a warning.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|source| {
            CliError::Io {
                context: format!(
                    "failed to restrict permissions on {what} {}",
                    path.display()
                ),
                source,
            }
        })
    }
    #[cfg(not(unix))]
    {
        write_file(path, contents, what)
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
