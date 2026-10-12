//! `lys ca issuer-cert`: the issuer certificate stored beside the CA key, the
//! same bytes `lys ca issue --issuer-out` writes on every call, and the
//! refusals that leave no stored file behind.
//!
//! Supplied creation seconds make regeneration observable without waiting:
//! a fresh certificate at a later instant differs from the stored one.

use std::path::Path;
use std::process::Command;

use crate::support::{
    Bench, assert_success, corrupt_first_leaf, dir_bytes, leaf_files, openssl, path_str,
    clock_work, pem_to_der, report, run_lys, run_lys_at, said,
};

const T0: i64 = 1_700_000_000;

fn issuer_cert(key: &Path, out: &Path) -> std::process::Output {
    run_lys(&[
        "--json",
        "ca",
        "issuer-cert",
        "--key",
        path_str(key),
        "--out",
        path_str(out),
    ])
}

fn issuer_cert_at(key: &Path, out: &Path, at: i64) -> std::process::Output {
    run_lys_at(&[
        "--json", "ca", "issuer-cert", "--key", path_str(key), "--out", path_str(out),
    ], &at.to_string())
}

fn generate(key: &Path) {
    assert_success(&run_lys(&["key", "generate", "--out", path_str(key)]));
}

fn bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap()
}

fn openssl_x509(pem: &Path, args: &[&str]) -> String {
    let output = Command::new(openssl())
        .args(["x509", "-in"])
        .arg(pem)
        .args(["-noout"])
        .args(args)
        .output()
        .unwrap();
    assert_success(&output);
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A logged issuance with the CA key only, writing the issuer certificate at
/// `issuer_out`.
fn issue_with_issuer_out(
    bench: &Bench,
    key: &Path,
    log_dir: &Path,
    subject: &str,
    issuer_out: &Path,
) -> std::process::Output {
    run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(key),
        "--subject",
        subject,
        "--validity",
        "1h",
        "--out",
        path_str(&bench.path(&format!("{subject}.pem"))),
        "--log",
        path_str(log_dir),
        "--leaf-out",
        path_str(&bench.path(&format!("{subject}.leaf"))),
        "--issuer-out",
        path_str(issuer_out),
    ])
}

fn issue_at(bench: &Bench, key: &Path, subject: &str, issuer_out: &Path, at: i64) -> std::process::Output {
    run_lys_at(&[
        "ca", "issue", "--key", path_str(key), "--subject", subject,
        "--validity", "1h", "--out", path_str(&bench.path(&format!("{subject}.pem"))),
        "--log", path_str(&bench.log_dir), "--leaf-out", path_str(&bench.path(&format!("{subject}.leaf"))),
        "--issuer-out", path_str(issuer_out),
    ], &at.to_string())
}

#[test]
fn issuer_cert_writes_the_stored_certificate_and_every_later_call_writes_the_same_bytes() {
    let bench = Bench::new();
    let issuer_pem = bench.path("issuer.pem");
    let stored = bench.path("issuer.key.issuer.pem");
    assert!(!stored.exists());

    let created = issuer_cert_at(&bench.issuer_key, &issuer_pem, T0);
    assert_eq!(clock_work(&created).provider_reads, 1);
    let written = report(&created);
    let text = std::fs::read_to_string(&issuer_pem).unwrap();
    let begin = "-----BEGIN CERTIFICATE-----";
    let blocks = text.lines().filter(|line| *line == begin).count();
    assert_eq!(blocks, 1);
    let inspected = report(&run_lys(&[
        "--json",
        "key",
        "inspect",
        "--key",
        path_str(&bench.issuer_key),
    ]));
    let issuer_hex = written["issuer_public_key"].as_str().unwrap();
    assert_eq!(issuer_hex.len(), 64);
    assert_eq!(Some(issuer_hex), inspected["public_key_ed25519"].as_str());
    assert_eq!(
        written["issuer_certificate_path"].as_str(),
        Some(path_str(&issuer_pem))
    );
    assert_eq!(bytes(&issuer_pem), bytes(&stored));

    // A CA whose subject is the issuer key, as standard tooling reads it.
    let subject = openssl_x509(&issuer_pem, &["-subject", "-nameopt", "RFC2253"]);
    assert_eq!(subject.trim(), format!("subject=CN={issuer_hex}"));
    let constraints = openssl_x509(&issuer_pem, &["-ext", "basicConstraints"]);
    assert!(constraints.contains("CA:TRUE"), "{constraints}");

    // Public: no key material anywhere in it.
    let seed = bytes(&bench.issuer_key);
    assert_eq!(seed.len(), 32);
    let der = pem_to_der(&text);
    let leaked = der.windows(seed.len()).any(|window| window == seed);
    assert!(!leaked, "the issuer key's seed is in the certificate");
    assert!(!text.lines().any(|line| line.contains("PRIVATE")));

    let again = bench.path("issuer-again.pem");
    let exported = issuer_cert_at(&bench.issuer_key, &again, T0 + 2);
    assert_success(&exported);
    assert_eq!(clock_work(&exported).provider_reads, 0);
    assert_eq!(bytes(&issuer_pem), bytes(&again));

    // `ca issue --issuer-out` writes the same stored bytes, and they anchor
    // the certificate it issued.
    let at_issue = bench.path("issuer-at-issue.pem");
    let issued = issue_at(
        &bench,
        &bench.issuer_key,
        "agent-x",
        &at_issue,
        T0 + 4,
    );
    assert_success(&issued);
    assert_eq!(clock_work(&issued).provider_reads, 2);
    assert_eq!(leaf_files(&bench.log_dir), 1);
    assert_eq!(bytes(&issuer_pem), bytes(&at_issue));
    let verified = crate::support::openssl_verify_at(bench.dir(), "issuer.pem", "agent-x.pem", T0 + 4);
    assert_success(&verified);
    assert_eq!(
        String::from_utf8_lossy(&verified.stdout).trim(),
        "agent-x.pem: OK"
    );
}

#[test]
fn an_issuer_certificate_first_built_by_issue_is_the_one_issuer_cert_writes_later() {
    let bench = Bench::new();
    let key = bench.path("issuer2.key");
    generate(&key);
    let stored = bench.path("issuer2.key.issuer.pem");
    assert!(!stored.exists());

    let first = bench.path("first.pem");
    let issued = issue_at(
        &bench,
        &key,
        "agent-y",
        &first,
        T0,
    );
    assert_success(&issued);
    assert_eq!(clock_work(&issued).provider_reads, 3);
    assert_eq!(leaf_files(&bench.log_dir), 1);
    assert_eq!(bytes(&first), bytes(&stored));

    let later = bench.path("later.pem");
    let exported = issuer_cert_at(&key, &later, T0 + 2);
    assert_success(&exported);
    assert_eq!(clock_work(&exported).provider_reads, 0);
    assert_eq!(bytes(&first), bytes(&later));
}

#[test]
fn issuer_regeneration_control() {
    use x509_parser::prelude::FromDer;
    let first = Bench::new();
    let later = Bench::new();
    std::fs::copy(&first.issuer_key, &later.issuer_key).unwrap();
    let first_pem = first.path("first.pem");
    let later_pem = later.path("later.pem");
    assert_success(&issuer_cert_at(&first.issuer_key, &first_pem, T0));
    assert_success(&issuer_cert_at(&later.issuer_key, &later_pem, T0 + 2));
    let first_der = pem_to_der(&std::fs::read_to_string(first_pem).unwrap());
    let later_der = pem_to_der(&std::fs::read_to_string(later_pem).unwrap());
    let (_, first_cert) = x509_parser::certificate::X509Certificate::from_der(&first_der).unwrap();
    let (_, later_cert) = x509_parser::certificate::X509Certificate::from_der(&later_der).unwrap();
    assert_eq!(first_cert.validity().not_before.timestamp(), T0);
    assert_eq!(later_cert.validity().not_before.timestamp(), T0 + 2);
    assert_eq!(first_cert.public_key(), later_cert.public_key());
    assert_eq!(first_cert.validity().not_after, later_cert.validity().not_after);
    assert_ne!(first_der, later_der);
}

#[test]
fn real_stored_issuer_export_does_not_read_a_failed_clock() {
    let bench = Bench::new();
    let first = bench.path("first.pem");
    assert_success(&issuer_cert_at(&bench.issuer_key, &first, T0));
    let later = bench.path("later.pem");
    let output = run_lys_at(&[
        "--json", "ca", "issuer-cert", "--key", path_str(&bench.issuer_key), "--out", path_str(&later),
    ], "unavailable");
    assert_success(&output);
    assert_eq!(clock_work(&output).provider_reads, 0);
    assert_eq!(bytes(&first), bytes(&later));
}

#[test]
fn a_stored_issuer_certificate_of_another_key_is_refused_by_name_and_left_alone() {
    let bench = Bench::new();
    assert_success(&issuer_cert(&bench.issuer_key, &bench.path("issuer.pem")));
    let other = bench.path("issuer2.key");
    generate(&other);
    assert_success(&issuer_cert(&other, &bench.path("issuer2.pem")));
    let other_stored = bench.path("issuer2.key.issuer.pem");
    let stored = bench.path("issuer.key.issuer.pem");
    std::fs::copy(&other_stored, &stored).unwrap();

    let wrong = bench.path("wrong.pem");
    let refused = issuer_cert(&bench.issuer_key, &wrong);
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains(path_str(&stored)), "{stderr}");
    assert!(!wrong.exists());
    assert_eq!(bytes(&other_stored), bytes(&stored));
}

#[test]
fn a_missing_issuer_key_is_refused_and_nothing_is_written() {
    let bench = Bench::new();
    let missing = bench.path("missing.key");
    let out = bench.path("issuer3.pem");
    let refused = issuer_cert(&missing, &out);
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("identity key file not found"), "{stderr}");
    for path in [out, missing, bench.path("missing.key.issuer.pem")] {
        assert!(!path.exists(), "{} was written", path.display());
    }
}

#[test]
fn an_existing_output_is_refused_by_name_before_the_stored_certificate_is_built() {
    let bench = Bench::new();
    let issuer_pem = bench.path("issuer.pem");
    assert_success(&issuer_cert(&bench.issuer_key, &issuer_pem));
    let before = bytes(&issuer_pem);

    let other = bench.path("issuer2.key");
    generate(&other);
    let refused = issuer_cert(&other, &issuer_pem);
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains(path_str(&issuer_pem)), "{stderr}");
    assert_eq!(bytes(&issuer_pem), before);

    let fresh = bench.path("issuer3.key");
    generate(&fresh);
    let taken = bench.path("taken.pem");
    std::fs::write(&taken, b"someone else's file").unwrap();
    let refused = issuer_cert(&fresh, &taken);
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains(path_str(&taken)), "{stderr}");
    assert_eq!(bytes(&taken), b"someone else's file");
    assert!(!bench.path("issuer3.key.issuer.pem").exists());
}

#[test]
fn a_log_that_refuses_the_entry_leaves_no_stored_issuer_certificate() {
    let bench = Bench::new();
    let key = bench.path("issuer5.key");
    generate(&key);
    let badlog = bench.path("badlog");
    assert_success(&run_lys(&[
        "log",
        "init",
        "--dir",
        path_str(&badlog),
        "--origin",
        "example.com/lys/issuance",
    ]));
    // A leaf to damage and an act behind it: an empty log has no record to
    // corrupt, and damage in the last act is an unfinished act an open cuts.
    for (name, leaf) in [
        (
            "badlog-seed.leaf",
            "a leaf the bad log held before it was damaged",
        ),
        (
            "badlog-later.leaf",
            "the act that came after the damaged leaf",
        ),
    ] {
        let seed_leaf = bench.path(name);
        std::fs::write(&seed_leaf, leaf).unwrap();
        assert_success(&run_lys(&[
            "log",
            "append",
            "--dir",
            path_str(&badlog),
            "--leaf",
            path_str(&seed_leaf),
        ]));
    }
    let leaves = leaf_files(&badlog);
    corrupt_first_leaf(&badlog);
    let damaged = dir_bytes(&badlog);

    let issuer_out = bench.path("issuer5.pem");
    let refused = issue_with_issuer_out(&bench, &key, &badlog, "agent-z", &issuer_out);
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    for name in [
        "agent-z.pem",
        "agent-z.leaf",
        "issuer5.pem",
        "issuer5.key.issuer.pem",
    ] {
        assert!(!bench.path(name).exists(), "{name} was written");
    }
    assert_eq!(
        dir_bytes(&badlog),
        damaged,
        "a refused issuance writes nothing to the log"
    );
    assert_eq!(leaves, 2);
}
