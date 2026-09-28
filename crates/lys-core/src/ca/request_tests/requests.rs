#![cfg(test)]
//! Certificate requests: the round trip, proof of possession, and the
//! refusals on keys, algorithms, trailing bytes, extensions and common
//! names.

use super::*;

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
