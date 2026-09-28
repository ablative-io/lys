#![cfg(test)]
//! Every verification failure is one error, and the verifier does the same
//! work, signature check included, for every kind of mismatch.

use super::*;

// ---------------------------------------------------------------------------
// §3.5.1 — non-oracle means the CALL, not just the returned value.
// ---------------------------------------------------------------------------

/// **THE test for the timing fix**: the Ed25519 verification must run exactly
/// once per call, whatever the mismatch is.
///
/// # Why this is counted rather than asserted behaviourally
///
/// **A behavioural test cannot see this defect at all.** The fix changes no
/// behaviour: an early return on a `kid` mismatch produces the same error for
/// the same inputs, so every outcome assertion in this file passes with the
/// defect present. That is measured, not supposed — restoring the early return
/// was injected as a drift and **all 61 tests stayed green**, this one included,
/// in its first and purely behavioural form. A rule whose violation changes only
/// the work done needs an instrument that observes the work.
///
/// # And the injection that "passed" without reproducing the defect
///
/// Worth recording because it nearly produced a confident false claim. The
/// first attempt at that drift injection placed the early return **after** the
/// signature check. The suite stayed green — correctly, because the expensive
/// work had already happened, so the oracle was never restored. Read carelessly
/// that looks like "the suite tolerates the defect"; read carelessly the other
/// way, after the counter was added, it would have looked like "the counter
/// catches nothing."
///
/// **That is phantom resolution: a probe that does not exercise the variable
/// under test, whose result is then attributed to the code.** The only reason it
/// surfaced is that the mutated source was read back rather than trusted. An
/// injection is evidence about a rule *only* once you have confirmed it
/// reproduces the defect; a green suite under a mis-aimed injection is evidence
/// about the injection.
///
/// Correctly placed — before the signature check — the injection fails exactly
/// this test and nothing else.
///
/// # Why not a wall-clock assertion
///
/// It is the direct measurement and the wrong tool here: flaky under load, it
/// needs a sampling design to mean anything, and a machine fast enough to blur
/// the difference would report success. The reviewer's 32.8× figure came from
/// round-robin sampling with medians and a control arm reproducing to 1.001×,
/// which belongs outside a unit suite. The counter is deterministic and answers
/// the same question.
///
/// The counter is thread-local and lives beside the verification it counts, so
/// an edit that skips the work skips the increment too.
#[test]
fn signature_verification_runs_for_every_mismatch_kind() {
    use crate::delegation::sign::SIGNATURE_VERIFICATIONS;

    let key = root();
    let expected = key.public_key_bytes();
    let count = || SIGNATURE_VERIFICATIONS.with(std::cell::Cell::get);

    // Each artifact is well-formed and parses, so every case reaches the
    // verification stage. Their signature validity differs; that is the point.
    let wrong_kid = sign_delegation(&attacker(), &claim()).unwrap();
    let wrong_origin = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();
    let mut bad_signature = sign_delegation(&key, &claim()).unwrap();
    let last = bad_signature.len() - 1;
    bad_signature[last] ^= 0x01;
    let honest = sign_delegation(&key, &claim()).unwrap();

    // ⛔ THE ARM THIS TEST WAS MISSING, and its absence is worth reading twice.
    //
    // `verify_delegation` grew a fourth comparison — the subject KIND — and this
    // test was not widened with it. Every arm above is a `Domain` artifact at a
    // `Domain` verifier, so `subject_kind_ok` was TRUE in all four: an early
    // return added to the kind comparison could only fire for an artifact none
    // of them produced, and the counter would have stayed at 1 throughout.
    // **The one field the specification flags as never having been measured for
    // leakage was the one field guarded by nothing here.**
    //
    // A valid seat delegation, genuinely signed, at a `Domain`-expecting
    // verifier: it parses, its pair is in the table, its `kid` matches and its
    // subject VALUE matches — `seat_claim_for(ORIGIN)` deliberately reuses the
    // domain fixture's string — so the kind is the only thing wrong with it, and
    // the signature check must still run.
    let seat_at_a_domain_verifier = sign_delegation(&key, &seat_claim_for(ORIGIN)).unwrap();

    let mut measured = 0;
    for (name, artifact, should_verify) in [
        ("wrong kid", wrong_kid, false),
        ("wrong subject value", wrong_origin, false),
        ("wrong subject kind", seat_at_a_domain_verifier, false),
        ("bad signature", bad_signature, false),
        ("honest", honest, true),
    ] {
        let before = count();
        let outcome = verify_delegation(&artifact, &expected, DOMAIN, ORIGIN);
        let after = count();

        assert_eq!(
            after - before,
            1,
            "{name}: the signature verification must run exactly once — a count \
             of 0 means an early return was restored and the 32.8x timing oracle \
             is back"
        );
        assert_eq!(outcome.is_ok(), should_verify, "{name}: wrong outcome");
        measured += 1;
    }
    assert_eq!(
        measured, 5,
        "every arm must have been measured — one per comparison in \
         `verify_delegation` plus the honest case. Widening that function \
         without widening this count leaves the new comparison unguarded, which \
         is exactly how the subject-kind arm came to be missing."
    );

    // The instrument's own control: a call that never reaches the verification
    // stage must NOT increment, or the counter would be measuring nothing and
    // every assertion above would pass regardless of where the check sits.
    let before = count();
    assert!(verify_delegation(b"not cbor at all", &expected, DOMAIN, ORIGIN).is_err());
    assert_eq!(
        count() - before,
        0,
        "a parse failure must not reach the signature check — if this counts, \
         the instrument is incrementing somewhere that proves nothing"
    );
}

/// The behavioural companion to the counted test above: a `kid` mismatch does
/// not cause the signature bytes to go unexamined.
///
/// Kept because it exercises a combination the counted test does not — an
/// artifact that is wrong in *two* ways at once — but note that on its own it
/// cannot detect the defect it is named for. `signature_verification_runs_for_
/// every_mismatch_kind` is the one that can.
#[test]
fn a_wrong_kid_does_not_skip_the_signature_check() {
    let key = root();
    let evil = attacker();

    // An artifact whose `kid` is the attacker's AND whose signature is garbage.
    // If the verifier returned early on the `kid` mismatch, the signature bytes
    // would never be looked at at all.
    let mut both_wrong = sign_delegation(&evil, &claim()).unwrap();
    let sig_at = both_wrong.len() - 64;
    both_wrong[sig_at..].copy_from_slice(&[0u8; 64]);

    assert!(matches!(
        verify_delegation(&both_wrong, &key.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // And the same artifact against the key it names is *also* refused, because
    // the signature really is invalid — which is what proves the signature arm
    // is live rather than vestigial.
    assert!(matches!(
        verify_delegation(&both_wrong, &evil.public_key_bytes(), DOMAIN, ORIGIN),
        Err(TrustError::DelegationVerification)
    ));

    // Positive control: with the signature restored, the attacker's artifact
    // verifies under the attacker's own key. So the refusals above are about
    // the checks and not about a mutant that is broken beyond recognition.
    let intact = sign_delegation(&evil, &claim()).unwrap();
    verify_delegation(&intact, &evil.public_key_bytes(), DOMAIN, ORIGIN).unwrap();
}

/// Every failure arm performs the *same* verification work, which is what makes
/// the collapsed error type an actual non-oracle rather than a cosmetic one.
///
/// An adversarial review measured a 32.8× separation between a `kid`/origin
/// mismatch and a bad signature, because the first two returned before the
/// Ed25519 verification ran. The error values were identical throughout — which
/// is the finding: **a collapsed error type is not a non-oracle if the work done
/// differs per cause.**
///
/// This test cannot measure time reliably, so it asserts the invariant that the
/// fix rests on: for every mismatch kind, the artifact is one whose signature
/// the verifier must have evaluated to reach its answer. The genuinely
/// time-based measurement is the reviewer's, and it belongs outside a unit
/// suite.
#[test]
fn the_verifier_does_the_same_work_for_every_kind_of_mismatch() {
    let key = root();
    let expected = key.public_key_bytes();

    // Three mismatches, each with a PERFECTLY VALID signature over its own
    // bytes. Under the old early-return code the first two skipped the Ed25519
    // verification entirely and the third did not; now all three run it.
    let wrong_kid = sign_delegation(&attacker(), &claim()).unwrap();
    let wrong_origin = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();
    let mut bad_signature = sign_delegation(&key, &claim()).unwrap();
    let last = bad_signature.len() - 1;
    bad_signature[last] ^= 0x01;

    let mut refused = 0;
    for (name, mutant, valid_signature) in [
        ("wrong kid", wrong_kid, true),
        ("wrong origin", wrong_origin, true),
        ("bad signature", bad_signature, false),
    ] {
        // Each mutant's own signature status, asserted rather than assumed, so
        // the three arms really are the three different causes.
        let parsed = Delegation::from_cose_bytes(&mutant).unwrap();
        let preimage = delegation_preimage(&parsed.root_public_key, &parsed.claim);
        let signature_ok =
            Ed25519Identity::verify(&parsed.root_public_key, &preimage, &parsed.signature).is_ok();
        assert_eq!(
            signature_ok, valid_signature,
            "{name} does not have the signature status this test assumes"
        );

        assert!(matches!(
            verify_delegation(&mutant, &expected, DOMAIN, ORIGIN),
            Err(TrustError::DelegationVerification)
        ));
        refused += 1;
    }
    assert_eq!(refused, 3, "every mismatch kind must have been tried");
}

// ---------------------------------------------------------------------------
// §3.5 — non-oracle failure.
// ---------------------------------------------------------------------------

#[test]
fn every_verification_failure_is_the_same_error() {
    let key = root();
    let expected = key.public_key_bytes();
    let good = sign_delegation(&key, &claim()).unwrap();

    // Positive control first: a verifier that refused everything would satisfy
    // every assertion below.
    verify_delegation(&good, &expected, DOMAIN, ORIGIN).unwrap();

    let wrong_root = sign_delegation(&attacker(), &claim()).unwrap();
    let wrong_origin = sign_delegation(&key, &claim_for(OTHER_ORIGIN)).unwrap();

    let mut bad_signature = good.clone();
    let last = bad_signature.len() - 1;
    bad_signature[last] ^= 0x01;

    let relabelled = signed_envelope(
        &encoding::protected_bytes("application/vnd.lys.receipt.v1+cbor", &expected),
        &encoding::payload_bytes(&claim()),
        &key,
    );

    let canonical_payload = encoding::payload_bytes(&claim());
    let role_at = role_offset(&canonical_payload);
    let mut unknown_role_payload = canonical_payload.clone();
    unknown_role_payload[role_at + 1] = 7;
    let unknown_role = signed_envelope(&protected_for(&key), &unknown_role_payload, &key);

    // A pair every field of which is individually valid: domain + speaks-for.
    let mut invalid_pair_payload = canonical_payload;
    invalid_pair_payload[role_at + 1] = 0x03;
    let invalid_pair = signed_envelope(&protected_for(&key), &invalid_pair_payload, &key);

    // A seat delegation, genuinely signed, presented to a domain verifier that
    // names the same subject string.
    let wrong_kind = sign_delegation(&key, &seat_claim_for(ORIGIN)).unwrap();

    let unprotected_at = 4 + encoding::PROTECTED_LEN;
    let mut non_empty_unprotected = good[..unprotected_at].to_vec();
    non_empty_unprotected.extend_from_slice(&[0xa1, 0x01, 0x01]);
    non_empty_unprotected.extend_from_slice(&good[unprotected_at + 1..]);

    let mut trailing = good;
    trailing.push(0x00);

    let cases = [
        ("wrong root key", wrong_root),
        ("wrong subject value", wrong_origin),
        ("forged signature", bad_signature),
        ("relabelled as a receipt", relabelled),
        ("unknown role", unknown_role),
        ("invalid (kind, role) pair", invalid_pair),
        ("wrong subject kind", wrong_kind),
        ("non-empty unprotected", non_empty_unprotected),
        ("trailing garbage", trailing),
        ("empty input", vec![]),
    ];

    let mut refused = 0;
    for (name, mutant) in cases {
        let err = verify_delegation(&mutant, &expected, DOMAIN, ORIGIN).unwrap_err();
        assert!(
            matches!(err, TrustError::DelegationVerification),
            "{name} produced a distinguishable error: {err:?}"
        );
        assert_eq!(
            format!("{err}"),
            format!("{}", TrustError::DelegationVerification),
            "{name} produced a distinguishable message"
        );
        refused += 1;
    }
    assert_eq!(refused, 10, "every case must have been rejected");
}
