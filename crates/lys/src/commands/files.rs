//! Filesystem helpers shared by subcommands.
//!
//! Thin wrappers over `std::fs` that attach the failing path and a
//! description of what the file was for, so every I/O failure surfaces as an
//! actionable [`CliError::Io`].
//!
//! Outputs that must never be torn or silently replaced go through
//! [`StagedFile`]: written in full to a fresh temporary file beside the
//! target, flushed to disk, and only then renamed over the target's name.
//! [`refuse_existing`] and [`refuse_shared_paths`] let a command refuse,
//! before it does anything irreversible, an output that would overwrite a
//! file or collide with another output of the same run.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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

/// Refuses an output path that already names something on disk, so a run
/// never silently replaces a file it did not write.
///
/// A dangling symbolic link counts as existing: renaming over it would replace
/// the link, which is still a file the run did not write.
///
/// # Errors
///
/// [`CliError::OutputExists`] naming the output and its path, or
/// [`CliError::Io`] if the path cannot be inspected at all.
pub fn refuse_existing(path: &Path, what: &'static str) -> CliResult<()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Err(CliError::OutputExists {
            what,
            path: path.to_path_buf(),
        }),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(CliError::Io {
            context: format!("failed to check whether {what} {} exists", path.display()),
            source,
        }),
    }
}

/// Refuses two outputs of one run that name the same path, after making each
/// path absolute against the working directory, so one output can never
/// overwrite another.
///
/// # Errors
///
/// [`CliError::OutputPathShared`] naming both outputs and the path, or
/// [`CliError::Io`] if a path cannot be made absolute.
pub fn refuse_shared_paths(outputs: &[(&'static str, &Path)]) -> CliResult<()> {
    let mut seen: Vec<(&'static str, PathBuf)> = Vec::with_capacity(outputs.len());
    for &(what, path) in outputs {
        let absolute = std::path::absolute(path).map_err(|source| CliError::Io {
            context: format!("failed to resolve {what} path {}", path.display()),
            source,
        })?;
        if let Some((first, _)) = seen.iter().find(|(_, earlier)| *earlier == absolute) {
            return Err(CliError::OutputPathShared {
                first,
                second: what,
                path: path.to_path_buf(),
            });
        }
        seen.push((what, absolute));
    }
    Ok(())
}

/// Distinguishes the temporary files one process stages, so two stagings of
/// the same target never share a name.
static STAGED_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Bytes written in full and flushed to disk beside their target, waiting to
/// be renamed onto it by [`StagedFile::place`].
///
/// The temporary file is created new in the target's own directory, so the
/// rename never crosses a filesystem, and it is `sync_all`ed before
/// [`StagedFile::stage`] returns. A crash therefore leaves the target either
/// absent or whole, never torn. A staged file dropped without being placed
/// removes its temporary file.
#[derive(Debug)]
pub struct StagedFile {
    temporary: PathBuf,
    target: PathBuf,
    what: &'static str,
    placed: bool,
}

impl StagedFile {
    /// Writes `contents` to a new temporary file beside `target` and flushes
    /// it to disk. The target itself is not touched.
    ///
    /// # Errors
    ///
    /// [`CliError::Io`] naming `what` and the target if the target has no
    /// file name, or the temporary file cannot be created, written or flushed
    /// (a missing parent directory surfaces here).
    pub fn stage(target: &Path, contents: &[u8], what: &'static str) -> CliResult<Self> {
        let failed = |verb: &str, source: std::io::Error| CliError::Io {
            context: format!("failed to {verb} {what} {}", target.display()),
            source,
        };
        let name = target.file_name().ok_or_else(|| {
            failed(
                "stage",
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "the path names no file"),
            )
        })?;
        let directory = target
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let sequence = STAGED_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = directory.join(format!(
            ".{}.{}-{sequence}.tmp",
            name.to_string_lossy(),
            std::process::id()
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|source| failed("stage", source))?;
        let staged = Self {
            temporary,
            target: target.to_path_buf(),
            what,
            placed: false,
        };
        file.write_all(contents)
            .map_err(|source| failed("write", source))?;
        file.sync_all().map_err(|source| failed("flush", source))?;
        Ok(staged)
    }

    /// Renames the flushed temporary file onto the target.
    ///
    /// # Errors
    ///
    /// [`CliError::Io`] naming the output and its target if the rename fails;
    /// the temporary file is then removed.
    pub fn place(mut self) -> CliResult<()> {
        std::fs::rename(&self.temporary, &self.target).map_err(|source| CliError::Io {
            context: format!("failed to place {} {}", self.what, self.target.display()),
            source,
        })?;
        self.placed = true;
        Ok(())
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        if self.placed {
            return;
        }
        if let Err(err) = std::fs::remove_file(&self.temporary) {
            eprintln!(
                "could not remove the unplaced temporary {} {}: {err}",
                self.what,
                self.temporary.display()
            );
        }
    }
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
