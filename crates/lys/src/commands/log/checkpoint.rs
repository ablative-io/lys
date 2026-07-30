//! `lys log checkpoint` — sign a C2SP signed-note checkpoint over the
//! log's current root.
//!
//! Checkpointing is a distinct signing act, separate from appending: the
//! identity key is loaded via the load-only path (never minted here), the
//! note is signed under the origin as the key name (the origin binding
//! third-party verifiers enforce), and the verifier key string is printed
//! so the operator can hand the trust anchor to a third party in the same
//! breath.

use std::path::Path;

use lys_core::checkpoint::{CheckpointBody, NoteVerifierKey, sign_note};

use crate::commands::error::CliResult;
use crate::commands::files::write_file;
use crate::commands::hex::hex_lower;
use crate::commands::key::load_identity;
use crate::commands::log::store;
use crate::commands::output::Emitter;

/// `lys log checkpoint --dir <log-dir> --key <keyfile> --out <file>`.
///
/// # Errors
///
/// Returns [`CliError::LogDirMissing`] / [`CliError::LogDirInvalid`] if the
/// log directory is absent or fails its integrity check,
/// [`CliError::KeyFileMissing`] if the key file does not exist,
/// [`CliError::Trust`] if signing fails, and [`CliError::Io`] if the note
/// cannot be written.
///
/// [`CliError::LogDirMissing`]: crate::commands::error::CliError::LogDirMissing
/// [`CliError::LogDirInvalid`]: crate::commands::error::CliError::LogDirInvalid
/// [`CliError::KeyFileMissing`]: crate::commands::error::CliError::KeyFileMissing
/// [`CliError::Trust`]: crate::commands::error::CliError::Trust
/// [`CliError::Io`]: crate::commands::error::CliError::Io
pub fn run(dir: &Path, key: &Path, out: &Path, json: bool) -> CliResult<()> {
    let log = store::open(dir)?;
    let identity = load_identity(key)?;
    let body = CheckpointBody::from_root(log.origin(), &log.tree().root())?;
    let note = sign_note(&body.encode(), log.origin(), &identity)?;
    write_file(out, note.as_bytes(), "checkpoint note file")?;
    let verifier = NoteVerifierKey::new(log.origin(), identity.public_key_bytes())?;
    let mut emit = Emitter::new(json);
    emit.field("origin", "origin", body.origin());
    emit.field("tree size", "tree_size", body.tree_size());
    emit.field(
        "root hash (sha256)",
        "root_hash",
        hex_lower(&body.root_hash()),
    );
    emit.field(
        "checkpoint written",
        "checkpoint_path",
        out.display().to_string(),
    );
    emit.field(
        "verifier key (signed-note)",
        "verifier_key",
        verifier.to_spec(),
    );
    emit.finish();
    Ok(())
}
