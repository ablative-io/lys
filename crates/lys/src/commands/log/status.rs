//! `lys log status` — read the state of a local log without its signing key.
//!
//! Every other way of learning a log's current size and root hash goes
//! through `lys log checkpoint`, which signs — and therefore demands the
//! signing key and mutates nothing but still requires custody of the most
//! sensitive material an operator holds. Observing your own append-only log
//! should not require the ability to sign for it. This command reads the
//! stored leaves, recomputes the root, and prints what it found.
//!
//! It deliberately does **not** print the signed-note verifier key: that is
//! derived from the signing key, which this command never touches. Use
//! `lys log checkpoint` when a third party needs the verifier key string.
//!
//! Nothing here is a trust statement. The root is recomputed from local
//! bytes that the local operator controls, so it says what this directory
//! currently contains — not that anybody else has attested to it.

use std::path::Path;

use crate::commands::error::CliResult;
use crate::commands::hex::hex_lower;
use crate::commands::log::store::LogStore;

/// `lys log status --dir <log-dir>`.
///
/// # Errors
///
/// Returns [`CliError::LogDirMissing`] if `dir` holds no log,
/// [`CliError::LogDirInvalid`] if it holds one whose stored format or pinned
/// state does not match the rebuilt tree, and [`CliError::Io`] on filesystem
/// failures.
///
/// [`CliError::LogDirMissing`]: crate::commands::error::CliError::LogDirMissing
/// [`CliError::LogDirInvalid`]: crate::commands::error::CliError::LogDirInvalid
/// [`CliError::Io`]: crate::commands::error::CliError::Io
pub fn run(dir: &Path) -> CliResult<()> {
    let store = LogStore::open(dir)?;
    let (root, tree_size) = store.tree().root().to_parts();

    println!("log directory: {}", dir.display());
    println!("origin: {}", store.origin());
    println!("tree size: {tree_size}");
    println!("root hash (sha256): {}", hex_lower(&root));
    Ok(())
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
