//! Adapters translating lys key types into rcgen's vocabulary.
//!
//! Three translations live here, shared by issuance ([`super::authority`]) and
//! request generation ([`super::request`]) so the two issuance paths cannot
//! drift apart in how they present keys to rcgen:
//!
//! - [`IdentitySigner`] exposes an [`Ed25519Identity`]'s public key and `sign`
//!   operation to rcgen through [`RemoteKeyPair`], so the private seed is
//!   never serialised into an rcgen `KeyPair`.
//! - [`PresentedKey`] carries a bare 32-byte Ed25519 public key whose private
//!   half this process has never seen, so a certificate can be signed over a
//!   key the *holder* controls. It is deliberately incapable of signing:
//!   nothing in this type can stand in for proof of possession, which is
//!   established once, in [`super::request::verify_certificate_request`].
//! - [`distinguished_name`] builds the single-common-name subject form used by
//!   every certificate and request this crate produces.

use std::sync::Arc;

use rcgen::{
    DistinguishedName, DnType, PKCS_ED25519, PublicKeyData, RemoteKeyPair, SignatureAlgorithm,
};

use crate::keys::Ed25519Identity;

/// rcgen [`RemoteKeyPair`] adapter over an [`Ed25519Identity`].
///
/// Exposes the identity's public key and `sign` operation to rcgen without
/// revealing the private seed. Held behind an [`Arc`] so it satisfies rcgen's
/// `'static` boxed-trait requirement while sharing one identity — cloning the
/// identity instead would put a second copy of the private seed in memory.
pub(crate) struct IdentitySigner {
    identity: Arc<Ed25519Identity>,
    public_key: Vec<u8>,
}

impl IdentitySigner {
    /// Wraps a shared identity as an rcgen remote keypair.
    pub(crate) fn new(identity: Arc<Ed25519Identity>) -> Self {
        let public_key = identity.public_key_bytes().to_vec();
        Self {
            identity,
            public_key,
        }
    }
}

impl RemoteKeyPair for IdentitySigner {
    fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    fn sign(&self, msg: &[u8]) -> Result<Vec<u8>, rcgen::Error> {
        Ok(self.identity.sign(msg).to_vec())
    }

    fn algorithm(&self) -> &'static SignatureAlgorithm {
        &PKCS_ED25519
    }
}

/// A 32-byte Ed25519 public key presented by its holder, for signing a
/// certificate over a key whose private half this process does not hold.
///
/// Carries no signing capability by construction. Possession of the matching
/// private key is proven separately, by the request's self-signature; this
/// type only transports the verified result to rcgen.
pub(crate) struct PresentedKey {
    public_key: [u8; 32],
}

impl PresentedKey {
    /// Wraps a verified subject public key for presentation to rcgen.
    pub(crate) fn new(public_key: [u8; 32]) -> Self {
        Self { public_key }
    }
}

impl PublicKeyData for PresentedKey {
    /// The raw 32-byte Ed25519 point, which is what rcgen wraps in the
    /// `subjectPublicKeyInfo` BIT STRING (RFC 8410 encodes Ed25519 keys with
    /// no additional structure).
    fn der_bytes(&self) -> &[u8] {
        &self.public_key
    }

    fn algorithm(&self) -> &SignatureAlgorithm {
        &PKCS_ED25519
    }
}

/// Builds a distinguished name carrying a single common-name component.
pub(crate) fn distinguished_name(common_name: &str) -> DistinguishedName {
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, common_name);
    dn
}
