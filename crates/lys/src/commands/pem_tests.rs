#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use super::*;

/// The boundary lines are pinned as literals rather than rebuilt from the
/// module's label constants: they are what a stranger's tooling matches on, so
/// the test should fail if the emitted text ever stops being exactly this,
/// however the code happens to construct it.
const BEGIN_CERTIFICATE: &str = "-----BEGIN CERTIFICATE-----";
const END_CERTIFICATE: &str = "-----END CERTIFICATE-----";
const BEGIN_REQUEST: &str = "-----BEGIN CERTIFICATE REQUEST-----";
const END_REQUEST: &str = "-----END CERTIFICATE REQUEST-----";

fn path() -> PathBuf {
    PathBuf::from("/certs/subject.pem")
}

#[test]
fn request_encode_then_decode_round_trips_der_bytes() {
    let der: Vec<u8> = (0u8..=255).cycle().take(300).collect();
    let pem = encode_certificate_request(&der);
    assert!(pem.starts_with(BEGIN_REQUEST));
    assert!(pem.ends_with(&format!("{END_REQUEST}\n")));
    assert_eq!(
        decode_certificate_request(pem.as_bytes(), &path()).unwrap(),
        der
    );
}

/// A certificate and a request are both signed DER structures, so accepting
/// one where the other is expected would hand a parser an artifact from the
/// wrong protocol. The label is the cheapest place to refuse that, and this
/// pins the refusal in both directions.
#[test]
fn a_certificate_and_a_request_are_not_interchangeable() {
    let der = vec![1u8, 2, 3, 4];

    let certificate_pem = encode_certificate(&der);
    let error = decode_certificate_request(certificate_pem.as_bytes(), &path()).unwrap_err();
    assert!(matches!(error, CliError::PemParse { .. }));

    let request_pem = encode_certificate_request(&der);
    let error = decode_certificate(request_pem.as_bytes(), &path()).unwrap_err();
    assert!(matches!(error, CliError::PemParse { .. }));
}

#[test]
fn encode_then_decode_round_trips_der_bytes() {
    let der: Vec<u8> = (0u8..=255).cycle().take(300).collect();
    let pem = encode_certificate(&der);
    assert!(pem.starts_with(BEGIN_CERTIFICATE));
    assert!(pem.ends_with("-----END CERTIFICATE-----\n"));
    assert_eq!(decode_certificate(pem.as_bytes(), &path()).unwrap(), der);
}

#[test]
fn encode_wraps_base64_body_at_sixty_four_characters() {
    let pem = encode_certificate(&[0xabu8; 100]);
    for line in pem.lines() {
        assert!(line.len() <= 64, "line too long: {line}");
    }
}

#[test]
fn decode_rejects_missing_begin_boundary() {
    let err = decode_certificate(b"not pem at all", &path()).unwrap_err();
    let display = err.to_string();
    assert!(display.contains("BEGIN CERTIFICATE"), "got: {display}");
    assert!(display.contains("subject.pem"), "got: {display}");
}

#[test]
fn decode_rejects_missing_end_boundary() {
    let pem = format!("{BEGIN_CERTIFICATE}\nYWJj\n");
    let err = decode_certificate(pem.as_bytes(), &path()).unwrap_err();
    assert!(err.to_string().contains("END CERTIFICATE"), "got: {err}");
}

#[test]
fn decode_rejects_empty_file_and_empty_body() {
    let empty = decode_certificate(b"", &path()).unwrap_err();
    assert!(empty.to_string().contains("empty"), "got: {empty}");

    let bodyless = format!("{BEGIN_CERTIFICATE}\n{END_CERTIFICATE}\n");
    let err = decode_certificate(bodyless.as_bytes(), &path()).unwrap_err();
    assert!(err.to_string().contains("body is empty"), "got: {err}");
}

#[test]
fn decode_rejects_invalid_base64_body() {
    let pem = format!("{BEGIN_CERTIFICATE}\n@@not base64@@\n{END_CERTIFICATE}\n");
    let err = decode_certificate(pem.as_bytes(), &path()).unwrap_err();
    assert!(err.to_string().contains("base64"), "got: {err}");
}

#[test]
fn decode_rejects_non_utf8_input() {
    let err = decode_certificate(&[0xff, 0xfe, 0x00, 0x01], &path()).unwrap_err();
    assert!(err.to_string().contains("UTF-8"), "got: {err}");
}
