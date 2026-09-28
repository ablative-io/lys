//! Failures under `--json`: emitted as JSON with `ok` false, nothing written
//! on a refused issuance, and verification failures indistinguishable.

use serde_json::Value;

use super::run_lys;

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

    let log = path("log");
    run_lys(&[
        "--json",
        "log",
        "init",
        "--dir",
        &log,
        "--origin",
        "example.com/lys/issuance",
    ]);

    let out = dir.join("never-written.pem");
    let leaf = dir.join("never-written.leaf");
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
        "--log",
        &log,
        "--leaf-out",
        &leaf.to_string_lossy(),
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
    assert!(
        !leaf.exists(),
        "a refused issuance must not leave a leaf behind"
    );
    let leaves = std::fs::read_dir(dir.join("log/leaves")).unwrap();
    assert_eq!(leaves.count(), 0);
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
