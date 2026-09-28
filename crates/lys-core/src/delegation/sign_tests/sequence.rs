#![cfg(test)]
//! Replay, `sequence` and `not_before`: what orders delegations and what
//! never does.

use super::*;

// ---------------------------------------------------------------------------
// §2.2 — `sequence` orders delegations, and why it had to exist.
// ---------------------------------------------------------------------------

/// The replay defect the `sequence` field was added to close, demonstrated
/// rather than described.
///
/// This test asserts a property that looks like a *weakness* — that two
/// issuances of one claim are byte-identical — because that property is exactly
/// what made ordering-by-log-position exploitable, and it is still true. What
/// changed is that the artifact now carries a number that makes a replayed copy
/// lose.
#[test]
fn a_replayed_delegation_is_byte_identical_which_is_why_sequence_exists() {
    let key = root();

    // 1. The original delegation, at sequence 1.
    let leaf_zero = sign_delegation(&key, &claim_at(ORIGIN, 1)).unwrap();

    // 2. The delegated key is compromised, so the operator supersedes it. The
    //    old artifact is not removed — revocation is an append.
    let mut superseding = claim_at(ORIGIN, 2);
    superseding.delegated_public_key = attacker().public_key_bytes();
    let leaf_n = sign_delegation(&key, &superseding).unwrap();

    // 3. An attacker holding NO key material re-appends the original bytes.
    //    They are public: they are in the log, which is the point of the log.
    let replayed = leaf_zero.clone();

    // Every one of the three verifies. That is not a bug — a replay of a
    // genuine artifact is a genuine artifact — and it is why the defence cannot
    // live in `verify_delegation`.
    for artifact in [&leaf_zero, &leaf_n, &replayed] {
        verify_delegation(artifact, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
    }

    // THE property. A replay and a legitimate re-issuance of the same claim are
    // indistinguishable byte strings, so no fold over the log can tell them
    // apart by inspecting them.
    let reissued = sign_delegation(&key, &claim_at(ORIGIN, 1)).unwrap();
    assert_eq!(
        replayed, reissued,
        "two issuances of one claim must be byte-identical — if this ever \
         stops holding, Ed25519 determinism or the encoder has changed and the \
         reasoning behind `sequence` needs revisiting"
    );

    // And the field that makes the replay a no-op rather than a rollback: the
    // replayed artifact carries a sequence already superseded, which a fold can
    // see. The ordering data is INSIDE the signed bytes, so the attacker cannot
    // alter it without the root key.
    let replayed_seq = verify_delegation(&replayed, &key.public_key_bytes(), DOMAIN, ORIGIN)
        .unwrap()
        .claim
        .sequence;
    let current_seq = verify_delegation(&leaf_n, &key.public_key_bytes(), DOMAIN, ORIGIN)
        .unwrap()
        .claim
        .sequence;
    assert!(
        replayed_seq < current_seq,
        "the replayed delegation must be recognisably superseded"
    );

    // The attacker cannot re-sequence it: bumping the number changes the signed
    // bytes, and they do not hold the root key.
    let mut forged = claim_at(ORIGIN, 3);
    forged.delegated_public_key = operational().public_key_bytes();
    let forged_bytes = sign_delegation(&attacker(), &forged).unwrap();
    assert!(
        verify_delegation(&forged_bytes, &key.public_key_bytes(), DOMAIN, ORIGIN).is_err(),
        "re-sequencing requires the root key"
    );
}

#[test]
fn sequence_is_signature_covered_and_survives_the_round_trip() {
    // A field outside the signed bytes would be re-writable by the replaying
    // attacker, which would defeat the whole point.
    let key = root();
    let mut checked = 0;
    for sequence in [
        0u64,
        1,
        23,
        24,
        255,
        256,
        65_535,
        65_536,
        1u64 << 63,
        // The largest issuable value: `u64::MAX` is refused so a successor
        // always exists.
        encoding::MAX_SEQUENCE - 1,
    ] {
        let cose = sign_delegation(&key, &claim_at(ORIGIN, sequence)).unwrap();
        let verified = verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
        assert_eq!(verified.claim.sequence, sequence);

        // Two claims differing only in `sequence` must sign different bytes.
        // Every value above is below MAX_SEQUENCE, so the successor is issuable.
        let other = sign_delegation(&key, &claim_at(ORIGIN, sequence + 1)).unwrap();
        assert_ne!(
            cose, other,
            "sequence {sequence} did not reach the signed bytes"
        );
        checked += 1;
    }
    assert_eq!(
        checked, 10,
        "every head-width boundary must have been tried"
    );
}

#[test]
fn a_sequence_with_no_successor_cannot_be_issued_or_verified() {
    // `u64::MAX` would leave the origin unable to rotate ever again, with no
    // in-band way out. Refused through every entry point, so neither the
    // convenience route nor the air-gapped two-phase route can mint one.
    let key = root();
    let root_key = key.public_key_bytes();

    let mut at_max = claim();
    at_max.sequence = u64::MAX;

    assert!(matches!(
        sign_delegation(&key, &at_max),
        Err(TrustError::DelegationEncoding { .. })
    ));
    let signature = key.sign(&delegation_preimage(&root_key, &at_max));
    assert!(matches!(
        assemble_delegation(&root_key, &at_max, &signature),
        Err(TrustError::DelegationEncoding { .. })
    ));

    // And verification refuses one built by hand and signed for real, so an
    // artifact from another implementation is refused too.
    let mutant = signed_envelope(
        &protected_for(&key),
        &encoding::payload_bytes(&at_max),
        &key,
    );
    assert!(matches!(
        verify_delegation(&mutant, &root_key, DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // Positive control at the boundary: one below is issuable and verifies, so
    // the refusal is about the maximum and not about large sequences.
    let mut at_limit = claim();
    at_limit.sequence = encoding::MAX_SEQUENCE;
    let ok = sign_delegation(&key, &at_limit).unwrap();
    assert_eq!(
        verify_delegation(&ok, &root_key, DOMAIN, ORIGIN)
            .unwrap()
            .claim
            .sequence,
        encoding::MAX_SEQUENCE
    );
}

// ---------------------------------------------------------------------------
// §2.2 — `not_before` still orders nothing.
// ---------------------------------------------------------------------------

#[test]
fn not_before_is_never_an_ordering_key() {
    // Two delegations sharing a `not_before`, and a later one bearing an earlier
    // `not_before`, are both perfectly well-formed. It is an effectivity claim;
    // `sequence` is what orders.
    let key = root();
    let mut earlier = claim();
    earlier.not_before_unix_ms = 1;
    let mut same = claim();
    same.not_before_unix_ms = 1;
    let mut zero = claim();
    zero.not_before_unix_ms = 0;
    let mut far_future = claim();
    far_future.not_before_unix_ms = u64::MAX;
    // The value an `i64` model would find wire-legal and undecodable: two
    // conforming implementations must not disagree about this artifact.
    let mut past_i64_max = claim();
    past_i64_max.not_before_unix_ms = 1u64 << 63;

    let mut accepted = 0;
    for candidate in [earlier, same, zero, far_future, past_i64_max] {
        let cose = sign_delegation(&key, &candidate).unwrap();
        let verified = verify_delegation(&cose, &key.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
        assert_eq!(
            verified.claim.not_before_unix_ms,
            candidate.not_before_unix_ms
        );
        accepted += 1;
    }
    assert_eq!(accepted, 5, "every timestamp must have been exercised");
}
