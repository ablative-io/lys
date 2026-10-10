#![cfg(test)]
//! AGENTS-002 R1 and R6 from the CLI's side: `lys seat` parses and explains
//! itself, refuses by name when there is no install to reach or no operator
//! token to act with, reaches the installed server over loopback with the
//! `lys-operator` header, shows the server's refusal by its own name and
//! words, and never prints the operator token.
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

/// The operator token every fixture install holds; it must never be shown.
const TOKEN: &str = "operator-token-fixture-7c1d0e9f2a4b6c8d0e1f2a3b4c5d6e7f";

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

/// An install at `root` listening on `listen` with screens, so its routes
/// are under `/api`; with `token`, a development install keeping that
/// operator token owner-only, else a service install keeping none.
fn install(root: &Path, listen: &str, token: Option<&str>) -> TestResult {
    let token_file = match token {
        Some(token) => {
            let state = root.join("state");
            std::fs::create_dir_all(&state)?;
            std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700))?;
            let file = state.join("operator-token");
            std::fs::write(&file, token)?;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
            Value::String(file.display().to_string())
        }
        None => Value::Null,
    };
    let config = json!({
        "listen": listen,
        "surface_dir": root.join("surface").display().to_string(),
        "operator_token_file": token_file,
    });
    std::fs::write(root.join("identity.json"), config.to_string())?;
    Ok(())
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

#[test]
fn seat_help_names_every_subcommand() -> TestResult {
    let root = tempfile::tempdir()?;
    let output = lys(root.path(), &["seat", "--help"])?;
    assert!(output.status.success(), "{}", printed(&output));
    let help = String::from_utf8(output.stdout)?;
    for word in [
        "add", "list", "start", "stop", "restart", "send", "--server",
    ] {
        assert!(help.contains(word), "{word} is missing from:\n{help}");
    }
    Ok(())
}

#[test]
fn seat_add_help_names_its_flags() -> TestResult {
    let root = tempfile::tempdir()?;
    let output = lys(root.path(), &["seat", "add", "--help"])?;
    assert!(output.status.success(), "{}", printed(&output));
    let help = String::from_utf8(output.stdout)?;
    for flag in [
        "--agent",
        "--profile-version",
        "--machine",
        "--working-folder",
    ] {
        assert!(help.contains(flag), "{flag} is missing from:\n{help}");
    }
    Ok(())
}

#[test]
fn seat_send_without_text_and_add_without_its_flags_are_usage_errors() -> TestResult {
    let root = tempfile::tempdir()?;
    let send = lys(root.path(), &["seat", "send", "waffles"])?;
    assert_eq!(send.status.code(), Some(2), "{}", printed(&send));
    let add = lys(root.path(), &["seat", "add", "waffles", "--agent", "a"])?;
    assert_eq!(add.status.code(), Some(2), "{}", printed(&add));
    let stop = lys(root.path(), &["seat", "stop"])?;
    assert_eq!(stop.status.code(), Some(2), "{}", printed(&stop));
    Ok(())
}

#[test]
fn no_install_is_refused_by_name() -> TestResult {
    let root = tempfile::tempdir()?;
    let output = lys(root.path(), &["seat", "list"])?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("not_installed"), "{stderr}");

    let output = lys(root.path(), &["--json", "seat", "start", "waffles"])?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let answer: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(answer["ok"], false, "{answer}");
    assert!(
        answer["error"]
            .as_str()
            .is_some_and(|error| error.contains("not_installed")),
        "{answer}"
    );
    Ok(())
}

#[test]
fn a_service_install_has_no_operator_token_to_act_with() -> TestResult {
    let root = tempfile::tempdir()?;
    install(root.path(), "127.0.0.1:1", None)?;
    let output = lys(root.path(), &["seat", "list"])?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("operator_token_absent"), "{stderr}");
    Ok(())
}

#[test]
fn a_server_off_loopback_is_refused_before_anything_is_sent() -> TestResult {
    let root = tempfile::tempdir()?;
    install(root.path(), "127.0.0.1:1", Some(TOKEN))?;
    let output = lys(
        root.path(),
        &["seat", "list", "--server", "http://192.0.2.1:8490/api"],
    )?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let all = printed(&output);
    assert!(all.contains("server_not_loopback"), "{all}");
    assert!(
        !all.contains(TOKEN),
        "the operator token was printed:\n{all}"
    );
    Ok(())
}

#[test]
fn an_unreachable_server_is_named_and_the_token_is_never_printed() -> TestResult {
    let root = tempfile::tempdir()?;
    // A port bound and released: nothing answers on it.
    let address = TcpListener::bind("127.0.0.1:0")?.local_addr()?.to_string();
    install(root.path(), &address, Some(TOKEN))?;
    for args in [
        &["seat", "list"][..],
        &["--json", "seat", "send", "waffles", "hello"][..],
    ] {
        let output = lys(root.path(), args)?;
        assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
        let all = printed(&output);
        assert!(all.contains("identity_server_unreachable"), "{all}");
        assert!(
            !all.contains(TOKEN),
            "the operator token was printed:\n{all}"
        );
    }
    Ok(())
}

#[test]
fn a_refusal_is_shown_by_the_servers_name_and_the_operator_header_is_sent() -> TestResult {
    let root = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(
        409,
        json!({
            "refusal": "seat_not_running",
            "reason": "seat waffles has no running session",
            "fields": [],
        }),
    )])?;
    install(root.path(), &address, Some(TOKEN))?;
    let output = lys(root.path(), &["seat", "send", "waffles", "hello", "there"])?;
    let requests = served(handle)?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let all = printed(&output);
    assert!(
        all.contains("seat_not_running: seat waffles has no running session"),
        "{all}"
    );
    assert!(
        !all.contains(TOKEN),
        "the operator token was printed:\n{all}"
    );

    let [request] = requests.as_slice() else {
        return Err(format!("expected one request, read {requests:?}").into());
    };
    assert!(
        request.starts_with("POST /api/seats/waffles/send HTTP/1.1\r\n"),
        "{request}"
    );
    assert!(
        request.contains(&format!("lys-operator: {TOKEN}\r\n")),
        "{request}"
    );
    let body: Value = serde_json::from_str(
        request
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .ok_or("no body")?,
    )?;
    assert_eq!(body["text"], "hello there", "{body}");
    assert!(
        body["operation"]
            .as_str()
            .is_some_and(|operation| operation.starts_with("op-") && operation.len() == 35),
        "{body}"
    );
    Ok(())
}

#[test]
fn a_send_prints_one_json_object_under_json() -> TestResult {
    let root = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(
        200,
        json!({ "seat": "waffles", "session": "session-1", "delivered": true }),
    )])?;
    install(root.path(), &address, Some(TOKEN))?;
    let output = lys(root.path(), &["--json", "seat", "send", "waffles", "hello"])?;
    let requests = served(handle)?;
    assert!(output.status.success(), "{}", printed(&output));
    let answer: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(answer["delivered"], true, "{answer}");
    assert_eq!(answer["session"], "session-1", "{answer}");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert!(!printed(&output).contains(TOKEN));
    Ok(())
}

#[test]
fn the_list_names_an_unread_runner_rather_than_showing_nothing() -> TestResult {
    let root = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(
        200,
        json!({ "seats": [], "runner": "unknown", "reason": "the runner did not answer" }),
    )])?;
    install(root.path(), &address, Some(TOKEN))?;
    let output = lys(root.path(), &["seat", "list"])?;
    let requests = served(handle)?;
    assert!(output.status.success(), "{}", printed(&output));
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("runner: unknown: the runner did not answer"),
        "{stdout}"
    );
    let [request] = requests.as_slice() else {
        return Err(format!("expected one request, read {requests:?}").into());
    };
    assert!(
        request.starts_with("GET /api/seats HTTP/1.1\r\n"),
        "{request}"
    );
    Ok(())
}
