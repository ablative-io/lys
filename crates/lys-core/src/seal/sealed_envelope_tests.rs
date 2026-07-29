#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::keys::identity::Ed25519Identity;
use tempfile::tempdir;

/// The canonical order-8 low-order point on Curve25519 (little-endian
/// u-coordinate), from the curve25519 "blacklisted" small-order list.
const LOW_ORDER_POINT_ORDER_8: [u8; 32] = [
    0xe0, 0xeb, 0x7a, 0x7c, 0x3b, 0x41, 0xb8, 0xae, 0x16, 0x56, 0xe3, 0xfa, 0xf1, 0x9f, 0xc4, 0x6a,
    0xda, 0x09, 0x8d, 0xeb, 0x9c, 0x32, 0xb1, 0xfd, 0x86, 0x62, 0x05, 0x16, 0x5f, 0x49, 0xb8, 0x00,
];

fn identity() -> Ed25519Identity {
    let dir = tempdir().unwrap();
    Ed25519Identity::load_or_generate(&dir.path().join("id.key")).unwrap()
}

#[test]
fn seal_returns_envelope_with_expected_field_shapes() {
    let recipient = identity();
    let payload = b"sealed credential bundle";
    let env = seal(payload, &recipient.x25519_public_key()).unwrap();
    assert_eq!(env.ephemeral_public_key.len(), 32);
    assert_eq!(env.nonce.len(), 12);
    // AES-GCM ciphertext = plaintext + 16-byte authentication tag.
    assert_eq!(env.ciphertext.len(), payload.len() + 16);
}

#[test]
fn seal_open_roundtrip_returns_original_payload() {
    let recipient = identity();
    let payload = b"API token: super-secret";
    let env = seal(payload, &recipient.x25519_public_key()).unwrap();
    let opened = open(&env, &recipient.x25519_static_secret()).unwrap();
    assert_eq!(opened.as_slice(), payload);
}

#[test]
fn open_with_wrong_private_key_returns_unseal_failed() {
    let recipient = identity();
    let wrong = identity();
    let env = seal(b"payload", &recipient.x25519_public_key()).unwrap();
    let err = open(&env, &wrong.x25519_static_secret()).unwrap_err();
    assert!(matches!(err, TrustError::UnsealFailed));
}

#[test]
fn tampered_ciphertext_returns_unseal_failed() {
    let recipient = identity();
    let mut env = seal(b"payload", &recipient.x25519_public_key()).unwrap();
    // Flip one bit in the ciphertext; AES-GCM authentication tag will fail.
    env.ciphertext[0] ^= 0x01;
    let err = open(&env, &recipient.x25519_static_secret()).unwrap_err();
    assert!(matches!(err, TrustError::UnsealFailed));
}

#[test]
fn tampered_nonce_returns_unseal_failed() {
    let recipient = identity();
    let mut env = seal(b"payload", &recipient.x25519_public_key()).unwrap();
    env.nonce[0] ^= 0x01;
    // The derived AES key is still correct, but the operative nonce no
    // longer matches the one the ciphertext was encrypted under — the
    // GCM tag check is the single arbiter and rejects it.
    let err = open(&env, &recipient.x25519_static_secret()).unwrap_err();
    assert!(matches!(err, TrustError::UnsealFailed));
}

#[test]
fn tampered_ephemeral_key_returns_unseal_failed() {
    let recipient = identity();
    let mut env = seal(b"payload", &recipient.x25519_public_key()).unwrap();
    // Replace ephemeral key with an unrelated one — the shared secret
    // changes, the derived AES key is wrong, and the GCM tag fails.
    let other = identity();
    env.ephemeral_public_key = other.x25519_public_key();
    let err = open(&env, &recipient.x25519_static_secret()).unwrap_err();
    assert!(matches!(err, TrustError::UnsealFailed));
}

#[test]
fn low_order_ephemeral_key_all_zero_returns_unseal_failed() {
    let recipient = identity();
    let mut env = seal(b"payload", &recipient.x25519_public_key()).unwrap();
    // The all-zero point is low-order: Diffie-Hellman with it yields an
    // all-zero shared secret regardless of the private key. The
    // contributory-behaviour check must reject it.
    env.ephemeral_public_key = [0u8; 32];
    let err = open(&env, &recipient.x25519_static_secret()).unwrap_err();
    assert!(matches!(err, TrustError::UnsealFailed));
}

#[test]
fn low_order_ephemeral_key_order_8_returns_unseal_failed() {
    let recipient = identity();
    let mut env = seal(b"payload", &recipient.x25519_public_key()).unwrap();
    env.ephemeral_public_key = LOW_ORDER_POINT_ORDER_8;
    let err = open(&env, &recipient.x25519_static_secret()).unwrap_err();
    assert!(matches!(err, TrustError::UnsealFailed));
}

#[test]
fn sealing_to_low_order_recipient_key_returns_seal_error() {
    for low_order in [[0u8; 32], LOW_ORDER_POINT_ORDER_8] {
        let err = seal(b"payload", &low_order).unwrap_err();
        match err {
            TrustError::Seal { reason } => {
                assert!(
                    reason.contains("low-order"),
                    "unexpected seal error reason: {reason}"
                );
            }
            other => panic!("expected TrustError::Seal, got {other:?}"),
        }
    }
}

#[test]
fn two_seals_of_same_payload_produce_distinct_ciphertexts_and_keys() {
    let recipient = identity();
    let payload = b"identical payload";
    let env_a = seal(payload, &recipient.x25519_public_key()).unwrap();
    let env_b = seal(payload, &recipient.x25519_public_key()).unwrap();
    assert_ne!(env_a.ephemeral_public_key, env_b.ephemeral_public_key);
    assert_ne!(env_a.ciphertext, env_b.ciphertext);
    assert_ne!(env_a.nonce, env_b.nonce);
}

#[test]
fn empty_payload_seals_and_opens() {
    let recipient = identity();
    let env = seal(&[], &recipient.x25519_public_key()).unwrap();
    // Empty plaintext still produces a 16-byte authentication tag.
    assert_eq!(env.ciphertext.len(), 16);
    let opened = open(&env, &recipient.x25519_static_secret()).unwrap();
    assert!(opened.is_empty());
}

#[test]
fn sealed_envelope_round_trips_through_postcard() {
    let recipient = identity();
    let env = seal(b"persisted", &recipient.x25519_public_key()).unwrap();
    let bytes = postcard::to_allocvec(&env).unwrap();
    let restored: SealedEnvelope = postcard::from_bytes(&bytes).unwrap();
    assert_eq!(restored, env);
    let opened = open(&restored, &recipient.x25519_static_secret()).unwrap();
    assert_eq!(opened.as_slice(), b"persisted");
}

/// Freeze guard for the `lys/sealed-envelope/v1` JSON shape.
///
/// `WIRE-FORMATS.md` freezes this format as "JSON (serde shape of
/// [`SealedEnvelope`])", so as of 0.1.0 the three field names and their
/// encodings are a permanent wire contract: an envelope written by any
/// version must be readable by every later one.
///
/// The postcard round-trip above cannot guard it. Postcard is positional,
/// so a stray `#[serde(rename)]` leaves that test green while silently
/// breaking every historical JSON envelope. This test pins the shape from
/// both directions — what we emit, and what we must accept from a
/// third-party implementation that only ever read the spec.
#[test]
fn sealed_envelope_json_shape_is_frozen() {
    let envelope = SealedEnvelope {
        ephemeral_public_key: [1u8; 32],
        ciphertext: vec![0xde, 0xad, 0xbe, 0xef],
        nonce: [2u8; NONCE_LEN],
    };

    let value = serde_json::to_value(&envelope).unwrap();
    let object = value
        .as_object()
        .expect("a sealed envelope serialises as a JSON object");

    // Exactly these three keys. An *added* field breaks older readers just
    // as surely as a renamed one, so the set is pinned, not just probed.
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["ciphertext", "ephemeral_public_key", "nonce"],
        "the frozen JSON field set changed"
    );

    // Encodings: byte arrays, with both fixed-width fields at their exact
    // declared lengths.
    assert_eq!(
        object["ephemeral_public_key"],
        serde_json::json!(vec![1u8; 32])
    );
    assert_eq!(
        object["ciphertext"],
        serde_json::json!([0xde, 0xad, 0xbe, 0xef])
    );
    assert_eq!(object["nonce"], serde_json::json!(vec![2u8; NONCE_LEN]));

    // The reading direction is the half that actually matters to a
    // stranger: a hand-written document must deserialise to the same
    // envelope, byte for byte.
    let from_wire: SealedEnvelope = serde_json::from_str(
        r#"{
            "ephemeral_public_key":
                [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
            "ciphertext": [222,173,190,239],
            "nonce": [2,2,2,2,2,2,2,2,2,2,2,2]
        }"#,
    )
    .unwrap();
    assert_eq!(from_wire, envelope);
}

#[test]
fn attestation_bytes_concatenates_in_declared_order() {
    let recipient = identity();
    let env = seal(b"x", &recipient.x25519_public_key()).unwrap();
    let bytes = env.attestation_bytes();
    let expected_len = env.ephemeral_public_key.len() + env.ciphertext.len() + env.nonce.len();
    assert_eq!(bytes.len(), expected_len);
    assert_eq!(&bytes[..32], env.ephemeral_public_key.as_slice());
    assert_eq!(
        &bytes[32..32 + env.ciphertext.len()],
        env.ciphertext.as_slice()
    );
    assert_eq!(&bytes[32 + env.ciphertext.len()..], env.nonce.as_slice());
}
