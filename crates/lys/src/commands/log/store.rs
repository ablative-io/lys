//! The `lys log` commands' view of [`lys_log_store`].
//!
//! The log directory layout, the write-once rules, the Merkle tree and the
//! integrity routine all live in `lys-log-store`. What remains here is the two
//! things only the CLI can decide: how a storage failure should read to an
//! operator, and where a recovery notice goes.
//!
//! # Why the notice is printed here and not there
//!
//! A library that writes to stderr has decided for its caller how a repair gets
//! reported — and a caller embedding lys may want it in a log line, a span, or a
//! structured field. So [`lys_log_store::Log`] *returns* the fact and this layer
//! prints it. The one thing neither may do is drop it: a silently repaired log
//! is indistinguishable from one that never needed repairing.

use std::path::Path;

use lys_log_store::{FileLeafStore, Log, StoreError};

use crate::commands::error::{CliError, CliResult};

/// The `lys` CLI's log: a Merkle log over a directory of leaf files.
pub type LogStore = Log<FileLeafStore>;

/// Creates and initializes a log directory at `dir` with the given origin.
///
/// # Errors
///
/// [`CliError::Trust`] if the origin violates the checkpoint-origin rules,
/// [`CliError::LogDirInvalid`] if `dir` is already an initialized log, and
/// [`CliError::Io`] on filesystem failure.
pub fn init(dir: &Path, origin: &str) -> CliResult<()> {
    FileLeafStore::create(dir, origin)?;
    Ok(())
}

/// Opens and integrity-verifies the log directory at `dir`, reporting an
/// interrupted append that was repaired.
///
/// # Errors
///
/// [`CliError::LogDirMissing`] if `dir` is not an initialized log,
/// [`CliError::LogDirInvalid`] with the specific discrepancy on an integrity
/// failure, and [`CliError::Io`] on filesystem failure.
pub fn open(dir: &Path) -> CliResult<LogStore> {
    let log = Log::open(FileLeafStore::open(dir)?).map_err(|err| integrity_failure(dir, err))?;
    if let Some(tree_size) = log.recovered_to() {
        eprintln!("recovered interrupted append: state advanced to {tree_size}");
    }
    Ok(log)
}

/// Attaches the log directory to an integrity failure that carries no path.
///
/// [`StoreError::PinMismatch`] is deliberately path-free in the library: the
/// disagreement is between a set of leaves and a pin, wherever those live, and
/// a backend that is not a directory has no path to name. The CLI's leaves and
/// pin *do* live in a directory, and an operator's first question is which one —
/// so this is where the path goes back on, keeping the
/// `log directory invalid: <path>: <reason>` shape every other corruption of a
/// log directory reports.
fn integrity_failure(dir: &Path, err: StoreError) -> CliError {
    match err {
        err @ (StoreError::PinMismatch { .. } | StoreError::LeafMissingWithinExtent { .. }) => {
            CliError::LogDirInvalid {
                path: dir.to_path_buf(),
                reason: err.to_string(),
            }
        }
        other => other.into(),
    }
}

impl From<StoreError> for CliError {
    /// Translates a storage failure into the CLI's vocabulary.
    ///
    /// Three variants are mapped by hand because the CLI can say something the
    /// library cannot: an uninitialized directory gets the `lys log init`
    /// remedy, and re-initialization explains that the origin is pinned at
    /// init. The rest keep the store's own message, which already names the
    /// index or path involved.
    ///
    /// The catch-all is deliberate rather than lazy: `StoreError` is
    /// `#[non_exhaustive]`, and a new storage failure arriving in a future
    /// version must reach the operator verbatim instead of being refused a
    /// mapping at compile time and then hurried into whichever variant was
    /// closest.
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotInitialized { path } => Self::LogDirMissing { path },
            StoreError::AlreadyInitialized { path } => Self::LogDirInvalid {
                path,
                reason: "already initialized (log.json exists); the origin is pinned at init \
                         and a log directory is never re-initialized"
                    .to_string(),
            },
            StoreError::Corrupt { path, reason } => Self::LogDirInvalid { path, reason },
            StoreError::Io { context, source } => Self::Io { context, source },
            StoreError::Trust(source) => Self::Trust(source),
            other => Self::LogStore(other),
        }
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
