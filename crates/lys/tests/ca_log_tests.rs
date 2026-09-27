//! `lys ca issue --log`: every certificate is entered in a transparency log
//! before it is written, and a stranger checks both the certificate and its
//! entry with standard tools and nothing from lys.
//!
//! The stranger's tools are `openssl verify` for the certificate and the
//! committed `scripts/verify_inclusion.py`, which walks RFC 6962 by itself,
//! for the entry. Neither is optional: a missing tool is a hard failure,
//! never a skip, because a check that never runs looks exactly like a pass.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run_lys(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(args)
        .output()
        .expect("failed to spawn lys binary")
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("tempdir path was not UTF-8")
}

fn assert_success(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The first candidate that runs and names itself with `expect` in the
/// output of `args`, after the override in `env`.
fn find_tool(env: &str, candidates: &[&str], args: &[&str], expect: &str) -> PathBuf {
    let from_env = std::env::var(env).ok();
    let tried: Vec<&str> = from_env
        .iter()
        .map(String::as_str)
        .chain(candidates.iter().copied())
        .collect();
    for candidate in &tried {
        let Ok(output) = Command::new(candidate).args(args).output() else {
            continue;
        };
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if said.contains(expect) {
            return PathBuf::from(candidate);
        }
    }
    panic!("no {expect} found among {tried:?}; install it or name it in {env}");
}

fn openssl() -> PathBuf {
    find_tool(
        "LYS_OPENSSL_BIN",
        &[
            "openssl",
            "/opt/homebrew/bin/openssl",
            "/opt/homebrew/opt/openssl@3/bin/openssl",
            "/usr/local/bin/openssl",
        ],
        &["version"],
        "OpenSSL 3",
    )
}

fn python() -> PathBuf {
    find_tool(
        "LYS_PYTHON_BIN",
        &[
            "python3",
            "/usr/bin/python3",
            "/opt/homebrew/bin/python3",
            "/usr/local/bin/python3",
        ],
        &["--version"],
        "Python 3",
    )
}

fn verify_inclusion_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_inclusion.py")
}

/// Decodes the one PEM block in `pem` to DER.
fn pem_to_der(pem: &str) -> Vec<u8> {
    use base64::Engine;
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .unwrap()
}

/// An issuer key, an initialized log with its operator key, and the paths an
/// issuance writes, all inside one temporary directory.
struct Bench {
    tmp: tempfile::TempDir,
    issuer_key: PathBuf,
    log_dir: PathBuf,
    log_key: PathBuf,
}

impl Bench {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let issuer_key = tmp.path().join("issuer.key");
        let log_dir = tmp.path().join("log");
        let log_key = tmp.path().join("operator.key");
        assert_success(&run_lys(&[
            "key",
            "generate",
            "--out",
            path_str(&issuer_key),
        ]));
        assert_success(&run_lys(&["key", "generate", "--out", path_str(&log_key)]));
        assert_success(&run_lys(&[
            "log",
            "init",
            "--dir",
            path_str(&log_dir),
            "--origin",
            "example.com/lys/issuance",
        ]));
        Self {
            tmp,
            issuer_key,
            log_dir,
            log_key,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.tmp.path().join(name)
    }

    fn issue_into(&self, log_dir: &Path, subject: &str) -> Output {
        run_lys(&[
            "--json",
            "ca",
            "issue",
            "--key",
            path_str(&self.issuer_key),
            "--subject",
            subject,
            "--validity",
            "1h",
            "--out",
            path_str(&self.path(&format!("{subject}.pem"))),
            "--issuer-out",
            path_str(&self.path("issuer.pem")),
            "--log",
            path_str(log_dir),
            "--log-key",
            path_str(&self.log_key),
            "--leaf-out",
            path_str(&self.path(&format!("{subject}.leaf"))),
            "--artifact-out",
            path_str(&self.path(&format!("{subject}.inclusion.json"))),
        ])
    }
}

#[test]
fn an_issued_certificate_is_its_logged_leaf_and_a_stranger_verifies_both() {
    let bench = Bench::new();
    let issued = bench.issue_into(&bench.log_dir, "agent-one");
    assert_success(&issued);

    let pem = std::fs::read_to_string(bench.path("agent-one.pem")).unwrap();
    let leaf = std::fs::read(bench.path("agent-one.leaf")).unwrap();
    assert_eq!(
        leaf,
        pem_to_der(&pem),
        "the leaf is the certificate's DER, exactly"
    );

    // The stranger holds four files and nothing from lys.
    let stranger = tempfile::tempdir().unwrap();
    for name in [
        "issuer.pem",
        "agent-one.pem",
        "agent-one.leaf",
        "agent-one.inclusion.json",
    ] {
        std::fs::copy(bench.path(name), stranger.path().join(name)).unwrap();
    }

    let checked = Command::new(openssl())
        .args(["verify", "-CAfile", "issuer.pem", "agent-one.pem"])
        .current_dir(stranger.path())
        .output()
        .unwrap();
    assert_success(&checked);
    assert!(String::from_utf8_lossy(&checked.stdout).contains("agent-one.pem: OK"));

    let included = Command::new(python())
        .arg(verify_inclusion_script())
        .args(["agent-one.inclusion.json", "agent-one.leaf"])
        .current_dir(stranger.path())
        .output()
        .unwrap();
    assert_success(&included);
}

#[test]
fn a_leaf_changed_by_one_byte_fails_the_stranger_check() {
    let bench = Bench::new();
    assert_success(&bench.issue_into(&bench.log_dir, "agent-two"));
    let tampered = bench.path("agent-two.tampered");
    let mut bytes = std::fs::read(bench.path("agent-two.leaf")).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    std::fs::write(&tampered, bytes).unwrap();

    let refused = Command::new(python())
        .arg(verify_inclusion_script())
        .arg(bench.path("agent-two.inclusion.json"))
        .arg(&tampered)
        .output()
        .unwrap();
    assert_eq!(
        refused.status.code(),
        Some(2),
        "exit 2 is VERIFICATION FAILED"
    );
}

#[test]
fn a_second_certificate_takes_the_next_leaf_and_both_stay_provable() {
    let bench = Bench::new();
    assert_success(&bench.issue_into(&bench.log_dir, "agent-a"));
    let second = bench.issue_into(&bench.log_dir, "agent-b");
    assert_success(&second);
    let report: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(report["leaf_index"], 1);
    assert_eq!(report["tree_size"], 2);

    let first = Command::new(python())
        .arg(verify_inclusion_script())
        .arg(bench.path("agent-a.inclusion.json"))
        .arg(bench.path("agent-a.leaf"))
        .output()
        .unwrap();
    assert_success(&first);
}

#[test]
fn a_log_that_cannot_take_the_entry_stops_the_issuance_and_writes_no_certificate() {
    let bench = Bench::new();
    let missing = bench.path("no-such-log");
    let refused = bench.issue_into(&missing, "agent-refused");
    assert_ne!(refused.status.code(), Some(0));
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&refused.stdout),
        String::from_utf8_lossy(&refused.stderr)
    );
    assert!(
        said.contains("lys log init"),
        "the refusal names its remedy: {said}"
    );
    for name in [
        "agent-refused.pem",
        "agent-refused.leaf",
        "agent-refused.inclusion.json",
        "issuer.pem",
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
}
