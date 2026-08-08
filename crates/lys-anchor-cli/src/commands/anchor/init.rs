//! `lys-anchor init` — create an anchor: a log directory, a pinned origin, and
//! a genesis leaf.
//!
//! # Three things are fixed here and can never be changed afterwards
//!
//! - **The origin.** It is the anchor's identity and the signed-note key name
//!   its checkpoints are signed under. Re-running init on an existing directory
//!   is refused, because a per-invocation origin is exactly the
//!   two-logs-one-origin confusion the origin binding exists to kill.
//! - **The genesis leaf.** The file's raw bytes become leaf 0, verbatim and
//!   uninterpreted. Storage offers no insert, no rewrite and no fork, so leaf 0
//!   is written now or never — and an anchor without one can never issue a
//!   conforming receipt for its first real entry.
//! - **Nothing about admission.** The policy named on this command line governs
//!   *this invocation*, and is not recorded in the directory. That is the
//!   library's shape and it is worth an operator knowing: the log is what was
//!   admitted, never why, and a later invocation under a different `--admit` is
//!   not detectable from the stored anchor.
//!
//! The genesis bytes are the operator's own, so the admission policy is not
//! consulted for them — an anchor's existence must not depend on its
//! access-control rule agreeing with its operator.

use std::path::Path;

use lys_anchor::AdmissionPolicy;

use crate::cli::AdmissionArgs;
use crate::commands::anchor::open::{self, STANDALONE_DISCLOSURE};
use crate::commands::anchor::policy::{AnchorTask, with_policy};
use crate::commands::error::CliResult;
use crate::commands::files::read_file;
use crate::commands::output::Emitter;

/// `lys-anchor init --dir <dir> --origin <origin> --key <keyfile>
/// --genesis <file> --admit <policy>`.
///
/// # Errors
///
/// [`CliError::AnchorDirInvalid`] if the directory is already initialized,
/// [`CliError::Anchor`] if the origin is refused or the key file is missing, and
/// [`CliError::Io`] if the genesis file cannot be read.
///
/// [`CliError::AnchorDirInvalid`]: crate::commands::error::CliError::AnchorDirInvalid
/// [`CliError::Anchor`]: crate::commands::error::CliError::Anchor
/// [`CliError::Io`]: crate::commands::error::CliError::Io
pub fn run(
    dir: &Path,
    origin: &str,
    key: &Path,
    genesis: &Path,
    admission: &AdmissionArgs,
    json: bool,
) -> CliResult<()> {
    with_policy(
        admission,
        Init {
            dir,
            origin,
            key,
            genesis,
            json,
        },
    )
}

/// `init`'s arguments, carried so the body can be monomorphised over the
/// concrete admission policy the operator named.
struct Init<'a> {
    dir: &'a Path,
    origin: &'a str,
    key: &'a Path,
    genesis: &'a Path,
    json: bool,
}

impl AnchorTask for Init<'_> {
    fn run<P: AdmissionPolicy>(self, policy: P) -> CliResult<()> {
        let genesis = read_file(self.genesis, "genesis leaf file")?;
        let anchor = open::create(self.dir, self.origin, self.key, &genesis, policy)?;

        let mut emit = Emitter::new(self.json);
        open::emit_anchor_facts(&mut emit, self.dir, &anchor)?;
        emit.field("genesis leaf index", "genesis_leaf_index", 0);
        emit.field(
            "genesis leaf source",
            "genesis_source",
            self.genesis.display().to_string(),
        );
        emit.field(
            "standalone anchor",
            "standalone_disclosure",
            STANDALONE_DISCLOSURE,
        );
        emit.finish();
        Ok(())
    }
}
