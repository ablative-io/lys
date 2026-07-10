#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::tempdir;

fn identity() -> Ed25519Identity {
    let dir = tempdir().unwrap();
    Ed25519Identity::load_or_generate(&dir.path().join("id.key")).unwrap()
}

#[test]
fn sign_attestation_populates_fields() {
    let id = identity();
    let payload = b"execution receipt v1";
    let att = sign_attestation(payload, &id);

    let expected = sha256_digest(payload);
    assert_eq!(att.payload_hash, expected);
    assert_eq!(att.signer_public_key, id.public_key_bytes());
    assert_eq!(att.signature.len(), 64);
    assert!(
        att.timestamp > 0,
        "timestamp should be a positive unix-millisecond value, got {}",
        att.timestamp
    );
}

#[test]
fn verify_attestation_accepts_valid_signature() {
    let id = identity();
    let payload = b"audit entry payload";
    let att = sign_attestation(payload, &id);
    verify_attestation(&att, payload).unwrap();
}

#[test]
fn verify_attestation_rejects_tampered_payload() {
    let id = identity();
    let payload = b"original";
    let att = sign_attestation(payload, &id);
    let tampered = b"originaL";
    let err = verify_attestation(&att, tampered).unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));
}

#[test]
fn verify_attestation_rejects_tampered_signature() {
    let id = identity();
    let payload = b"payload";
    let mut att = sign_attestation(payload, &id);
    att.signature[0] ^= 0x01;
    let err = verify_attestation(&att, payload).unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));
}

#[test]
fn verify_attestation_rejects_tampered_timestamp() {
    let id = identity();
    let payload = b"timestamped payload";
    let mut att = sign_attestation(payload, &id);
    // The timestamp is a signed claim — shifting it by one millisecond
    // must invalidate the signature.
    att.timestamp += 1;
    let err = verify_attestation(&att, payload).unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));
}

#[test]
fn verify_attestation_rejects_wrong_signer_key() {
    let id_a = identity();
    let id_b = identity();
    let payload = b"payload";
    let mut att = sign_attestation(payload, &id_a);
    // Swap in a different (valid) public key — the signature was produced
    // by id_a, and the kid is signature-covered, so verification against
    // id_b must fail.
    att.signer_public_key = id_b.public_key_bytes();
    let err = verify_attestation(&att, payload).unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));
}

#[test]
fn attestation_signature_differs_from_raw_sign_over_hash() {
    // Domain separation: an attestation signature is never the same as a
    // raw Ed25519 signature over the bare payload hash, so the two signing
    // contexts cannot be confused.
    let id = identity();
    let payload = b"domain separated";
    let att = sign_attestation(payload, &id);
    let raw = id.sign(&att.payload_hash);
    assert_ne!(att.signature, raw);
}

#[test]
fn attestation_signature_differs_from_raw_sign_over_payload() {
    // Nor over the raw payload bytes themselves: the Sig_structure framing
    // separates the attestation from any raw-sign use.
    let id = identity();
    let payload = b"domain separated";
    let att = sign_attestation(payload, &id);
    let raw = id.sign(payload);
    assert_ne!(att.signature, raw);
}

#[test]
fn old_v1_preimage_signature_is_rejected() {
    // Cross-form kill: a signature over the deleted v1 preimage
    // (`lys/attestation/v1 || timestamp_le || hash`), spliced into an
    // otherwise-valid v2 artifact, must never verify. The Sig_structure
    // begins 0x84 0x6A "Signature1" while the v1 preimage began 0x6C, so
    // the signed byte languages are disjoint at byte 0 — this test makes
    // the confusion provably dead in the only direction that is even
    // expressible (the reverse cannot be constructed: no v1 preimage can
    // begin with a Sig_structure encoding's bytes and still carry the v1
    // tag at position 0).
    let id = identity();
    let payload = b"cross-form confusion";
    let payload_hash = sha256_digest(payload);
    let timestamp = 1_700_000_000_000_i64;

    let mut preimage = b"lys/attestation/v1".to_vec();
    preimage.extend_from_slice(&timestamp.to_le_bytes());
    preimage.extend_from_slice(&payload_hash);

    let att = Attestation {
        payload_hash,
        signature: id.sign(&preimage),
        signer_public_key: id.public_key_bytes(),
        timestamp,
    };
    let err = verify_attestation(&att, payload).unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));
}

#[test]
fn bare_hash_signed_attestation_is_rejected() {
    // Cross-form kill: a signature over the bare payload hash (the
    // pre-domain-separation scheme stripped at extraction), spliced into an
    // otherwise-valid attestation, must never verify — there is no legacy
    // fallback path. A 32-byte hash can never begin with the Sig_structure
    // framing bytes as a message the verifier would rebuild, so the only
    // expressible confusion direction is this splice.
    let id = identity();
    let payload = b"pre-domain-separation";
    let payload_hash = sha256_digest(payload);

    let att = Attestation {
        payload_hash,
        signature: id.sign(&payload_hash),
        signer_public_key: id.public_key_bytes(),
        timestamp: 1_700_000_000_000,
    };
    let err = verify_attestation(&att, payload).unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));
}

#[test]
fn attestation_round_trips_through_cose_bytes() {
    let id = identity();
    let payload = b"persisted attestation";
    let att = sign_attestation(payload, &id);
    let bytes = att.to_cose_bytes();
    let restored = Attestation::from_cose_bytes(&bytes).unwrap();
    assert_eq!(restored, att);
    verify_attestation(&restored, payload).unwrap();
}

#[test]
fn empty_payload_signs_and_verifies() {
    let id = identity();
    let att = sign_attestation(&[], &id);
    verify_attestation(&att, &[]).unwrap();
    // The empty-input SHA-256 digest is well-known: e3b0c442….
    let expected: [u8; 32] = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];
    assert_eq!(att.payload_hash, expected);
}

#[test]
fn verify_attestation_bytes_accepts_valid_artifact() {
    let id = identity();
    let payload = b"one-shot verification";
    let att = sign_attestation(payload, &id);
    let restored = verify_attestation_bytes(&att.to_cose_bytes(), payload).unwrap();
    assert_eq!(restored, att);
}

#[test]
fn verify_attestation_bytes_rejects_wrong_payload_and_malformed_bytes() {
    let id = identity();
    let att = sign_attestation(b"right payload", &id);
    let bytes = att.to_cose_bytes();

    let err = verify_attestation_bytes(&bytes, b"wrong payload").unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));

    let err = verify_attestation_bytes(b"not cose", b"right payload").unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));

    // A flipped signature byte fails identically (non-oracle).
    let mut tampered = bytes;
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    let err = verify_attestation_bytes(&tampered, b"right payload").unwrap_err();
    assert!(matches!(err, TrustError::InvalidSignature));
}
