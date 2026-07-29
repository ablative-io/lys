#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use tempfile::tempdir;

fn identity() -> Ed25519Identity {
    let dir = tempdir().unwrap();
    Ed25519Identity::load_or_generate(&dir.path().join("id.key")).unwrap()
}

#[test]
fn sign_and_seal_returns_envelope_and_attestation() {
    let sender = identity();
    let recipient = identity();
    let payload = b"authenticated credential bundle";
    let (env, att) = sign_and_seal(payload, &sender, &recipient.x25519_public_key()).unwrap();
    assert_eq!(att.signer_public_key, sender.public_key_bytes());
    assert_eq!(env.ciphertext.len(), payload.len() + 16);
}

#[test]
fn open_and_verify_returns_payload_for_correct_sender() {
    let sender = identity();
    let recipient = identity();
    let payload = b"audit-bound payload";
    let (env, att) = sign_and_seal(payload, &sender, &recipient.x25519_public_key()).unwrap();
    let opened = open_and_verify(
        &env,
        &att,
        &sender.public_key_bytes(),
        &recipient.x25519_static_secret(),
    )
    .unwrap();
    assert_eq!(opened.as_slice(), payload);
}

#[test]
fn wrong_sender_public_key_returns_attestation_failed_without_unsealing() {
    let sender = identity();
    let imposter = identity();
    let recipient = identity();
    let (env, att) = sign_and_seal(b"payload", &sender, &recipient.x25519_public_key()).unwrap();
    let err = open_and_verify(
        &env,
        &att,
        &imposter.public_key_bytes(),
        &recipient.x25519_static_secret(),
    )
    .unwrap_err();
    assert!(matches!(err, TrustError::AttestationFailed));
}

#[test]
fn tampered_attestation_signature_returns_attestation_failed() {
    let sender = identity();
    let recipient = identity();
    let (env, mut att) =
        sign_and_seal(b"payload", &sender, &recipient.x25519_public_key()).unwrap();
    att.signature[0] ^= 0x01;
    let err = open_and_verify(
        &env,
        &att,
        &sender.public_key_bytes(),
        &recipient.x25519_static_secret(),
    )
    .unwrap_err();
    assert!(matches!(err, TrustError::AttestationFailed));
}

#[test]
fn forged_signer_key_in_attestation_returns_attestation_failed() {
    let sender = identity();
    let imposter = identity();
    let recipient = identity();
    let (env, mut att) =
        sign_and_seal(b"payload", &sender, &recipient.x25519_public_key()).unwrap();
    // Adversary swaps in their own public key claiming the signature is
    // theirs. The two-step gate must reject this — first the explicit
    // key comparison would pass (we now ask to verify against the
    // imposter's key), but the signature was produced by `sender` over
    // the original hash, so verification against `imposter` fails.
    att.signer_public_key = imposter.public_key_bytes();
    let err = open_and_verify(
        &env,
        &att,
        &imposter.public_key_bytes(),
        &recipient.x25519_static_secret(),
    )
    .unwrap_err();
    assert!(matches!(err, TrustError::AttestationFailed));
}

#[test]
fn tampered_envelope_after_signing_returns_attestation_failed() {
    let sender = identity();
    let recipient = identity();
    let (mut env, att) =
        sign_and_seal(b"payload", &sender, &recipient.x25519_public_key()).unwrap();
    env.ciphertext[0] ^= 0x01;
    let err = open_and_verify(
        &env,
        &att,
        &sender.public_key_bytes(),
        &recipient.x25519_static_secret(),
    )
    .unwrap_err();
    // The attestation covers the envelope bytes, so a post-signing
    // tamper is rejected at the attestation gate, not the AES-GCM gate.
    assert!(matches!(err, TrustError::AttestationFailed));
}

#[test]
fn generic_attestation_over_bare_envelope_bytes_is_rejected() {
    // Context binding: an attestation the sender produced with the
    // generic primitive over the envelope's canonical bytes (no context
    // tag) must not be accepted by the authenticated composition.
    let sender = identity();
    let recipient = identity();
    let (env, _att) = sign_and_seal(b"payload", &sender, &recipient.x25519_public_key()).unwrap();
    let generic = crate::attestation::sign::sign_attestation(&env.attestation_bytes(), &sender);
    let err = open_and_verify(
        &env,
        &generic,
        &sender.public_key_bytes(),
        &recipient.x25519_static_secret(),
    )
    .unwrap_err();
    assert!(matches!(err, TrustError::AttestationFailed));
}

#[test]
fn standalone_seal_and_open_still_work() {
    // The composed API is a strict superset; the standalone primitives
    // must remain functional and unrelated — `seal`/`open` are not
    // replaced by the authenticated composition.
    let recipient = identity();
    let env = seal(b"anonymous", &recipient.x25519_public_key()).unwrap();
    let opened = open(&env, &recipient.x25519_static_secret()).unwrap();
    assert_eq!(opened.as_slice(), b"anonymous");
}
