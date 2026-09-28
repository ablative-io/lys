//! One given path canonicalised by the record's rule (HOME-010 R1, ADR-029):
//! the path a `lys.given` entry records, and the path `given-check` compares,
//! so one file reached through a symlinked directory, a `..` or a trailing
//! slash is one path.
//!
//! A directory path (the working directory before the chain is walked, the
//! config directory) is resolved whole: every symlink, `..` and trailing
//! slash. A document path is its parent directory resolved whole with its
//! own final name joined unchanged, so a document that is itself a symlink
//! is named at its link position and a dangling symlink there counts as
//! existing; a document path whose final component is not a normal name
//! (`..`, or the root) is resolved whole as a directory path.
//!
//! A relative path (the two files the render wrote, named relative to its
//! out directory) is returned unchanged and never resolved against the
//! process's working directory. A path that cannot be resolved because it,
//! its parent or one of its components does not exist is returned byte for
//! byte as given and marked unresolved, never dropped and never an error.
//! Any other failure (permission denied, a loop of symlinks) is refused as
//! [`HomeError::Io`] naming the operation and the path as given, and nothing
//! else. No file's bytes are opened or read: only directory entries are.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::HomeError;

/// The operation a canonicalisation refusal names.
pub const CANONICALISING: &str = "canonicalising a given path";

/// What became of a given path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GivenPath {
    /// The path resolved by the rule.
    Canonical(PathBuf),
    /// The path as given, since it, its parent or a component of it does
    /// not exist.
    Unresolved(PathBuf),
    /// The path as given, since it is relative.
    Relative(PathBuf),
}

impl GivenPath {
    /// The path, whichever of the three it is.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Canonical(path) | Self::Unresolved(path) | Self::Relative(path) => path,
        }
    }

    /// The path, owned.
    #[must_use]
    pub fn into_path(self) -> PathBuf {
        match self {
            Self::Canonical(path) | Self::Unresolved(path) | Self::Relative(path) => path,
        }
    }

    /// Whether the path was left as given because it does not exist.
    #[must_use]
    pub fn is_unresolved(&self) -> bool {
        matches!(self, Self::Unresolved(_))
    }
}

/// Canonicalise a directory path whole: every symlink, `..` and trailing
/// slash resolved.
pub fn canonical_dir(path: &Path) -> Result<GivenPath, HomeError> {
    if !path.is_absolute() {
        return Ok(GivenPath::Relative(path.to_path_buf()));
    }
    match std::fs::canonicalize(path) {
        Ok(canonical) => Ok(GivenPath::Canonical(canonical)),
        Err(e) => unresolved_or_refused(path, e),
    }
}

/// Canonicalise a document path: its parent directory whole, its own final
/// name kept, and that entry's own existence read without following it.
pub fn canonical_document(path: &Path) -> Result<GivenPath, HomeError> {
    if !path.is_absolute() {
        return Ok(GivenPath::Relative(path.to_path_buf()));
    }
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return canonical_dir(path);
    };
    let parent = match std::fs::canonicalize(parent) {
        Ok(parent) => parent,
        Err(e) => return unresolved_or_refused(path, e),
    };
    let joined = parent.join(name);
    match std::fs::symlink_metadata(&joined) {
        Ok(_) => Ok(GivenPath::Canonical(joined)),
        Err(e) => unresolved_or_refused(path, e),
    }
}

/// The path as given, unresolved, when the failure is that something does
/// not exist; otherwise the refusal by operation and path.
fn unresolved_or_refused(path: &Path, e: std::io::Error) -> Result<GivenPath, HomeError> {
    if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory) {
        Ok(GivenPath::Unresolved(path.to_path_buf()))
    } else {
        Err(HomeError::io(CANONICALISING, path, e))
    }
}
