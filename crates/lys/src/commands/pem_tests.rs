#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use super::*;

fn path() -> PathBuf {
    PathBuf::from("/certs/subject.pem")
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
