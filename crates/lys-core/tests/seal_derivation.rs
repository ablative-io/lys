//! Second-party pin on the `lys/sealed-envelope/v1` key derivation (D6/C45):
//! an independent re-derivation of the HKDF output, written from the
//! specification rather than from the implementation.
//!
//! # Why this file hardcodes the tag instead of importing it
//!
//! The construction is documented in `docs/design/lys-core/DESIGN.md` (D6)
//! and `docs/design/lys-core/CHECKLIST.md` (C45) as
//!
//! ```text
//! HKDF-SHA256 info = b"lys-sealed-envelope/v1"
//!                    || ephemeral_public_key(32)
//!                    || recipient_public_key(32)
//! ```
//!
//! Before this file existed, nothing in the workspace disagreed with the
//! implementation about any of that. Swapping the two public keys in the
//! `info` at both call sites left the whole suite green; renaming the domain
//! tag from `v1` to `v2` left the whole suite green. Both drifts are
//! invisible to a seal/open round-trip, because a round-trip only asks
//! whether the two sides agree with *each other* — and they still did.
//!
//! So this file is deliberately written to be able to disagree:
//!
//! - The tag is the byte literal `b"lys-sealed-envelope/v1"`, typed out here.
//!   It is **not** read from the crate's `SEAL_INFO` constant. A test that
//!   imported that constant would move wherever the constant moved and would
//!   be blind to exactly the drift it exists to catch — it would rebuild the
//!   defect rather than fix it. Note that this is the hyphen form; the
//!   slash-form `lys/sealed-envelope/v1` is the attestation context tag of the
//!   authenticated composition and is a different string in a different
//!   domain (DESIGN.md D6).
//! - The lengths 44 / 32 / 12 and the offset split `okm[0..32]` / `okm[32..44]`
//!   are written as numbers, not imported.
//! - The `info` length is asserted to be exactly 86 bytes (22 + 32 + 32), so a
//!   change to the tag's length is caught by name rather than only as a
//!   downstream key mismatch.
//!
//! # How it can check any of this without the ephemeral secret
//!
//! It never needs the ephemeral secret. Diffie-Hellman is recoverable from
//! the recipient's side: the test holds the recipient's X25519 static secret
//! and reads the envelope's public `ephemeral_public_key`, which yields the
//! identical shared secret.
//!
//! And `SealedEnvelope::nonce` is 12 bytes of *published HKDF output*.
//! Asserting that the independently derived `okm[32..44]` equals
//! `envelope.nonce` therefore pins, in one comparison: the domain tag, the
//! order of the two public keys in the `info`, the absent (`None`) salt, the
//! 44-byte output length, and the 32/12 split offsets. Any of those changing
//! in `src/` changes the derived bytes and fails this assertion.
//!
//! # The positive control
//!
//! A check made only of comparisons that *could* fail cannot tell a working
//! re-derivation from a broken one that happens to mirror a broken
//! implementation. So every case also decrypts `envelope.ciphertext` with
//! `okm[0..32]` under `envelope.nonce` with an empty AAD and asserts the
//! plaintext is the original payload. That is the assertion which proves the
//! derivation here is genuinely correct rather than merely self-consistent.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use hkdf::Hkdf;
use lys_core::Ed25519Identity;
use lys_core::seal::{SealedEnvelope, seal};
use sha2::Sha256;
use x25519_dalek::{PublicKey, StaticSecret};

/// The HKDF domain-separation tag, typed out from the specification.
///
/// Deliberately a literal and deliberately not the crate's own constant —
/// see the module docs.
const DOCUMENTED_SEAL_INFO: &[u8] = b"lys-sealed-envelope/v1";

/// Fixed 32-byte Ed25519 seed, so the recipient identity is deterministic and
/// a failure here is reproducible rather than dependent on a generated key.
const RECIPIENT_SEED: &[u8; 32] = b"lys-seal-derivation-test-seed-01";

/// Builds the deterministic recipient identity from [`RECIPIENT_SEED`].
///
/// The seed is written to a temp file because `Ed25519Identity::from_seed` is
/// private — `load` is the public route to a known key, and is the same route
/// `signed_note_crosscheck.rs` uses for its golden identity. The `TempDir` is
/// returned so it outlives the identity's construction.
fn recipient_identity() -> (tempfile::TempDir, Ed25519Identity) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recipient.key");
    std::fs::write(&path, RECIPIENT_SEED).unwrap();
    let identity = Ed25519Identity::load(&path).unwrap();
    (dir, identity)
}

/// Re-derives the 44-byte HKDF output for `envelope` from the specification,
/// using only the recipient's secret and the envelope's public fields.
///
/// Every input is either read off the envelope or written out here as a
/// literal; nothing is imported from the seal module beyond the envelope type
/// itself.
fn rederive_okm(
    recipient_secret: &StaticSecret,
    recipient_public: &[u8; 32],
    envelope: &SealedEnvelope,
) -> [u8; 44] {
    let ephemeral_public = PublicKey::from(envelope.ephemeral_public_key);
    let shared = recipient_secret.diffie_hellman(&ephemeral_public);

    // info = tag || ephemeral_public_key || recipient_public_key
    let mut info = Vec::new();
    info.extend_from_slice(DOCUMENTED_SEAL_INFO);
    info.extend_from_slice(&envelope.ephemeral_public_key);
    info.extend_from_slice(recipient_public);
    assert_eq!(
        info.len(),
        86,
        "documented HKDF info is 22 + 32 + 32 = 86 bytes; got {} — the domain \
         tag or a key field has changed length",
        info.len()
    );

    // Salt is None; output length is 44 bytes.
    let mut okm = [0u8; 44];
    Hkdf::<Sha256>::new(None, shared.as_bytes())
        .expand(&info, &mut okm)
        .unwrap();
    okm
}

/// Asserts one envelope against the documented derivation.
///
/// Two assertions, in this order:
///
/// 1. the derived `okm[32..44]` equals the envelope's published nonce — the
///    load-bearing check, which fails if the tag, the key order, the salt,
///    the output length or the split offsets drift;
/// 2. the positive control — `okm[0..32]` actually decrypts the ciphertext
///    back to `payload`, so a re-derivation that agreed with a *wrong*
///    implementation would still be caught here.
fn assert_matches_documented_derivation(
    recipient_secret: &StaticSecret,
    recipient_public: &[u8; 32],
    envelope: &SealedEnvelope,
    payload: &[u8],
) {
    let okm = rederive_okm(recipient_secret, recipient_public, envelope);

    assert_eq!(
        &okm[32..44],
        &envelope.nonce[..],
        "HKDF okm[32..44] does not equal the envelope nonce: the as-built \
         derivation no longer matches the documented \
         b\"lys-sealed-envelope/v1\" || ephemeral || recipient info"
    );

    // Positive control: the derived key must genuinely open the ciphertext.
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&okm[0..32]));
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&envelope.nonce),
            Payload {
                msg: &envelope.ciphertext,
                aad: &[],
            },
        )
        .expect("independently derived AES-256-GCM key must decrypt the envelope");
    assert_eq!(
        plaintext, payload,
        "independently derived key decrypted to the wrong plaintext"
    );
}

/// The derivation as built matches the derivation as documented.
#[test]
fn seal_derives_key_material_exactly_as_documented() {
    let (_dir, identity) = recipient_identity();
    let recipient_secret = identity.x25519_static_secret();
    let recipient_public = identity.x25519_public_key();

    let payload = b"sealed-envelope key derivation, pinned by a second party";
    let envelope = seal(payload, &recipient_public).unwrap();

    assert_matches_documented_derivation(&recipient_secret, &recipient_public, &envelope, payload);
}

/// The same check over two envelopes with distinct ephemeral keys.
///
/// One passing envelope could in principle be a lucky value; the derivation
/// must hold for an ephemeral key it has never seen. The distinctness of the
/// two ephemeral keys and of the two nonces is asserted, so the test cannot
/// quietly degenerate into running the same case twice.
#[test]
fn derivation_holds_across_distinct_ephemeral_keys() {
    let (_dir, identity) = recipient_identity();
    let recipient_secret = identity.x25519_static_secret();
    let recipient_public = identity.x25519_public_key();

    let first_payload = b"first envelope to this recipient".as_slice();
    let second_payload = b"second envelope, different ephemeral key".as_slice();

    let first = seal(first_payload, &recipient_public).unwrap();
    let second = seal(second_payload, &recipient_public).unwrap();

    assert_ne!(
        first.ephemeral_public_key, second.ephemeral_public_key,
        "two seals must use fresh ephemeral keys, otherwise this test runs \
         the same case twice"
    );
    assert_ne!(
        first.nonce, second.nonce,
        "distinct ephemeral keys must yield distinct derived nonces"
    );

    assert_matches_documented_derivation(
        &recipient_secret,
        &recipient_public,
        &first,
        first_payload,
    );
    assert_matches_documented_derivation(
        &recipient_secret,
        &recipient_public,
        &second,
        second_payload,
    );
}
