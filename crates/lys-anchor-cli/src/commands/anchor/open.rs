//! The `lys-anchor` commands' view of [`lys_anchor`] and [`lys_log_store`].
//!
//! The directory layout, the write-once rules, the Merkle tree, the integrity
//! routine and the genesis invariant all live in the libraries. What remains
//! here is the three things only the CLI can decide: how a storage failure
//! should read to an operator, where a recovery notice goes, and what a
//! standalone anchor is told about itself.
//!
//! # Why the recovery notice is printed here and not there
//!
//! A library that writes to stderr has decided for its caller how a repair gets
//! reported — and a caller embedding lys may want it in a log line, a span, or a
//! structured field. So `Log::open` *returns* the fact, `Anchor::recovered_to`
//! forwards it unchanged, and this layer prints it. The one thing none of the
//! three may do is drop it: a silently repaired log is indistinguishable from
//! one that never needed repairing. That reasoning is
//! `crates/lys/src/commands/log/store.rs`'s and is followed here rather than
//! restated differently.
//!
//! **This CLI surfaces it twice, and the second place is not redundant.** The
//! `lys` binary prints the notice to stderr. A `--json` consumer reads stdout,
//! so for that consumer a stderr line is exactly the repair it never hears
//! about. Every command here therefore also emits `recovered_to` as a field of
//! the success object — always present, `null` when the log opened clean, so a
//! consumer can tell "no repair" from "an older binary that never reported one".
//!
//! # The disclosure this module carries, and why it is a constant
//!
//! `BUILD-PLAN.md` §2.2 requires four places to state that a standalone anchor
//! can equivocate undetectably, and names the CLI as one of them. It also
//! requires the posture be *computed from the anchor's own log* and returned as
//! a value, so that it cannot be set to a comfortable value by an operator who
//! has not arranged a witness. **That value does not exist yet** — there is no
//! `Anchor::status`, no `AnchorStatus` and no `WitnessPosture` in `lys-anchor` —
//! so this module states the disclosure as an unconditional constant.
//!
//! That is correct today for a reason worth writing down rather than relying on:
//! no anchor can currently be witnessed, because the federation path that would
//! record a parent's receipt is not built. When `WitnessPosture` lands, this
//! constant must become a rendering of the value the library returns; a
//! hard-coded sentence that survived into a world with real witnesses would be
//! telling a witnessed anchor's operator something false.

use std::path::Path;

#[cfg(feature = "unstable-anchor")]
use lys_anchor::SubmitterContext;
use lys_anchor::{AdmissionPolicy, Anchor, AnchorConfig, AnchorError, FileSigner, Signer};
use lys_core::checkpoint::NoteVerifierKey;
use lys_log_store::{FileLeafStore, StoreError};
use serde_json::Value;

use crate::commands::error::{CliError, CliResult};
use crate::commands::hex::hex_lower;
use crate::commands::output::Emitter;

/// The `lys-anchor` CLI's anchor: a file-backed log signed by a key on disk.
pub type FileAnchor<P> = Anchor<FileLeafStore, FileSigner, P>;

/// The standalone-equivocation disclosure, in the words `lys-anchor`'s own
/// module docs use.
///
/// Not a warning that can be silenced and not conditional on a flag. See the
/// [module docs](self) for why it is a constant in this version and what must
/// replace it.
pub const STANDALONE_DISCLOSURE: &str = "this anchor has no witnesses: an anchor with no \
     witnesses can equivocate undetectably — it can hold two histories and show each observer \
     whichever suits, and no local check catches that. What changes it is one external party \
     keeping its own durable memory of this anchor's checkpoints.";

/// The context this CLI puts a submission to the admission policy under.
///
/// `Unidentified` when no credential was supplied, and `AssertedBySubmitter`
/// when one was — **never** `AuthenticatedByTransport`. That arm asserts a peer
/// demonstrated possession of a private key, and this binary performs no
/// handshake and observes no peer, so constructing it would be a claim nobody
/// made. The library made that arm require a named constructor precisely so the
/// mistake has to be typed out; typing it out here would be the mistake.
#[cfg(feature = "unstable-anchor")]
pub fn submitter_context(credential: Option<&[u8]>) -> SubmitterContext<'_> {
    match credential {
        None => SubmitterContext::Unidentified,
        Some(bytes) => SubmitterContext::AssertedBySubmitter(bytes),
    }
}

/// Creates an anchor directory at `dir`, pinning `origin` and writing `genesis`
/// as leaf 0.
///
/// # Errors
///
/// [`CliError::AnchorDirInvalid`] if `dir` is already an initialized log,
/// [`CliError::Anchor`] if the key file is missing or the genesis leaf cannot be
/// written, and [`CliError::Io`] on filesystem failure.
pub fn create<P: AdmissionPolicy>(
    dir: &Path,
    origin: &str,
    key: &Path,
    genesis: &[u8],
    policy: P,
) -> CliResult<FileAnchor<P>> {
    let store = FileLeafStore::create(dir, origin)?;
    let signer = FileSigner::load(key)?;
    Anchor::create(store, genesis, signer, policy, AnchorConfig::unconfigured())
        .map_err(|err| anchor_failure(dir, err))
}

/// Opens the anchor at `dir`, reporting an interrupted append that was repaired.
///
/// # Errors
///
/// [`CliError::AnchorDirMissing`] if `dir` holds no log,
/// [`CliError::AnchorDirInvalid`] with the specific discrepancy on an integrity
/// failure, [`CliError::Anchor`] if the key file is missing or the log has no
/// genesis leaf, and [`CliError::Io`] on filesystem failure.
pub fn open<P: AdmissionPolicy>(dir: &Path, key: &Path, policy: P) -> CliResult<FileAnchor<P>> {
    let store = FileLeafStore::open(dir)?;
    let signer = FileSigner::load(key)?;
    let anchor = Anchor::open(store, signer, policy, AnchorConfig::unconfigured())
        .map_err(|err| anchor_failure(dir, err))?;
    if let Some(tree_size) = anchor.recovered_to() {
        eprintln!("recovered interrupted append: state advanced to {tree_size}");
    }
    Ok(anchor)
}

/// The signed-note verifier key string a third party needs to check anything
/// this anchor signs.
///
/// # Errors
///
/// [`CliError::Trust`] if `lys-core` refuses to encode the key under this
/// origin — unreachable for an origin a store was created with, and propagated
/// rather than assumed away.
pub fn verifier_key<P: AdmissionPolicy>(anchor: &FileAnchor<P>) -> CliResult<String> {
    let verifier = NoteVerifierKey::new(anchor.origin(), anchor.signer().public_key())?;
    Ok(verifier.to_spec())
}

/// Emits the facts every command reports about the anchor it opened: identity,
/// size, and whether a repair happened.
///
/// # Errors
///
/// [`CliError::Trust`], from [`verifier_key`].
pub fn emit_anchor_facts<P: AdmissionPolicy>(
    emit: &mut Emitter,
    dir: &Path,
    anchor: &FileAnchor<P>,
) -> CliResult<()> {
    emit.field(
        "anchor directory",
        "anchor_directory",
        dir.display().to_string(),
    );
    emit.field("origin", "origin", anchor.origin());
    emit.field("tree size", "tree_size", anchor.tree_size());
    emit.field(
        "signer public key (ed25519)",
        "signer_public_key",
        hex_lower(&anchor.signer().public_key()),
    );
    emit.field(
        "verifier key (signed-note)",
        "verifier_key",
        verifier_key(anchor)?,
    );
    emit_recovered_to(emit, anchor.recovered_to());
    Ok(())
}

/// Emits `recovered_to` in both output modes, always.
///
/// Present even when nothing was repaired, because an absent field cannot be
/// told apart from a binary that does not report repairs — and the whole reason
/// the fact is threaded up from the storage layer is that an unreported repair
/// is indistinguishable from one that never happened.
pub fn emit_recovered_to(emit: &mut Emitter, recovered_to: Option<u64>) {
    if emit.is_json() {
        emit.field(
            "recovered to",
            "recovered_to",
            recovered_to.map_or(Value::Null, Value::from),
        );
    } else {
        emit.field(
            "recovered interrupted append to",
            "recovered_to",
            recovered_to.map_or_else(
                || "none (opened clean)".to_string(),
                |tree_size| tree_size.to_string(),
            ),
        );
    }
}

/// Attaches the anchor directory to a storage failure that carries no path.
///
/// `StoreError::PinMismatch` is deliberately path-free in the library: the
/// disagreement is between a set of leaves and a pin, wherever those live, and a
/// backend that is not a directory has no path to name. This CLI's leaves and
/// pin *do* live in a directory, and an operator's first question is which one —
/// so this is where the path goes back on, keeping the
/// `anchor directory invalid: <path>: <reason>` shape every other corruption
/// reports.
///
/// The `_` arm is required rather than chosen: `AnchorError` is
/// `#[non_exhaustive]`, and a variant this CLI has never seen must reach the
/// operator verbatim.
fn anchor_failure(dir: &Path, err: AnchorError) -> CliError {
    match err {
        AnchorError::Store(store_error) => store_failure(dir, store_error),
        other => CliError::from(other),
    }
}

/// The same treatment for a storage failure that did not arrive wrapped.
fn store_failure(dir: &Path, err: StoreError) -> CliError {
    match err {
        err @ (StoreError::PinMismatch { .. } | StoreError::LeafMissingWithinExtent { .. }) => {
            CliError::AnchorDirInvalid {
                path: dir.to_path_buf(),
                reason: err.to_string(),
            }
        }
        other => CliError::from(other),
    }
}

#[cfg(test)]
#[path = "open_tests.rs"]
mod tests;
