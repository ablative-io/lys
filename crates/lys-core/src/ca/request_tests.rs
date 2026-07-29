#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::Duration;

use rcgen::{CertificateParams, DnType, KeyPair, SanType};
use x509_parser::prelude::{FromDer, X509Certificate};

use super::*;
use crate::ca::rcgen_bridge::{IdentitySigner, distinguished_name};
use crate::ca::{CertificateAuthority, verify_certificate_chain};
use crate::error::TrustError;
use crate::keys::Ed25519Identity;

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

#[test]
fn request_round_trips_and_proves_possession_of_the_identity_key() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    let verified = verify_certificate_request(&der).unwrap();
    assert_eq!(
        verified.subject_public_key(),
        &identity.public_key_bytes(),
        "the verified request must carry the identity's own key"
    );
    assert_eq!(verified.common_name(), "agent-noor");
}

/// The G1 property, and the gap this module exists to close: a certificate
/// issued over a request binds the key the holder actually controls, so the
/// certificate and anything that key later signs are provably about the same
/// identity. Before this path existed, the certificate named a key the
/// authority had generated and the holder's own signatures were made under a
/// different, unrelated key — both halves verified and nothing joined them.
#[test]
fn issuance_over_a_request_binds_the_holders_own_key() {
    let identity = test_identity();
    let ca = test_authority();

    let request = create_certificate_request(&identity, "agent-noor").unwrap();
    let certified = ca
        .issue_certificate_for_request(&request, "agent-noor", Duration::from_secs(3600), vec![])
        .unwrap();

    // The certificate chains to the authority.
    verify_certificate_chain(&certified.der_bytes, &ca.public_key_bytes()).unwrap();

    // And the key it certifies is the holder's, read back out of the DER
    // rather than taken from the struct that claims it.
    let (_, parsed) = X509Certificate::from_der(&certified.der_bytes).unwrap();
    let spki = parsed
        .tbs_certificate
        .subject_pki
        .subject_public_key
        .data
        .as_ref();
    assert_eq!(
        spki,
        identity.public_key_bytes().as_slice(),
        "the certificate must bind the holder's key, not one the authority minted"
    );
    assert_eq!(certified.subject_public_key, identity.public_key_bytes());
    assert_eq!(certified.issuer_public_key, ca.public_key_bytes());

    // The join that was previously unprovable: a signature the holder makes
    // verifies under the very key the certificate vouches for.
    let message = b"session checkpoint";
    let signature = identity.sign(message);
    Ed25519Identity::verify(&certified.subject_public_key, message, &signature).unwrap();
}

#[test]
fn substituting_the_subject_key_breaks_proof_of_possession() {
    let identity = test_identity();
    let attacker = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    // The classic attack: take somebody's request and swap in your own key,
    // hoping the authority certifies a key you control under their name. The
    // key sits inside the signed CertificationRequestInfo, so the
    // self-signature no longer matches.
    let forged = patch_first(
        &der,
        &identity.public_key_bytes(),
        &attacker.public_key_bytes(),
    );

    let error = verify_certificate_request(&forged).unwrap_err();
    match error {
        TrustError::CertificateVerification { reason } => {
            assert!(
                reason.contains("proof of possession failed"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a verification failure, got {other:?}"),
    }
}

#[test]
fn tampering_with_the_requested_name_breaks_proof_of_possession() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    // Renaming the request is the other half of the same attack: get the
    // authority to certify a key under a name its holder never asked for.
    let forged = patch_first(&der, b"agent-noor", b"agent-root");

    let error = verify_certificate_request(&forged).unwrap_err();
    assert!(matches!(error, TrustError::CertificateVerification { .. }));
}

#[test]
fn a_small_order_subject_key_is_rejected() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    // The all-zero encoding is the Ed25519 identity element: a point of order
    // one. Under non-strict verification, small-order and torsion keys admit
    // signatures that validate without anyone knowing a private key, which
    // would turn proof of possession into a formality anybody could satisfy
    // for a key they do not control. `verify_strict` refuses such keys, and
    // this asserts the refusal rather than assuming the dependency's default.
    let forged = patch_first(&der, &identity.public_key_bytes(), &[0u8; 32]);

    let error = verify_certificate_request(&forged).unwrap_err();
    match error {
        TrustError::CertificateVerification { reason } => {
            assert!(
                reason.contains("proof of possession failed")
                    || reason.contains("not a valid Ed25519 point"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a verification failure, got {other:?}"),
    }
}

#[test]
fn trailing_bytes_after_the_request_are_rejected() {
    let identity = test_identity();
    let mut der = create_certificate_request(&identity, "agent-noor").unwrap();
    der.push(0x00);

    let error = verify_certificate_request(&der).unwrap_err();
    match error {
        TrustError::CertificateParsing { reason } => {
            assert!(reason.contains("trailing"), "unexpected reason: {reason}");
        }
        other => panic!("expected a parsing failure, got {other:?}"),
    }
}

#[test]
fn a_non_ed25519_signature_algorithm_is_rejected() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    // The signature AlgorithmIdentifier is the last of the two Ed25519 OIDs
    // in the encoding; the first belongs to the subject key.
    let forged = patch_last(&der, &ED25519_OID_DER, &ED448_OID_DER);

    let error = verify_certificate_request(&forged).unwrap_err();
    match error {
        TrustError::CertificateParsing { reason } => {
            assert!(
                reason.contains("signature algorithm is not Ed25519"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a parsing failure, got {other:?}"),
    }
}

#[test]
fn a_non_ed25519_subject_key_algorithm_is_rejected() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    let forged = patch_first(&der, &ED25519_OID_DER, &ED448_OID_DER);

    let error = verify_certificate_request(&forged).unwrap_err();
    match error {
        TrustError::CertificateParsing { reason } => {
            assert!(
                reason.contains("subject key algorithm is not Ed25519"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a parsing failure, got {other:?}"),
    }
}

#[test]
fn requested_extensions_are_refused_rather_than_stripped() {
    let identity = test_identity();
    let mut params = CertificateParams::new(vec!["agent-noor.example".to_string()]).unwrap();
    params.distinguished_name = distinguished_name("agent-noor");
    params
        .subject_alt_names
        .push(SanType::DnsName("other.example".try_into().unwrap()));
    let der = request_with_params(&identity, &params);

    // The request is perfectly well-formed and its signature is valid — this
    // is a policy refusal, not a cryptographic failure. A holder whose asks
    // were silently dropped would believe they had been honoured.
    let error = verify_certificate_request(&der).unwrap_err();
    match error {
        TrustError::CertificateParsing { reason } => {
            assert!(
                reason.contains("requested extensions"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a parsing failure, got {other:?}"),
    }
}

/// Guards the shape `create_certificate_request` emits: if rcgen ever started
/// attaching an extension request to a minimal request, our own requests would
/// be refused by the check above. This asserts the two stay compatible.
#[test]
fn our_own_requests_carry_no_requested_extensions() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();
    verify_certificate_request(&der).expect("our own request must be acceptable");
}

/// Neither of the two malformed-name shapes below is reachable through rcgen:
/// it substitutes a default common name for an empty distinguished name, and
/// `DistinguishedName::push` replaces rather than appends for a repeated
/// attribute type. Both are built instead by patching an attribute OID to
/// another of identical length, which leaves every DER length prefix intact.
/// The name checks run before signature verification, so these reach the
/// intended rejection rather than failing as broken signatures — asserting the
/// specific reason is what proves that.
#[test]
fn more_than_one_common_name_is_rejected() {
    let identity = test_identity();
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(DnType::CommonName, "agent-noor");
    dn.push(DnType::CountryName, "AU");
    params.distinguished_name = dn;
    let der = request_with_params(&identity, &params);

    // Turn the country attribute into a second common name.
    let forged = patch_first(&der, &COUNTRY_OID_DER, &CN_OID_DER);

    // Two names is an ambiguity with no legitimate use: the authority would
    // compare one against the subject it was asked to certify while a reader
    // might display the other.
    let error = verify_certificate_request(&forged).unwrap_err();
    match error {
        TrustError::CertificateParsing { reason } => {
            assert!(
                reason.contains("more than one common name"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a parsing failure, got {other:?}"),
    }
}

#[test]
fn a_request_with_no_common_name_is_rejected() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    // Turn the only common name into a country attribute, leaving none.
    let forged = patch_first(&der, &CN_OID_DER, &COUNTRY_OID_DER);

    let error = verify_certificate_request(&forged).unwrap_err();
    match error {
        TrustError::CertificateParsing { reason } => {
            assert!(
                reason.contains("no common name"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a parsing failure, got {other:?}"),
    }
}

#[test]
fn a_whitespace_only_common_name_is_rejected() {
    let identity = test_identity();
    let der = create_certificate_request(&identity, "agent-noor").unwrap();

    // A name that is present but blank would otherwise be compared against the
    // authority's chosen subject and could never match anything meaningful.
    let forged = patch_first(&der, b"agent-noor", b"          ");

    let error = verify_certificate_request(&forged).unwrap_err();
    match error {
        TrustError::CertificateParsing { reason } => {
            assert!(
                reason.contains("must not be empty"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a parsing failure, got {other:?}"),
    }
}

#[test]
fn creating_a_request_for_an_empty_subject_is_refused() {
    let identity = test_identity();
    for subject in ["", "   "] {
        let error = create_certificate_request(&identity, subject).unwrap_err();
        assert!(matches!(error, TrustError::CertificateGeneration { .. }));
    }
}

#[test]
fn issuance_refuses_a_request_whose_name_disagrees_with_the_subject() {
    let identity = test_identity();
    let ca = test_authority();
    let request = create_certificate_request(&identity, "agent-noor").unwrap();

    // The authority chooses the name it vouches for, but it may not certify a
    // holder under a name that holder never asked for.
    let error = ca
        .issue_certificate_for_request(&request, "agent-root", Duration::from_secs(3600), vec![])
        .unwrap_err();
    match error {
        TrustError::CertificateGeneration { reason } => {
            assert!(
                reason.contains("common name") && reason.contains("agent-root"),
                "unexpected reason: {reason}"
            );
        }
        other => panic!("expected a generation failure, got {other:?}"),
    }
}

#[test]
fn issuance_over_a_request_rejects_a_forged_request_before_signing_anything() {
    let identity = test_identity();
    let attacker = test_identity();
    let ca = test_authority();
    let request = create_certificate_request(&identity, "agent-noor").unwrap();
    let forged = patch_first(
        &request,
        &identity.public_key_bytes(),
        &attacker.public_key_bytes(),
    );

    let error = ca
        .issue_certificate_for_request(&forged, "agent-noor", Duration::from_secs(3600), vec![])
        .unwrap_err();
    assert!(matches!(error, TrustError::CertificateVerification { .. }));
}

#[test]
fn issuance_over_a_request_carries_the_authoritys_extensions_only() {
    let identity = test_identity();
    let ca = test_authority();
    let request = create_certificate_request(&identity, "agent-noor").unwrap();

    let oid = [1, 3, 6, 1, 4, 1, 66364, 1];
    let claims = br#"{"capability":"read"}"#.to_vec();
    let certified = ca
        .issue_certificate_for_request(
            &request,
            "agent-noor",
            Duration::from_secs(3600),
            vec![crate::ca::encode_extension(&oid, claims.clone())],
        )
        .unwrap();

    let decoded = crate::ca::decode_extension(&certified.der_bytes, &oid).unwrap();
    assert_eq!(
        decoded.as_deref(),
        Some(claims.as_slice()),
        "the authority's own extension must be carried verbatim"
    );
}

#[test]
fn issuance_over_a_request_still_validates_ttl_and_subject() {
    let identity = test_identity();
    let ca = test_authority();
    let request = create_certificate_request(&identity, "agent-noor").unwrap();

    let zero_ttl = ca
        .issue_certificate_for_request(&request, "agent-noor", Duration::ZERO, vec![])
        .unwrap_err();
    assert!(matches!(zero_ttl, TrustError::CertificateGeneration { .. }));

    let empty_subject = ca
        .issue_certificate_for_request(&request, "  ", Duration::from_secs(3600), vec![])
        .unwrap_err();
    assert!(matches!(
        empty_subject,
        TrustError::CertificateGeneration { .. }
    ));
}

#[test]
fn a_certified_key_debug_exposes_no_private_material() {
    let identity = test_identity();
    let ca = test_authority();
    let request = create_certificate_request(&identity, "agent-noor").unwrap();
    let certified = ca
        .issue_certificate_for_request(&request, "agent-noor", Duration::from_secs(3600), vec![])
        .unwrap();

    // There is no private material to leak — that is the point of the type —
    // so this pins the absence rather than a redaction.
    let rendered = format!("{certified:?}");
    assert!(rendered.contains("subject_public_key"));
    assert!(!rendered.to_lowercase().contains("signing"));
    assert!(!rendered.to_lowercase().contains("private"));
}

#[test]
fn a_request_is_not_accepted_as_a_certificate() {
    let identity = test_identity();
    let ca = test_authority();
    let request = create_certificate_request(&identity, "agent-noor").unwrap();

    // Cross-protocol confusion: a PKCS#10 request and an X.509 certificate are
    // both self-describing DER sequences signed by a key. Feeding one to the
    // other's verifier must fail rather than partially succeed.
    let error = verify_certificate_chain(&request, &identity.public_key_bytes()).unwrap_err();
    assert!(matches!(
        error,
        TrustError::CertificateParsing { .. } | TrustError::CertificateVerification { .. }
    ));

    let certified = ca
        .issue_certificate_for_request(&request, "agent-noor", Duration::from_secs(3600), vec![])
        .unwrap();
    let error = verify_certificate_request(&certified.der_bytes).unwrap_err();
    assert!(matches!(
        error,
        TrustError::CertificateParsing { .. } | TrustError::CertificateVerification { .. }
    ));
}

#[test]
fn generated_and_presented_issuance_produce_the_same_certificate_shape() {
    let identity = test_identity();
    let ca = test_authority();
    let request = create_certificate_request(&identity, "agent-noor").unwrap();

    let generated = ca
        .issue_certificate("agent-noor", Duration::from_secs(3600), vec![])
        .unwrap();
    let presented = ca
        .issue_certificate_for_request(&request, "agent-noor", Duration::from_secs(3600), vec![])
        .unwrap();

    let (_, generated_cert) = X509Certificate::from_der(&generated.der_bytes).unwrap();
    let (_, presented_cert) = X509Certificate::from_der(&presented.der_bytes).unwrap();

    // Both paths share `leaf_params`, so everything except the subject key and
    // the instants must match. A divergence here would mean the two issuance
    // paths had drifted into producing differently shaped certificates.
    assert_eq!(
        generated_cert.subject().as_raw(),
        presented_cert.subject().as_raw()
    );
    assert_eq!(
        generated_cert.issuer().as_raw(),
        presented_cert.issuer().as_raw()
    );
    assert_eq!(
        generated_cert.basic_constraints().unwrap().is_some(),
        presented_cert.basic_constraints().unwrap().is_some()
    );
    assert_eq!(
        generated_cert.signature_algorithm.algorithm,
        presented_cert.signature_algorithm.algorithm
    );
}
