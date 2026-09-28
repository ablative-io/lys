#![cfg(test)]
//! Bundles that verify, and chains that cannot be spliced, reordered or
//! extended with links that do not join.

use super::*;

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
        Err(TrustError::BundleVerification)
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
