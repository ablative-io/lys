#![cfg(test)]

use std::sync::Arc;
use std::time::Duration;

use rcgen::{
    CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ECDSA_P256_SHA256, PKCS_ED25519,
};
use x509_parser::prelude::{FromDer, X509Certificate};

use super::*;
use crate::error::{TrustError, TrustResult};
use crate::keys::Ed25519Identity;

/// The reason `verify_certificate_chain_at` gives a certificate self-signed by
/// keys.
const SELF_SIGNED_REASON: &str = "self-signed certificate rejected (subject key is the issuer key)";

/// The reason `verify_certificate_chain_at` gives a signature that does not
/// verify under the supplied issuer key.
const SIGNATURE_REASON: &str = "certificate signature did not verify against the issuer public key";

fn test_identity() -> Ed25519Identity {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ca.key");
    Ed25519Identity::load_or_generate(&path).unwrap()
}

fn test_authority() -> CertificateAuthority {
    CertificateAuthority::new(test_identity())
}

/// The 32-byte Ed25519 public key of an rcgen keypair.
fn raw_public_key(key: &KeyPair) -> [u8; 32] {
    key.public_key_raw()
        .try_into()
        .expect("an Ed25519 public key is 32 bytes")
}

/// Certificate parameters carrying only the common name `common_name`.
fn named_params(common_name: &str) -> CertificateParams {
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    let mut name = DistinguishedName::new();
    name.push(DnType::CommonName, common_name);
    params.distinguished_name = name;
    params
}

/// Asserts `result` is a verification refusal whose reason is exactly
/// `expected`.
fn assert_verification_reason(result: TrustResult<()>, expected: &str) {
    match result {
        Err(TrustError::CertificateVerification { reason }) => assert_eq!(reason, expected),
        other => panic!("expected a verification refusal {expected:?}, got {other:?}"),
    }
}

/// Whether the certificate's raw subject DN equals its raw issuer DN.
fn names_match(der: &[u8]) -> bool {
    let (_, parsed) = X509Certificate::from_der(der).unwrap();
    parsed.subject().as_raw() == parsed.issuer().as_raw()
}

#[test]
fn issue_certificate_returns_non_empty_parseable_der() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-001", Duration::from_hours(1), vec![])
        .unwrap();

    assert!(!issued.der_bytes.is_empty());
    let (_, parsed) = X509Certificate::from_der(&issued.der_bytes)
        .expect("issued certificate must parse as valid DER");
    assert_eq!(issued.issuer_public_key, ca.public_key_bytes());
    // Leaf subject differs from the CA-derived issuer.
    assert_ne!(parsed.subject().as_raw(), parsed.issuer().as_raw());
}

/// The generated-key path's counterpart to the presented-key binding check in
/// [`CertifiedKey::from_der_and_public_key`](crate::ca::CertifiedKey::from_der_and_public_key):
/// the certificate must actually carry the keypair returned alongside it. The
/// struct's own fields cannot witness this — they would agree with each other
/// even if the DER bound something else entirely — so it is read out of the
/// signed bytes.
#[test]
fn issued_certificate_binds_the_generated_subject_key() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-001", Duration::from_hours(1), vec![])
        .unwrap();

    let (_, parsed) = X509Certificate::from_der(&issued.der_bytes).unwrap();
    assert_eq!(
        parsed
            .tbs_certificate
            .subject_pki
            .subject_public_key
            .data
            .as_ref(),
        issued.subject_verifying_key.to_bytes().as_slice(),
        "the certificate must bind the subject key it was issued with"
    );
}

#[test]
fn certificate_subject_public_key_reads_back_what_was_certified() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-001", Duration::from_hours(1), vec![])
        .unwrap();

    let recovered = crate::ca::certificate_subject_public_key(&issued.der_bytes).unwrap();
    assert_eq!(recovered, issued.subject_verifying_key.to_bytes());
}

/// The recovery path must refuse anything that is not a certificate rather than
/// returning bytes from a partially-understood structure — the value it returns
/// is compared against an attestation's signer key, so a wrong answer here
/// would silently break the one check that joins an identity to its statements.
#[test]
fn certificate_subject_public_key_refuses_non_certificates() {
    for bytes in [
        b"".as_slice(),
        b"not der at all".as_slice(),
        &[0x30, 0x03, 0x02, 0x01, 0x00],
    ] {
        let error = crate::ca::certificate_subject_public_key(bytes).unwrap_err();
        assert!(
            matches!(error, TrustError::CertificateParsing { .. }),
            "expected a parsing failure for {bytes:?}, got {error:?}"
        );
    }
}

#[test]
fn issuer_common_name_matches_ca_public_key_hex() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-002", Duration::from_hours(1), vec![])
        .unwrap();

    let (_, parsed) = X509Certificate::from_der(&issued.der_bytes).unwrap();
    let issuer_cn = parsed
        .issuer()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .expect("issuer must carry a common name");

    let expected_hex = crate::hex_lower(&ca.public_key_bytes());
    assert_eq!(issuer_cn, expected_hex);
}

#[test]
fn empty_subject_is_rejected() {
    let ca = test_authority();
    let result = ca.issue_certificate("   ", Duration::from_hours(1), vec![]);
    assert!(matches!(
        result,
        Err(TrustError::CertificateGeneration { .. })
    ));
}

#[test]
fn zero_ttl_is_rejected() {
    let ca = test_authority();
    let result = ca.issue_certificate("agent-003", Duration::ZERO, vec![]);
    assert!(matches!(
        result,
        Err(TrustError::CertificateGeneration { .. })
    ));
}

#[test]
fn legitimately_issued_certificate_verifies() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-004", Duration::from_hours(1), vec![])
        .unwrap();

    // Free function with explicit issuer key.
    verify_certificate_chain(&issued.der_bytes, &ca.public_key_bytes()).unwrap();
    // Convenience method.
    ca.verify_certificate_chain(&issued.der_bytes).unwrap();
}

#[test]
fn certificate_from_other_ca_fails_verification() {
    let ca_a = test_authority();
    let ca_b = test_authority();
    let issued = ca_a
        .issue_certificate("agent-005", Duration::from_hours(1), vec![])
        .unwrap();

    let result = verify_certificate_chain(&issued.der_bytes, &ca_b.public_key_bytes());
    assert!(matches!(
        result,
        Err(TrustError::CertificateVerification { .. })
    ));
    // And the wrong CA's convenience method rejects it too.
    assert!(ca_b.verify_certificate_chain(&issued.der_bytes).is_err());
}

/// A certificate signed by a key over that same key is self-signed and is
/// refused as such under that key. Under any other key the signature fails
/// first, and the refusal names the signature rather than the self-signed
/// rule: the self-signed judgement is made only after the signature verifies.
#[test]
fn self_signed_certificate_is_rejected() {
    let key = KeyPair::generate_for(&PKCS_ED25519).unwrap();
    let params = CertificateParams::new(Vec::<String>::new()).unwrap();
    let cert = params.self_signed(&key).unwrap();
    let der = cert.der().to_vec();

    let result = verify_certificate_chain(&der, &raw_public_key(&key));
    assert_verification_reason(result, SELF_SIGNED_REASON);

    let other = KeyPair::generate_for(&PKCS_ED25519).unwrap();
    let result = verify_certificate_chain(&der, &raw_public_key(&other));
    assert_verification_reason(result, SIGNATURE_REASON);
}

#[test]
fn malformed_der_fails_to_parse() {
    let result = verify_certificate_chain(b"not a certificate", &[0u8; 32]);
    assert!(matches!(result, Err(TrustError::CertificateParsing { .. })));
}

// ─── validity window (verify_certificate_chain_at) ────────────────

#[test]
fn expired_certificate_is_rejected_at_future_instant() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-006", Duration::from_mins(1), vec![])
        .unwrap();

    let future = chrono::Utc::now() + chrono::Duration::hours(2);
    let err =
        verify_certificate_chain_at(&issued.der_bytes, &ca.public_key_bytes(), future).unwrap_err();
    assert!(matches!(err, TrustError::CertificateVerification { .. }));
    let msg = err.to_string();
    assert!(msg.contains("expired"), "got: {msg}");
    assert!(msg.contains("notAfter"), "got: {msg}");
    assert!(!msg.contains("not yet valid"), "got: {msg}");
}

#[test]
fn not_yet_valid_certificate_is_rejected_at_past_instant() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-007", Duration::from_hours(1), vec![])
        .unwrap();

    let past = chrono::Utc::now() - chrono::Duration::hours(2);
    let err =
        verify_certificate_chain_at(&issued.der_bytes, &ca.public_key_bytes(), past).unwrap_err();
    assert!(matches!(err, TrustError::CertificateVerification { .. }));
    let msg = err.to_string();
    assert!(msg.contains("not yet valid"), "got: {msg}");
    assert!(msg.contains("notBefore"), "got: {msg}");
}

#[test]
fn certificate_verifies_at_explicit_instant_inside_window() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-008", Duration::from_hours(1), vec![])
        .unwrap();

    let mid_window = chrono::Utc::now() + chrono::Duration::minutes(30);
    verify_certificate_chain_at(&issued.der_bytes, &ca.public_key_bytes(), mid_window).unwrap();
}

#[test]
fn not_after_boundary_is_inclusive() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-009", Duration::from_hours(1), vec![])
        .unwrap();

    // expires_at is truncated to the exact DER notAfter instant, so
    // verification exactly at the boundary must still pass (X.509 validity
    // is inclusive), and one second past it must fail.
    verify_certificate_chain_at(&issued.der_bytes, &ca.public_key_bytes(), issued.expires_at)
        .unwrap();
    let err = verify_certificate_chain_at(
        &issued.der_bytes,
        &ca.public_key_bytes(),
        issued.expires_at + chrono::Duration::seconds(1),
    )
    .unwrap_err();
    assert!(err.to_string().contains("expired"), "got: {err}");
}

/// The notBefore half of the inclusive-boundary contract.
///
/// The sibling test above covers only notAfter, and the coarse −2 h instant
/// used elsewhere would still pass if notBefore flipped to exclusive. This
/// pins the exact instant, read back from the DER rather than reconstructed,
/// because `issue_certificate` truncates to whole seconds and a
/// locally-computed "now" would be testing the truncation instead of the
/// boundary.
#[test]
fn not_before_boundary_is_inclusive() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-011", Duration::from_hours(1), vec![])
        .unwrap();

    let (_, parsed) = X509Certificate::from_der(&issued.der_bytes).unwrap();
    let not_before = chrono::DateTime::<chrono::Utc>::from_timestamp(
        parsed.validity().not_before.timestamp(),
        0,
    )
    .expect("notBefore is within the representable range");

    // Exactly at notBefore must PASS: X.509 validity is inclusive at both
    // ends, and `check_validity_window` rejects only on `at < not_before`.
    verify_certificate_chain_at(&issued.der_bytes, &ca.public_key_bytes(), not_before).unwrap();

    // One second earlier must fail, and must name the boundary it violated.
    // Asserting the reason — not merely that it errored — is what catches a
    // swapped comparison, which would still reject here but call it expired.
    let err = verify_certificate_chain_at(
        &issued.der_bytes,
        &ca.public_key_bytes(),
        not_before - chrono::Duration::seconds(1),
    )
    .unwrap_err();
    assert!(err.to_string().contains("not yet valid"), "got: {err}");
}

// ─── expires_at / DER notAfter agreement ──────────────────────────

#[test]
fn expires_at_is_whole_seconds_and_matches_der_not_after() {
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-010", Duration::from_hours(1), vec![])
        .unwrap();

    assert_eq!(
        issued.expires_at.timestamp_subsec_nanos(),
        0,
        "expires_at must be truncated to whole-second precision"
    );

    let (_, parsed) = X509Certificate::from_der(&issued.der_bytes).unwrap();
    assert_eq!(
        issued.expires_at.timestamp(),
        parsed.validity().not_after.timestamp(),
        "expires_at must be the same instant encoded in the DER notAfter"
    );
}

// ─── strict verification (weak issuer keys) ───────────────────────

#[test]
fn small_order_issuer_key_forgery_is_rejected() {
    use ed25519_dalek::Verifier;

    // Take a legitimately issued certificate and replace its trailing
    // 64-byte signature with the small-order forgery (R = basepoint, s = 1).
    // Against the identity-point issuer key ([1, 0, ..., 0], order 1) the
    // non-strict equation s·B = R + k·A reduces to B = R, so the forged
    // signature passes NON-strict verification for any tbs bytes. Strict
    // verification rejects the weak key outright.
    let ca = test_authority();
    let issued = ca
        .issue_certificate("agent-011", Duration::from_hours(1), vec![])
        .unwrap();

    let mut der = issued.der_bytes;
    let sig_offset = der.len() - 64;
    let basepoint: [u8; 32] = [
        0x58, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66,
        0x66, 0x66,
    ];
    der[sig_offset..sig_offset + 32].copy_from_slice(&basepoint);
    der[sig_offset + 32..].copy_from_slice(&{
        let mut s = [0u8; 32];
        s[0] = 1; // s = 1, little-endian
        s
    });

    let mut weak_issuer = [0u8; 32];
    weak_issuer[0] = 1; // Edwards identity point encoding

    // Sanity: dalek's non-strict verify accepts the forgery over the tbs —
    // this is exactly the hole verify_strict closes.
    let (_, parsed) = X509Certificate::from_der(&der).unwrap();
    let vk = ed25519_dalek::VerifyingKey::from_bytes(&weak_issuer).unwrap();
    let forged_sig = ed25519_dalek::Signature::from_bytes(
        der[sig_offset..].try_into().expect("64-byte signature"),
    );
    vk.verify(parsed.tbs_certificate.as_ref(), &forged_sig)
        .expect("non-strict verify accepts the small-order forgery");

    // Chain verification with strict checking must reject it.
    let result = verify_certificate_chain(&der, &weak_issuer);
    assert!(matches!(
        result,
        Err(TrustError::CertificateVerification { .. })
    ));
}

// ─── self-signed is judged by keys, never by names ────────────────

/// A certificate the authority legitimately issued to the subject named by its
/// own lowercase-hex key carries the same subject and issuer names. It is not
/// self-signed — its subject key is a fresh one — so it verifies under the
/// authority and is refused on its signature under anyone else.
#[test]
fn hex_common_name_certificate_is_judged_on_its_real_issuer() {
    let ca_a = test_authority();
    let ca_b = test_authority();
    let subject = crate::hex_lower(&ca_a.public_key_bytes());
    let issued = ca_a
        .issue_certificate(&subject, Duration::from_hours(1), vec![])
        .unwrap();

    assert!(
        names_match(&issued.der_bytes),
        "the hex common name must make the subject and issuer DNs equal"
    );
    verify_certificate_chain(&issued.der_bytes, &ca_a.public_key_bytes()).unwrap();

    let result = verify_certificate_chain(&issued.der_bytes, &ca_b.public_key_bytes());
    assert_verification_reason(result, SIGNATURE_REASON);
}

/// The authority certifying its own key over a request made with it produces a
/// certificate whose names differ and whose subject key is the issuer key:
/// self-signed by keys, and refused.
#[test]
fn ca_own_key_certificate_is_refused_as_self_signed() {
    let ca_a = test_authority();
    let request = crate::ca::create_certificate_request(&ca_a.identity, "agent-self").unwrap();
    let certified = ca_a
        .issue_certificate_for_request(&request, "agent-self", Duration::from_hours(1), vec![])
        .unwrap();

    assert!(
        !names_match(&certified.der_bytes),
        "the subject and issuer DNs must differ"
    );
    let result = verify_certificate_chain(&certified.der_bytes, &ca_a.public_key_bytes());
    assert_verification_reason(result, SELF_SIGNED_REASON);
}

/// A certificate any other tool signs with key K over subject key K is
/// self-signed whatever names it carries.
#[test]
fn key_self_signed_certificate_with_differing_names_is_refused() {
    let key = KeyPair::generate_for(&PKCS_ED25519).unwrap();
    let issuer = named_params("issuer-x").self_signed(&key).unwrap();
    let leaf = named_params("leaf-y")
        .signed_by(&key, &issuer, &key)
        .unwrap();
    let der = leaf.der().to_vec();

    assert!(!names_match(&der), "the subject and issuer DNs must differ");
    let result = verify_certificate_chain(&der, &raw_public_key(&key));
    assert_verification_reason(result, SELF_SIGNED_REASON);
}

/// Supplying a certificate's own subject key as the issuer key does not make
/// it self-signed: the authority signed it, so the signature fails under the
/// subject key and the refusal says so.
#[test]
fn subject_key_as_supplied_key_without_its_signature_fails_the_signature() {
    let ca_a = test_authority();
    let holder = Arc::new(test_identity());
    let request = crate::ca::create_certificate_request(&holder, "agent-holder").unwrap();
    let certified = ca_a
        .issue_certificate_for_request(&request, "agent-holder", Duration::from_hours(1), vec![])
        .unwrap();

    let result = verify_certificate_chain(&certified.der_bytes, &holder.public_key_bytes());
    assert_verification_reason(result, SIGNATURE_REASON);
}

/// The same-key judgement compares decoded points, not bytes: a non-canonical
/// encoding of a point is that point, distinct points differ, and bytes that
/// decode to no point are the same key as nothing, themselves included.
#[test]
fn self_signed_judgement_compares_decoded_points() {
    let mut canonical_three = [0u8; 32];
    canonical_three[0] = 0x03;
    // y = p + 3, little-endian: 0xf0, thirty 0xff, then 0x7f.
    let mut non_canonical_three = [0xffu8; 32];
    non_canonical_three[0] = 0xf0;
    non_canonical_three[31] = 0x7f;
    let mut four = [0u8; 32];
    four[0] = 0x04;
    // No point on the curve has y = 2.
    let mut two = [0u8; 32];
    two[0] = 0x02;

    let judgements = [
        is_same_ed25519_key(&canonical_three, &non_canonical_three),
        is_same_ed25519_key(&canonical_three, &four),
        is_same_ed25519_key(&two, &two),
    ];
    assert_eq!(judgements, [true, false, false]);
    assert_eq!(judgements.iter().filter(|same| **same).count(), 1);
    assert_eq!(judgements.iter().filter(|same| !**same).count(), 2);
}

/// A subject key that is not a readable Ed25519 key is not the issuer's key,
/// so a certificate carrying one under a valid signature is not refused as
/// self-signed; it verifies, as it did before the rule was keyed on keys.
#[test]
fn unreadable_subject_key_is_not_judged_self_signed() {
    let key = KeyPair::generate_for(&PKCS_ED25519).unwrap();
    let issuer = named_params("issuer-x").self_signed(&key).unwrap();
    let p256 = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
    let leaf = named_params("leaf-y")
        .signed_by(&p256, &issuer, &key)
        .unwrap();
    let der = leaf.der().to_vec();

    assert!(matches!(
        crate::ca::certificate_subject_public_key(&der),
        Err(TrustError::CertificateParsing { .. })
    ));
    verify_certificate_chain(&der, &raw_public_key(&key)).unwrap();
}
