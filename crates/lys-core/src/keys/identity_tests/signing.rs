#![cfg(test)]
//! Debug redaction, the public key bytes, signing, and the refusals `verify`
//! makes on its signature and public key arguments.

use super::*;

// ─── Debug redaction ──────────────────────────────────────────────

#[test]
fn debug_redacts_signing_key() {
    let seed = [7u8; 32];
    let id = identity_from_seed(seed);
    let dbg = format!("{id:?}");
    assert!(dbg.contains("[REDACTED]"), "got: {dbg}");
    assert!(
        !dbg.contains("07, 07, 07"),
        "raw seed bytes leaked in debug (hex): {dbg}"
    );
    assert!(
        !dbg.contains("7, 7, 7, 7"),
        "raw seed bytes leaked in debug (decimal array): {dbg}"
    );
    assert!(
        !dbg.contains("SigningKey("),
        "default SigningKey tuple debug leaked: {dbg}"
    );
    assert!(
        !dbg.contains("SigningKey {"),
        "default SigningKey struct debug leaked: {dbg}"
    );
}

#[test]
fn debug_includes_verifying_key_hex() {
    let seed = [7u8; 32];
    let id = identity_from_seed(seed);
    let dbg = format!("{id:?}");
    let expected_hex = id.public_key_bytes_hex();
    assert!(
        dbg.contains(&expected_hex),
        "verifying key hex missing from debug: {dbg}"
    );
}

// ─── public_key_bytes accessor ────────────────────────────────────

#[test]
fn public_key_bytes_returns_32_bytes() {
    let id = identity_from_seed([1u8; 32]);
    let bytes = id.public_key_bytes();
    assert_eq!(bytes.len(), 32);
}

#[test]
fn public_key_bytes_round_trips_through_verifying_key() {
    let id = identity_from_seed([2u8; 32]);
    let bytes = id.public_key_bytes();
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&bytes).unwrap();
    assert_eq!(vk.to_bytes(), bytes);
}

#[test]
fn public_key_bytes_stable_across_calls() {
    let id = identity_from_seed([3u8; 32]);
    assert_eq!(id.public_key_bytes(), id.public_key_bytes());
}

// ─── sign ─────────────────────────────────────────────────────────

#[test]
fn sign_produces_64_byte_signature() {
    let id = identity_from_seed([4u8; 32]);
    let sig = id.sign(b"hello");
    assert_eq!(sig.len(), 64);
}

#[test]
fn sign_then_verify_roundtrip() {
    let id = identity_from_seed([5u8; 32]);
    let msg = b"hello world";
    let sig = id.sign(msg);
    Ed25519Identity::verify(&id.public_key_bytes(), msg, &sig).unwrap();
}

#[test]
fn sign_different_messages_yields_different_signatures() {
    let id = identity_from_seed([6u8; 32]);
    let sig_a = id.sign(b"message A");
    let sig_b = id.sign(b"message B");
    assert_ne!(sig_a, sig_b);
}

#[test]
fn sign_empty_message() {
    let id = identity_from_seed([8u8; 32]);
    let sig = id.sign(b"");
    assert_eq!(sig.len(), 64);
    Ed25519Identity::verify(&id.public_key_bytes(), b"", &sig).unwrap();
}

// ─── verify ───────────────────────────────────────────────────────

#[test]
fn verify_rejects_tampered_message() {
    let id = identity_from_seed([10u8; 32]);
    let sig = id.sign(b"original");
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"tampered", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_wrong_public_key() {
    let id_a = identity_from_seed([11u8; 32]);
    let id_b = identity_from_seed([12u8; 32]);
    let sig = id_a.sign(b"msg");
    let result = Ed25519Identity::verify(&id_b.public_key_bytes(), b"msg", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_short_signature() {
    let id = identity_from_seed([13u8; 32]);
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"msg", &[0u8; 32]);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_long_signature() {
    let id = identity_from_seed([14u8; 32]);
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"msg", &[0u8; 128]);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_empty_signature() {
    let id = identity_from_seed([15u8; 32]);
    let result = Ed25519Identity::verify(&id.public_key_bytes(), b"msg", &[]);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_malformed_public_key() {
    // all-`0xff` is the easy non-canonical encoding: `y = p + 18` once the
    // sign bit is masked. It is now refused by the key rule rather than by
    // the signature check — see the canonical-encoding section below for why
    // that distinction is invisible to this assertion.
    let id = identity_from_seed([16u8; 32]);
    let sig = id.sign(b"msg");
    let result = Ed25519Identity::verify(&[0xff; 32], b"msg", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_small_order_public_key() {
    // [0u8; 32] encodes the point with y = 0, which lies on the curve and
    // has order 4. dalek's `VerifyingKey::from_bytes` accepts it (it is a
    // valid point encoding), so a small-order rule is the layer that must
    // reject it.
    //
    // That rule now fires in TWO places: `is_usable_ed25519_public_key`'s
    // third condition, checked by `verify` before it decodes the key, and
    // `verify_strict`'s own weak-key check behind it. Deleting either leaves
    // this test passing, so it proves neither alone — it is the end-to-end
    // statement, and the isolating tests are
    // `is_usable_ed25519_public_key_refuses_the_all_zero_key` (which pins
    // that the small-order clause and not the canonical-y clause is what
    // refuses it) and the dependency pin below.
    let weak_pk = [0u8; 32];
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&weak_pk)
        .expect("y=0 small-order point is a valid encoding dalek accepts");
    assert!(vk.is_weak(), "y=0 point must be classified as weak");

    let id = identity_from_seed([17u8; 32]);
    let sig = id.sign(b"msg");
    let result = Ed25519Identity::verify(&weak_pk, b"msg", &sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}

#[test]
fn verify_rejects_identity_point_forgery_that_passes_non_strict() {
    use ed25519_dalek::Verifier;

    // The Edwards identity point (order 1) encodes as y = 1: [1, 0, ..., 0].
    // For a public key A equal to the identity, k·A is the identity for any
    // hash scalar k, so the verification equation s·B = R + k·A reduces to
    // s·B = R. The forged signature (R = basepoint, s = 1) therefore passes
    // NON-strict verification for ANY message — total forgery. Strict
    // verification rejects the small-order public key outright, which is why
    // Ed25519Identity::verify uses verify_strict.
    let mut weak_pk = [0u8; 32];
    weak_pk[0] = 1;

    // Compressed Ed25519 basepoint.
    let basepoint: [u8; 32] = [
        0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66,
    ];
    let mut forged_sig = [0u8; 64];
    forged_sig[..32].copy_from_slice(&basepoint);
    forged_sig[32] = 1; // s = 1, little-endian

    // Sanity: the forgery really does pass dalek's non-strict verify, for
    // two unrelated messages. This is the exact hole verify_strict closes.
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&weak_pk).unwrap();
    let sig = ed25519_dalek::Signature::from_bytes(&forged_sig);
    vk.verify(b"any message at all", &sig)
        .expect("non-strict verify accepts the small-order forgery");
    vk.verify(b"a completely different message", &sig)
        .expect("non-strict verify accepts the forgery for every message");

    // Our verify must reject it. Since the key rule was added this is also a
    // two-place rejection — `is_usable_ed25519_public_key` refuses the
    // identity point, and `verify_strict` refuses it again — so this test
    // states the outcome and proves neither layer on its own.
    let result = Ed25519Identity::verify(&weak_pk, b"any message at all", &forged_sig);
    assert!(matches!(result, Err(TrustError::InvalidSignature)));
}
