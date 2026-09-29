#![cfg(test)]
//! The real binary checks structure without opening the configured runtime.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn config(root: &std::path::Path, listen: std::net::SocketAddr) -> Value {
    json!({
        "listen": listen.to_string(), "log_dir": root.join("log"), "log_origin": "test",
        "event_key_file": root.join("missing-key"), "issuer": "http://127.0.0.1:1",
        "client_id": "test", "client_secret_file": root.join("missing-secret"),
        "redirect_url": "http://127.0.0.1:1/callback",
        "administrator": {"issuer": "http://127.0.0.1:1", "subject": "admin"},
        "link_audit_source": {"issuer": "http://127.0.0.1:1", "subject": "audit"},
        "session_seconds": 60, "secure_cookie": false,
        "grant_log_dir": root.join("grants"), "grant_log_origin": "test",
        "grant_model_file": root.join("missing-model")
    })
}

fn check(bytes: &[u8]) -> Result<Output, Box<dyn std::error::Error>> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_lys-identity-server"))
        .args(["--check-config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("missing stdin")?
        .write_all(bytes)?;
    Ok(child.wait_with_output()?)
}

#[test]
fn checks_without_binding_or_opening_missing_stores_and_secrets() -> TestResult {
    let dir = tempfile::tempdir()?;
    let held = std::net::TcpListener::bind("127.0.0.1:0")?;
    let bytes = serde_json::to_vec(&config(dir.path(), held.local_addr()?))?;
    let output = check(&bytes)?;
    assert!(output.status.success(), "{:?}", output.stderr);
    let receipt: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(receipt["format"], "lys-config-check/1");
    assert_eq!(receipt["build"], lys_identity_server::read_api::BUILD);
    assert_eq!(
        receipt["config_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 0);
    let path = dir.path().join("input.json");
    std::fs::write(&path, &bytes)?;
    let output = Command::new(env!("CARGO_BIN_EXE_lys-identity-server"))
        .arg("--check-config")
        .arg(path)
        .output()?;
    assert!(output.status.success());
    assert_eq!(serde_json::from_slice::<Value>(&output.stdout)?, receipt);
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 1);
    Ok(())
}

#[test]
fn refuses_unknown_fields_types_and_invalid_values_without_echoing_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let base = config(dir.path(), "127.0.0.1:1".parse()?);
    let mut unknown = base.clone();
    unknown["private-marker"] = json!("private-value");
    let mut mistyped = base.clone();
    mistyped["session_seconds"] = json!("private-value");
    let mut invalid = base;
    invalid["session_seconds"] = json!(0);
    for value in [unknown, mistyped, invalid] {
        let output = check(&serde_json::to_vec(&value)?)?;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let message = String::from_utf8(output.stderr)?;
        assert!(message.contains("config_check_"));
        assert!(!message.contains("private-"));
    }
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 0);
    Ok(())
}

#[test]
fn argument_errors_never_fall_through_to_serve() -> TestResult {
    for args in [vec!["--check-config"], vec!["--check-config", "-", "extra"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_lys-identity-server"))
            .args(args)
            .stdin(Stdio::null())
            .output()?;
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)?.contains("usage: --check-config"));
    }
    Ok(())
}
