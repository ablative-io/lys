//! `lys-anchor status` — read the state of a local anchor.
//!
//! Nothing here is a trust statement. The size and origin are read from local
//! bytes the local operator controls, so this command says what this directory
//! currently contains — not that anybody else has attested to it. It signs
//! nothing and writes nothing.
//!
//! # Two absences an operator will notice, and neither is a choice made here
//!
//! - **It asks for `--key` although it signs nothing.** An anchor holds its
//!   signer from construction, so `Anchor::open` cannot be called without one.
//!   `lys log status` deliberately has a key-free read path — *"observing your
//!   own append-only log should not require the ability to sign for it"* — and
//!   an anchor currently has no equivalent.
//! - **It reports no root hash.** `Anchor` exposes `origin`, `tree_size` and
//!   `recovered_to`, and no accessor for its current root. The only route to
//!   the root is `publish_checkpoint`, which *signs* — and a status command
//!   that emitted a signed artifact as a side effect of being asked a question
//!   would be a worse answer than an incomplete one. Run
//!   `lys-anchor checkpoint` when the root is what you need.
//!
//! Both are reported as findings against the library rather than worked around
//! here, because a workaround in the binary is a workaround no library consumer
//! gets.

use std::path::Path;

use lys_anchor::AdmissionPolicy;

use crate::cli::AdmissionArgs;
use crate::commands::anchor::open::{self, STANDALONE_DISCLOSURE};
use crate::commands::anchor::policy::{AnchorTask, with_policy};
use crate::commands::error::CliResult;
use crate::commands::output::Emitter;

/// `lys-anchor status --dir <dir> --key <keyfile> --admit <policy>`.
///
/// # Errors
///
/// [`CliError::AnchorDirMissing`] if `dir` holds no anchor,
/// [`CliError::AnchorDirInvalid`] if its stored state does not match the
/// rebuilt tree, and [`CliError::Anchor`] if the key file is missing or the log
/// has no genesis leaf.
///
/// [`CliError::AnchorDirMissing`]: crate::commands::error::CliError::AnchorDirMissing
/// [`CliError::AnchorDirInvalid`]: crate::commands::error::CliError::AnchorDirInvalid
/// [`CliError::Anchor`]: crate::commands::error::CliError::Anchor
pub fn run(dir: &Path, key: &Path, admission: &AdmissionArgs, json: bool) -> CliResult<()> {
    with_policy(admission, Status { dir, key, json })
}

/// `status`'s arguments, carried so the body can be monomorphised over the
/// concrete admission policy the operator named.
struct Status<'a> {
    dir: &'a Path,
    key: &'a Path,
    json: bool,
}

impl AnchorTask for Status<'_> {
    fn run<P: AdmissionPolicy>(self, policy: P) -> CliResult<()> {
        let anchor = open::open(self.dir, self.key, policy)?;
        let mut emit = Emitter::new(self.json);
        open::emit_anchor_facts(&mut emit, self.dir, &anchor)?;
        emit.field(
            "standalone anchor",
            "standalone_disclosure",
            STANDALONE_DISCLOSURE,
        );
        emit.finish();
        Ok(())
    }
}
