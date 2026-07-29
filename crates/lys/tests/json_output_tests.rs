//! `--json` coverage across every subcommand.
//!
//! The flag is documented as global and honoured everywhere, and that claim is
//! the whole reason it is useful: a caller must not have to discover which
//! subcommands support it. A command that quietly printed human text under
//! `--json` would hand a pipeline something it cannot parse while still
//! exiting 0 — the silent-failure shape this repo refuses.
//!
//! So this file walks the entire command surface and asserts, for each, that
//! stdout is exactly one parseable JSON object carrying `ok`. It is
//! deliberately breadth-first rather than deep: the per-field shapes are
//! pinned by unit tests, what needs pinning here is that nothing is missed.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn run_lys(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(args)
        .output()
        .expect("failed to spawn lys binary")
}

/// Runs a command with `--json` and returns the parsed object, asserting the
/// process succeeded and stdout held exactly one JSON object.
fn json_ok(args: &[&str]) -> Value {
    let output = run_lys(args);
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout was not UTF-8");
    assert!(
        output.status.success(),
        "expected success for {args:?}\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let trimmed = stdout.trim();
    assert!(
        trimmed.lines().count() == 1,
        "expected exactly one line of JSON for {args:?}, got:\n{stdout}"
    );
    let value: Value = serde_json::from_str(trimmed)
        .unwrap_or_else(|e| panic!("stdout was not JSON for {args:?}: {e}\n{stdout}"));
    assert_eq!(
        value["ok"],
        Value::Bool(true),
        "expected ok:true for {args:?}, got {value}"
    );
    value
}

fn write(path: &Path, bytes: &[u8]) {
    std::fs::write(path, bytes).unwrap();
}

fn s(value: &Value, key: &str) -> String {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("field {key} missing or not a string in {value}"))
        .to_string()
}

/// Every subcommand, in one pass, sharing artifacts as it goes.
///
/// A single test rather than twenty: the commands form a chain (a key signs a
/// log, a log yields a proof, a proof is verified) and rebuilding that chain
/// per command would spawn the binary dozens of extra times for no added
/// coverage.
#[test]
fn every_subcommand_honours_the_global_json_flag() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let p = |name: &str| dir.join(name).to_string_lossy().to_string();

    // ── key ──────────────────────────────────────────────────────────
    let signer = p("signer.key");
    let generated = json_ok(&["--json", "key", "generate", "--out", &signer]);
    assert_eq!(generated["generated"], Value::Bool(true));
    let signer_pub = s(&generated, "public_key_ed25519");

    // Re-running reports the key as pre-existing rather than generated.
    let reloaded = json_ok(&["--json", "key", "generate", "--out", &signer]);
    assert_eq!(reloaded["generated"], Value::Bool(false));

    let recipient = p("recipient.key");
    let recipient_json = json_ok(&["--json", "key", "generate", "--out", &recipient]);
    let recipient_x25519 = {
        let inspected = json_ok(&["--json", "key", "inspect", "--key", &recipient]);
        s(&inspected, "public_key_x25519")
    };
    let recipient_pub = s(&recipient_json, "public_key_ed25519");

    let inspected = json_ok(&[
        "--json",
        "key",
        "inspect",
        "--key",
        &signer,
        "--note-name",
        "example.com/json",
        "--ssh",
        "--allowed-signers",
        "tom@example.com",
    ]);
    assert!(s(&inspected, "public_key_openssh").starts_with("ssh-ed25519 "));
    assert!(s(&inspected, "allowed_signers").contains("namespaces=\"git\""));
    assert!(s(&inspected, "verifier_key").starts_with("example.com/json+"));

    // ── log ──────────────────────────────────────────────────────────
    let log = p("log");
    json_ok(&[
        "--json",
        "log",
        "init",
        "--dir",
        &log,
        "--origin",
        "example.com/json",
    ]);

    let leaf = dir.join("leaf.json");
    write(&leaf, b"{\"event\":\"one\"}\n");
    let leaf_path = leaf.to_string_lossy().to_string();
    let appended = json_ok(&[
        "--json", "log", "append", "--dir", &log, "--leaf", &leaf_path,
    ]);
    assert_eq!(appended["leaf_index"], 0);
    assert_eq!(appended["tree_size"], 1);

    let leaf_two = dir.join("leaf2.json");
    write(&leaf_two, b"{\"event\":\"two\"}\n");
    json_ok(&[
        "--json",
        "log",
        "append",
        "--dir",
        &log,
        "--leaf",
        &leaf_two.to_string_lossy(),
    ]);

    let status = json_ok(&["--json", "log", "status", "--dir", &log]);
    assert_eq!(status["tree_size"], 2);
    assert_eq!(status["origin"], "example.com/json");

    let checkpoint = p("checkpoint");
    let checkpointed = json_ok(&[
        "--json",
        "log",
        "checkpoint",
        "--dir",
        &log,
        "--key",
        &signer,
        "--out",
        &checkpoint,
    ]);
    let verifier_key = s(&checkpointed, "verifier_key");

    let inclusion = p("inclusion.json");
    json_ok(&[
        "--json",
        "log",
        "prove",
        "inclusion",
        "--dir",
        &log,
        "--key",
        &signer,
        "--leaf-index",
        "0",
        "--out",
        &inclusion,
    ]);
    let verified = json_ok(&[
        "--json",
        "log",
        "verify",
        "inclusion",
        "--artifact",
        &inclusion,
        "--leaf",
        &leaf_path,
        "--verifier-key",
        &verifier_key,
    ]);
    assert_eq!(verified["verified"], Value::Bool(true));

    let consistency = p("consistency.json");
    json_ok(&[
        "--json",
        "log",
        "prove",
        "consistency",
        "--dir",
        &log,
        "--key",
        &signer,
        "--old-size",
        "1",
        "--out",
        &consistency,
    ]);
    let consistent = json_ok(&[
        "--json",
        "log",
        "verify",
        "consistency",
        "--artifact",
        &consistency,
        "--verifier-key",
        &verifier_key,
    ]);
    assert_eq!(consistent["verified"], Value::Bool(true));

    // ── attest / verify / inspect ────────────────────────────────────
    let payload = dir.join("payload.bin");
    write(&payload, b"payload bytes");
    let payload_path = payload.to_string_lossy().to_string();
    let cose = p("payload.cose");
    json_ok(&[
        "--json",
        "attest",
        "--key",
        &signer,
        "--payload",
        &payload_path,
        "--out",
        &cose,
    ]);
    let attest_verified = json_ok(&[
        "--json",
        "verify",
        "--attestation",
        &cose,
        "--payload",
        &payload_path,
    ]);
    assert_eq!(attest_verified["verified"], Value::Bool(true));
    assert_eq!(attest_verified["signer_public_key"], signer_pub);

    // `inspect` must say verified:false — a machine consumer has to be able
    // to tell unverified fields from verified ones without reading prose.
    let inspected_attestation =
        json_ok(&["--json", "inspect", "attestation", "--attestation", &cose]);
    assert_eq!(inspected_attestation["verified"], Value::Bool(false));
    assert!(s(&inspected_attestation, "warning").contains("UNVERIFIED"));

    // ── ca ───────────────────────────────────────────────────────────
    let claims = dir.join("claims.json");
    write(&claims, b"{\"capabilities\":[\"repo:read\"]}");
    let cert = p("agent.pem");
    let issued = json_ok(&[
        "--json",
        "ca",
        "issue",
        "--key",
        &signer,
        "--subject",
        "agent-json",
        "--claims",
        &claims.to_string_lossy(),
        "--validity-days",
        "7",
        "--out",
        &cert,
    ]);
    assert_eq!(issued["subject"], "agent-json");
    let ca_verified = json_ok(&[
        "--json",
        "ca",
        "verify",
        "--cert",
        &cert,
        "--issuer-public-key",
        &signer_pub,
    ]);
    assert_eq!(ca_verified["verified"], Value::Bool(true));
    assert!(s(&ca_verified, "capability_claims").contains("repo:read"));

    let inspected_cert = json_ok(&["--json", "inspect", "cert", "--cert", &cert]);
    assert_eq!(inspected_cert["verified"], Value::Bool(false));
    assert!(s(&inspected_cert, "capability_claims_warning").contains("UNVERIFIED"));

    // ── ca request / issue --request ──────────────────────────────────
    let holder = p("holder.key");
    let holder_pub = s(
        &json_ok(&["--json", "key", "generate", "--out", &holder]),
        "public_key_ed25519",
    );
    let request = p("holder.csr.pem");
    let requested = json_ok(&[
        "--json",
        "ca",
        "request",
        "--key",
        &holder,
        "--subject",
        "agent-holder",
        "--out",
        &request,
    ]);
    assert_eq!(requested["subject"], "agent-holder");
    assert_eq!(s(&requested, "subject_public_key"), holder_pub);

    let presented_cert = p("holder.pem");
    let presented = json_ok(&[
        "--json",
        "ca",
        "issue",
        "--key",
        &signer,
        "--subject",
        "agent-holder",
        "--request",
        &request,
        "--validity-days",
        "7",
        "--out",
        &presented_cert,
    ]);
    // The certificate binds the holder's own key rather than one the issuer
    // minted, and says so — a consumer must be able to tell the two paths
    // apart, because only one of them is evidence about the holder.
    assert_eq!(s(&presented, "subject_public_key"), holder_pub);
    assert!(s(&presented, "subject_key_origin").contains("proof of possession verified"));
    assert!(s(&issued, "subject_key_origin").contains("never proved possession"));
    assert_ne!(s(&issued, "subject_public_key"), holder_pub);

    json_ok(&[
        "--json",
        "ca",
        "verify",
        "--cert",
        &presented_cert,
        "--issuer-public-key",
        &signer_pub,
    ]);

    // ── seal / open ──────────────────────────────────────────────────
    let envelope = p("secret.sealed.json");
    let seal_att = p("secret.sealed.cose");
    json_ok(&[
        "--json",
        "seal",
        "--key",
        &signer,
        "--recipient-public-key",
        &recipient_x25519,
        "--payload",
        &payload_path,
        "--out",
        &envelope,
        "--attestation-out",
        &seal_att,
    ]);
    let opened = json_ok(&[
        "--json",
        "open",
        "--key",
        &recipient,
        "--sender-public-key",
        &signer_pub,
        "--envelope",
        &envelope,
        "--attestation",
        &seal_att,
        "--out",
        &p("recovered.bin"),
    ]);
    assert_eq!(opened["opened"], Value::Bool(true));
    assert_eq!(opened["payload_bytes"], 13);
    assert_eq!(opened["sender_public_key"], signer_pub);
    let _ = recipient_pub;
}

/// A failure under `--json` must still be JSON on stdout.
///
/// This is the half that is easy to forget and worst to get wrong: a pipeline
/// that gates on `ok` receives unparseable output at exactly the moment
/// something went wrong. The diagnostic must also remain on stderr.
#[test]
fn failures_are_emitted_as_json_with_ok_false() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("no-such-log").to_string_lossy().to_string();
    let output = run_lys(&["--json", "log", "status", "--dir", &missing]);

    assert!(!output.status.success(), "expected a failing exit code");

    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("failure stdout was not JSON: {e}\n{stdout}"));
    assert_eq!(value["ok"], Value::Bool(false));
    assert!(
        value["error"].as_str().unwrap().contains("not initialized"),
        "got {value}"
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("error:"),
        "the human diagnostic must still reach stderr, got: {stderr}"
    );
}

/// A refused issuance must refuse completely: non-zero exit, a machine-readable
/// failure, and — the part worth pinning — no certificate left on disk. Writing
/// the output file before validating the request would leave a refusal that
/// still produced an artifact, which a later step could pick up as though
/// issuance had succeeded.
#[test]
fn a_refused_request_issuance_writes_no_certificate() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let path = |name: &str| dir.join(name).to_string_lossy().to_string();

    let ca = path("ca.key");
    run_lys(&["--json", "key", "generate", "--out", &ca]);
    let holder = path("holder.key");
    run_lys(&["--json", "key", "generate", "--out", &holder]);
    let request = path("holder.csr.pem");
    run_lys(&[
        "--json",
        "ca",
        "request",
        "--key",
        &holder,
        "--subject",
        "agent-noor",
        "--out",
        &request,
    ]);

    let out = dir.join("never-written.pem");
    let output = run_lys(&[
        "--json",
        "ca",
        "issue",
        "--key",
        &ca,
        "--subject",
        "agent-root",
        "--request",
        &request,
        "--validity-days",
        "1",
        "--out",
        &out.to_string_lossy(),
    ]);

    assert!(!output.status.success(), "expected a failing exit code");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("failure stdout was not JSON: {e}\n{stdout}"));
    assert_eq!(value["ok"], Value::Bool(false));
    assert!(
        !out.exists(),
        "a refused issuance must not leave a certificate behind"
    );
}

/// A verification failure stays non-oracle in JSON mode.
///
/// The human path collapses every rejected check to one indistinguishable
/// message. JSON mode reformats that message; it must not enrich it, or the
/// machine surface becomes an oracle the human surface deliberately is not.
///
/// The property is *indistinguishability*, not the absence of particular
/// words: the shipped message names every possible cause as a disjunction
/// precisely so it reveals none of them. So this compares the message across
/// three genuinely different failures — wrong payload, corrupted signature,
/// and a truncated artifact — and requires all three to be byte-identical.
/// An earlier draft of this test grepped for words like "signature" and
/// failed against correct code, which is its own small lesson: assert the
/// property, not a proxy for it.
#[test]
fn verification_failures_are_indistinguishable_in_json() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let key = dir.join("k.key").to_string_lossy().to_string();
    let payload = dir.join("p.bin");
    std::fs::write(&payload, b"real payload").unwrap();
    let cose_path = dir.join("p.cose");
    let cose = cose_path.to_string_lossy().to_string();

    run_lys(&["key", "generate", "--out", &key]);
    run_lys(&[
        "attest",
        "--key",
        &key,
        "--payload",
        &payload.to_string_lossy(),
        "--out",
        &cose,
    ]);
    let good = std::fs::read(&cose_path).unwrap();

    let error_for = |args: &[&str]| -> String {
        let output = run_lys(args);
        assert!(!output.status.success(), "expected failure for {args:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        let value: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|e| panic!("not JSON: {e}\n{stdout}"));
        assert_eq!(value["ok"], Value::Bool(false));
        value["error"].as_str().unwrap().to_string()
    };

    // 1. Correct artifact, wrong payload.
    let wrong_payload = dir.join("t.bin");
    std::fs::write(&wrong_payload, b"different!!!").unwrap();
    let mismatch = error_for(&[
        "--json",
        "verify",
        "--attestation",
        &cose,
        "--payload",
        &wrong_payload.to_string_lossy(),
    ]);

    // 2. Correct payload, signature bits flipped.
    let mut corrupted = good.clone();
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0xff;
    let corrupted_path = dir.join("corrupt.cose");
    std::fs::write(&corrupted_path, &corrupted).unwrap();
    let bad_signature = error_for(&[
        "--json",
        "verify",
        "--attestation",
        &corrupted_path.to_string_lossy(),
        "--payload",
        &payload.to_string_lossy(),
    ]);

    // 3. Correct payload, artifact truncated so it cannot even decode.
    let truncated_path = dir.join("short.cose");
    std::fs::write(&truncated_path, &good[..good.len() / 2]).unwrap();
    let truncated = error_for(&[
        "--json",
        "verify",
        "--attestation",
        &truncated_path.to_string_lossy(),
        "--payload",
        &payload.to_string_lossy(),
    ]);

    assert_eq!(
        mismatch, bad_signature,
        "a payload mismatch and a bad signature must be indistinguishable"
    );
    assert_eq!(
        bad_signature, truncated,
        "a bad signature and an undecodable artifact must be indistinguishable"
    );
}
