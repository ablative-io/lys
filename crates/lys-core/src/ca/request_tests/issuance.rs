#![cfg(test)]
//! Issuance over a request: the holder's own key, the subject, forged
//! requests, the authority's extensions, ttl, and the shape shared with
//! generated issuance.

use super::*;

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
