#![cfg(test)]
//! AGENTS-003 R4 from the CLI's side: `lys seat import dry-run|confirm|status`
//! parses and explains itself, refuses a bulk seat name and an unreadable
//! or malformed manifest by name before anything is sent, sends the
//! manifest the person named, the exact plan id and revision, and a fresh
//! operation id over loopback with the `lys-operator` header, shows the
//! server's refusal by its own name and words, and never prints the
//! operator token.
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
const TOKEN: &str = "operator-token-fixture-1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e";

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

/// A development install at `root` listening on `listen` with screens, so
/// its routes are under `/api`, keeping the operator token owner-only.
fn install(root: &Path, listen: &str) -> TestResult {
    let state = root.join("state");
    std::fs::create_dir_all(&state)?;
    std::fs::set_permissions(&state, std::fs::Permissions::from_mode(0o700))?;
    let file = state.join("operator-token");
    std::fs::write(&file, TOKEN)?;
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
    let config = json!({
        "listen": listen,
        "surface_dir": root.join("surface").display().to_string(),
        "operator_token_file": file.display().to_string(),
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
                .unwrap_or_else(|| "0".to_owned())
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

/// The one request the fixture server read.
fn one_request(handle: Served) -> Result<String, Box<dyn Error>> {
    let requests = handle
        .join()
        .map_err(|panic| format!("the fixture server panicked: {panic:?}"))??;
    let [request] = requests.as_slice() else {
        return Err(format!("expected one request, read {requests:?}").into());
    };
    Ok(request.clone())
}

/// The JSON body of `request`.
fn body(request: &str) -> Result<Value, Box<dyn Error>> {
    let (_, body) = request.split_once("\r\n\r\n").ok_or("no body")?;
    Ok(serde_json::from_str(body)?)
}

/// A port bound and released: nothing answers on it.
fn silent() -> Result<String, Box<dyn Error>> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.to_string())
}

#[test]
fn seat_import_help_names_every_subcommand_and_flag() -> TestResult {
    let root = tempfile::tempdir()?;
    let output = lys(root.path(), &["seat", "import", "--help"])?;
    assert!(output.status.success(), "{}", printed(&output));
    let help = String::from_utf8(output.stdout)?;
    for word in ["dry-run", "confirm", "status"] {
        assert!(help.contains(word), "{word} is missing from:\n{help}");
    }
    let output = lys(root.path(), &["seat", "import", "dry-run", "--help"])?;
    assert!(String::from_utf8(output.stdout)?.contains("--manifest"));
    let output = lys(root.path(), &["seat", "import", "confirm", "--help"])?;
    let help = String::from_utf8(output.stdout)?;
    assert!(help.contains("--plan") && help.contains("--revision"), "{help}");
    Ok(())
}

#[test]
fn seat_import_without_its_flags_is_a_usage_error() -> TestResult {
    let root = tempfile::tempdir()?;
    for args in [
        &["seat", "import", "dry-run", "waffles"][..],
        &["seat", "import", "confirm", "waffles", "--plan", "plan-1"][..],
        &["seat", "import", "status"][..],
    ] {
        let output = lys(root.path(), args)?;
        assert_eq!(output.status.code(), Some(2), "{args:?}: {}", printed(&output));
    }
    Ok(())
}

#[test]
fn a_bulk_name_and_a_bad_manifest_are_refused_before_anything_is_sent() -> TestResult {
    let root = tempfile::tempdir()?;
    install(root.path(), &silent()?)?;
    let malformed = root.path().join("manifest.json");
    std::fs::write(&malformed, "{ not json")?;
    let absent = root.path().join("absent.json");
    let malformed = malformed.display().to_string();
    let absent = absent.display().to_string();
    for (args, refusal) in [
        (
            vec!["seat", "import", "confirm", "*", "--plan", "p", "--revision", "r"],
            "import_bulk_refused",
        ),
        (vec!["seat", "import", "status", "waffles,gaia"], "import_bulk_refused"),
        (
            vec!["seat", "import", "dry-run", "waffles", "--manifest", absent.as_str()],
            "import_manifest_unreadable",
        ),
        (
            vec!["seat", "import", "dry-run", "waffles", "--manifest", malformed.as_str()],
            "import_manifest_malformed",
        ),
    ] {
        let output = lys(root.path(), &args)?;
        assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
        let all = printed(&output);
        assert!(all.contains(refusal), "{args:?}: {all}");
        assert!(!all.contains("identity_server_unreachable"), "{args:?}: {all}");
        assert!(!all.contains(TOKEN), "the operator token was printed:\n{all}");
    }
    Ok(())
}

#[test]
fn a_dry_run_sends_the_named_manifest_and_shows_the_whole_plan() -> TestResult {
    let root = tempfile::tempdir()?;
    let plan = json!({
        "version": "lys-seat-import-plan/v1", "plan_id": "plan-0123", "plan_revision": "ab".repeat(32),
        "captured_at": 1_700_000_000, "seat": "waffles", "agent": "agent-1", "responsible": "person-1",
        "harness": "claude",
        "sources": [{ "id": "s", "kind": "claude_settings", "locator": "/seats/waffles/settings.json",
                      "scope": "waffles", "revision_kind": "content_sha256", "source_revision": "cd",
                      "completeness": { "state": "complete" } }],
        "destinations": [{ "record_kind": "words_slot", "record_id": "agent:agent-1/wake_up",
                           "expected_revision": 0, "change": {}, "source_entry_ids": ["s"] }],
        "references": [], "replacements": ["Argus hooks become Lys hooks"],
        "schedule_counts": { "total": 7, "live": 3, "expired": 2, "finished": 2, "not_imported": 4 },
        "excluded": [], "prerequisites": [], "refusals": [], "bounds": {},
    });
    let (address, handle) = serve(vec![(200, plan)])?;
    install(root.path(), &address)?;
    let manifest = root.path().join("waffles.json");
    let declared = json!({
        "seat": "waffles", "harness": "claude",
        "claude_folder": "/Users/tom/Developer/seats/ablative/waffles",
    });
    std::fs::write(&manifest, declared.to_string())?;
    let path = manifest.display().to_string();
    let output = lys(
        root.path(),
        &["seat", "import", "dry-run", "waffles", "--manifest", path.as_str()],
    )?;
    let request = one_request(handle)?;
    assert!(output.status.success(), "{}", printed(&output));
    let stdout = String::from_utf8(output.stdout)?;
    for line in [
        "plan plan-0123 of seat waffles",
        "writes words_slot agent:agent-1/wake_up  at revision 0",
        "replaces: Argus hooks become Lys hooks",
        "schedules: total 7 live 3 expired 2 finished 2 not imported 4",
        "confirm with: lys seat import confirm waffles --plan plan-0123 --revision",
    ] {
        assert!(stdout.contains(line), "{line} is missing from:\n{stdout}");
    }
    assert!(
        request.starts_with("POST /api/seats/waffles/import/dry-run HTTP/1.1\r\n"),
        "{request}"
    );
    assert!(request.contains(&format!("lys-operator: {TOKEN}\r\n")), "{request}");
    assert_eq!(body(&request)?, json!({ "manifest": declared }));
    assert!(!printed(&output).contains(TOKEN));
    Ok(())
}

#[test]
fn a_confirmation_names_the_exact_plan_and_shows_the_servers_refusal() -> TestResult {
    let root = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(
        409,
        json!({
            "refusal": "import_source_changed",
            "reason": "/seats/waffles/settings.json was read at revision cd and reads ef now",
            "fields": [],
        }),
    )])?;
    install(root.path(), &address)?;
    let revision = "ab".repeat(32);
    let output = lys(
        root.path(),
        &[
            "seat",
            "import",
            "confirm",
            "waffles",
            "--plan",
            "plan-0123",
            "--revision",
            revision.as_str(),
        ],
    )?;
    let request = one_request(handle)?;
    assert_eq!(output.status.code(), Some(1), "{}", printed(&output));
    let all = printed(&output);
    assert!(all.contains("import_source_changed: /seats/waffles/settings.json"), "{all}");
    assert!(!all.contains(TOKEN), "the operator token was printed:\n{all}");
    assert!(
        request.starts_with("POST /api/seats/waffles/import/confirm HTTP/1.1\r\n"),
        "{request}"
    );
    let sent = body(&request)?;
    assert_eq!(sent["plan_id"], "plan-0123", "{sent}");
    assert_eq!(sent["plan_revision"], revision.as_str(), "{sent}");
    assert!(
        sent["operation"]
            .as_str()
            .is_some_and(|operation| operation.starts_with("op-") && operation.len() == 35),
        "{sent}"
    );
    Ok(())
}

#[test]
fn the_status_prints_one_json_object_under_json() -> TestResult {
    let root = tempfile::tempdir()?;
    let (address, handle) = serve(vec![(
        200,
        json!({ "seat": "waffles", "selected": "op-1", "previewed": null, "imports": [] }),
    )])?;
    install(root.path(), &address)?;
    let output = lys(root.path(), &["--json", "seat", "import", "status", "waffles"])?;
    let request = one_request(handle)?;
    assert!(output.status.success(), "{}", printed(&output));
    let answer: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(answer["selected"], "op-1", "{answer}");
    assert!(
        request.starts_with("GET /api/seats/waffles/import HTTP/1.1\r\n"),
        "{request}"
    );
    Ok(())
}
