#![cfg(test)]

use std::sync::Arc;
use std::time::Duration;

use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use x509_parser::prelude::{FromDer, X509Certificate};

use super::*;
use crate::ca::rcgen_bridge::{IdentitySigner, distinguished_name};
use crate::ca::{CertificateAuthority, verify_certificate_chain};
use crate::error::TrustError;
use crate::keys::Ed25519Identity;

#[path = "request_tests/issuance.rs"]
mod issuance;
#[path = "request_tests/requests.rs"]
mod requests;

/// DER encoding of the Ed25519 algorithm OID 1.3.101.112, as it appears in
/// both the subject-key and signature `AlgorithmIdentifier`s.
const ED25519_OID_DER: [u8; 5] = [0x06, 0x03, 0x2b, 0x65, 0x70];
/// DER encoding of the Ed448 OID 1.3.101.113 — the same length as Ed25519's,
/// so it can be substituted without disturbing any DER length prefix.
const ED448_OID_DER: [u8; 5] = [0x06, 0x03, 0x2b, 0x65, 0x71];
/// DER encoding of the `commonName` attribute OID 2.5.4.3.
const CN_OID_DER: [u8; 5] = [0x06, 0x03, 0x55, 0x04, 0x03];
/// DER encoding of the `countryName` attribute OID 2.5.4.6 — same length as
/// `commonName`, so one can be patched into the other in place.
const COUNTRY_OID_DER: [u8; 5] = [0x06, 0x03, 0x55, 0x04, 0x06];

fn test_identity() -> Arc<Ed25519Identity> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subject.key");
    Arc::new(Ed25519Identity::load_or_generate(&path).unwrap())
}

fn test_authority() -> CertificateAuthority {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ca.key");
    CertificateAuthority::new(Ed25519Identity::load_or_generate(&path).unwrap())
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .rposition(|window| window == needle)
}

/// Replaces the first occurrence of `needle` with `replacement` (equal length,
/// so every DER length prefix stays correct and the structure still parses).
fn patch_first(bytes: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    assert_eq!(
        needle.len(),
        replacement.len(),
        "patch must preserve length"
    );
    let at = find_subslice(bytes, needle).expect("needle must be present");
    let mut patched = bytes.to_vec();
    patched[at..at + replacement.len()].copy_from_slice(replacement);
    patched
}

/// Replaces the last occurrence of `needle` with `replacement`.
fn patch_last(bytes: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    assert_eq!(
        needle.len(),
        replacement.len(),
        "patch must preserve length"
    );
    let at = rfind_subslice(bytes, needle).expect("needle must be present");
    let mut patched = bytes.to_vec();
    patched[at..at + replacement.len()].copy_from_slice(replacement);
    patched
}

/// Builds a request through rcgen directly, so tests can construct shapes
/// `create_certificate_request` deliberately never produces.
fn request_with_params(identity: &Arc<Ed25519Identity>, params: &CertificateParams) -> Vec<u8> {
    let signer = IdentitySigner::new(Arc::clone(identity));
    let key_pair = KeyPair::from_remote(Box::new(signer)).unwrap();
    params.serialize_request(&key_pair).unwrap().der().to_vec()
}
