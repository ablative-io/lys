//! `lys-anchor checkpoint` — sign a C2SP signed-note checkpoint over the
//! anchor's current root.
//!
//! Checkpointing is a distinct signing act, separate from submitting. The note
//! is signed under the anchor's own origin as the signed-note key name — the
//! origin binding a third-party verifier enforces, so one anchor's checkpoint
//! can never be accepted for another — and the verifier key string is printed so
//! the operator can hand the trust anchor to a third party in the same breath.
//!
//! **Publishing is not an append.** The log is untouched; a checkpoint that
//! appended would change the very tree it had just committed to. The note also
//! carries no timestamp, so two publications of an unchanged anchor are
//! byte-identical.

use std::path::Path;

use lys_anchor::AdmissionPolicy;

use crate::cli::AdmissionArgs;
use crate::commands::anchor::open;
use crate::commands::anchor::policy::{AnchorTask, with_policy};
use crate::commands::error::CliResult;
use crate::commands::files::write_file;
use crate::commands::hex::hex_lower;
use crate::commands::output::Emitter;

/// `lys-anchor checkpoint --dir <dir> --key <keyfile> --out <file>
/// --admit <policy>`.
///
/// # Errors
///
/// [`CliError::AnchorDirMissing`] / [`CliError::AnchorDirInvalid`] for
/// directory problems, [`CliError::Anchor`] if the key file is missing or
/// `lys-core` refuses to sign, and [`CliError::Io`] if the note cannot be
/// written.
///
/// [`CliError::AnchorDirMissing`]: crate::commands::error::CliError::AnchorDirMissing
/// [`CliError::AnchorDirInvalid`]: crate::commands::error::CliError::AnchorDirInvalid
/// [`CliError::Anchor`]: crate::commands::error::CliError::Anchor
/// [`CliError::Io`]: crate::commands::error::CliError::Io
pub fn run(
    dir: &Path,
    key: &Path,
    out: &Path,
    admission: &AdmissionArgs,
    json: bool,
) -> CliResult<()> {
    with_policy(
        admission,
        Checkpoint {
            dir,
            key,
            out,
            json,
        },
    )
}

/// `checkpoint`'s arguments, carried so the body can be monomorphised over the
/// concrete admission policy the operator named.
struct Checkpoint<'a> {
    dir: &'a Path,
    key: &'a Path,
    out: &'a Path,
    json: bool,
}

impl AnchorTask for Checkpoint<'_> {
    fn run<P: AdmissionPolicy>(self, policy: P) -> CliResult<()> {
        let anchor = open::open(self.dir, self.key, policy)?;
        let published = anchor.publish_checkpoint()?;
        write_file(self.out, published.note.as_bytes(), "checkpoint note file")?;

        let mut emit = Emitter::new(self.json);
        open::emit_anchor_facts(&mut emit, self.dir, &anchor)?;
        // From the body the note was built from, not a second computation of the
        // same facts: the two cannot disagree because there is only one of them.
        emit.field(
            "root hash (sha256)",
            "root_hash",
            hex_lower(&published.body.root_hash()),
        );
        emit.field(
            "checkpoint written",
            "checkpoint_path",
            self.out.display().to_string(),
        );
        emit.finish();
        Ok(())
    }
}
