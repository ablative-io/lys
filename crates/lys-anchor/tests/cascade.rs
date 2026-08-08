//! The cascade gate: a real two- and three-anchor cascade, packaged, and handed
//! to a judge that has never heard of this crate.
//!
//! BUILD-PLAN §7.5. **`lys-core`'s `verify_bundle` is the second party**, and it
//! is the strongest one available for the *relationships* between artifacts,
//! because it is code written **before `lys-anchor` existed, by a party that
//! does not know how the cascade was produced.** It cannot have been written to
//! agree with the producer, because the producer did not exist to agree with.
//!
//! That is also the payoff of §4.2's unification. Cascading and witnessing are
//! one mechanism, so the cascade a stranger checks is the *same* act a witness
//! performs — if they had been built as two mechanisms, one of them would have
//! no judge at all, and it would be whichever one nobody thought to write a
//! verifier for.
//!
//! # What this file reaches, and what it deliberately leaves alone
//!
//! Only the public surface, from outside the crate, exactly as an integration
//! would: `Anchor`, `pin`, `bundle_for`, and `lys-core`'s verifiers. The
//! producer's own two refusals — the link cap and the first-link join it can
//! break by appending — are `upward::bundle`'s unit tests', reached through
//! `bundle_for` and asserted as `AnchorError` variants `verify_bundle` cannot
//! produce. **Every bundle this file refuses is assembled by hand**, through
//! `lys-core`'s own constructors and never through `bundle_for`, so the judge's
//! checks and the producer's checks are isolated: a drift in one cannot decide
//! the other's cases.
//!
//! # One rule per case
//!
//! | rule | the only case that may fail when it is drifted |
//! |---|---|
//! | a two-anchor cascade verifies end to end | [`a_two_anchor_cascade_verifies`] |
//! | a three-anchor cascade verifies, and reports both rungs | [`a_three_anchor_cascade_verifies_both_rungs`] |
//! | the first link must be about *this* log's checkpoint | [`a_notarization_of_another_moment_does_not_join`] |
//! | the rung between links is checked, not assumed | [`a_parent_checkpoint_from_a_later_size_breaks_the_rung`] |
//! | an unnotarized bundle is accepted and says so | [`an_unnotarized_bundle_verifies_and_reports_no_notarization`] |
//!
//! **Every negative opens with a positive control**, and the controls here are
//! the shape `lys-core`'s own bundle suite established: assert each half is
//! valid *on its own* first, so when the combined check refuses, the refusal is
//! demonstrably the relationship and not a broken fixture. A chain test that
//! only shows "invalid input rejected" proves nothing about whether the links
//! are checked at all — and a suite made only of refusals cannot tell a working
//! verifier from one that refuses everything.
//!
//! # The independence axis
//!
//! *Implementation*: the judge is a different crate, written earlier, with no
//! knowledge of the producer. It is **not** independence of platform, language,
//! toolchain or dependency resolution — one machine, one `cargo`, one lockfile.
//! The cross-language claims belong to the Go conformance gates and are not
//! restated here.
//!
//! The crate attributes sit **below** these docs rather than above them, which
//! is `lys-core`'s pattern for a gated integration test and not a style
//! preference: a crate-level `#![cfg]` placed first strips the doc attributes
//! with everything else, and the default build then fails `missing_docs` on an
//! empty crate. That is a real gate catching a real ordering, so the ordering
//! is recorded here rather than rediscovered.

#![cfg(feature = "federation")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use lys_anchor::{
    AcceptAll, Anchor, AnchorConfig, FileSigner, Signer, Submission, SubmitterContext, UpwardPin,
    bundle_for, pin,
};
use lys_core::bundle::{BundleLink, VerificationBundle, verify_bundle};
use lys_core::checkpoint::{NoteVerifierKey, verify_checkpoint};
use lys_core::receipt::verify_receipt_bytes;
use lys_core::tlog::verify_inclusion_artifact;
use lys_log_store::FileLeafStore;
use tempfile::TempDir;

/// The genesis bytes every anchor in this file is created with.
const GENESIS: &[u8] = b"lys-anchor cascade gate genesis";

/// The statement whose provenance the bundle carries. Every check that asks
/// what the bundle is *about* is keyed on this literal.
const STATEMENT: &[u8] = b"the statement a cascade is assembled around";

/// Origins, as **this file** supplies them. Verifier keys are built from these
/// literals, never from what an anchor reports back.
const CHILD: &str = "example.com/lys/cascade-gate-child";
const PARENT: &str = "example.com/lys/cascade-gate-parent";
const GRANDPARENT: &str = "example.com/lys/cascade-gate-grandparent";

/// One distinct seed per anchor, so no anchor's note can verify under
/// another's key by accident.
const CHILD_SEED: &[u8; 32] = b"lys-go-conformance-test-seed-01!";
const PARENT_SEED: &[u8; 32] = b"lys-anchor-cascade-gate-parent-k";
const GRANDPARENT_SEED: &[u8; 32] = b"lys-anchor-cascade-gate-grandpar";

/// An anchor, with its storage held so it outlives the value.
struct Party {
    anchor: Anchor<FileLeafStore, FileSigner, AcceptAll>,
    origin: String,
    _dir: TempDir,
}

impl Party {
    fn new(origin: &str, seed: &[u8; 32]) -> Self {
        let dir = TempDir::new().unwrap();
        Self {
            anchor: build(dir.path(), origin, seed),
            origin: origin.to_string(),
            _dir: dir,
        }
    }

    /// The verifier key a third party is given: the origin literal above and
    /// the public key the signer advertises.
    fn verifier(&self) -> NoteVerifierKey {
        NoteVerifierKey::new(&self.origin, self.anchor.signer().public_key()).unwrap()
    }
}

fn build(
    dir: &Path,
    origin: &str,
    seed: &[u8; 32],
) -> Anchor<FileLeafStore, FileSigner, AcceptAll> {
    let key_path = dir.join("anchor.key");
    std::fs::write(&key_path, seed).unwrap();
    let store = FileLeafStore::create(dir, origin).unwrap();
    Anchor::create(
        store,
        GENESIS,
        FileSigner::load(&key_path).unwrap(),
        AcceptAll,
        AnchorConfig::unconfigured(),
    )
    .unwrap()
}

/// Appends `statement` to `party` and returns the index it landed at — read off
/// the anchor's own report rather than counted here.
fn append(party: &mut Party, statement: &[u8]) -> u64 {
    party
        .anchor
        .append(Submission { statement }, SubmitterContext::Unidentified)
        .unwrap()
        .leaf_index
}

/// Pins `child`'s current checkpoint to `parent`, as an ordinary unidentified
/// submitter.
fn pin_up(child: &Party, parent: &mut Party) -> UpwardPin {
    pin(
        &child.anchor,
        &mut parent.anchor,
        SubmitterContext::Unidentified,
    )
    .unwrap()
}

/// The bytes of a pin's receipt, as a bundle carries them.
fn receipt_bytes(pinned: &UpwardPin) -> Vec<u8> {
    pinned.recorded.receipt.to_cose_bytes()
}

#[test]
fn a_two_anchor_cascade_verifies() {
    let mut child = Party::new(CHILD, CHILD_SEED);
    let mut parent = Party::new(PARENT, PARENT_SEED);

    let index = append(&mut child, STATEMENT);
    let pinned = pin_up(&child, &mut parent);

    let bundle = bundle_for(&child.anchor, index, std::slice::from_ref(&pinned)).unwrap();

    let verified = verify_bundle(&bundle, &child.verifier(), &[parent.verifier()])
        .expect("a cascade this crate produced must satisfy a judge that predates it");

    // Keyed on values this file supplied, not on values read back off the
    // bundle and compared with themselves.
    assert_eq!(verified.leaf(), STATEMENT);
    assert_eq!(verified.log_checkpoint().origin(), CHILD);
    assert_eq!(verified.notarizations().len(), 1);
    assert_eq!(
        verified.notarizations()[0].leaf_index(),
        pinned.recorded.leaf_index
    );
    // The notarized root is the *parent's* root, recomputed by the judge from
    // the child's note and the parent's inclusion path — so this is the judge
    // and the parent agreeing about the parent's tree.
    assert_eq!(
        verified.notarizations()[0].anchor_tree_size(),
        parent.anchor.tree_size()
    );
}

#[test]
fn a_three_anchor_cascade_verifies_both_rungs() {
    let mut child = Party::new(CHILD, CHILD_SEED);
    let mut parent = Party::new(PARENT, PARENT_SEED);
    let mut grandparent = Party::new(GRANDPARENT, GRANDPARENT_SEED);

    let index = append(&mut child, STATEMENT);
    // The child's checkpoint into the parent, then — with no append in
    // between — the parent's own checkpoint into the grandparent. The
    // sequencing is the cascade's requirement, not the bundle's: the rung the
    // judge enforces is that link 1's checkpoint states exactly the root and
    // size link 0's receipt vouched for.
    let low = pin_up(&child, &mut parent);
    let high = pin_up(&parent, &mut grandparent);

    let bundle = bundle_for(&child.anchor, index, &[low.clone(), high.clone()]).unwrap();
    assert_eq!(bundle.links.len(), 2);

    let verified = verify_bundle(
        &bundle,
        &child.verifier(),
        &[parent.verifier(), grandparent.verifier()],
    )
    .expect("a three-anchor cascade must verify, rung included");

    assert_eq!(verified.leaf(), STATEMENT);
    assert_eq!(verified.notarizations().len(), 2);
    assert_eq!(
        verified.notarizations()[0].leaf_index(),
        low.recorded.leaf_index
    );
    assert_eq!(
        verified.notarizations()[1].leaf_index(),
        high.recorded.leaf_index
    );
    assert_eq!(
        verified.notarizations()[1].anchor_tree_size(),
        grandparent.anchor.tree_size()
    );
}

#[test]
fn a_notarization_of_another_moment_does_not_join() {
    let mut child = Party::new(CHILD, CHILD_SEED);
    let mut parent = Party::new(PARENT, PARENT_SEED);

    let index = append(&mut child, STATEMENT);
    // The artifact is taken here, at this size, and never regenerated.
    let artifact = child.anchor.inclusion_artifact(index).unwrap();

    // The child then grows and is pinned again. Everything about this second
    // pin is genuine: a real checkpoint of a real log, notarized by a real
    // parent that really held it.
    append(
        &mut child,
        b"a statement admitted after the artifact was taken",
    );
    let later = pin_up(&child, &mut parent);

    // ---- positive controls: both halves are valid on their own ----
    verify_inclusion_artifact(&artifact, STATEMENT, &child.verifier())
        .expect("control: the inclusion proof is valid on its own");
    verify_receipt_bytes(
        &receipt_bytes(&later),
        later.checkpoint.note.as_bytes(),
        &parent.verifier().public_key(),
    )
    .expect("control: the notarization is valid on its own");
    verify_checkpoint(later.checkpoint.note.as_bytes(), &child.verifier())
        .expect("control: the notarized checkpoint is genuinely the child's");
    assert_ne!(
        later.checkpoint.note, artifact.checkpoint,
        "control: the two checkpoints must actually differ, or this case tests \
         nothing"
    );

    // ---- the bundle, assembled by hand so only the judge decides it ----
    let forged = VerificationBundle::new(
        STATEMENT,
        artifact,
        vec![BundleLink::new(
            &later.checkpoint.note,
            &receipt_bytes(&later),
        )],
    );
    assert!(
        verify_bundle(&forged, &child.verifier(), &[parent.verifier()]).is_err(),
        "two unrelated true facts side by side are not a chain: the first link \
         must notarize the very checkpoint the inclusion proof was verified \
         against"
    );
}

#[test]
fn a_parent_checkpoint_from_a_later_size_breaks_the_rung() {
    let mut child = Party::new(CHILD, CHILD_SEED);
    let mut parent = Party::new(PARENT, PARENT_SEED);
    let mut grandparent = Party::new(GRANDPARENT, GRANDPARENT_SEED);

    let index = append(&mut child, STATEMENT);
    let artifact = child.anchor.inclusion_artifact(index).unwrap();
    let low = pin_up(&child, &mut parent);

    // The parent then admits somebody else's statement before publishing the
    // checkpoint it hands upward. Nothing here is forged: the parent's later
    // checkpoint is a true statement about the parent's larger tree, and the
    // grandparent really notarized it.
    append(
        &mut parent,
        b"another submitter's statement, admitted in between",
    );
    let high = pin_up(&parent, &mut grandparent);

    // ---- positive controls: every artifact is valid on its own ----
    verify_inclusion_artifact(&artifact, STATEMENT, &child.verifier())
        .expect("control: the inclusion proof is valid on its own");
    assert_eq!(
        low.checkpoint.note, artifact.checkpoint,
        "control: link 0 does join the inclusion proof, so the refusal below is \
         the rung and not the join"
    );
    verify_receipt_bytes(
        &receipt_bytes(&low),
        low.checkpoint.note.as_bytes(),
        &parent.verifier().public_key(),
    )
    .expect("control: link 0's receipt is valid on its own");
    let high_body = verify_checkpoint(high.checkpoint.note.as_bytes(), &parent.verifier())
        .expect("control: link 1's checkpoint is genuinely the parent's, signed by the parent");
    verify_receipt_bytes(
        &receipt_bytes(&high),
        high.checkpoint.note.as_bytes(),
        &grandparent.verifier().public_key(),
    )
    .expect("control: link 1's receipt is valid on its own");
    assert!(
        high_body.tree_size() > low.recorded.tree_size,
        "control: the parent must really have moved on, or this case tests nothing"
    );

    // ---- the bundle, assembled by hand ----
    let forged = VerificationBundle::new(
        STATEMENT,
        artifact,
        vec![
            BundleLink::new(&low.checkpoint.note, &receipt_bytes(&low)),
            BundleLink::new(&high.checkpoint.note, &receipt_bytes(&high)),
        ],
    );
    assert!(
        verify_bundle(
            &forged,
            &child.verifier(),
            &[parent.verifier(), grandparent.verifier()]
        )
        .is_err(),
        "the rung is mandatory: a pile of individually valid receipts proves \
         nothing about their relationship, and an implementation that verified \
         each and skipped the joins would accept a fabricated history"
    );
}

#[test]
fn an_unnotarized_bundle_verifies_and_reports_no_notarization() {
    let mut child = Party::new(CHILD, CHILD_SEED);
    let index = append(&mut child, STATEMENT);

    let bundle = bundle_for(&child.anchor, index, &[]).unwrap();
    let verified = verify_bundle(&bundle, &child.verifier(), &[])
        .expect("a leaf in a log nobody notarized is a weaker claim, not a failure");

    assert_eq!(verified.leaf(), STATEMENT);
    assert!(
        verified.notarizations().is_empty(),
        "the judge must report the absence rather than let a reader assume \
         otherwise"
    );
}
