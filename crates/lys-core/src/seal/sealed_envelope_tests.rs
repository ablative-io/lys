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

// ---------------------------------------------------------------------------
// ⭐ THE KDF GOLDEN VECTOR — the second party `seal` did not have.
//
// Until this existed, NOTHING pinned the key-derivation construction. Every
// test above round-trips through `seal` and `open`, and both call the same
// `derive_key_and_nonce` — so any SYMMETRIC change is invisible to all of
// them. Swapping the two public keys in the `info` input, renaming the
// domain-separation tag, or dropping the tag entirely leaves the whole suite
// green, because both sides derive the same wrong key and the payload still
// comes back.
//
// That is the round-trip trap in its purest form: a round trip through your own
// encoder and decoder proves nothing about the wire. What it needs is a value
// this crate did not compute.
//
// PROVENANCE OF THE EXPECTED BYTES: computed by a hand-written RFC 5869
// HKDF-SHA256 in Python — not the `hkdf` crate, not this code — and that
// implementation was first validated against RFC 5869 Appendix A test cases 1
// and 3 (case 3 is the zero-length-salt case, which is exactly what
// `Hkdf::new(None, ..)` performs here), and shown able to disagree when its
// `info` was perturbed. So the axis of independence is IMPLEMENTATION AND
// LIBRARY. It is not independence of platform: one machine, one Python.
//
// WHAT THIS VECTOR PINS, each of which was previously unpinned:
//   1. the exact bytes of SEAL_INFO,
//   2. that the tag is present and comes FIRST,
//   3. the ORDER ephemeral-then-recipient (the two inputs are distinct here on
//      purpose, so a swap changes the output),
//   4. the absent/zero salt,
//   5. the OKM split — key is the first 32 bytes, nonce the last 12.
// ---------------------------------------------------------------------------

/// Distinct, non-uniform, and different from each other so that a swap, a
/// repeated buffer, or a dropped input all change the answer.
const KDF_SHARED_SECRET: [u8; 32] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];
const KDF_EPHEMERAL_PK: [u8; 32] = [
    0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e, 0x4f,
    0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x5b, 0x5c, 0x5d, 0x5e, 0x5f,
];
const KDF_RECIPIENT_PK: [u8; 32] = [
    0x60, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e, 0x6f,
    0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e, 0x7f,
];

/// The frozen output of `HKDF-SHA256(ikm = shared, salt = none,
/// info = "lys-sealed-envelope/v1" ‖ ephemeral ‖ recipient, L = 44)`.
const KDF_EXPECTED_KEY: &str = "2174f9e33ad52c304ce42776fdcc71edd909c28bc12712802f1c6eb0960a08bf";
const KDF_EXPECTED_NONCE: &str = "9676a253de3231538aa9c948";

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, b| {
        // Writing into a String is infallible; the Result is discarded rather
        // than unwrapped so this helper cannot panic inside a failing test and
        // obscure the assertion that was actually being made.
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

#[test]
fn the_kdf_reproduces_bytes_this_crate_did_not_compute() {
    let (key, nonce) =
        derive_key_and_nonce(&KDF_SHARED_SECRET, &KDF_EPHEMERAL_PK, &KDF_RECIPIENT_PK)
            .expect("HKDF expansion of 44 bytes is well within the SHA-256 output limit");

    assert_eq!(
        hex(key.as_slice()),
        KDF_EXPECTED_KEY,
        "the derived AES key no longer matches the independently computed HKDF output — \
         the info input, its ordering, the tag, or the OKM split has changed"
    );
    assert_eq!(
        hex(&nonce),
        KDF_EXPECTED_NONCE,
        "the derived nonce no longer matches; note the nonce is the LAST 12 bytes of the \
         44-byte OKM, so a changed split moves it even when the key still matches"
    );
}

/// ⛔ **This test is ORDER-SENSITIVITY only, and its previous name —
/// `swapping_the_two_public_keys_changes_the_derived_key` — claimed more than
/// it measures.**
///
/// It was renamed after an injection refuted it. Swapping the two
/// `extend_from_slice` calls inside `derive_key_and_nonce` failed the golden
/// vector and **left this test green**, because it compares
/// `derive(s, e, r)` against `derive(s, r, e)`: when the implementation
/// swaps, both sides swap with it and the two are still different. It can
/// therefore detect an implementation that ignores order altogether — one that
/// sorted or xor-ed the two keys — and it can **never** detect a wrong order.
///
/// ⭐ A test that varies an input symmetrically with the code under test is
/// blind to any change the code applies to both sides. Only the golden vector,
/// whose expected bytes came from outside this crate, catches a swap — which
/// is why the swap injection fails exactly one test and it is that one.
///
/// It is kept, at its honest name, for a narrower reason: if the golden vector
/// is ever deleted or regenerated from this code (the usual way a golden
/// vector dies), this still refuses an order-blind derivation.
#[test]
fn the_info_input_is_order_sensitive_which_is_weaker_than_catching_a_swap() {
    let (straight, _) =
        derive_key_and_nonce(&KDF_SHARED_SECRET, &KDF_EPHEMERAL_PK, &KDF_RECIPIENT_PK).unwrap();
    let (swapped, _) =
        derive_key_and_nonce(&KDF_SHARED_SECRET, &KDF_RECIPIENT_PK, &KDF_EPHEMERAL_PK).unwrap();

    assert_ne!(
        hex(straight.as_slice()),
        hex(swapped.as_slice()),
        "the info input must be order-sensitive; if it is not, the ephemeral and recipient \
         keys are interchangeable and the NaCl crypto_box_seal binding is not present"
    );

    // POSITIVE CONTROL: the same inputs twice must agree, or `assert_ne` above
    // would pass for a function that simply returned fresh randomness.
    let (again, _) =
        derive_key_and_nonce(&KDF_SHARED_SECRET, &KDF_EPHEMERAL_PK, &KDF_RECIPIENT_PK).unwrap();
    assert_eq!(
        hex(straight.as_slice()),
        hex(again.as_slice()),
        "derivation must be deterministic"
    );
}

/// Every input must actually reach the derivation. A parameter that is
/// accepted and then ignored is indistinguishable from one that is used, until
/// something varies it.
#[test]
fn each_input_independently_changes_the_output() {
    let (base, base_nonce) =
        derive_key_and_nonce(&KDF_SHARED_SECRET, &KDF_EPHEMERAL_PK, &KDF_RECIPIENT_PK).unwrap();

    let mut varied = 0usize;

    let mut secret = KDF_SHARED_SECRET;
    secret[0] ^= 0x01;
    let (k, _) = derive_key_and_nonce(&secret, &KDF_EPHEMERAL_PK, &KDF_RECIPIENT_PK).unwrap();
    assert_ne!(
        hex(base.as_slice()),
        hex(k.as_slice()),
        "shared secret ignored"
    );
    varied += 1;

    let mut ephemeral = KDF_EPHEMERAL_PK;
    ephemeral[31] ^= 0x01;
    let (k, _) = derive_key_and_nonce(&KDF_SHARED_SECRET, &ephemeral, &KDF_RECIPIENT_PK).unwrap();
    assert_ne!(
        hex(base.as_slice()),
        hex(k.as_slice()),
        "ephemeral key ignored"
    );
    varied += 1;

    let mut recipient = KDF_RECIPIENT_PK;
    recipient[31] ^= 0x01;
    let (k, n) = derive_key_and_nonce(&KDF_SHARED_SECRET, &KDF_EPHEMERAL_PK, &recipient).unwrap();
    assert_ne!(
        hex(base.as_slice()),
        hex(k.as_slice()),
        "recipient key ignored"
    );
    assert_ne!(
        hex(&base_nonce),
        hex(&n),
        "the nonce must move with the info too"
    );
    varied += 1;

    assert_eq!(varied, 3, "every input must have been varied exactly once");
}
