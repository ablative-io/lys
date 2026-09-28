#![cfg(test)]
//! go-cose refuses a flipped signature and the wrong root key.

use super::*;

/// go-cose refuses what it should refuse.
///
/// A gate made only of successes cannot tell a working verifier from a broken
/// one, and a gate made only of refusals cannot tell a working verifier from one
/// that refuses everything — so the control is first and the refusals follow.
#[test]
fn go_cose_refuses_a_flipped_signature_and_the_wrong_root_key() {
    let Some(go) = go_or_skip("go-cose delegation negatives") else {
        return;
    };
    let (workdir, bin) = cose_tool(&go);

    let (_, root) = identity(&ROOT_SEED);
    let (_, delegated) = identity(&DELEGATED_SEED);
    let (_, other_root) = identity(&OTHER_ROOT_SEED);
    let root_public_key = root.public_key_bytes();
    let other_root_public_key = other_root.public_key_bytes();
    assert_ne!(
        root_public_key, other_root_public_key,
        "the two root keys must differ or the wrong-key case is a no-op"
    );
    let root_hex = to_hex(&root_public_key);
    let other_hex = to_hex(&other_root_public_key);

    let claim = DelegationClaim {
        subject_kind: DelegationSubjectKind::Domain,
        subject_value: ORIGIN.to_string(),
        delegated_public_key: delegated.public_key_bytes(),
        role: DelegationRole::Operational,
        not_before_unix_ms: 1_700_000_000_000,
        sequence: 300,
    };
    let honest = sign_delegation(&root, &claim).unwrap();

    // ---- Positive control, FIRST. Every case below asserts a refusal, and a
    // tool that refused everything would satisfy all of them at once. That is
    // not hypothetical in this repository: drifting a Go content-type constant
    // by one character once left a conformance test made of refusals entirely
    // green.
    let (ok, stdout) = run_built_tool(&bin, &["delegation-verify", root_hex.as_str()], &honest);
    assert!(
        ok,
        "the control failed: go-cose refused an honest delegation, so no \
         refusal below means anything"
    );
    let report = parse_report(&stdout);
    assert_eq!(
        report["sig_structure"],
        to_hex(&delegation_preimage(&root_public_key, &claim)),
        "the control failed: go-cose exited 0 without agreeing about the \
         signed bytes"
    );

    // ---- A flipped bit in the signature. The last 64 bytes of the artifact are
    // the signature; the assertion below proves the flip landed there rather
    // than trusting the layout.
    let mut flipped = honest.clone();
    let sig_start = flipped.len() - 64;
    flipped[sig_start] ^= 0x01;
    assert_ne!(flipped, honest, "the flip must change the artifact");
    assert_eq!(
        flipped[..sig_start],
        honest[..sig_start],
        "the flip must land in the signature, not in the signed bytes"
    );
    let (ok, _) = run_built_tool(&bin, &["delegation-verify", root_hex.as_str()], &flipped);
    assert!(
        !ok,
        "go-cose accepted a delegation with a flipped signature bit"
    );
    assert!(
        verify_delegation(
            &flipped,
            &root_public_key,
            DelegationSubjectKind::Domain,
            ORIGIN
        )
        .is_err(),
        "lys accepted a delegation with a flipped signature bit"
    );

    // ---- The central trap (spec §3.2), from the Go side. The artifact is
    // untouched and perfectly signed; only the key the verifier was TOLD to use
    // changes. A tool that read the key out of `kid` would accept this.
    let (ok, _) = run_built_tool(&bin, &["delegation-verify", other_hex.as_str()], &honest);
    assert!(
        !ok,
        "go-cose accepted an honest delegation against a root key that did not \
         sign it — it must be verifying against `kid` rather than against the \
         key it was given"
    );

    // ---- And the same trap from the lys side, so the two implementations are
    // shown to refuse it for the same reason rather than only the Go one being
    // checked. A delegation genuinely signed by the OTHER root key is
    // cryptographically perfect and vouches for nothing.
    let attacker = sign_delegation(&other_root, &claim).unwrap();
    let (ok, _) = run_built_tool(&bin, &["delegation-verify", other_hex.as_str()], &attacker);
    assert!(
        ok,
        "the control failed: an attacker's delegation must be internally valid, \
         otherwise the refusal below is about malformedness rather than about trust"
    );
    assert!(
        verify_delegation(
            &attacker,
            &root_public_key,
            DelegationSubjectKind::Domain,
            ORIGIN
        )
        .is_err(),
        "lys accepted a delegation signed by a root key the caller does not trust"
    );
    assert!(
        verify_delegation(
            &attacker,
            &other_root_public_key,
            DelegationSubjectKind::Domain,
            ORIGIN
        )
        .is_ok(),
        "the control failed: the attacker's own delegation must verify under \
         the attacker's own key"
    );
    drop(workdir);
}
