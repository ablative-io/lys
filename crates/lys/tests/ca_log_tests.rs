//! `lys ca issue --log`: every certificate is entered in a transparency log
//! before it is written, and a stranger checks both the certificate and its
//! entry with standard tools and nothing from lys.
//!
//! The stranger's tools are `openssl verify` for the certificate and the
//! committed `scripts/verify_inclusion.py`, which walks RFC 6962 by itself,
//! for the entry. Neither is optional: a missing tool is a hard failure,
//! never a skip, because a check that never runs looks exactly like a pass.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[path = "ca_log/outputs.rs"]
mod outputs;
#[path = "ca_log/support.rs"]
mod support;
#[path = "ca_log/tamper.rs"]
mod tamper;

use support::{
    Bench, assert_success, openssl_verify, path_str, pem_to_der, report, run_lys, verify_inclusion,
};

#[test]
fn an_issued_certificate_is_its_logged_leaf_and_a_stranger_verifies_both() {
    let bench = Bench::new();
    let issued = report(&bench.issue_into(&bench.log_dir, "agent-one"));
    let root = issued["root_base64"].as_str().unwrap().to_string();

    let pem = std::fs::read_to_string(bench.path("agent-one.pem")).unwrap();
    let leaf = std::fs::read(bench.path("agent-one.leaf")).unwrap();
    assert_eq!(
        leaf,
        pem_to_der(&pem),
        "the leaf is the certificate's DER, exactly"
    );

    // The stranger holds four files and the root, and nothing from lys.
    let stranger = tempfile::tempdir().unwrap();
    for name in [
        "agent-one.issuer.pem",
        "agent-one.pem",
        "agent-one.leaf",
        "agent-one.inclusion.json",
    ] {
        std::fs::copy(bench.path(name), stranger.path().join(name)).unwrap();
    }

    let checked = openssl_verify(stranger.path(), "agent-one.issuer.pem", "agent-one.pem");
    assert_success(&checked);
    assert!(String::from_utf8_lossy(&checked.stdout).contains("agent-one.pem: OK"));

    // The reported root is the one the checkpoint carries, in the form the
    // script takes, so the script's end-to-end check runs.
    let included = verify_inclusion(
        &stranger.path().join("agent-one.inclusion.json"),
        &stranger.path().join("agent-one.leaf"),
        Some(&root),
    );
    assert_success(&included);
}

#[test]
fn lys_log_verify_inclusion_accepts_the_artifact_under_the_logs_verifier_key() {
    let bench = Bench::new();
    assert_success(&bench.issue_into(&bench.log_dir, "agent-lys"));
    let verified = run_lys(&[
        "log",
        "verify",
        "inclusion",
        "--artifact",
        path_str(&bench.path("agent-lys.inclusion.json")),
        "--leaf",
        path_str(&bench.path("agent-lys.leaf")),
        "--verifier-key",
        &bench.verifier(),
    ]);
    assert_success(&verified);
    assert!(String::from_utf8_lossy(&verified.stdout).contains("inclusion verified"));
}

#[test]
fn the_exported_issuer_is_a_path_length_zero_ca_that_signs_only_certificates_and_crls() {
    let bench = Bench::new();
    assert_success(&bench.issue_into(&bench.log_dir, "agent-anchor"));
    let text = std::process::Command::new(support::openssl())
        .args(["x509", "-noout", "-text", "-in"])
        .arg(bench.path("agent-anchor.issuer.pem"))
        .output()
        .unwrap();
    assert_success(&text);
    let text = String::from_utf8_lossy(&text.stdout);
    for expected in [
        "X509v3 Basic Constraints: critical",
        "CA:TRUE, pathlen:0",
        "X509v3 Key Usage: critical",
        "Certificate Sign, CRL Sign",
        "Not After : Dec 31 23:59:59 9999 GMT",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
    assert!(
        !text.contains("Not Before: Jan  1 00:00:00 1975"),
        "the issuer keeps rcgen's default notBefore:\n{text}"
    );
}

#[test]
fn a_leaf_changed_by_one_byte_fails_the_stranger_check() {
    let bench = Bench::new();
    let issued = report(&bench.issue_into(&bench.log_dir, "agent-two"));
    let tampered = bench.path("agent-two.tampered");
    let mut bytes = std::fs::read(bench.path("agent-two.leaf")).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    std::fs::write(&tampered, bytes).unwrap();

    let refused = verify_inclusion(
        &bench.path("agent-two.inclusion.json"),
        &tampered,
        issued["root_base64"].as_str(),
    );
    assert_eq!(
        refused.status.code(),
        Some(2),
        "exit 2 is VERIFICATION FAILED"
    );
}

#[test]
fn a_second_certificate_takes_the_next_leaf_and_both_stay_provable() {
    let bench = Bench::new();
    let first = report(&bench.issue_into(&bench.log_dir, "agent-a"));
    let second = report(&bench.issue_into(&bench.log_dir, "agent-b"));
    assert_eq!(second["leaf_index"], 1);
    assert_eq!(second["tree_size"], 2);
    let root_two = second["root_base64"].as_str().unwrap();
    assert_ne!(first["root_base64"].as_str(), Some(root_two));

    // An issuer exported later still anchors a certificate issued earlier.
    for cert in ["agent-a.pem", "agent-b.pem"] {
        assert_success(&openssl_verify(bench.dir(), "agent-b.issuer.pem", cert));
    }

    // Each artifact proves its own leaf under the root reported with it.
    assert_success(&verify_inclusion(
        &bench.path("agent-a.inclusion.json"),
        &bench.path("agent-a.leaf"),
        first["root_base64"].as_str(),
    ));
    assert_success(&verify_inclusion(
        &bench.path("agent-b.inclusion.json"),
        &bench.path("agent-b.leaf"),
        Some(root_two),
    ));

    // The first certificate is still in the grown tree: a fresh proof of leaf
    // 0 at size 2 walks a real path to the second root.
    let reproved = bench.path("agent-a.at-two.json");
    assert_success(&run_lys(&[
        "log",
        "prove",
        "inclusion",
        "--dir",
        path_str(&bench.log_dir),
        "--key",
        path_str(&bench.log_key),
        "--leaf-index",
        "0",
        "--out",
        path_str(&reproved),
    ]));
    assert_success(&verify_inclusion(
        &reproved,
        &bench.path("agent-a.leaf"),
        Some(root_two),
    ));
}

#[test]
fn a_log_that_cannot_take_the_entry_stops_the_issuance_and_writes_no_certificate() {
    let bench = Bench::new();
    let missing = bench.path("no-such-log");
    let refused = bench.issue_into(&missing, "agent-refused");
    assert_ne!(refused.status.code(), Some(0));
    let said = support::said(&refused);
    assert!(
        said.contains("lys log init"),
        "the refusal names its remedy: {said}"
    );
    for name in [
        "agent-refused.pem",
        "agent-refused.leaf",
        "agent-refused.inclusion.json",
        "agent-refused.issuer.pem",
    ] {
        assert!(!bench.path(name).exists(), "{name} was written");
    }
}

#[test]
fn the_log_flags_come_together_or_not_at_all() {
    let bench = Bench::new();
    let partial = run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&bench.issuer_key),
        "--subject",
        "agent-partial",
        "--validity",
        "1h",
        "--out",
        path_str(&bench.path("agent-partial.pem")),
        "--log",
        path_str(&bench.log_dir),
    ]);
    assert_ne!(partial.status.code(), Some(0));
    assert!(!bench.path("agent-partial.pem").exists());
    assert_eq!(bench.log_size(), 0);
}
