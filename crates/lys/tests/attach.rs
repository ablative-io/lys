#![cfg(test)]
//! AGENTS-002 R4 from the CLI's side: `lys attach` parses and explains
//! itself, refuses by name when there is no install to reach, follows a
//! seat's session page by page from the cursor each page gives, printing
//! assistant text as it is and every other line after its kind, follows a
//! PTY session read-only as its exact bytes, and never prints the operator
//! token.
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

/// The JSON body of `request`.
fn body_of(request: &str) -> Result<Value, Box<dyn Error>> {
    let body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .ok_or("the request has no body")?;
    Ok(serde_json::from_str(body)?)
}

#[test]
fn attach_help_names_the_seat_and_its_flags() -> TestResult {
    let root = tempfile::tempdir()?;
    let output = lys(root.path(), &["attach", "--help"])?;
    assert!(output.status.success(), "{}", printed(&output));
    let help = String::from_utf8(output.stdout)?;
    for word in ["[SEAT]", "--type", "--session", "--server", "detach"] {
        assert!(help.contains(word), "{word} is missing from:\n{help}");
    }
    Ok(())
}

#[test]
fn attach_needs_one_target_and_type_is_for_a_seat_only() -> TestResult {
    let root = tempfile::tempdir()?;
    for args in [
        &["attach"][..],
        &["attach", "waffles", "--session", "session-1"][..],
        &["attach", "--session", "session-1", "--type"][..],
    ] {
        let output = lys(root.path(), args)?;
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            printed(&output)
        );
    }
    Ok(())
}

#[test]
fn no_install_is_refused_by_name() -> TestResult {
    let root = tempfile::tempdir()?;
    let output = lys(root.path(), &["attach", "waffles"])?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("not_installed"), "{stderr}");
    Ok(())
}

#[test]
fn an_unreachable_server_is_named_and_the_token_is_never_printed() -> TestResult {
    let root = tempfile::tempdir()?;
    // A port bound and released: nothing answers on it.
    let address = TcpListener::bind("127.0.0.1:0")?.local_addr()?.to_string();
    install(root.path(), &address, Some(TOKEN))?;
    for args in [
        &["attach", "waffles"][..],
        &["--json", "attach", "--session", "session-1"][..],
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
fn a_seat_is_followed_from_each_pages_cursor_until_it_ends() -> TestResult {
    let root = tempfile::tempdir()?;
    let first = json!({
        "seat": "waffles",
        "session": "session-1",
        "lines": [
            { "at": 1, "kind": "user", "text": "hi" },
            { "at": 2, "kind": "assistant", "text": "hello there" },
        ],
        "cursor": 2,
        "ended": false,
    });
    let second = json!({
        "seat": "waffles",
        "session": "session-1",
        "lines": [{ "at": 3, "kind": "status", "text": "ended" }],
        "cursor": 3,
        "ended": true,
    });
    let (address, handle) = serve(vec![(200, first), (200, second)])?;
    install(root.path(), &address, Some(TOKEN))?;
    let output = lys(root.path(), &["attach", "waffles"])?;
    let requests = served(handle)?;
    assert!(output.status.success(), "{}", printed(&output));
    assert_eq!(
        String::from_utf8(output.stdout.clone())?,
        "user: hi\nhello there\nstatus: ended\n"
    );
    assert!(!printed(&output).contains(TOKEN));

    let [opening, next] = requests.as_slice() else {
        return Err(format!("expected two requests, read {requests:?}").into());
    };
    assert!(
        opening.starts_with("POST /api/seats/waffles/attach HTTP/1.1\r\n"),
        "{opening}"
    );
    assert!(
        opening.contains(&format!("lys-operator: {TOKEN}\r\n")),
        "{opening}"
    );
    let opening = body_of(opening)?;
    assert_eq!(opening["follow"], true, "{opening}");
    assert!(opening.get("cursor").is_none(), "{opening}");
    let next = body_of(next)?;
    assert_eq!(next["cursor"], 2, "{next}");
    Ok(())
}

#[test]
fn a_refused_attach_is_shown_by_the_servers_name() -> TestResult {
    let root = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(
        403,
        json!({
            "refusal": "seat_attach_refused",
            "reason": "only the seat's responsible person or an administrator may attach",
            "fields": [],
        }),
    )])?;
    install(root.path(), &address, Some(TOKEN))?;
    let output = lys(root.path(), &["attach", "waffles"])?;
    served(handle)?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("seat_attach_refused"), "{stderr}");
    Ok(())
}

#[test]
fn a_pty_session_is_followed_read_only_as_its_exact_bytes() -> TestResult {
    let root = tempfile::tempdir()?;
    let read = |from: u64, cursor: u64, data: &[u8], ended: Value| {
        json!({
            "session": "session-1",
            "answer": {
                "kind": "bytes",
                "output": {
                    "session": "session-1",
                    "from": from,
                    "cursor": cursor,
                    "oldest": 0,
                    "data": data,
                    "ended": ended,
                },
            },
            "receipt": {},
        })
    };
    let first = read(0, 9, b"\x1b[1mhello", Value::Null);
    let second = read(
        9,
        17,
        b" world\r\n",
        json!({ "how": "exited", "at": 5, "status": 0, "signal": null }),
    );
    let (address, handle) = serve(vec![(200, first), (200, second)])?;
    install(root.path(), &address, Some(TOKEN))?;
    let output = lys(root.path(), &["attach", "--session", "session-1"])?;
    let requests = served(handle)?;
    assert!(output.status.success(), "{}", printed(&output));
    assert_eq!(output.stdout, b"\x1b[1mhello world\r\n");
    assert!(!printed(&output).contains(TOKEN));

    let [opening, next] = requests.as_slice() else {
        return Err(format!("expected two requests, read {requests:?}").into());
    };
    assert!(
        opening.starts_with("POST /api/runtime/sessions/session-1/read-bytes HTTP/1.1\r\n"),
        "{opening}"
    );
    let opening = body_of(opening)?;
    assert_eq!(opening, json!({ "cursor": null, "follow": true }));
    let next = body_of(next)?;
    assert_eq!(next, json!({ "cursor": 9, "follow": true }));
    Ok(())
}
