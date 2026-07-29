//! Bundle verification tests, built from real logs, real anchors and real
//! receipts throughout — no synthetic artifacts.
//!
//! The tests that matter most follow the shape that made the certificate↔
//! attestation join convincing: **assert each half is valid on its own first**,
//! so when the combined check refuses, the refusal is demonstrably the join and
//! not a broken fixture. A chain test that only shows "invalid input rejected"
//! proves nothing about whether the links are checked at all.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::bundle::artifact::BundleLink;
use crate::checkpoint::{CheckpointBody, NoteVerifierKey, sign_note, verify_checkpoint};
use crate::merkle::tree::{AppendOnlyTree, RawLeaf};
use crate::receipt::{sign_receipt, verify_receipt_bytes};
use crate::tlog::{build_inclusion_artifact, verify_inclusion_artifact};
use crate::{Ed25519Identity, TrustError};

// ------------------------------------------------------------------ fixtures

/// A log or anchor: an origin, a key, and a tree.
struct Party {
    origin: String,
    identity: Ed25519Identity,
    tree: AppendOnlyTree<RawLeaf>,
    _dir: tempfile::TempDir,
}

impl Party {
    fn new(origin: &str, seed: &[u8; 32]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("key");
        std::fs::write(&path, seed).unwrap();
        Self {
            origin: origin.to_string(),
            identity: Ed25519Identity::load(&path).unwrap(),
            tree: AppendOnlyTree::<RawLeaf>::new(),
            _dir: dir,
        }
    }

    fn verifier(&self) -> NoteVerifierKey {
        NoteVerifierKey::new(&self.origin, self.identity.public_key_bytes()).unwrap()
    }

    /// This party's own signed checkpoint at its current size.
    fn checkpoint(&self) -> String {
        let body = CheckpointBody::from_root(&self.origin, &self.tree.root()).unwrap();
        sign_note(&body.encode(), &self.origin, &self.identity).unwrap()
    }

    /// A receipt from this party proving `leaf` at `index`.
    fn receipt_over(&self, leaf: &[u8], index: u64) -> Vec<u8> {
        let size = self.tree.root().to_parts().1;
        let proof = self.tree.prove_inclusion(index).unwrap();
        let path: Vec<[u8; 32]> = proof
            .as_bytes()
            .chunks_exact(32)
            .map(|c| <[u8; 32]>::try_from(c).unwrap())
            .collect();
        sign_receipt(leaf, index, size, &path, &self.identity)
            .unwrap()
            .to_cose_bytes()
    }
}

/// An anchor that has notarized `notarized` (appended after a genesis leaf, so
/// its tree size is never 1 — the receipt format cannot express size 1).
fn anchor_over(origin: &str, seed: &[u8; 32], notarized: &[u8]) -> Party {
    let mut anchor = Party::new(origin, seed);
    anchor.tree.append_raw(b"genesis");
    anchor.tree.append_raw(notarized);
    anchor
}

const CHILD_LEAF: &[u8] = b"entry-1";

/// A child log holding three entries, with the leaf at index 1 proven.
fn child_log() -> Party {
    let mut child = Party::new("child.example", b"lys-bundle-test-child-seed-0001a");
    for leaf in [b"entry-0".as_slice(), CHILD_LEAF, b"entry-2".as_slice()] {
        child.tree.append_raw(leaf);
    }
    child
}

/// A complete one-link scenario: child log, one anchor over its checkpoint.
struct OneLink {
    child: Party,
    anchor: Party,
    bundle: VerificationBundle,
}

fn one_link() -> OneLink {
    let child = child_log();
    let artifact =
        build_inclusion_artifact(&child.tree, CHILD_LEAF, &child.origin, &child.identity, 1)
            .unwrap();
    let child_note = artifact.checkpoint.clone();

    let anchor = anchor_over(
        "anchor-a.example",
        b"lys-bundle-test-anchor-a-seed-01",
        child_note.as_bytes(),
    );
    let receipt = anchor.receipt_over(child_note.as_bytes(), 1);

    let bundle = VerificationBundle::new(
        CHILD_LEAF,
        artifact,
        vec![BundleLink::new(&child_note, &receipt)],
    );
    OneLink {
        child,
        anchor,
        bundle,
    }
}

/// A two-link scenario: anchor B notarizes anchor A's own checkpoint.
struct TwoLink {
    child: Party,
    anchor_a: Party,
    anchor_b: Party,
    bundle: VerificationBundle,
}

fn two_link() -> TwoLink {
    let OneLink {
        child,
        anchor: anchor_a,
        bundle: base,
    } = one_link();

    let a_note = anchor_a.checkpoint();
    let anchor_b = anchor_over(
        "anchor-b.example",
        b"lys-bundle-test-anchor-b-seed-01",
        a_note.as_bytes(),
    );
    let receipt_b = anchor_b.receipt_over(a_note.as_bytes(), 1);

    let mut links = base.links.clone();
    links.push(BundleLink::new(&a_note, &receipt_b));
    let bundle = VerificationBundle::new(CHILD_LEAF, base.inclusion_proof, links);

    TwoLink {
        child,
        anchor_a,
        anchor_b,
        bundle,
    }
}

// -------------------------------------------------------------- happy paths

#[test]
fn a_one_link_bundle_verifies_and_reports_what_it_established() {
    let s = one_link();
    let verified = verify_bundle(&s.bundle, &s.child.verifier(), &[s.anchor.verifier()]).unwrap();

    assert_eq!(verified.leaf(), CHILD_LEAF);
    assert_eq!(verified.log_checkpoint().origin(), "child.example");
    assert_eq!(verified.log_checkpoint().tree_size(), 3);

    let notarizations = verified.notarizations();
    assert_eq!(notarizations.len(), 1);
    assert_eq!(notarizations[0].anchor_tree_size(), 2);
    assert_eq!(notarizations[0].leaf_index(), 1);
    // The reported root is the anchor's real root, recomputed.
    assert_eq!(
        notarizations[0].anchor_root(),
        s.anchor.tree.root().to_parts().0
    );
}

#[test]
fn a_two_link_bundle_verifies_and_the_rung_holds() {
    let s = two_link();
    let verified = verify_bundle(
        &s.bundle,
        &s.child.verifier(),
        &[s.anchor_a.verifier(), s.anchor_b.verifier()],
    )
    .unwrap();

    assert_eq!(verified.notarizations().len(), 2);
    assert_eq!(
        verified.notarizations()[0].anchor_root(),
        s.anchor_a.tree.root().to_parts().0
    );
    assert_eq!(
        verified.notarizations()[1].anchor_root(),
        s.anchor_b.tree.root().to_parts().0
    );
}

#[test]
fn an_unnotarized_bundle_verifies_and_says_so_rather_than_pretending() {
    // A leaf in a log nobody witnessed is a weaker claim, not an invalid one.
    // The type reports it so a reader cannot mistake it for notarization.
    let s = one_link();
    let bundle = VerificationBundle::new(CHILD_LEAF, s.bundle.inclusion_proof.clone(), vec![]);
    let verified = verify_bundle(&bundle, &s.child.verifier(), &[]).unwrap();
    assert!(verified.notarizations().is_empty());
    assert_eq!(verified.leaf(), CHILD_LEAF);
}

#[test]
fn dropping_the_last_link_leaves_a_true_weaker_bundle() {
    // Truncation is not an attack: it removes a notarization rather than
    // fabricating one. Asserted so nobody "hardens" it into a rejection.
    let s = two_link();
    let mut truncated = s.bundle.clone();
    truncated.links.truncate(1);
    verify_bundle(&truncated, &s.child.verifier(), &[s.anchor_a.verifier()]).unwrap();
}

// ------------------------------------------------ THE JOIN: each half valid

/// The test that matters. A bundle whose inclusion proof is valid on its own,
/// and whose receipt is valid on its own, and which must still be refused
/// because the receipt notarizes a **different log's** checkpoint.
///
/// Without the link check this bundle verifies, and a reader concludes their
/// leaf was witnessed when what was witnessed is an unrelated log.
#[test]
fn a_receipt_over_an_unrelated_log_can_never_satisfy_the_join() {
    let s = one_link();

    // A second, entirely legitimate log, notarized by the same anchor.
    let mut decoy = Party::new("decoy.example", b"lys-bundle-test-decoy-seed-0001a");
    decoy.tree.append_raw(b"decoy-entry");
    let decoy_artifact = build_inclusion_artifact(
        &decoy.tree,
        b"decoy-entry",
        &decoy.origin,
        &decoy.identity,
        0,
    )
    .unwrap();
    let decoy_note = decoy_artifact.checkpoint;
    let decoy_anchor = anchor_over(
        "anchor-a.example",
        b"lys-bundle-test-anchor-a-seed-01",
        decoy_note.as_bytes(),
    );
    let decoy_receipt = decoy_anchor.receipt_over(decoy_note.as_bytes(), 1);

    // ---- Both halves are valid ON THEIR OWN. Asserted first, so the failure
    // below is demonstrably the join.
    verify_inclusion_artifact(&s.bundle.inclusion_proof, CHILD_LEAF, &s.child.verifier()).unwrap();
    verify_receipt_bytes(
        &decoy_receipt,
        decoy_note.as_bytes(),
        &decoy_anchor.identity.public_key_bytes(),
    )
    .unwrap();

    // ---- And the bundle that pairs them is refused.
    let spliced = VerificationBundle::new(
        CHILD_LEAF,
        s.bundle.inclusion_proof.clone(),
        vec![BundleLink::new(&decoy_note, &decoy_receipt)],
    );
    assert!(matches!(
        verify_bundle(&spliced, &s.child.verifier(), &[decoy_anchor.verifier()]),
        Err(TrustError::LogArtifactVerification)
    ));
}

/// The middle-of-chain analog: a second link that is a genuine, valid
/// notarization of a genuine checkpoint — just not the checkpoint the first
/// link's anchor actually vouched for.
#[test]
fn a_valid_but_unrelated_link_cannot_be_spliced_into_the_chain() {
    let s = two_link();

    // Anchor A at a DIFFERENT size, so its checkpoint states a different root
    // than the one A's receipt in link 0 vouched for.
    let mut anchor_a_grown = anchor_over(
        "anchor-a.example",
        b"lys-bundle-test-anchor-a-seed-01",
        s.bundle.links[0].checkpoint.as_bytes(),
    );
    anchor_a_grown.tree.append_raw(b"a-later-entry");
    let grown_note = anchor_a_grown.checkpoint();

    let anchor_b = anchor_over(
        "anchor-b.example",
        b"lys-bundle-test-anchor-b-seed-01",
        grown_note.as_bytes(),
    );
    let receipt_b = anchor_b.receipt_over(grown_note.as_bytes(), 1);

    // ---- Each piece is valid alone: the grown checkpoint is genuinely signed
    // by anchor A, and B's receipt genuinely proves it.
    verify_checkpoint(grown_note.as_bytes(), &anchor_a_grown.verifier()).unwrap();
    verify_receipt_bytes(
        &receipt_b,
        grown_note.as_bytes(),
        &anchor_b.identity.public_key_bytes(),
    )
    .unwrap();

    // ---- The chain still must not accept it: A's receipt in link 0 vouched
    // for A's root at size 2, and this checkpoint states size 3.
    let mut spliced = s.bundle.clone();
    spliced.links[1] = BundleLink::new(&grown_note, &receipt_b);
    assert!(
        verify_bundle(
            &spliced,
            &s.child.verifier(),
            &[s.anchor_a.verifier(), anchor_b.verifier()],
        )
        .is_err()
    );
}

#[test]
fn a_reordered_chain_is_refused() {
    let s = two_link();
    let mut reordered = s.bundle.clone();
    reordered.links.swap(0, 1);
    assert!(
        verify_bundle(
            &reordered,
            &s.child.verifier(),
            &[s.anchor_a.verifier(), s.anchor_b.verifier()],
        )
        .is_err()
    );
    // Also with the anchors reordered to match, in case the swap merely
    // mismatched keys rather than breaking the chain.
    assert!(
        verify_bundle(
            &reordered,
            &s.child.verifier(),
            &[s.anchor_b.verifier(), s.anchor_a.verifier()],
        )
        .is_err()
    );
}

#[test]
fn an_extra_link_from_an_unrelated_anchor_is_refused() {
    let s = one_link();
    // An anchor that notarized something else entirely, appended as link 1.
    let stranger = anchor_over(
        "stranger.example",
        b"lys-bundle-test-stranger-seed-01",
        b"something-else",
    );
    let stranger_note = stranger.checkpoint();
    let stranger_receipt = stranger.receipt_over(b"something-else", 1);

    let mut bundle = s.bundle.clone();
    bundle
        .links
        .push(BundleLink::new(&stranger_note, &stranger_receipt));
    assert!(
        verify_bundle(
            &bundle,
            &s.child.verifier(),
            &[s.anchor.verifier(), stranger.verifier()],
        )
        .is_err()
    );
}

// ---------------------------------------------------------- misattribution

#[test]
fn a_bundle_does_not_verify_under_an_anchor_the_caller_did_not_get_a_receipt_from() {
    let s = one_link();
    let other = Party::new("anchor-z.example", b"lys-bundle-test-anchor-z-seed-01");
    assert!(verify_bundle(&s.bundle, &s.child.verifier(), &[other.verifier()]).is_err());
}

#[test]
fn a_bundle_does_not_verify_under_the_wrong_log_key() {
    let s = one_link();
    let other = Party::new("child.example", b"lys-bundle-test-otherlog-seed-01");
    assert!(verify_bundle(&s.bundle, &other.verifier(), &[s.anchor.verifier()]).is_err());
}

#[test]
fn the_anchor_key_must_serve_both_roles() {
    // The rung verifies anchor A's checkpoint under the SAME key that verifies
    // A's receipts. An anchor signing receipts with a key absent from its
    // published checkpoints could not be cross-checked against its own log, so
    // the binding is enforced rather than assumed.
    let s = two_link();
    // Anchor A's note verified under A's own key: the precondition holding.
    verify_checkpoint(
        s.bundle.links[1].checkpoint.as_bytes(),
        &s.anchor_a.verifier(),
    )
    .unwrap();
    // Under anchor B's key it must not.
    assert!(
        verify_checkpoint(
            s.bundle.links[1].checkpoint.as_bytes(),
            &s.anchor_b.verifier()
        )
        .is_err()
    );
}

// --------------------------------------------------------- container checks

#[test]
fn an_unrecognised_format_is_refused_before_anything_else() {
    let s = one_link();
    for format in ["lys/verification-bundle/v2", "", "lys/verification-bundle"] {
        let mut bundle = s.bundle.clone();
        bundle.format = format.to_string();
        assert!(verify_bundle(&bundle, &s.child.verifier(), &[s.anchor.verifier()]).is_err());
    }
}

#[test]
fn a_populated_counter_anchor_is_refused_while_nothing_can_check_one() {
    // The slot exists so a future version needs no v2. Carrying an attestation
    // nothing verifies is how a reader comes to believe it.
    let s = one_link();
    let mut bundle = s.bundle.clone();
    bundle.counter_anchor = Some("AAAA".to_string());
    assert!(verify_bundle(&bundle, &s.child.verifier(), &[s.anchor.verifier()]).is_err());
}

#[test]
fn a_tampered_leaf_is_refused() {
    let s = one_link();
    let mut bundle = s.bundle.clone();
    bundle.leaf = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        b"entry-1-tampered",
    );
    assert!(verify_bundle(&bundle, &s.child.verifier(), &[s.anchor.verifier()]).is_err());
}

#[test]
fn malformed_base64_is_refused() {
    let s = one_link();
    for bad in ["not base64!", "AAA", ""] {
        let mut bundle = s.bundle.clone();
        bundle.leaf = bad.to_string();
        assert!(verify_bundle(&bundle, &s.child.verifier(), &[s.anchor.verifier()]).is_err());

        let mut bundle = s.bundle.clone();
        bundle.links[0].receipt = bad.to_string();
        assert!(verify_bundle(&bundle, &s.child.verifier(), &[s.anchor.verifier()]).is_err());
    }
}

#[test]
fn a_tampered_checkpoint_in_a_link_is_refused() {
    let s = one_link();
    let mut bundle = s.bundle.clone();
    // One byte, inside the note body.
    let mut bytes = bundle.links[0].checkpoint.clone().into_bytes();
    bytes[0] ^= 0x01;
    bundle.links[0].checkpoint = String::from_utf8(bytes).unwrap();
    assert!(verify_bundle(&bundle, &s.child.verifier(), &[s.anchor.verifier()]).is_err());
}

#[test]
fn the_anchor_count_must_match_the_link_count_exactly() {
    // Refusing rather than checking a prefix: a bundle claiming more
    // notarization than the verifier will check cannot be checked, and
    // succeeding on part of it would report more than was established.
    let s = two_link();
    assert!(
        verify_bundle(&s.bundle, &s.child.verifier(), &[s.anchor_a.verifier()]).is_err(),
        "too few anchors must be refused, not silently truncated"
    );
    assert!(
        verify_bundle(
            &s.bundle,
            &s.child.verifier(),
            &[
                s.anchor_a.verifier(),
                s.anchor_b.verifier(),
                s.anchor_b.verifier()
            ],
        )
        .is_err(),
        "too many anchors must be refused"
    );
}

#[test]
fn a_chain_beyond_the_link_cap_is_refused_before_any_work() {
    let s = one_link();
    let mut bundle = s.bundle.clone();
    let link = bundle.links[0].clone();
    while bundle.links.len() <= MAX_LINKS {
        bundle.links.push(link.clone());
    }
    let anchors = vec![s.anchor.verifier(); bundle.links.len()];
    assert!(verify_bundle(&bundle, &s.child.verifier(), &anchors).is_err());
}

#[test]
fn every_failure_is_the_same_error() {
    let s = one_link();
    let child_v = s.child.verifier();
    let anchors = [s.anchor.verifier()];

    let mut bad_format = s.bundle.clone();
    bad_format.format = "nope".to_string();

    let mut bad_leaf = s.bundle.clone();
    bad_leaf.leaf = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, b"wrong");

    let mut bad_receipt = s.bundle.clone();
    bad_receipt.links[0].receipt = "AAAA".to_string();

    let mut bad_counter = s.bundle.clone();
    bad_counter.counter_anchor = Some("AAAA".to_string());

    let mut bad_proof = s.bundle;
    bad_proof.inclusion_proof.leaf_index = 2;

    for (name, bundle) in [
        ("format", bad_format),
        ("leaf", bad_leaf),
        ("receipt", bad_receipt),
        ("counter_anchor", bad_counter),
        ("inclusion proof", bad_proof),
    ] {
        let err = verify_bundle(&bundle, &child_v, &anchors).unwrap_err();
        assert!(
            matches!(err, TrustError::LogArtifactVerification),
            "{name} produced a distinguishable error: {err:?}"
        );
        assert_eq!(
            format!("{err}"),
            format!("{}", TrustError::LogArtifactVerification),
            "{name} produced a distinguishable message"
        );
    }
}

// ----------------------------------------------------------- shape contract

#[test]
fn the_serialized_shape_is_exactly_the_five_wire_fields_in_order() {
    let s = one_link();
    let json = serde_json::to_string(&s.bundle).unwrap();

    let keys: Vec<&str> = [
        "format",
        "leaf",
        "inclusion_proof",
        "links",
        "counter_anchor",
    ]
    .to_vec();
    let mut cursor = 0;
    for key in &keys {
        let needle = format!("\"{key}\":");
        let at = json[cursor..]
            .find(&needle)
            .unwrap_or_else(|| panic!("missing field {key}"));
        cursor += at + needle.len();
    }

    // And nothing that could be mistaken for a signature over the container.
    for forbidden in ["signature", "sig", "signed", "mac"] {
        assert!(
            !json.contains(&format!("\"{forbidden}\"")),
            "the bundle must carry no {forbidden} field: a signature over packaging \
             invites checking the wrapper and skipping the contents"
        );
    }
    // Nor any key material.
    assert!(!json.contains("anchor_key"));
    assert!(!json.contains("verifier"));
}

#[test]
fn an_unknown_field_is_not_a_valid_v1_bundle() {
    let s = one_link();
    let json = serde_json::to_string(&s.bundle).unwrap();
    let smuggled = json.replace(
        "\"counter_anchor\":null",
        "\"counter_anchor\":null,\"extra\":1",
    );
    assert!(serde_json::from_str::<VerificationBundle>(&smuggled).is_err());
}

#[test]
fn a_duplicate_key_is_not_a_valid_v1_bundle() {
    let s = one_link();
    let json = serde_json::to_string(&s.bundle).unwrap();
    let duplicated = json.replace(
        "\"counter_anchor\":null",
        "\"counter_anchor\":null,\"format\":\"lys/verification-bundle/v1\"",
    );
    assert!(serde_json::from_str::<VerificationBundle>(&duplicated).is_err());
}

#[test]
fn a_bundle_round_trips_through_json_and_still_verifies() {
    // The bundle is a file people move around; a round trip through the codec
    // must not change what it establishes.
    let s = one_link();
    let json = serde_json::to_string_pretty(&s.bundle).unwrap();
    let restored: VerificationBundle = serde_json::from_str(&json).unwrap();
    let verified = verify_bundle(&restored, &s.child.verifier(), &[s.anchor.verifier()]).unwrap();
    assert_eq!(verified.leaf(), CHILD_LEAF);
    assert_eq!(verified.notarizations().len(), 1);
}

#[test]
fn the_constructor_sets_the_frozen_format_and_an_empty_slot() {
    let s = one_link();
    assert_eq!(s.bundle.format, VERIFICATION_BUNDLE_FORMAT);
    assert_eq!(s.bundle.format, "lys/verification-bundle/v1");
    assert!(s.bundle.counter_anchor.is_none());
}

#[test]
fn the_debug_form_carries_no_private_material() {
    let s = one_link();
    let verified = verify_bundle(&s.bundle, &s.child.verifier(), &[s.anchor.verifier()]).unwrap();
    let rendered = format!("{verified:?}");
    assert!(rendered.contains("VerifiedBundle"));
    for forbidden in ["seed", "secret", "private", "signingkey"] {
        assert!(!rendered.to_lowercase().contains(forbidden));
    }
}
