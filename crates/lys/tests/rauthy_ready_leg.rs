#![cfg(test)]
//! The rauthy-ready demand leg, `scripts/identity-gates/rauthy_ready.py`, run as a
//! child process against a stub container runtime.
//!
//! The stub `docker` this test writes records every argv it receives and, for every
//! `--env-file`, the file's contents, so the record holds what the runtime was
//! actually given. The secrets searched for in the leg's output are read from that
//! record, never from a value this test or the script substitutes, and the search
//! is proven to fire by running it over the record itself. The health endpoint is a
//! `TcpListener` this test owns on 127.0.0.1, which counts the requests it answers.
//! No real container, daemon or port other than that listener is touched, and
//! nothing under `deploy/identity/` is written.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::hash::{BuildHasher, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use serde_json::{Value, json};

const SCRIPT: &str = "scripts/identity-gates/rauthy_ready.py";
const REVISION_LABEL: &str = "org.opencontainers.image.revision";
const NAME_PREFIX: &str = "lys-rauthy-ready-";
const BUILD_PREFIX: &str = "docker buildx build --label org.opencontainers.image.revision=";
const DIFFERENT_REVISION: &str = "0000000000000000000000000000000000000000";
const NOT_FOUND: &str = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// The env-file keys whose values are the leg's five secrets; the database
/// password is handed over twice, as `POSTGRES_PASSWORD` and `PG_PASSWORD`.
const SECRET_KEYS: [&str; 6] = [
    "POSTGRES_PASSWORD",
    "PG_PASSWORD",
    "ENC_KEYS",
    "BOOTSTRAP_ADMIN_PASSWORD_PLAIN",
    "HQL_SECRET_RAFT",
    "HQL_SECRET_API",
];

/// The stub runtime's body. It reads its answers from `stub.json` beside it.
const STUB_BODY: &str = r#"
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(HERE, "stub.json"), encoding="utf-8") as handle:
    cfg = json.load(handle)
argv = sys.argv[1:]
with open(cfg["record"], "a", encoding="utf-8") as record:
    record.write(json.dumps({"argv": argv}) + "\n")
    for i, arg in enumerate(argv[:-1]):
        if arg == "--env-file":
            with open(argv[i + 1], encoding="utf-8") as env:
                record.write(json.dumps({"env_file": env.read()}) + "\n")

def answer(code, text=None):
    if text is not None:
        sys.stdout.write(text + "\n")
    sys.exit(code)

first, head = argv[:1], argv[:2]
if first == ["version"]:
    answer(0, "stub")
if head == ["image", "inspect"]:
    target = argv[-1]
    fmt = argv[argv.index("--format") + 1] if "--format" in argv else ""
    if target == cfg["rauthy_image_id"]:
        if not cfg["rauthy_present"]:
            answer(1)
        if "Labels" in fmt:
            answer(0, json.dumps(cfg["labels"]))
        answer(0, target)
    if target == cfg["postgres_image"]:
        answer(0 if cfg["postgres_present"] else 1)
    answer(1)
if head == ["network", "create"]:
    answer(0, "stub-network")
if head == ["network", "rm"]:
    answer(0)
if head == ["container", "inspect"]:
    answer(1)
if first == ["run"]:
    answer(0, "stub-container")
if first == ["exec"]:
    answer(0)
if first == ["port"]:
    answer(0, "127.0.0.1:%d" % cfg["port"])
if first in (["stop"], ["rm"]):
    answer(0)
answer(2)
"#;

/// The revision label the stub reports for the Rauthy image.
enum Revision {
    /// The pin the test read with git.
    Pin,
    /// A commit that is not the pin.
    Different,
    /// No revision label at all.
    Absent,
}

/// What the stub and the listener answer for the case under test.
struct Stub {
    rauthy_present: bool,
    postgres_present: bool,
    revision: Revision,
    health_status: u16,
}

impl Stub {
    fn ready() -> Self {
        Stub {
            rauthy_present: true,
            postgres_present: true,
            revision: Revision::Pin,
            health_status: 200,
        }
    }
}

/// One case: its own temporary directory, versions file, stub and record.
struct Case {
    dir: tempfile::TempDir,
    root: PathBuf,
    python: PathBuf,
    git: PathBuf,
    pin: String,
    rauthy_image_id: String,
    postgres_image: String,
    versions: PathBuf,
    record: PathBuf,
    requests: Arc<AtomicUsize>,
}

/// The leg's result together with every runtime call the stub recorded.
struct Run {
    code: Option<i32>,
    requests: usize,
    stdout: String,
    stderr: String,
    argvs: Vec<Vec<String>>,
    env_text: String,
}

fn find_on_path(name: &str) -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let found = std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file());
    let Some(found) = found else {
        panic!("{name} is absent from the test's environment; the test does not skip");
    };
    found
}

/// The real interpreter behind `python3`, so a shim needs nothing else on PATH.
fn python_interpreter() -> PathBuf {
    let python3 = find_on_path("python3");
    let output = Command::new(&python3)
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .expect("python3 could not be run");
    assert!(output.status.success(), "python3 named no interpreter");
    PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
}

fn made_hex64() -> String {
    let state = std::collections::hash_map::RandomState::new();
    let mut hex = String::new();
    for word in 0..4_u64 {
        let mut hasher = state.build_hasher();
        hasher.write_u64(word);
        let value = hasher.finish();
        hex.push_str(&format!("{value:016x}"));
    }
    hex
}

fn serve_health(status: u16) -> (u16, Arc<AtomicUsize>) {
    let bound = TcpListener::bind("127.0.0.1:0");
    let listener = bound.expect("bind the health listener");
    let port = listener.local_addr().expect("listener address").port();
    let requests = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&requests);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            answer_health(stream, status, &counted);
        }
    });
    (port, requests)
}

fn answer_health(mut stream: TcpStream, status: u16, counted: &AtomicUsize) {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !request.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => request.extend_from_slice(&buffer[..read]),
        }
    }
    let head = String::from_utf8_lossy(&request);
    let response = if head.starts_with("GET /auth/v1/health ") {
        counted.fetch_add(1, Ordering::SeqCst);
        let (reason, body) = if status == 200 {
            ("OK", r#"{"db_healthy":true,"cache_healthy":true}"#)
        } else {
            ("Error", r#"{"db_healthy":false,"cache_healthy":false}"#)
        };
        let length = body.len();
        format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\n\
             Content-Length: {length}\r\nConnection: close\r\n\r\n{body}"
        )
    } else {
        NOT_FOUND.to_owned()
    };
    if let Err(error) = stream.write_all(response.as_bytes()) {
        eprintln!("the health listener could not answer: {error}");
    }
}

fn git_output(git: &Path, root: &Path, args: &[&str]) -> String {
    let output = Command::new(git)
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

impl Case {
    fn new(stub: &Stub) -> Self {
        let python = python_interpreter();
        let git = find_on_path("git");
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("repository root");
        let pin = git_output(&git, &root, &["rev-parse", "HEAD:vendor/rauthy"]);
        let pin = pin.trim().to_owned();

        let dir = tempfile::tempdir().expect("temporary directory");
        let rauthy_image_id = format!("sha256:{}", made_hex64());
        let postgres_image = format!("registry.invalid/postgres@sha256:{}", made_hex64());
        let versions = dir.path().join("versions.json");
        let versions_json = json!({
            "rauthy_image_id": rauthy_image_id,
            "postgres_image": postgres_image,
        });
        let written = fs::write(&versions, versions_json.to_string());
        written.expect("write the versions file");

        let revision = match stub.revision {
            Revision::Pin => Some(pin.as_str()),
            Revision::Different => Some(DIFFERENT_REVISION),
            Revision::Absent => None,
        };
        let mut labels = Value::Null;
        if let Some(value) = revision {
            let mut map = serde_json::Map::new();
            map.insert(REVISION_LABEL.to_owned(), json!(value));
            labels = Value::Object(map);
        }
        let (port, requests) = serve_health(stub.health_status);
        let record = dir.path().join("record.jsonl");
        let stub_dir = dir.path().join("stub");
        fs::create_dir(&stub_dir).expect("stub directory");
        let config = json!({
            "record": record,
            "rauthy_image_id": rauthy_image_id,
            "postgres_image": postgres_image,
            "rauthy_present": stub.rauthy_present,
            "postgres_present": stub.postgres_present,
            "labels": labels,
            "port": port,
        });
        let written = fs::write(stub_dir.join("stub.json"), config.to_string());
        written.expect("write stub.json");
        let docker = stub_dir.join("docker");
        let script = format!("#!{}\n{STUB_BODY}", python.display());
        fs::write(&docker, script).expect("write the stub runtime");
        let mode = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&docker, mode).expect("stub mode");

        Case {
            dir,
            root,
            python,
            git,
            pin,
            rauthy_image_id,
            postgres_image,
            versions,
            record,
            requests,
        }
    }

    /// PATH starting with the stub runtime, then the environment's own.
    fn stub_path(&self) -> OsString {
        let inherited = std::env::var_os("PATH").unwrap_or_default();
        let mut dirs = vec![self.dir.path().join("stub")];
        dirs.extend(std::env::split_paths(&inherited));
        std::env::join_paths(dirs).expect("join PATH")
    }

    /// PATH holding only python3 and git, and so no docker at all.
    fn bare_path(&self) -> OsString {
        let bare = self.dir.path().join("bare");
        fs::create_dir(&bare).expect("bare PATH directory");
        let python = bare.join("python3");
        symlink(&self.python, python).expect("link python3");
        symlink(&self.git, bare.join("git")).expect("link git");
        OsString::from(bare)
    }

    fn deploy_identity_status(&self) -> String {
        let args = ["status", "--porcelain", "--", "deploy/identity"];
        git_output(&self.git, &self.root, &args)
    }

    fn run(&self, path: &OsStr) -> Run {
        let before = self.deploy_identity_status();
        let output = Command::new(&self.python)
            .arg(SCRIPT)
            .arg("--versions-file")
            .arg(&self.versions)
            .current_dir(&self.root)
            .env("PATH", path)
            .output()
            .expect("run the rauthy-ready leg");
        let after = self.deploy_identity_status();
        assert_eq!(before, after, "the leg changed deploy/identity");
        let (argvs, env_text) = read_record(&self.record);
        Run {
            code: output.status.code(),
            requests: self.requests.load(Ordering::SeqCst),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            argvs,
            env_text,
        }
    }
}

fn read_record(record: &Path) -> (Vec<Vec<String>>, String) {
    let mut argvs = Vec::new();
    let mut env_text = String::new();
    if !record.exists() {
        return (argvs, env_text);
    }
    let text = fs::read_to_string(record).expect("read the stub record");
    for line in text.lines() {
        let entry: Value = serde_json::from_str(line).expect("a record line is JSON");
        if let Some(list) = entry.get("argv").and_then(Value::as_array) {
            let words = list.iter().filter_map(Value::as_str).map(str::to_owned);
            argvs.push(words.collect());
        }
        if let Some(env) = entry.get("env_file").and_then(Value::as_str) {
            env_text.push_str(env);
        }
    }
    (argvs, env_text)
}

fn first_is(argv: &[String], word: &str) -> bool {
    argv.first().is_some_and(|first| first == word)
}

impl Run {
    fn refusal(&self, name: &str) -> &str {
        let prefix = format!("refused: {name}");
        let found = self.stderr.lines().find(|line| line.starts_with(&prefix));
        let Some(line) = found else {
            panic!("no stderr line begins {prefix:?}: {:?}", self.stderr);
        };
        line
    }

    fn ready_lines(&self) -> usize {
        let lines = self.stdout.lines();
        lines.filter(|line| line.starts_with("ready:")).count()
    }

    fn run_argvs(&self) -> Vec<&Vec<String>> {
        let calls = self.argvs.iter();
        calls.filter(|call| first_is(call, "run")).collect()
    }

    /// The distinct secret values the runtime received through its env files.
    fn secrets(&self) -> BTreeSet<String> {
        let lines = self.env_text.lines();
        let pairs = lines.filter_map(|line| line.split_once('='));
        let secret = pairs.filter(|(key, _)| SECRET_KEYS.contains(key));
        secret.map(|(_, value)| value.to_owned()).collect()
    }

    fn refused_early(&self, name: &str) -> &str {
        assert_eq!(self.code, Some(1), "stderr: {}", self.stderr);
        assert_eq!(self.ready_lines(), 0);
        assert!(self.run_argvs().is_empty(), "a container was run");
        self.refusal(name)
    }

    fn assert_leaks_none(&self) {
        let secrets = self.secrets();
        assert_eq!(secrets.len(), 5, "5 distinct secret values");
        let output = format!("{}{}", self.stdout, self.stderr);
        for value in &secrets {
            assert_eq!(occurrences(&output, value), 0, "a secret leaked");
        }
    }
}

/// Occurrences of a value in text, both as written and as its lowercase hexadecimal.
fn occurrences(text: &str, value: &str) -> usize {
    let mut hex = String::new();
    for byte in value.bytes() {
        hex.push_str(&format!("{byte:02x}"));
    }
    text.matches(value).count() + text.matches(hex.as_str()).count()
}

/// Every name created by `run --name` or `network create`, and every name removed
/// by `rm` or `network rm`, each with the index of its argv in the record.
fn created_and_removed(argvs: &[Vec<String>]) -> (Vec<(usize, String)>, Vec<(usize, String)>) {
    let mut created = Vec::new();
    let mut removed = Vec::new();
    for (index, call) in argvs.iter().enumerate() {
        let words: Vec<&str> = call.iter().map(String::as_str).collect();
        let last = call.last().cloned().unwrap_or_default();
        match words.as_slice() {
            ["run", ..] => {
                let named = words.windows(2).find(|pair| pair[0] == "--name");
                let name = named.expect("docker run carries --name")[1];
                created.push((index, name.to_owned()));
            }
            ["network", "create", ..] => created.push((index, last)),
            ["rm", ..] | ["network", "rm", ..] => removed.push((index, last)),
            _ => {}
        }
    }
    (created, removed)
}

fn assert_every_creation_removed(argvs: &[Vec<String>]) {
    let (created, removed) = created_and_removed(argvs);
    assert_eq!(created.len(), 3, "a network and two containers");
    for (made_at, name) in &created {
        let later = removed.iter().any(|(i, n)| n == name && i > made_at);
        assert!(later, "{name} was created and not removed afterwards");
    }
    assert_eq!(created.len(), removed.len(), "created != removed");
    for (_, name) in &removed {
        let ours = name.starts_with(NAME_PREFIX);
        assert!(ours, "removed {name}, not a name the leg made");
    }
}

fn ready_run() -> Run {
    let case = Case::new(&Stub::ready());
    let run = case.run(&case.stub_path());
    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    run
}

#[test]
fn ready() {
    let run = ready_run();
    assert_eq!(run.ready_lines(), 1, "stdout: {}", run.stdout);
    assert!(run.requests >= 1, "the listener counted no request");
}

#[test]
fn ready_secrets() {
    let run = ready_run();
    let secrets = run.secrets();
    assert_eq!(secrets.len(), 5, "5 distinct secret values");
    // Search control: the same search over the env files the runtime received
    // finds every value, so a zero below is a search that fired and found none.
    for value in &secrets {
        let found = occurrences(&run.env_text, value);
        assert!(found >= 1, "the search did not fire");
    }
    run.assert_leaks_none();
}

#[test]
fn ready_cleanup() {
    let run = ready_run();
    assert_every_creation_removed(&run.argvs);
}

#[test]
fn ready_no_pull() {
    let run = ready_run();
    let pulls = run.argvs.iter().filter(|call| first_is(call, "pull"));
    assert_eq!(pulls.count(), 0);
    let runs = run.run_argvs();
    assert_eq!(runs.len(), 2);
    for argv in runs {
        let never = argv.windows(2).any(|pair| pair == ["--pull", "never"]);
        assert!(never, "docker run without --pull never: {argv:?}");
    }
}

#[test]
fn no_runtime() {
    let case = Case::new(&Stub::ready());
    let run = case.run(&case.bare_path());
    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    run.refusal("container_runtime_missing");
    assert_eq!(run.ready_lines(), 0);
}

#[test]
fn no_rauthy_image() {
    let mut stub = Stub::ready();
    stub.rauthy_present = false;
    let case = Case::new(&stub);
    let run = case.run(&case.stub_path());
    let line = run.refused_early("image_missing");
    assert!(line.contains("rauthy"), "{line}");
    assert!(line.contains(&case.rauthy_image_id), "{line}");
    let build = format!("{BUILD_PREFIX}{} --load .", case.pin);
    assert!(line.contains(&build), "{line}");
    assert_eq!(line.matches("pull").count(), 0, "{line}");
}

#[test]
fn no_postgres_image() {
    let mut stub = Stub::ready();
    stub.postgres_present = false;
    let case = Case::new(&stub);
    let run = case.run(&case.stub_path());
    let line = run.refused_early("image_missing");
    assert!(line.contains("postgres"), "{line}");
    assert!(line.contains(&case.postgres_image), "{line}");
    assert!(line.contains("pull"), "{line}");
}

#[test]
fn revision_differs() {
    let mut stub = Stub::ready();
    stub.revision = Revision::Different;
    let case = Case::new(&stub);
    let run = case.run(&case.stub_path());
    let line = run.refused_early("rauthy_revision_mismatch");
    assert!(line.contains(DIFFERENT_REVISION), "{line}");
    assert!(line.contains(&case.pin), "{line}");
}

#[test]
fn revision_absent() {
    let mut stub = Stub::ready();
    stub.revision = Revision::Absent;
    let case = Case::new(&stub);
    let run = case.run(&case.stub_path());
    let line = run.refused_early("rauthy_revision_mismatch");
    assert!(line.contains("absent"), "{line}");
    assert!(line.contains(&case.pin), "{line}");
    assert!(line.contains(BUILD_PREFIX), "{line}");
}

#[test]
fn never_ready() {
    let mut stub = Stub::ready();
    stub.health_status = 500;
    let case = Case::new(&stub);
    let run = case.run(&case.stub_path());
    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    assert_eq!(run.ready_lines(), 0);
    let line = run.refusal("readiness_timeout");
    assert!(line.contains("rauthy"), "{line}");
    assert!(run.requests >= 1, "the listener counted no request");
    assert_every_creation_removed(&run.argvs);
    run.assert_leaks_none();
}
