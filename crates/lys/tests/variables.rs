#![cfg(test)]
//! AGENTS-001 R2 from the CLI's side: `lys variables` reads the run pass
//! from the launch's mcp.json or the Codex variables, sends it as the
//! `lys-agent-pass` header to the run's own routes and never prints it;
//! `get` reads the agent map or, with --session, the session map; `set`
//! patches with the revision read, key=value as JSON when it parses and as
//! text otherwise, key= removing; and a run with no pass is refused by name.
//!
//! The server here is a loopback listener that answers each connection with
//! a fixed answer and keeps the request it read, so what the CLI sent is
//! asserted, not assumed.

use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::thread::JoinHandle;

use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

type Served = JoinHandle<Result<Vec<String>, String>>;

/// The run pass every fixture launch holds; it must never be shown.
const PASS: &str = "run-pass-fixture-0123456789abcdef0123456789";

fn lys(folder: &Path, args: &[&str], env: &[(&str, &str)]) -> Result<Output, Box<dyn Error>> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lys"));
    command
        .env_remove("LYS_AGENT_PASS")
        .env_remove("LYS_MCP_URL")
        .env_remove("LYS_SEAT")
        .current_dir(folder)
        .args(args);
    for (name, value) in env {
        command.env(name, value);
    }
    Ok(command.output()?)
}

fn printed(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A launch folder whose mcp.json carries the lys entry at `address`.
fn launch(folder: &Path, address: &str) -> TestResult {
    let config = json!({
        "mcpServers": {
            "lys": {
                "type": "http",
                "url": format!("http://{address}/api/mcp"),
                "headers": { "lys-agent-pass": PASS, "lys-seat": "seat-fixture" },
            }
        }
    });
    std::fs::write(folder.join("mcp.json"), config.to_string())?;
    Ok(())
}

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

fn served(handle: Served) -> Result<Vec<String>, Box<dyn Error>> {
    Ok(handle
        .join()
        .map_err(|panic| format!("the fixture server panicked: {panic:?}"))??)
}

fn body_of(request: &str) -> Result<Value, Box<dyn Error>> {
    let (_, body) = request.split_once("\r\n\r\n").ok_or("no body")?;
    Ok(serde_json::from_str(body)?)
}

fn read_answer(session: bool) -> Value {
    json!({
        "scope": { "kind": if session { "session" } else { "agent" }, "id": "x" },
        "revision": 2,
        "values": { "focus": { "value": "the door", "revision": 2, "author": "agent-x", "at": 1 } },
        "expired": ["lantern"],
    })
}

#[test]
fn get_sends_the_pass_to_the_runs_own_route_and_never_prints_it() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(200, read_answer(false))])?;
    launch(folder.path(), &address)?;
    let output = lys(folder.path(), &["variables", "get"], &[])?;
    let shown = printed(&output);
    assert!(output.status.success(), "{shown}");
    assert!(shown.contains("agent x at revision 2"), "{shown}");
    assert!(shown.contains("focus = \"the door\" (revision 2, by agent-x)"), "{shown}");
    assert!(shown.contains("expired: lantern"), "{shown}");
    assert!(!shown.contains(PASS), "the pass was printed: {shown}");
    let requests = served(handle)?;
    let head = requests[0].lines().next().unwrap_or_default().to_owned();
    assert_eq!(head, "GET /api/me/variables HTTP/1.1");
    assert!(requests[0].contains(&format!("lys-agent-pass: {PASS}")), "{}", requests[0]);
    assert!(requests[0].contains("lys-seat: seat-fixture"), "{}", requests[0]);
    Ok(())
}

#[test]
fn set_patches_with_the_revision_read_and_removes_with_an_empty_value() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(200, read_answer(true))])?;
    launch(folder.path(), &address)?;
    let output = lys(
        folder.path(),
        &[
            "--json",
            "variables",
            "set",
            "--session",
            "--revision",
            "1",
            "--expires-at",
            "1800000000",
            "focus=the door",
            "steps=[1,2]",
            "count=3",
            "old=",
        ],
        &[],
    )?;
    let shown = printed(&output);
    assert!(output.status.success(), "{shown}");
    let answer: Value = serde_json::from_str(shown.trim())?;
    assert_eq!(answer["scope"]["kind"], "session");
    let requests = served(handle)?;
    assert!(requests[0].starts_with("POST /api/me/session/variables HTTP/1.1"), "{}", requests[0]);
    let body = body_of(&requests[0])?;
    assert_eq!(body["revision"], 1);
    assert_eq!(body["expires_at"], 1_800_000_000_u64);
    assert_eq!(body["values"]["focus"], "the door");
    assert_eq!(body["values"]["steps"], json!([1, 2]));
    assert_eq!(body["values"]["count"], 3);
    assert_eq!(body["values"]["old"], Value::Null);
    Ok(())
}

#[test]
fn a_refusal_is_shown_by_its_name_and_the_codex_variables_carry_the_pass() -> TestResult {
    let folder = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(
        409,
        json!({ "refusal": "variables_stale", "reason": "agent x is at revision 3, not 1" }),
    )])?;
    let output = lys(
        folder.path(),
        &["variables", "set", "--revision", "1", "focus=late"],
        &[
            ("LYS_AGENT_PASS", PASS),
            ("LYS_MCP_URL", &format!("http://{address}/api/mcp")),
        ],
    )?;
    let shown = printed(&output);
    assert!(!output.status.success(), "{shown}");
    assert!(shown.contains("variables_stale"), "{shown}");
    assert!(shown.contains("revision 3, not 1"), "{shown}");
    assert!(!shown.contains(PASS), "the pass was printed: {shown}");
    let requests = served(handle)?;
    assert!(requests[0].contains(&format!("lys-agent-pass: {PASS}")), "{}", requests[0]);
    Ok(())
}

#[test]
fn a_run_with_no_pass_is_refused_by_name_and_sends_nothing() -> TestResult {
    let folder = tempfile::tempdir()?;
    let output = lys(folder.path(), &["variables", "get"], &[])?;
    let shown = printed(&output);
    assert!(!output.status.success(), "{shown}");
    assert!(shown.contains("run_pass_absent"), "{shown}");
    Ok(())
}
