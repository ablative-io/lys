//! The bench every `lys ca issue --log` test stands on, and the stranger's
//! tools: `openssl`, Python 3 and the committed `scripts/verify_inclusion.py`,
//! run offline under a network-denying wrapper with nothing else on `PATH`.

use std::os::unix::fs::symlink;
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

pub(crate) fn python() -> PathBuf {
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

/// `tool` as an absolute path: itself when it is one, otherwise the first
/// entry of this process's `PATH` that holds it.
fn absolute_tool(tool: &Path) -> PathBuf {
    if tool.is_absolute() {
        return tool.to_path_buf();
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join(tool))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{} is not on PATH", tool.display()))
}

/// The network-denying wrapper for this platform, as an absolute path and the
/// arguments that come before the wrapped command. A platform with neither
/// wrapper fails the test; it is never skipped.
fn network_denying_wrapper() -> (PathBuf, Vec<&'static str>) {
    if cfg!(target_os = "macos") {
        (
            PathBuf::from("/usr/bin/sandbox-exec"),
            vec!["-p", "(version 1)(allow default)(deny network*)"],
        )
    } else if cfg!(target_os = "linux") {
        let unshare = ["/usr/bin/unshare", "/bin/unshare"]
            .iter()
            .map(PathBuf::from)
            .find(|candidate| candidate.is_file())
            .unwrap_or_else(|| panic!("no unshare in /usr/bin or /bin to deny the network"));
        (unshare, vec!["--net", "--map-root-user"])
    } else {
        panic!("no network-denying wrapper on this platform: need sandbox-exec or unshare");
    }
}

/// The stranger's machine: a `PATH` of one directory holding only `openssl`
/// and `python3`, and a cleared environment. Nothing from lys is reachable.
pub(crate) struct Stranger {
    bin: tempfile::TempDir,
}

impl Stranger {
    pub(crate) fn new() -> Self {
        let bin = tempfile::tempdir().unwrap();
        let openssl_path = absolute_tool(&openssl());
        let python_path = absolute_tool(&python());
        symlink(openssl_path, bin.path().join("openssl")).unwrap();
        symlink(python_path, bin.path().join("python3")).unwrap();
        Self { bin }
    }

    /// The names the stranger's `PATH` directory holds, sorted.
    pub(crate) fn tools(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.bin.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// `tool args` in `dir` under the network-denying wrapper, with a cleared
    /// environment whose `PATH` is the stranger's one directory.
    pub(crate) fn offline(&self, dir: &Path, tool: &str, args: &[&str]) -> Output {
        let (wrapper, before) = network_denying_wrapper();
        let output = Command::new(&wrapper)
            .args(before)
            .arg(tool)
            .args(args)
            .env_clear()
            .env("PATH", self.bin.path())
            .current_dir(dir)
            .output()
            .expect("the network-denying wrapper did not start");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("sandbox_apply") && !stderr.contains("unshare failed"),
            "the wrapper {} failed to start: {stderr}",
            wrapper.display()
        );
        output
    }

    /// `tool args` in `dir` with the same environment and no wrapper.
    pub(crate) fn online(&self, dir: &Path, tool: &str, args: &[&str]) -> Output {
        Command::new(self.bin.path().join(tool))
            .args(args)
            .env_clear()
            .env("PATH", self.bin.path())
            .current_dir(dir)
            .output()
            .unwrap()
    }
}

/// A copy of the committed `scripts/verify_inclusion.py` in `dir`.
pub(crate) fn copy_verify_inclusion(dir: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify_inclusion.py");
    std::fs::copy(script, dir.join("verify_inclusion.py")).unwrap();
}

/// The line after which argument parsing lists the flags a run left out.
const REQUIRED: &str = "the following required arguments were not provided:";

/// The flags argument parsing lists under [`REQUIRED`], up to the next blank
/// line.
pub(crate) fn missing_required(output: &Output) -> Vec<String> {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr
        .lines()
        .skip_while(|line| !line.contains(REQUIRED))
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect()
}

/// Damages the first leaf of the log at `log_dir` in place: one byte of the
/// leaf's own bytes inside the first segment record is flipped, so the record
/// fails its CRC and the log fails its integrity check when it is opened
/// (LYSLOGSTORE-008 R1: a record is its u32 length, the leaf bytes, the pin
/// and a CRC, so byte 4 of the first segment is the first byte of leaf 0).
pub(crate) fn corrupt_first_leaf(log_dir: &Path) {
    let segment = log_dir
        .join("leaves")
        .join("segments")
        .join(format!("{:020}", 0));
    let mut bytes = std::fs::read(&segment).unwrap();
    assert!(bytes.len() > 4, "the first segment holds no leaf to damage");
    bytes[4] ^= 0x01;
    std::fs::write(&segment, bytes).unwrap();
}

/// Every file under `dir` with its bytes, so a test can say an act changed
/// nothing in a directory it may no longer be able to open as a store.
pub(crate) fn dir_bytes(dir: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                files.insert(path, bytes);
            }
        }
    }
    files
}

/// The number of leaves in the log at `log_dir`, read through the store
/// itself (its extent), never from the files under the directory.
pub(crate) fn leaf_files(log_dir: &Path) -> usize {
    use lys_log_store::LeafStore;
    let store = lys_log_store::FileLeafStore::open_read_only(log_dir).unwrap();
    usize::try_from(store.extent()).unwrap()
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

    /// The arguments of an issuance with the CA key only: `--log` and
    /// `--leaf-out`, no log key and no artifact. With `request`, the subject
    /// key comes from that certificate-signing request.
    pub(crate) fn issuer_only_args(
        &self,
        log_dir: &Path,
        subject: &str,
        request: Option<&Path>,
    ) -> Vec<String> {
        let at = |name: String| path_str(&self.path(&name)).to_string();
        let mut args: Vec<String> = [
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
            "--log",
            path_str(log_dir),
            "--leaf-out",
            at(format!("{subject}.leaf")).as_str(),
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        if let Some(request) = request {
            args.push("--request".to_string());
            args.push(path_str(request).to_string());
        }
        args
    }

    /// The log's operator, holding the log's key, makes the inclusion-proof
    /// artifact of the leaf at `leaf_index` at `out`.
    pub(crate) fn prove(&self, leaf_index: u64, out: &Path) -> Output {
        run_lys(&[
            "log",
            "prove",
            "inclusion",
            "--dir",
            path_str(&self.log_dir),
            "--key",
            path_str(&self.log_key),
            "--leaf-index",
            &leaf_index.to_string(),
            "--out",
            path_str(out),
        ])
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
