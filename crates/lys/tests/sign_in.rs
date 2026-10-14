//! Native sign-in and person-authenticated CLI calls use real loopback HTTP fixtures.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use base64::Engine;
use lys::commands::sign_in::{Pass, pass_file, sign_in_at};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const PERSON: &str = "person-0123456789abcdef0123456789abcdef";
type Outcome = Result<(), Box<dyn Error>>;

#[path = "support/sign_in_errors.rs"]
mod errors_tests;

struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: String,
}

fn pairs(text: &str) -> BTreeMap<String, String> {
    text.split('&')
        .filter_map(|part| part.split_once('='))
        .map(|(key, value)| {
            let mut result = Vec::new();
            let mut bytes = value.bytes();
            while let Some(byte) = bytes.next() {
                if byte == b'%' {
                    let digits = bytes.by_ref().take(2).map(char::from).collect::<String>();
                    if let Ok(byte) = u8::from_str_radix(&digits, 16) {
                        result.push(byte);
                    }
                } else {
                    result.push(if byte == b'+' { b' ' } else { byte });
                }
            }
            (
                key.to_owned(),
                String::from_utf8_lossy(&result).into_owned(),
            )
        })
        .collect()
}

type FixtureWorker = thread::JoinHandle<Result<(), String>>;
type Served = Result<(String, FixtureWorker), Box<dyn Error>>;

fn serve(
    count: usize,
    answer: impl Fn(&Request, &str) -> Result<(u16, Value), String> + Send + 'static,
) -> Served {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let base = format!("http://{}", listener.local_addr()?);
    let origin = base.clone();
    let worker = thread::spawn(move || {
        for _ in 0..count {
            let (stream, _) = listener.accept().map_err(|error| error.to_string())?;
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|error| error.to_string())?;
            let mut words = line.split_whitespace();
            let method = words.next().ok_or("method absent")?.to_owned();
            let path = words.next().ok_or("path absent")?.to_owned();
            let mut headers = BTreeMap::new();
            loop {
                line.clear();
                reader
                    .read_line(&mut line)
                    .map_err(|error| error.to_string())?;
                if line == "\r\n" {
                    break;
                }
                let (name, value) = line.split_once(':').ok_or("header malformed")?;
                headers.insert(name.to_ascii_lowercase(), value.trim().to_owned());
            }
            let length = headers
                .get("content-length")
                .ok_or("length absent")?
                .parse::<usize>()
                .map_err(|error| error.to_string())?;
            if length > 65536 {
                return Err("fixture body too large".to_owned());
            }
            let mut body = vec![0; length];
            reader
                .read_exact(&mut body)
                .map_err(|error| error.to_string())?;
            let request = Request {
                method,
                path,
                headers,
                body: String::from_utf8(body).map_err(|error| error.to_string())?,
            };
            let (status, answer) = answer(&request, &origin)?;
            let answer = serde_json::to_vec(&answer).map_err(|error| error.to_string())?;
            let mut stream = reader.into_inner();
            write!(stream, "HTTP/1.1 {status} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", answer.len())
                .and_then(|()| stream.write_all(&answer)).map_err(|error| error.to_string())?;
        }
        Ok(())
    });
    Ok((base, worker))
}

fn joined(worker: thread::JoinHandle<Result<(), String>>) -> Outcome {
    worker
        .join()
        .map_err(|payload| {
            format!(
                "http_fixture_panicked: payload type {:?}",
                payload.type_id()
            )
        })?
        .map_err(Into::into)
}

fn access() -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}

fn held(root: &std::path::Path, server: &str, expires_at: u64) -> Result<String, Box<dyn Error>> {
    let access = access();
    let pass = Pass {
        server: server.to_owned(),
        person: PERSON.to_owned(),
        expires_at,
        access_token: access.clone(),
        refresh_token: access.clone(),
    };
    lys::identity::private_files::ensure_dir(&root.join("client"))?;
    lys::identity::private_files::write(&pass_file(root), &serde_json::to_vec(&pass)?)?;
    Ok(access)
}

#[test]
fn sign_in_opens_authorize_receives_loopback_pkce_and_writes_only_an_owner_readable_pass() -> Outcome
{
    let dir = tempfile::tempdir()?;
    let challenge = Arc::new(Mutex::new(String::new()));
    let checked = Arc::clone(&challenge);
    let bearer = access();
    let issued = bearer.clone();
    let (base, worker) = serve(2, move |request, origin| {
        if request.path == "/.well-known/oauth-authorization-server" {
            return Ok((
                200,
                json!({"authorization_endpoint":format!("{origin}/oauth/mcp/authorize"),"token_endpoint":format!("{origin}/oauth/mcp/token")}),
            ));
        }
        if request.method != "POST" || request.path != "/oauth/mcp/token" {
            return Err("wrong exchange route".to_owned());
        }
        let form = pairs(&request.body);
        if form.get("client_id").map(String::as_str) != Some("lys-cli") {
            return Err("wrong client".to_owned());
        }
        let verifier = form.get("code_verifier").ok_or("verifier absent")?;
        let actual = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(Sha256::digest(verifier.as_bytes()));
        if actual
            != *checked
                .lock()
                .map_err(|error| format!("fixture_lock_poisoned: {error}"))?
        {
            return Err("PKCE did not verify".to_owned());
        }
        Ok((
            200,
            json!({"person":PERSON,"access_token":issued,"refresh_token":issued,"expires_in":3600}),
        ))
    })?;
    let mut callback = None;
    let signed = sign_in_at(&format!("{base}/api"), dir.path(), |authorize| {
        let query = pairs(
            authorize
                .split_once('?')
                .ok_or_else(|| lys::commands::seat_client::refused("fixture_invalid", "no query"))?
                .1,
        );
        let redirect = query
            .get("redirect_uri")
            .ok_or_else(|| lys::commands::seat_client::refused("fixture_invalid", "no redirect"))?
            .clone();
        let state = query
            .get("state")
            .ok_or_else(|| lys::commands::seat_client::refused("fixture_invalid", "no state"))?
            .clone();
        *challenge.lock().map_err(|error| {
            lys::commands::seat_client::refused(
                "fixture_invalid",
                format!("fixture lock poisoned: {error}"),
            )
        })? = query
            .get("code_challenge")
            .ok_or_else(|| lys::commands::seat_client::refused("fixture_invalid", "no challenge"))?
            .clone();
        callback = Some(thread::spawn(move || {
            let rest = redirect
                .strip_prefix("http://")
                .ok_or("redirect is not loopback HTTP")?;
            let (host, path) = rest.split_once('/').ok_or("redirect path absent")?;
            let mut stream = TcpStream::connect(host).map_err(|error| error.to_string())?;
            write!(stream, "GET /{path}?state={state}&code=fixture-answer HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").map_err(|error| error.to_string())?;
            let mut reply = String::new();
            stream
                .read_to_string(&mut reply)
                .map_err(|error| error.to_string())?;
            if !reply.starts_with("HTTP/1.1 200") {
                return Err("callback refused".to_owned());
            }
            Ok::<_, String>(())
        }));
        Ok(())
    })?;
    joined(worker)?;
    joined(callback.ok_or("callback did not start")?)?;
    assert_eq!(signed.person, PERSON);
    assert_eq!(
        std::fs::metadata(&signed.file)?.permissions().mode() & 0o777,
        0o600
    );
    let receipt = serde_json::to_string(&signed)?;
    assert!(!receipt.contains(&bearer));
    let stored: Pass = serde_json::from_slice(&std::fs::read(&signed.file)?)?;
    assert_eq!(stored.server, format!("{base}/api"));
    let credential_matches = stored.access_token == bearer;
    assert!(
        credential_matches,
        "the stored credential differs from the approved one"
    );
    Ok(())
}

#[test]
fn a_pass_is_renewed_without_opening_the_browser() -> Outcome {
    let dir = tempfile::tempdir()?;
    let refreshed = access();
    let issued = refreshed.clone();
    let (base, worker) = serve(2, move |request, origin| {
        if request.path.starts_with("/.well-known/") {
            return Ok((
                200,
                json!({"authorization_endpoint":format!("{origin}/oauth/mcp/authorize"),"token_endpoint":format!("{origin}/oauth/mcp/token")}),
            ));
        }
        if pairs(&request.body).get("grant_type").map(String::as_str) != Some("refresh_token") {
            return Err("not a renewal".to_owned());
        }
        Ok((
            200,
            json!({"person":PERSON,"access_token":issued,"refresh_token":issued,"expires_in":3600}),
        ))
    })?;
    held(dir.path(), &format!("{base}/api"), 0)?;
    sign_in_at(&format!("{base}/api"), dir.path(), |_| {
        Err(lys::commands::seat_client::refused(
            "fixture_browser_opened",
            "renewal opened a browser",
        ))
    })?;
    joined(worker)?;
    assert_eq!(
        std::fs::metadata(pass_file(dir.path()).with_extension("previous.json"))?
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let pass = lys::commands::sign_in::load_at(dir.path(), None, true)?;
    let credential_matches = pass.access_token == refreshed;
    assert!(
        credential_matches,
        "the stored credential differs from the approved one"
    );
    Ok(())
}

#[test]
fn seat_list_without_an_install_sends_only_the_person_bearer_and_shows_the_version_mark() -> Outcome
{
    let dir = tempfile::tempdir()?;
    let bearer = access();
    let expected = bearer.clone();
    let (base, worker) = serve(1, move |request, _| {
        if request.path != "/api/seats"
            || request.headers.get("authorization") != Some(&format!("Bearer {expected}"))
            || request.headers.contains_key("lys-operator")
        {
            return Err("the request did not use only the person's pass".to_owned());
        }
        Ok((
            200,
            json!({"seats":[{"name":"newer","qualification":"unqualified version 2.1.300; qualified 2.1.296"},{"name":"pinned","qualification":null}],"runner":"read"}),
        ))
    })?;
    let pass = Pass {
        server: format!("{base}/api"),
        person: PERSON.to_owned(),
        expires_at: u64::MAX,
        access_token: bearer.clone(),
        refresh_token: bearer.clone(),
    };
    lys::identity::private_files::ensure_dir(&dir.path().join("client"))?;
    lys::identity::private_files::write(&pass_file(dir.path()), &serde_json::to_vec(&pass)?)?;
    let result = Command::new(env!("CARGO_BIN_EXE_lys"))
        .env("LYS_IDENTITY_HOME", dir.path())
        .args(["seat", "list", "--server", &format!("{base}/api")])
        .output()?;
    joined(worker)?;
    assert!(result.status.success());
    let output = String::from_utf8(result.stdout)?;
    assert_eq!(
        output
            .matches("unqualified version 2.1.300; qualified 2.1.296")
            .count(),
        1
    );
    assert!(!output.contains(&bearer));
    assert!(!dir.path().join("identity.json").exists());
    Ok(())
}

#[test]
fn missing_and_expired_passes_name_sign_in_and_personal_start_keeps_not_admitted() -> Outcome {
    let dir = tempfile::tempdir()?;
    let missing = Command::new(env!("CARGO_BIN_EXE_lys"))
        .env("LYS_IDENTITY_HOME", dir.path())
        .args(["seat", "list", "--server", "https://example.test/api"])
        .output()?;
    assert!(!missing.status.success());
    assert!(String::from_utf8(missing.stderr)?.contains("lys sign-in"));
    let (base, worker) = serve(1, |request, _| {
        if request.method != "POST" || request.path != "/api/seats/own/start" {
            return Err("wrong start route".to_owned());
        }
        Ok((
            403,
            json!({"refusal":"NotAdmitted","reason":"this person may not start this seat"}),
        ))
    })?;
    held(dir.path(), &format!("{base}/api"), 0)?;
    let expired = Command::new(env!("CARGO_BIN_EXE_lys"))
        .env("LYS_IDENTITY_HOME", dir.path())
        .args(["seat", "list", "--server", &format!("{base}/api")])
        .output()?;
    assert!(!expired.status.success());
    assert!(String::from_utf8(expired.stderr)?.contains("lys sign-in"));
    held(dir.path(), &format!("{base}/api"), u64::MAX)?;
    let refused = Command::new(env!("CARGO_BIN_EXE_lys"))
        .env("LYS_IDENTITY_HOME", dir.path())
        .args(["seat", "start", "own", "--server", &format!("{base}/api")])
        .output()?;
    joined(worker)?;
    assert!(!refused.status.success());
    assert!(
        String::from_utf8(refused.stderr)?
            .contains("NotAdmitted: this person may not start this seat")
    );
    Ok(())
}

#[test]
fn an_admitted_person_starts_a_seat_without_a_local_install() -> Outcome {
    let dir = tempfile::tempdir()?;
    let (base, worker) = serve(1, |request, _| {
        if request.method != "POST"
            || request.path != "/api/seats/own/start"
            || !request.headers.contains_key("authorization")
            || request.headers.contains_key("lys-operator")
        {
            return Err("personal start did not use its own authority".to_owned());
        }
        Ok((
            200,
            json!({"seat":{"name":"own"},"started":{"session":"own-session"}}),
        ))
    })?;
    held(dir.path(), &format!("{base}/api"), u64::MAX)?;
    let result = Command::new(env!("CARGO_BIN_EXE_lys"))
        .env("LYS_IDENTITY_HOME", dir.path())
        .args(["seat", "start", "own", "--server", &format!("{base}/api")])
        .output()?;
    joined(worker)?;
    assert!(result.status.success());
    assert!(!dir.path().join("identity.json").exists());
    Ok(())
}
