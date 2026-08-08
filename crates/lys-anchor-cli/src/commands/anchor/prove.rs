//! `lys-anchor prove` — build the self-contained JSON inclusion proof for one
//! leaf.
//!
//! # This is the artifact a stranger can check, and it is in every build
//!
//! `lys/log-inclusion-proof/v1` is an RFC 6962 inclusion path plus a signed
//! checkpoint over the root it leads to. Its reader needs the leaf bytes and
//! this anchor's verifier key and nothing else, and `lys log verify inclusion`
//! checks it without ever seeing the anchor directory.
//!
//! It is **not** behind `unstable-anchor`, and that is a property rather than a
//! convenience: gating it would have meant the artifact any stock tooling can
//! verify is opt-in while the draft binary receipt is what a default build
//! gets, inverting *verification must outlive the vendor*.
//!
//! # A proof is a statement about a tree at a size
//!
//! Each call embeds a freshly signed checkpoint of the tree as it stands now, so
//! two proofs of one leaf taken at two sizes declare different `tree_size`
//! values and commit to different roots. Both verify, against different trees;
//! neither supersedes the other. An append-only log guarantees the later
//! contains the earlier.
//!
//! Producing a proof is not an append, and a genesis-only anchor still has one:
//! an empty path is the correct proof that the sole leaf of a one-leaf tree is
//! its root. The receipt path refuses that case and this one does not, because a
//! refusal copied from a constraint that does not apply is a refusal with no
//! reason behind it.

use std::path::Path;

use lys_anchor::AdmissionPolicy;

use crate::cli::AdmissionArgs;
use crate::commands::anchor::open;
use crate::commands::anchor::policy::{AnchorTask, with_policy};
use crate::commands::error::CliResult;
use crate::commands::files::write_artifact;
use crate::commands::output::Emitter;

/// `lys-anchor prove --dir <dir> --key <keyfile> --leaf-index <n> --out <file>
/// --admit <policy>`.
///
/// # Errors
///
/// [`CliError::AnchorDirMissing`] / [`CliError::AnchorDirInvalid`] for
/// directory problems, [`CliError::Anchor`] if the key file is missing or the
/// index is not in the log, and [`CliError::Io`] /
/// [`CliError::JsonSerialize`] on output failures.
///
/// [`CliError::AnchorDirMissing`]: crate::commands::error::CliError::AnchorDirMissing
/// [`CliError::AnchorDirInvalid`]: crate::commands::error::CliError::AnchorDirInvalid
/// [`CliError::Anchor`]: crate::commands::error::CliError::Anchor
/// [`CliError::Io`]: crate::commands::error::CliError::Io
/// [`CliError::JsonSerialize`]: crate::commands::error::CliError::JsonSerialize
pub fn run(
    dir: &Path,
    key: &Path,
    leaf_index: u64,
    out: &Path,
    admission: &AdmissionArgs,
    json: bool,
) -> CliResult<()> {
    with_policy(
        admission,
        Prove {
            dir,
            key,
            leaf_index,
            out,
            json,
        },
    )
}

/// `prove`'s arguments, carried so the body can be monomorphised over the
/// concrete admission policy the operator named.
struct Prove<'a> {
    dir: &'a Path,
    key: &'a Path,
    leaf_index: u64,
    out: &'a Path,
    json: bool,
}

impl AnchorTask for Prove<'_> {
    fn run<P: AdmissionPolicy>(self, policy: P) -> CliResult<()> {
        let anchor = open::open(self.dir, self.key, policy)?;
        let artifact = anchor.inclusion_artifact(self.leaf_index)?;
        write_artifact(self.out, &artifact, "inclusion proof artifact")?;

        let mut emit = Emitter::new(self.json);
        open::emit_anchor_facts(&mut emit, self.dir, &anchor)?;
        emit.field(
            "artifact format",
            "artifact_format",
            artifact.format.clone(),
        );
        emit.field("leaf index", "leaf_index", artifact.leaf_index);
        emit.field(
            "inclusion path nodes",
            "inclusion_path_nodes",
            artifact.hashes.len(),
        );
        emit.field(
            "artifact written",
            "artifact_path",
            self.out.display().to_string(),
        );
        emit.finish();
        Ok(())
    }
}
