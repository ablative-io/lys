#![cfg(test)]
//! The X25519 key derived from the identity: the pinned vector, sealing to
//! it, and agreement across identities.

use super::*;

// ─── X25519 derivation ────────────────────────────────────────────

/// Golden vector for the Ed25519→X25519 derivation.
///
/// Every other test in this section is self-consistent — deterministic,
/// matches its own secret, differs from another identity, agrees under DH —
/// so all of them would pass unchanged if the derivation itself moved. This
/// one pins it to a constant, and it is the guard that makes touching this
/// code safe: the X25519 public key is what senders seal to, so a change
/// here silently renders every previously sealed envelope undecryptable.
///
/// A failure means the derivation moved. Do not update the constant to make
/// it pass — that would be recording the break rather than catching it.
#[test]
fn x25519_public_key_matches_the_pinned_golden_vector() {
    let id = identity_from_seed([7u8; 32]);
    assert_eq!(
        crate::hex_lower(&id.x25519_public_key()),
        "761d88ec830413919dfe9d4d1d56f17e653c8c994082df5b137b90a0ae6edf74",
        "the Ed25519-to-X25519 derivation changed; every existing sealed \
         envelope addressed to this identity would become undecryptable"
    );
}

/// A sealed payload survives the round trip through the derived secret.
///
/// The golden vector above pins the public half; this pins that the private
/// half still opens what that public half seals. Together they cover both
/// directions of the derivation, which is what a zeroize refactor could
/// plausibly disturb without changing any signature.
#[test]
fn payload_sealed_to_the_derived_public_key_still_opens() {
    let id = identity_from_seed([7u8; 32]);
    let envelope = crate::seal::seal(b"round trip", &id.x25519_public_key()).unwrap();
    let opened = crate::seal::open(&envelope, &id.x25519_static_secret()).unwrap();
    assert_eq!(opened.as_slice(), b"round trip");
}

#[test]
fn x25519_public_key_matches_derived_static_secret() {
    let id = identity_from_seed([21u8; 32]);
    let secret = id.x25519_static_secret();
    let expected = x25519_dalek::PublicKey::from(&secret).to_bytes();
    assert_eq!(id.x25519_public_key(), expected);
}

#[test]
fn x25519_public_key_is_deterministic() {
    let id = identity_from_seed([22u8; 32]);
    assert_eq!(id.x25519_public_key(), id.x25519_public_key());
}

#[test]
fn different_identities_produce_different_x25519_public_keys() {
    let id_a = identity_from_seed([23u8; 32]);
    let id_b = identity_from_seed([24u8; 32]);
    assert_ne!(id_a.x25519_public_key(), id_b.x25519_public_key());
}

#[test]
fn x25519_static_secret_agrees_across_identities() {
    // Diffie–Hellman sanity: two identities derive the same shared secret
    // from each other's X25519 public keys.
    let id_a = identity_from_seed([25u8; 32]);
    let id_b = identity_from_seed([26u8; 32]);

    let a_secret = id_a.x25519_static_secret();
    let b_secret = id_b.x25519_static_secret();

    let b_public = x25519_dalek::PublicKey::from(id_b.x25519_public_key());
    let a_public = x25519_dalek::PublicKey::from(id_a.x25519_public_key());

    let shared_ab = a_secret.diffie_hellman(&b_public);
    let shared_ba = b_secret.diffie_hellman(&a_public);
    assert_eq!(shared_ab.as_bytes(), shared_ba.as_bytes());
}
