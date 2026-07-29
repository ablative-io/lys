//! `lys verify --cert` — the join between a certificate and an attestation.
//!
//! Verifying a certificate proves an authority issued it. Verifying an
//! attestation proves a key signed a payload. Both can succeed while concerning
//! two entirely unrelated identities, which is the gap this flag closes, so the
//! tests that matter here are the ones where each half is individually valid
//! and the *pairing* is wrong.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn run_lys(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(args)
        .output()
        .expect("failed to spawn lys binary")
}

/// Runs with `--json`, asserting success, and returns the parsed object.
fn json_ok(args: &[&str]) -> Value {
    let output = run_lys(args);
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    assert!(
        output.status.success(),
        "expected success for {args:?}\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout was not JSON for {args:?}: {e}\n{stdout}"))
}

fn s(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("field {key} missing or not a string in {value}"))
        .to_string()
}

/// A holder with a key, a request, and a certificate binding that key.
struct Fixture {
    dir: PathBuf,
    ca_public_key: String,
    payload: String,
}

impl Fixture {
    fn new(dir: &Path) -> Self {
        let path = |name: &str| dir.join(name).to_string_lossy().to_string();
        let ca_public_key = s(
            &json_ok(&["--json", "key", "generate", "--out", &path("ca.key")]),
            "public_key_ed25519",
        );
        let payload = path("statement.json");
        std::fs::write(&payload, b"{\"action\":\"deployed release 4.2\"}").unwrap();
        std::fs::write(
            dir.join("claims.json"),
            b"{\"capabilities\":[\"release:publish\"]}",
        )
        .unwrap();
        Self {
            dir: dir.to_path_buf(),
            ca_public_key,
            payload,
        }
    }

    fn path(&self, name: &str) -> String {
        self.dir.join(name).to_string_lossy().to_string()
    }

    /// Generates a holder key and returns its public key hex.
    fn holder(&self, name: &str) -> String {
        s(
            &json_ok(&["--json", "key", "generate", "--out", &self.path(name)]),
            "public_key_ed25519",
        )
    }

    /// Issues a certificate over `holder`'s presented key.
    fn certify_presented(&self, holder: &str, subject: &str, cert: &str) {
        let request = self.path(&format!("{subject}.csr.pem"));
        json_ok(&[
            "--json",
            "ca",
            "request",
            "--key",
            &self.path(holder),
            "--subject",
            subject,
            "--out",
            &request,
        ]);
        json_ok(&[
            "--json",
            "ca",
            "issue",
            "--key",
            &self.path("ca.key"),
            "--subject",
            subject,
            "--request",
            &request,
            "--claims",
            &self.path("claims.json"),
            "--validity-days",
            "1",
            "--out",
            &self.path(cert),
        ]);
    }

    /// Attests `self.payload` with `holder`'s key.
    fn attest(&self, holder: &str, out: &str) {
        json_ok(&[
            "--json",
            "attest",
            "--key",
            &self.path(holder),
            "--payload",
            &self.payload,
            "--out",
            &self.path(out),
        ]);
    }

    fn verify_certified(&self, attestation: &str, cert: &str) -> Output {
        run_lys(&[
            "--json",
            "verify",
            "--attestation",
            &self.path(attestation),
            "--payload",
            &self.payload,
            "--cert",
            &self.path(cert),
            "--issuer-public-key",
            &self.ca_public_key,
        ])
    }
}

#[test]
fn a_certified_attestation_verifies_and_reports_the_join() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(tmp.path());
    let holder_key = fixture.holder("noor.key");
    fixture.certify_presented("noor.key", "agent-noor", "noor.pem");
    fixture.attest("noor.key", "statement.cose");

    let output = fixture.verify_certified("statement.cose", "noor.pem");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_str(String::from_utf8(output.stdout).unwrap().trim())
        .expect("stdout was not JSON");

    assert_eq!(value["verified"], Value::Bool(true));
    assert_eq!(value["signer_matches_certificate"], Value::Bool(true));
    assert_eq!(s(&value, "signer_public_key"), holder_key);
    // Claims are only reachable once all three checks pass, so their presence
    // here is itself part of the assertion.
    assert!(s(&value, "capability_claims").contains("release:publish"));
}

/// The exact hole this flag exists to close, stated as a test: before proof of
/// possession, `ca issue` generated the subject keypair and discarded it, so a
/// certificate named a key nobody ever held. Such a certificate verifies
/// perfectly on its own and the holder's attestation verifies perfectly on its
/// own — and no identity connects them. The join must refuse it.
#[test]
fn a_generated_key_certificate_can_never_satisfy_the_join() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(tmp.path());
    fixture.holder("noor.key");
    fixture.attest("noor.key", "statement.cose");

    // No --request: the pre-proof-of-possession path.
    json_ok(&[
        "--json",
        "ca",
        "issue",
        "--key",
        &fixture.path("ca.key"),
        "--subject",
        "agent-noor",
        "--claims",
        &fixture.path("claims.json"),
        "--validity-days",
        "1",
        "--out",
        &fixture.path("generated.pem"),
    ]);

    // Each half is individually valid — prove that, so the failure below is
    // demonstrably the join and not a broken fixture.
    json_ok(&[
        "--json",
        "ca",
        "verify",
        "--cert",
        &fixture.path("generated.pem"),
        "--issuer-public-key",
        &fixture.ca_public_key,
    ]);
    json_ok(&[
        "--json",
        "verify",
        "--attestation",
        &fixture.path("statement.cose"),
        "--payload",
        &fixture.payload,
    ]);

    let output = fixture.verify_certified("statement.cose", "generated.pem");
    assert!(
        !output.status.success(),
        "a certificate over a key nobody holds must not certify anybody's statement"
    );
}

#[test]
fn one_holders_attestation_is_not_certified_by_anothers_certificate() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(tmp.path());
    fixture.holder("noor.key");
    fixture.holder("mallory.key");
    fixture.certify_presented("noor.key", "agent-noor", "noor.pem");
    fixture.attest("mallory.key", "mallory.cose");

    // Mallory's attestation is real and noor's certificate is real; presenting
    // them together is the substitution the join refuses.
    let output = fixture.verify_certified("mallory.cose", "noor.pem");
    assert!(!output.status.success());

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("release:publish"),
        "capability claims must not be echoed from a certificate whose join failed, got: {combined}"
    );
}

/// The three halves must be indistinguishable in failure. A caller who could
/// tell "the certificate was bad" from "the signer did not match" would have an
/// oracle the individual commands deliberately withhold.
#[test]
fn all_three_failure_modes_are_indistinguishable() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(tmp.path());
    fixture.holder("noor.key");
    fixture.holder("mallory.key");
    fixture.certify_presented("noor.key", "agent-noor", "noor.pem");
    fixture.certify_presented("mallory.key", "agent-mallory", "mallory.pem");
    fixture.attest("noor.key", "statement.cose");

    // 1. Wrong payload — the attestation half fails.
    let other_payload = fixture.path("other.json");
    std::fs::write(&other_payload, b"{\"action\":\"something else\"}").unwrap();
    let wrong_payload = run_lys(&[
        "--json",
        "verify",
        "--attestation",
        &fixture.path("statement.cose"),
        "--payload",
        &other_payload,
        "--cert",
        &fixture.path("noor.pem"),
        "--issuer-public-key",
        &fixture.ca_public_key,
    ]);

    // 2. Certificate outside its validity window — the certificate half fails.
    let expired = run_lys(&[
        "--json",
        "verify",
        "--attestation",
        &fixture.path("statement.cose"),
        "--payload",
        &fixture.payload,
        "--cert",
        &fixture.path("noor.pem"),
        "--issuer-public-key",
        &fixture.ca_public_key,
        "--at",
        "2020-01-01T00:00:00Z",
    ]);

    // 3. Valid certificate, valid attestation, wrong pairing — the join fails.
    let mismatched = fixture.verify_certified("statement.cose", "mallory.pem");

    let message = |output: &Output| {
        let value: Value =
            serde_json::from_str(String::from_utf8_lossy(&output.stdout).trim()).unwrap();
        assert_eq!(value["ok"], Value::Bool(false));
        s(&value, "error")
    };

    let first = message(&wrong_payload);
    assert_eq!(
        first,
        message(&expired),
        "certificate failure distinguishable"
    );
    assert_eq!(first, message(&mismatched), "join failure distinguishable");
    for output in [&wrong_payload, &expired, &mismatched] {
        assert!(!output.status.success());
    }
}

/// Without `--cert` the command still answers the weaker question, and says so
/// rather than letting a reader mistake it for the stronger one.
#[test]
fn the_uncertified_path_names_its_own_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(tmp.path());
    fixture.holder("noor.key");
    fixture.attest("noor.key", "statement.cose");

    let output = run_lys(&[
        "verify",
        "--attestation",
        &fixture.path("statement.cose"),
        "--payload",
        &fixture.payload,
    ]);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("not who holds that key"), "got: {stdout}");
}

/// `--cert` and `--issuer-public-key` are useless apart, and clap must refuse
/// the half-specified form rather than silently skipping the certificate half.
#[test]
fn the_certificate_flags_cannot_be_supplied_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let fixture = Fixture::new(tmp.path());
    fixture.holder("noor.key");
    fixture.certify_presented("noor.key", "agent-noor", "noor.pem");
    fixture.attest("noor.key", "statement.cose");

    let attestation = fixture.path("statement.cose");
    let cert = fixture.path("noor.pem");

    let only_cert = run_lys(&[
        "verify",
        "--attestation",
        &attestation,
        "--payload",
        &fixture.payload,
        "--cert",
        &cert,
    ]);
    assert!(
        !only_cert.status.success(),
        "--cert without --issuer-public-key must be refused, not silently ignored"
    );

    let only_issuer = run_lys(&[
        "verify",
        "--attestation",
        &attestation,
        "--payload",
        &fixture.payload,
        "--issuer-public-key",
        &fixture.ca_public_key,
    ]);
    assert!(
        !only_issuer.status.success(),
        "--issuer-public-key without --cert must be refused, not silently ignored"
    );
}
