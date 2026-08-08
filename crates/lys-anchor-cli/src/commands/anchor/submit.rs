//! `lys-anchor submit` — offer a statement to the anchor and, if it is
//! admitted, write the receipt and the JSON proof for it.
//!
//! # This subcommand does not exist in a default build
//!
//! It is compiled out without `unstable-anchor`, because `Anchor::submit` is —
//! and `Anchor::submit` is gated because the receipt it returns is a draft wire
//! format that is not ratified. A build that did not ask for a draft must not be
//! able to sign one, since signing a durable artifact under a tag is one of the
//! two acts that freeze it.
//!
//! **The consequence is worth stating plainly rather than leaving to be
//! discovered:** `submit` is the anchor's only mutator, so a default-feature
//! build can create an anchor and can never grow it. That is reported as a
//! finding against the library, not worked around here — appending through
//! `Log` directly would bypass the admission policy, which is the one decision
//! this binary must never make on the operator's behalf.
//!
//! # Both artifacts are written, and the JSON one is not optional
//!
//! DP2: *a receipt that only specialised tooling can check violates
//! "verification must outlive the vendor"*. So every receipt is accompanied by
//! the `lys/log-inclusion-proof/v1` artifact, which stock tooling verifies.
//!
//! The two are taken against the same tree because `inclusion_artifact` reads
//! this process's own in-memory tree on the line after `submit` returns, and
//! nothing between them can advance it — another process appending to the same
//! directory would not be visible to this handle at all. That is why no runtime
//! agreement check is performed: a check that cannot fire is indistinguishable
//! from one that passed. It is asserted once, in a test, where a future change
//! that reopened the anchor between the two calls would break it.
//!
//! # What the CLI tells the policy about the submitter
//!
//! `--credential` is presented as **asserted by the submitter**. This binary
//! performs no handshake and observes no peer, so it cannot honestly build the
//! authenticated arm; with no credential at all the context is `Unidentified`,
//! which a policy requiring one refuses — the fail-closed direction.

use std::path::Path;

use lys_anchor::{AdmissionPolicy, Submission};

use crate::cli::AdmissionArgs;
use crate::commands::anchor::open;
use crate::commands::anchor::policy::{AnchorTask, with_policy};
use crate::commands::error::CliResult;
use crate::commands::files::{read_file, write_artifact, write_file};
use crate::commands::hex::hex_lower;
use crate::commands::output::Emitter;

/// `lys-anchor submit --dir <dir> --key <keyfile> --statement <file>
/// --receipt-out <file> --artifact-out <file> --admit <policy>`.
///
/// # Errors
///
/// [`CliError::Anchor`] carrying `AnchorError::NotAdmitted` if the policy
/// refused — in which case **nothing was appended**, and the message is
/// identical for every reason any policy might have had. Also
/// [`CliError::AnchorDirMissing`] / [`CliError::AnchorDirInvalid`] for directory
/// problems, and [`CliError::Io`] / [`CliError::JsonSerialize`] on output
/// failures.
///
/// [`CliError::Anchor`]: crate::commands::error::CliError::Anchor
/// [`CliError::AnchorDirMissing`]: crate::commands::error::CliError::AnchorDirMissing
/// [`CliError::AnchorDirInvalid`]: crate::commands::error::CliError::AnchorDirInvalid
/// [`CliError::Io`]: crate::commands::error::CliError::Io
/// [`CliError::JsonSerialize`]: crate::commands::error::CliError::JsonSerialize
pub fn run(paths: SubmitPaths<'_>, admission: &AdmissionArgs, json: bool) -> CliResult<()> {
    with_policy(admission, Submit { paths, json })
}

/// The six paths `submit` reads and writes.
///
/// Grouped into a struct rather than passed as six positional arguments so a
/// caller cannot transpose two of them — the pairs `--statement`/`--credential`
/// and `--receipt-out`/`--artifact-out` are each two paths of the same type, and
/// swapping either would compile, run, and produce a wrong artifact under a
/// right-looking name.
pub struct SubmitPaths<'a> {
    /// The anchor directory.
    pub dir: &'a Path,
    /// The anchor's signing key file.
    pub key: &'a Path,
    /// The file whose raw bytes are submitted as the statement.
    pub statement: &'a Path,
    /// A certificate to present to the admission policy, if any.
    pub credential: Option<&'a Path>,
    /// Where the `COSE_Sign1` receipt is written.
    pub receipt_out: &'a Path,
    /// Where the JSON inclusion-proof artifact is written.
    pub artifact_out: &'a Path,
}

/// `submit`'s arguments, carried so the body can be monomorphised over the
/// concrete admission policy the operator named.
struct Submit<'a> {
    paths: SubmitPaths<'a>,
    json: bool,
}

impl AnchorTask for Submit<'_> {
    fn run<P: AdmissionPolicy>(self, policy: P) -> CliResult<()> {
        let statement = read_file(self.paths.statement, "statement file")?;
        let credential = self
            .paths
            .credential
            .map(|path| read_file(path, "credential file"))
            .transpose()?;
        let mut anchor = open::open(self.paths.dir, self.paths.key, policy)?;

        let outcome = anchor.submit(
            Submission {
                statement: &statement,
            },
            open::submitter_context(credential.as_deref()),
        )?;
        // Immediately, against the same in-memory tree — see the module docs for
        // why that is what makes the two artifacts agree, and why the agreement
        // is asserted in a test rather than re-checked here.
        let artifact = anchor.inclusion_artifact(outcome.leaf_index)?;

        write_file(
            self.paths.receipt_out,
            &outcome.receipt.to_cose_bytes(),
            "receipt file",
        )?;
        write_artifact(
            self.paths.artifact_out,
            &artifact,
            "inclusion proof artifact",
        )?;

        let mut emit = Emitter::new(self.json);
        open::emit_anchor_facts(&mut emit, self.paths.dir, &anchor)?;
        emit.field("leaf index", "leaf_index", outcome.leaf_index);
        // RFC 6962's SHA-256(0x00 || statement), signed by nothing at all:
        // anybody holding the statement recomputes it. Reported because a
        // submitter who has just handed over bytes wants the handle the log will
        // know them by, not because they should take the anchor's word for it.
        emit.field(
            "leaf hash (sha256)",
            "leaf_hash",
            hex_lower(&outcome.leaf_hash),
        );
        emit.field(
            "receipt written",
            "receipt_path",
            self.paths.receipt_out.display().to_string(),
        );
        emit.field(
            "artifact written",
            "artifact_path",
            self.paths.artifact_out.display().to_string(),
        );
        emit.finish();
        Ok(())
    }
}
