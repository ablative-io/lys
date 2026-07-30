#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn capability_claims_oid_extends_the_lys_arc_by_one() {
    let oid = capability_claims_oid();
    assert_eq!(oid, vec![1, 3, 6, 1, 4, 1, 66364, 1]);
    assert_eq!(&oid[..LYS_OID_ARC.len()], LYS_OID_ARC);
}

/// The requested window must reach the certificate. Parsing tests prove the
/// spec is read correctly; this proves it is *used* — the failure it guards
/// against is a `--validity 30m` that quietly issues for a day, which would look
/// entirely successful and hand out a grant 48 times longer than asked for.
///
/// Checked by behaviour rather than by reading DER fields: the certificate must
/// verify inside the window and be refused outside it, which is what any relying
/// party will actually do with it.
#[test]
fn a_sub_day_validity_window_reaches_the_certificate() {
    let dir = tempfile::tempdir().unwrap();
    let key_path = dir.path().join("ca.key");
    let cert_path = dir.path().join("agent.pem");
    crate::commands::key::generate(&key_path, true).unwrap();

    let issued_at = Utc::now();
    issue(
        &key_path,
        "agent-short-lived",
        None,
        Duration::from_secs(30 * 60),
        &cert_path,
        None,
        true,
    )
    .unwrap();

    let pem_bytes = std::fs::read(&cert_path).unwrap();
    let der = pem::decode_certificate(&pem_bytes, &cert_path).unwrap();
    let identity = load_identity(&key_path).unwrap();
    let issuer = identity.public_key_bytes();

    // Inside the window.
    verify_certificate_chain_at(&der, &issuer, issued_at + chrono::Duration::minutes(29)).unwrap();
    // Past it — and well short of the day a `--validity-days 1` floor would have
    // produced, which is the regression this pins.
    assert!(
        verify_certificate_chain_at(&der, &issuer, issued_at + chrono::Duration::minutes(31))
            .is_err()
    );
    assert!(
        verify_certificate_chain_at(&der, &issuer, issued_at + chrono::Duration::hours(23))
            .is_err()
    );
}

#[test]
fn terminal_safety_rejects_escape_and_carriage_control() {
    assert!(is_terminal_safe("{\"role\": \"reviewer\"}"));
    assert!(is_terminal_safe("multi\nline\tclaims"));
    assert!(!is_terminal_safe("claims \u{1b}[2K\u{1b}[1A spoofed"));
    assert!(!is_terminal_safe("overwrite\rme"));
    assert!(!is_terminal_safe("bell\u{07}"));
}
