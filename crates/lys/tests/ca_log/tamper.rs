//! Tampering: a stranger's checks must refuse every changed certificate,
//! issuer and artifact field, not only accept the genuine ones.

use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::support::{
    Bench, assert_success, der_to_pem, openssl_verify, path_str, pem_to_der, report, run_lys,
    verify_inclusion,
};

#[test]
fn a_certificate_changed_by_one_byte_fails_openssl_verify() {
    let bench = Bench::new();
    assert_success(&bench.issue_into(&bench.log_dir, "agent-cert"));
    let pem = std::fs::read_to_string(bench.path("agent-cert.pem")).unwrap();
    let mut der = pem_to_der(&pem);
    let last = der.len() - 1;
    der[last] ^= 0x01;
    std::fs::write(bench.path("agent-cert.tampered.pem"), der_to_pem(&der)).unwrap();

    assert_success(&openssl_verify(
        bench.dir(),
        "agent-cert.issuer.pem",
        "agent-cert.pem",
    ));
    let refused = openssl_verify(
        bench.dir(),
        "agent-cert.issuer.pem",
        "agent-cert.tampered.pem",
    );
    assert_ne!(
        refused.status.code(),
        Some(0),
        "a changed signature verified"
    );
}

#[test]
fn a_certificate_checked_against_another_issuer_fails_openssl_verify() {
    let bench = Bench::new();
    assert_success(&bench.issue_into(&bench.log_dir, "agent-issuer"));

    let other_key = bench.path("other-issuer.key");
    assert_success(&run_lys(&[
        "key",
        "generate",
        "--out",
        path_str(&other_key),
    ]));
    assert_success(&run_lys(&[
        "ca",
        "issue",
        "--key",
        path_str(&other_key),
        "--subject",
        "someone-else",
        "--validity",
        "1h",
        "--out",
        path_str(&bench.path("someone-else.pem")),
        "--issuer-out",
        path_str(&bench.path("other-issuer.pem")),
    ]));

    let refused = openssl_verify(bench.dir(), "other-issuer.pem", "agent-issuer.pem");
    assert_ne!(
        refused.status.code(),
        Some(0),
        "a certificate verified under an issuer that never signed it"
    );
}

/// Two leaves, so leaf 1's proof has a real path and a relabelled index or
/// size has something to contradict.
struct Proven {
    bench: Bench,
    artifact: serde_json::Value,
    root: String,
}

impl Proven {
    fn new() -> Self {
        let bench = Bench::new();
        assert_success(&bench.issue_into(&bench.log_dir, "agent-first"));
        let issued = report(&bench.issue_into(&bench.log_dir, "agent-second"));
        let root = issued["root_base64"].as_str().unwrap().to_string();
        let text = std::fs::read_to_string(bench.path("agent-second.inclusion.json")).unwrap();
        let artifact = serde_json::from_str(&text).unwrap();
        Self {
            bench,
            artifact,
            root,
        }
    }

    fn leaf(&self) -> PathBuf {
        self.bench.path("agent-second.leaf")
    }

    /// Writes the artifact with `change` applied, and returns its path.
    fn tampered(&self, name: &str, change: impl FnOnce(&mut serde_json::Value)) -> PathBuf {
        let mut artifact = self.artifact.clone();
        change(&mut artifact);
        assert_ne!(
            artifact, self.artifact,
            "{name}: the change changed nothing"
        );
        let path = self.bench.path(&format!("{name}.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
        path
    }

    fn assert_script_refuses(&self, artifact: &Path, root: Option<&str>) {
        let refused = verify_inclusion(artifact, &self.leaf(), root);
        assert_eq!(
            refused.status.code(),
            Some(2),
            "{}: exit 2 is VERIFICATION FAILED\n{}",
            artifact.display(),
            String::from_utf8_lossy(&refused.stderr)
        );
    }

    fn assert_lys_refuses(&self, artifact: &Path) {
        let refused = run_lys(&[
            "log",
            "verify",
            "inclusion",
            "--artifact",
            path_str(artifact),
            "--leaf",
            path_str(&self.leaf()),
            "--verifier-key",
            &self.bench.verifier(),
        ]);
        assert_ne!(refused.status.code(), Some(0), "{}", artifact.display());
    }
}

/// Replaces the root line (the third) of the artifact's checkpoint.
fn replace_checkpoint_root(artifact: &mut serde_json::Value, root: &str) {
    let note = artifact["checkpoint"].as_str().unwrap();
    let mut lines: Vec<&str> = note.split('\n').collect();
    lines[2] = root;
    artifact["checkpoint"] = serde_json::Value::String(lines.join("\n"));
}

#[test]
fn the_untampered_artifact_passes_every_check_the_tampered_ones_fail() {
    let proven = Proven::new();
    let genuine = proven.bench.path("agent-second.inclusion.json");
    assert_success(&verify_inclusion(
        &genuine,
        &proven.leaf(),
        Some(&proven.root),
    ));
    let verifier = proven.bench.verifier();
    assert_success(&run_lys(&[
        "log",
        "verify",
        "inclusion",
        "--artifact",
        path_str(&genuine),
        "--leaf",
        path_str(&proven.leaf()),
        "--verifier-key",
        &verifier,
    ]));
}

#[test]
fn a_changed_leaf_index_fails_both_verifiers() {
    let proven = Proven::new();
    let artifact = proven.tampered("leaf-index", |a| a["leaf_index"] = 0.into());
    proven.assert_script_refuses(&artifact, Some(&proven.root));
    proven.assert_script_refuses(&artifact, None);
    proven.assert_lys_refuses(&artifact);
}

#[test]
fn a_changed_tree_size_fails_both_verifiers() {
    let proven = Proven::new();
    let artifact = proven.tampered("tree-size", |a| a["tree_size"] = 3.into());
    proven.assert_script_refuses(&artifact, Some(&proven.root));
    proven.assert_script_refuses(&artifact, None);
    proven.assert_lys_refuses(&artifact);
}

#[test]
fn a_changed_root_fails_both_verifiers() {
    let proven = Proven::new();
    let forged = STANDARD.encode([0x5a_u8; 32]);
    let artifact = proven.tampered("root", |a| replace_checkpoint_root(a, &forged));
    proven.assert_script_refuses(&artifact, Some(&proven.root));
    proven.assert_script_refuses(&artifact, None);
    proven.assert_lys_refuses(&artifact);
}

#[test]
fn a_changed_path_hash_fails_both_verifiers() {
    let proven = Proven::new();
    let forged = STANDARD.encode([0xa5_u8; 32]);
    let artifact = proven.tampered("hashes", |a| a["hashes"][0] = forged.into());
    proven.assert_script_refuses(&artifact, Some(&proven.root));
    proven.assert_lys_refuses(&artifact);
}

#[test]
fn a_genuine_artifact_under_a_root_the_stranger_does_not_trust_fails_the_script() {
    let proven = Proven::new();
    let genuine = proven.bench.path("agent-second.inclusion.json");
    let other = STANDARD.encode([0x33_u8; 32]);
    proven.assert_script_refuses(&genuine, Some(&other));
}
