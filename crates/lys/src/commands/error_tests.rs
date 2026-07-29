#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn key_file_missing_display_names_path_and_remedy() {
    let err = CliError::KeyFileMissing {
        path: PathBuf::from("/keys/agent.key"),
    };
    let display = err.to_string();
    assert!(display.contains("/keys/agent.key"), "got: {display}");
    assert!(display.contains("lys key generate"), "got: {display}");
}

#[test]
fn io_display_carries_context_and_source() {
    let err = CliError::Io {
        context: "failed to read payload file /tmp/p".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "gone"),
    };
    let display = err.to_string();
    assert!(
        display.contains("failed to read payload file /tmp/p"),
        "got: {display}"
    );
    assert!(display.contains("gone"), "got: {display}");
}

#[test]
fn verification_failed_display_is_actionable() {
    let display = CliError::VerificationFailed.to_string();
    assert!(
        display.contains("attestation verification failed"),
        "got: {display}"
    );
}

#[test]
fn certificate_verification_failed_display_is_single_and_generic() {
    let display = CliError::CertificateVerificationFailed.to_string();
    assert!(
        display.contains("certificate verification failed"),
        "got: {display}"
    );
    // Non-oracle: the message must not single out one failing check.
    assert!(!display.contains("expired"), "got: {display}");
    assert!(!display.contains("self-signed"), "got: {display}");
}

#[test]
fn open_failed_display_is_single_and_generic() {
    let display = CliError::OpenFailed.to_string();
    assert!(
        display.contains("sealed envelope open failed"),
        "got: {display}"
    );
    // Non-oracle: the message must not single out one failing check.
    assert!(!display.contains("wrong"), "got: {display}");
    assert!(!display.contains("tampered"), "got: {display}");
    assert!(!display.contains("signer"), "got: {display}");
}

#[test]
fn log_dir_missing_display_names_path_and_remedy() {
    let err = CliError::LogDirMissing {
        path: PathBuf::from("/logs/mylog"),
    };
    let display = err.to_string();
    assert!(display.contains("/logs/mylog"), "got: {display}");
    assert!(display.contains("lys log init"), "got: {display}");
}

#[test]
fn log_dir_invalid_display_names_path_and_reason() {
    let err = CliError::LogDirInvalid {
        path: PathBuf::from("/logs/mylog"),
        reason: "leaf 3 is missing".to_string(),
    };
    let display = err.to_string();
    assert!(display.contains("log directory invalid"), "got: {display}");
    assert!(display.contains("/logs/mylog"), "got: {display}");
    assert!(display.contains("leaf 3 is missing"), "got: {display}");
}

#[test]
fn log_inclusion_verification_failed_display_is_single_and_generic() {
    let display = CliError::LogInclusionVerificationFailed.to_string();
    assert!(
        display.contains("inclusion proof verification failed"),
        "got: {display}"
    );
    // Non-oracle: the message must not single out one failing check.
    assert!(!display.contains("signature"), "got: {display}");
    assert!(!display.contains("origin"), "got: {display}");
    assert!(!display.contains("mismatch"), "got: {display}");
    assert!(!display.contains("tampered"), "got: {display}");
}

#[test]
fn log_consistency_verification_failed_display_is_single_and_generic() {
    let display = CliError::LogConsistencyVerificationFailed.to_string();
    assert!(
        display.contains("consistency proof verification failed"),
        "got: {display}"
    );
    // Non-oracle: the message must not single out one failing check.
    assert!(!display.contains("signature"), "got: {display}");
    assert!(!display.contains("origin"), "got: {display}");
    assert!(!display.contains("mismatch"), "got: {display}");
    assert!(!display.contains("tampered"), "got: {display}");
}

#[test]
fn json_parse_display_names_role_and_path() {
    let err = CliError::JsonParse {
        what: "sealed envelope",
        path: PathBuf::from("/envelopes/e.json"),
        source: serde_json::from_str::<serde_json::Value>("{").unwrap_err(),
    };
    let display = err.to_string();
    assert!(
        display.contains("failed to parse sealed envelope JSON"),
        "got: {display}"
    );
    assert!(display.contains("/envelopes/e.json"), "got: {display}");
}

#[test]
fn pem_parse_display_names_path_and_reason() {
    let err = CliError::PemParse {
        path: PathBuf::from("/certs/agent.pem"),
        reason: "first line must be the BEGIN boundary".to_string(),
    };
    let display = err.to_string();
    assert!(display.contains("/certs/agent.pem"), "got: {display}");
    assert!(display.contains("BEGIN boundary"), "got: {display}");
}
