#![cfg(test)]
//! Refusals on the anchor key, the format, the counter anchor, tampered
//! contents, the anchor count and the link cap, and the single error every
//! failure produces.

use super::*;

// ---------------------------------------------------------- misattribution

#[test]
fn a_bundle_does_not_verify_under_an_anchor_the_caller_did_not_get_a_receipt_from() {
    let s = one_link();
    let other = Party::new("anchor-z.example", b"lys-bundle-test-anchor-z-seed-01");
    assert!(verify_bundle(&s.bundle, &s.child.verifier(), &[other.verifier()]).is_err());
    s.close().unwrap();
    other.close().unwrap();
}

#[test]
fn a_bundle_does_not_verify_under_the_wrong_log_key() {
    let s = one_link();
    let other = Party::new("child.example", b"lys-bundle-test-otherlog-seed-01");
    assert!(verify_bundle(&s.bundle, &other.verifier(), &[s.anchor.verifier()]).is_err());
    s.close().unwrap();
    other.close().unwrap();
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
    s.close().unwrap();
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
    s.close().unwrap();
}

#[test]
fn a_populated_counter_anchor_is_refused_while_nothing_can_check_one() {
    // The slot exists so a future version needs no v2. Carrying an attestation
    // nothing verifies is how a reader comes to believe it.
    let s = one_link();
    let mut bundle = s.bundle.clone();
    bundle.counter_anchor = Some("AAAA".to_string());
    assert!(verify_bundle(&bundle, &s.child.verifier(), &[s.anchor.verifier()]).is_err());
    s.close().unwrap();
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
    s.close().unwrap();
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
    s.close().unwrap();
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
    s.close().unwrap();
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
    s.close().unwrap();
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
    s.close().unwrap();
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
            matches!(err, TrustError::BundleVerification),
            "{name} produced a distinguishable error: {err:?}"
        );
        assert_eq!(
            format!("{err}"),
            format!("{}", TrustError::BundleVerification),
            "{name} produced a distinguishable message"
        );
    }
    s.child.close().unwrap();
    s.anchor.close().unwrap();
}
