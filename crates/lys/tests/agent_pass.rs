#![cfg(test)]
//! AGENTS-006 R3 from the CLI's side: `lys agent pass` reads the grant
//! credential from an owner-only file, asks the installed server over
//! loopback carrying that credential and no operator token, writes the pass
//! to a file only its owner may read, and never prints the credential or
//! the pass. A refusal is shown by the server's own name; a missing or
//! shared credential file is refused before anything is sent.
//!
//! The server here is a loopback listener that answers each connection with
//! a fixed answer and keeps the request it read, so what the CLI sent is
//! asserted, not assumed.

use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};
use std::thread::JoinHandle;

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// What the fixture server's thread hands back: each request it read.
type Served = JoinHandle<Result<Vec<String>, String>>;

/// The grant credential every fixture holds; it must never be shown.
const CREDENTIAL: &str = "grant-credential-fixture-4f2a9c1e7b3d5a6c8e0f1a2b3c4d5e6";

/// The pass the fixture server answers; it must never be shown.
const PASS: &str = "eyJhbGciOiJFZERTQSJ9.pass-fixture-body.pass-fixture-signature";

/// The agent the fixture asks for.
const AGENT: &str = "agent-0123456789abcdef0123456789abcdef";

/// `lys` with `args`, the install at `root`.
fn lys(root: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_lys"))
        .env("LYS_IDENTITY_HOME", root)
        .args(args)
        .output()?)
}

/// Everything `output` printed, standard output then standard error.
fn printed(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A service install at `root` listening on `listen` with screens, so its
/// routes are under `/api`, keeping no operator token.
fn install(root: &Path, listen: &str) -> TestResult {
    let config = json!({
        "listen": listen,
        "surface_dir": root.join("surface").display().to_string(),
        "operator_token_file": null,
    });
    std::fs::write(root.join("identity.json"), config.to_string())?;
    Ok(())
}

/// The credential written owner-only at `root`, answering its path.
fn credential_file(root: &Path, mode: u32) -> Result<String, Box<dyn Error>> {
    let file = root.join("credential");
    std::fs::write(&file, format!("{CREDENTIAL}\n"))?;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode))?;
    Ok(file.display().to_string())
}

/// A loopback listener answering each of `answers` in turn, one connection
/// each, keeping every request it read.
fn serve(answers: Vec<(u16, Value)>) -> Result<(String, Served), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in answers {
            let (mut stream, peer) = listener.accept().map_err(|error| error.to_string())?;
            let request = read_request(&mut stream).map_err(|error| format!("{peer}: {error}"))?;
            requests.push(request);
            let body = body.to_string();
            let answer = format!(
                "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(answer.as_bytes())
                .map_err(|error| error.to_string())?;
        }
        Ok(requests)
    });
    Ok((address, handle))
}

/// One whole request: its head, then the body its length declares.
fn read_request(stream: &mut std::net::TcpStream) -> Result<String, Box<dyn Error>> {
    let mut raw = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(end) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8(raw[..end].to_vec())?;
            let length = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|value| value.trim().to_owned())
                })
                .ok_or("the request declares no length")?
                .parse::<usize>()?;
            if raw.len() >= end + 4 + length {
                return Ok(String::from_utf8(raw)?);
            }
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err("the request ended before it was whole".into());
        }
        raw.extend_from_slice(&chunk[..read]);
    }
}

/// The requests the fixture server read.
fn served(handle: Served) -> Result<Vec<String>, Box<dyn Error>> {
    let requests = handle
        .join()
        .map_err(|panic| format!("the fixture server panicked: {panic:?}"))??;
    Ok(requests)
}

/// `lys agent pass` for the fixture agent, with `extra` arguments first.
fn ask(
    root: &Path,
    extra: &[&str],
    credential: &str,
    out: &Path,
) -> Result<Output, Box<dyn Error>> {
    let out = out.display().to_string();
    let mut args = extra.to_vec();
    args.extend(["agent", "pass", "--agent", AGENT, "--audience", "notes"]);
    args.extend(["--credential-file", credential, "--out", out.as_str()]);
    lys(root, &args)
}

#[test]
fn agent_pass_help_names_its_flags() -> TestResult {
    let root = tempfile::tempdir()?;
    let output = lys(root.path(), &["agent", "pass", "--help"])?;
    assert!(output.status.success(), "{}", printed(&output));
    let help = String::from_utf8(output.stdout)?;
    for flag in [
        "--agent",
        "--audience",
        "--credential-file",
        "--out",
        "--server",
    ] {
        assert!(help.contains(flag), "{flag} is missing from:\n{help}");
    }
    Ok(())
}

#[test]
fn the_pass_is_written_owner_only_and_neither_secret_is_printed() -> TestResult {
    let root = tempfile::tempdir()?;
    let answer = json!({"pass": PASS, "expires_at": 1_900_000_000_u64, "audience": "notes"});
    let (address, handle) = serve(vec![(200, answer)])?;
    install(root.path(), &address)?;
    let credential = credential_file(root.path(), 0o600)?;
    let out = root.path().join("notes.pass");

    let output = ask(root.path(), &["--json"], &credential, &out)?;
    assert!(output.status.success(), "{}", printed(&output));
    let shown = printed(&output);
    assert!(!shown.contains(CREDENTIAL), "the credential was printed");
    assert!(!shown.contains(PASS), "the pass was printed");
    let answer: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(answer["audience"], "notes", "{answer}");
    assert_eq!(answer["expires_at"], 1_900_000_000_u64, "{answer}");

    assert_eq!(std::fs::read_to_string(&out)?, PASS);
    let mode = std::fs::metadata(&out)?.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "the pass file is {mode:o}");

    let requests = served(handle)?;
    let [request] = requests.as_slice() else {
        return Err(format!("one request: {requests:?}").into());
    };
    let lower = request.to_ascii_lowercase();
    assert!(
        request.starts_with(&format!("POST /api/agents/{AGENT}/pass ")),
        "{request}"
    );
    let header = format!("lys-grant-token: {CREDENTIAL}");
    assert!(lower.contains(&header), "{request}");
    assert!(!lower.contains("lys-operator"), "{request}");
    assert!(request.ends_with(r#"{"audience":"notes"}"#), "{request}");
    Ok(())
}

#[test]
fn the_servers_refusal_is_shown_by_its_name_and_nothing_is_written() -> TestResult {
    let root = tempfile::tempdir()?;
    let refusal = json!({"refusal": "agent_pass_unproven", "reason": "no"});
    let (address, handle) = serve(vec![(401, refusal)])?;
    install(root.path(), &address)?;
    let credential = credential_file(root.path(), 0o600)?;
    let out = root.path().join("notes.pass");

    let output = ask(root.path(), &[], &credential, &out)?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("agent_pass_unproven"), "{stderr}");
    assert!(!stderr.contains(CREDENTIAL), "{stderr}");
    assert!(!out.exists(), "a refused ask writes no pass");
    served(handle)?;
    Ok(())
}

#[test]
fn a_missing_or_shared_credential_file_is_refused_before_anything_is_sent() -> TestResult {
    let root = tempfile::tempdir()?;
    // Nothing listens here: a request sent would be refused unreachable.
    install(root.path(), "127.0.0.1:9")?;
    let out = root.path().join("notes.pass");
    let absent = root.path().join("absent").display().to_string();

    let output = ask(root.path(), &[], &absent, &out)?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("credential_file_absent"), "{stderr}");

    let shared = credential_file(root.path(), 0o644)?;
    let output = ask(root.path(), &[], &shared, &out)?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("chmod go-rwx"), "{stderr}");
    assert!(!stderr.contains(CREDENTIAL), "{stderr}");
    assert!(!stderr.contains("identity_server_unreachable"), "{stderr}");
    Ok(())
}
