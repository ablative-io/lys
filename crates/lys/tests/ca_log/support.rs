//! The bench every `lys ca issue --log` test stands on, and the stranger's
//! tools: `openssl`, Python 3 and the committed `scripts/verify_inclusion.py`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub(crate) fn run_lys<S: AsRef<std::ffi::OsStr>>(args: &[S]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lys"))
        .args(args)
        .output()
        .expect("failed to spawn lys binary")
}

pub(crate) fn path_str(path: &Path) -> &str {
    path.to_str().expect("tempdir path was not UTF-8")
}

/// Everything a command said, on either stream.
pub(crate) fn said(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

pub(crate) fn assert_success(output: &Output) {
    assert_eq!(output.status.code(), Some(0), "{}", said(output));
}

/// The JSON report a `--json` command printed.
pub(crate) fn report(output: &Output) -> serde_json::Value {
    assert_success(output);
    serde_json::from_slice(&output.stdout).unwrap()
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
        if said(&output).contains(expect) {
            return PathBuf::from(candidate);
        }
    }
    panic!("no {expect} found among {tried:?}; install it or name it in {env}");
}

pub(crate) fn openssl() -> PathBuf {
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

/// `openssl verify -CAfile <issuer> <cert>`, run where both files are.
pub(crate) fn openssl_verify(dir: &Path, issuer: &str, cert: &str) -> Output {
    Command::new(openssl())
        .args(["verify", "-CAfile", issuer, cert])
        .current_dir(dir)
        .output()
        .unwrap()
}

/// `scripts/verify_inclusion.py <artifact> <leaf> [expected-root-base64]`.
pub(crate) fn verify_inclusion(artifact: &Path, leaf: &Path, root: Option<&str>) -> Output {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_inclusion.py");
    let mut command = Command::new(python());
    command.arg(script).arg(artifact).arg(leaf);
    if let Some(root) = root {
        command.arg(root);
    }
    command.output().unwrap()
}

/// Decodes the one PEM block in `pem` to DER.
pub(crate) fn pem_to_der(pem: &str) -> Vec<u8> {
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    STANDARD.decode(body).unwrap()
}

/// Encodes DER as one PEM `CERTIFICATE` block.
pub(crate) fn der_to_pem(der: &[u8]) -> String {
    let body = STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in body.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(line).unwrap());
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}

/// Standard base64 of the 32 bytes a lowercase hex string spells.
pub(crate) fn hex_to_base64(hex: &str) -> String {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
        .collect();
    STANDARD.encode(bytes)
}

/// An issuer key, an initialized log with its operator key, and the paths an
/// issuance writes, all inside one temporary directory.
pub(crate) struct Bench {
    tmp: tempfile::TempDir,
    pub(crate) issuer_key: PathBuf,
    pub(crate) log_dir: PathBuf,
    pub(crate) log_key: PathBuf,
}

impl Bench {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn dir(&self) -> &Path {
        self.tmp.path()
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.tmp.path().join(name)
    }

    /// The arguments of a logged issuance for `subject` into `log_dir`, every
    /// output named after the subject beside the bench's other files.
    pub(crate) fn issue_args(&self, log_dir: &Path, subject: &str) -> Vec<String> {
        let at = |name: String| path_str(&self.path(&name)).to_string();
        [
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
            at(format!("{subject}.pem")).as_str(),
            "--issuer-out",
            at(format!("{subject}.issuer.pem")).as_str(),
            "--log",
            path_str(log_dir),
            "--log-key",
            path_str(&self.log_key),
            "--leaf-out",
            at(format!("{subject}.leaf")).as_str(),
            "--artifact-out",
            at(format!("{subject}.inclusion.json")).as_str(),
        ]
        .iter()
        .map(ToString::to_string)
        .collect()
    }

    pub(crate) fn issue_into(&self, log_dir: &Path, subject: &str) -> Output {
        run_lys(&self.issue_args(log_dir, subject))
    }

    /// The log's tree size, as `lys log status` reports it.
    pub(crate) fn log_size(&self) -> u64 {
        let status = run_lys(&["--json", "log", "status", "--dir", path_str(&self.log_dir)]);
        report(&status)["tree_size"].as_u64().unwrap()
    }

    /// The log's verifier key, as `lys log checkpoint` hands it out.
    pub(crate) fn verifier(&self) -> String {
        let note = self.path("checkpoint.note");
        let checkpoint = run_lys(&[
            "--json",
            "log",
            "checkpoint",
            "--dir",
            path_str(&self.log_dir),
            "--key",
            path_str(&self.log_key),
            "--out",
            path_str(&note),
        ]);
        std::fs::remove_file(&note).unwrap();
        report(&checkpoint)["verifier_key"]
            .as_str()
            .unwrap()
            .to_string()
    }
}

/// Replaces the value that follows `flag` in `args`.
pub(crate) fn with_flag(mut args: Vec<String>, flag: &str, value: &Path) -> Vec<String> {
    let at = args.iter().position(|arg| arg == flag).unwrap();
    args[at + 1] = path_str(value).to_string();
    args
}
