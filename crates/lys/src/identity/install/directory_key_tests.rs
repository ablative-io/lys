#![cfg(test)]

use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::JoinHandle;

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

const STOP: &[u8] = b"STOP / HTTP/1.1\r\nContent-Length: 0\r\n\r\n";

/// The token the stand-in answers for the directory key.
const TOKEN: &str = "lys_directory$made-by-the-stand-in";

fn read_request(stream: &mut TcpStream) -> TestResult<(String, Vec<u8>)> {
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 4096];
    let head_end = loop {
        if let Some(at) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            break at;
        }
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err("the request ended before its head".into());
        }
        raw.extend_from_slice(&buffer[..read]);
    };
    let head = String::from_utf8(raw[..head_end].to_vec())?;
    let length = head
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .map_or(Ok(0), |(_, value)| value.trim().parse::<usize>())?;
    let mut body = raw[head_end + 4..].to_vec();
    while body.len() < length {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err("the request ended before its body".into());
        }
        body.extend_from_slice(&buffer[..read]);
    }
    let line = head.lines().next().unwrap_or_default().to_owned();
    Ok((line, body))
}

/// How the stand-in treats a request to change a key's rights.
#[derive(Clone, Copy)]
enum OnUpdate {
    Store,
    Ignore,
}

/// What the stand-in saw once stopped: every request line, and every
/// request body it was sent, in order.
type Seen = TestResult<(Vec<String>, Vec<Value>)>;

/// A stand-in for the sign-in service's API key endpoints, holding the
/// directory key with `access` when one is given. `forbidden` answers 403
/// to every request.
fn fake_keys(
    access: Option<Value>,
    on_update: OnUpdate,
    forbidden: bool,
) -> TestResult<(String, JoinHandle<Seen>)> {
    fake_keys_with_configure(access, on_update, forbidden, None)
}

fn fake_keys_with_configure(
    access: Option<Value>,
    on_update: OnUpdate,
    forbidden: bool,
    configure: Option<Value>,
) -> TestResult<(String, JoinHandle<Seen>)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let url = format!("http://{}", listener.local_addr()?);
    let handle = std::thread::spawn(move || -> Seen {
        let mut held = access;
        let (mut seen, mut bodies) = (Vec::new(), Vec::new());
        for stream in listener.incoming() {
            let mut stream = stream?;
            let (line, body) = read_request(&mut stream)?;
            if line.starts_with("STOP") {
                break;
            }
            if !body.is_empty() {
                bodies.push(serde_json::from_slice(&body)?);
            }
            let (status, answer) = if forbidden {
                (403, r#"{"message":"missing access rights"}"#.to_owned())
            } else if line.starts_with("GET /auth/v1/api_keys ") {
                let keys: Vec<Value> = held
                    .iter()
                    .map(|access| json!({"name": DIRECTORY_KEY_NAME, "access": access}))
                    .chain(configure.iter().map(|access| json!({"name": API_KEY_NAME, "access": access})))
                    .collect();
                (200, json!({ "keys": keys }).to_string())
            } else if line.starts_with("POST /auth/v1/api_keys ") {
                let sent: Value = serde_json::from_slice(&body)?;
                held = Some(sent["access"].clone());
                (200, TOKEN.to_owned())
            } else if line.starts_with("PUT /auth/v1/api_keys/lys_directory/secret ") {
                (200, TOKEN.to_owned())
            } else if line.starts_with("PUT /auth/v1/api_keys/lys_directory ") {
                if let OnUpdate::Store = on_update {
                    let sent: Value = serde_json::from_slice(&body)?;
                    held = Some(sent["access"].clone());
                }
                (200, String::new())
            } else {
                (404, String::new())
            };
            seen.push(line);
            stream.write_all(
                format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\n\r\n{answer}",
                    answer.len()
                )
                .as_bytes(),
            )?;
        }
        Ok((seen, bodies))
    });
    Ok((url, handle))
}

fn declared_configure_access() -> TestResult<Value> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(bootstrap_api_key())?;
    Ok(serde_json::from_slice::<Value>(&bytes)?["access"].clone())
}

fn deployment(root: &std::path::Path, api: &str, token: &[u8]) -> TestResult<DeploymentConfig> {
    let text = include_str!("../../../../../deploy/identity/config.example.toml");
    let mut config = DeploymentConfig::parse(text, root.to_path_buf())?;
    config.issuer.admin_url = api.to_owned();
    private_files::write(&config.state_dir().join(API_KEY_SECRET.file), &[b'A'; 64])?;
    private_files::write(&config.state_dir().join(server_config::PROVIDERS_KEY_FILE), token)?;
    Ok(config)
}

#[test]
fn provide_refuses_a_forbidden_live_key_without_rewriting_the_service_token() -> TestResult {
    let root = tempfile::tempdir()?;
    let (url, handle) = fake_keys(None, OnUpdate::Store, true)?;
    let config = deployment(root.path(), &url, b"lys_configure$held")?;
    let outcome = provide(&config);
    let (seen, bodies) = stop(&url, handle)?;
    let error = outcome.err().ok_or("the forbidden live key passed install")?;
    assert_eq!(error.kind(), ErrorKind::RauthyForbidden);
    assert!(error.to_string().contains("ApiKeys/read"), "{error}");
    assert_eq!(std::fs::read(config.state_dir().join(server_config::PROVIDERS_KEY_FILE))?, b"lys_configure$held");
    assert_eq!(seen.len(), 1);
    assert!(bodies.is_empty());
    Ok(())
}

#[test]
fn upgrade_preflight_checks_live_rights_and_never_mutates_a_key() -> TestResult {
    for (configure, directory, token, refused) in [
        (json!([{"group":"Users","access_rights":["read"]}]), directory_key_access(), TOKEN.as_bytes(), Some("ApiKeys/create")),
        (declared_configure_access()?, widened(), TOKEN.as_bytes(), Some("Secrets/update")),
        (declared_configure_access()?, directory_key_access(), b"lys_configure$held".as_slice(), Some("service key file")),
        (declared_configure_access()?, directory_key_access(), TOKEN.as_bytes(), None),
    ] {
        let root = tempfile::tempdir()?;
        let (url, handle) = fake_keys_with_configure(Some(directory), OnUpdate::Store, false, Some(configure))?;
        let config = deployment(root.path(), &url, token)?;
        let layout = crate::identity::install::layout::Layout::at(root.path().to_path_buf());
        let text = include_str!("../../../../../deploy/identity/config.example.toml")
            .replace("admin_url = \"http://127.0.0.1:8480\"", &format!("admin_url = \"{url}\""));
        std::fs::write(layout.deployment_config(), text)?;
        let before = std::fs::read(layout.deployment_config())?;
        let outcome = crate::identity::upgrade::verified_config(&layout);
        let (seen, bodies) = stop(&url, handle)?;
        match refused {
            Some(detail) => {
                let error = outcome.err().ok_or("invalid live authority passed upgrade")?;
                assert_eq!(error.kind(), ErrorKind::ReadBackMismatch);
                assert!(error.to_string().contains(detail), "{error}");
                assert!(error.to_string().contains("issuer administrator"), "{error}");
            }
            None => { outcome?; }
        }
        assert_eq!(seen, ["GET /auth/v1/api_keys HTTP/1.1"]);
        assert!(bodies.is_empty());
        assert_eq!(std::fs::read(config.state_dir().join(server_config::PROVIDERS_KEY_FILE))?, token);
        assert_eq!(std::fs::read(layout.deployment_config())?, before);
    }
    Ok(())
}

fn stop(url: &str, handle: JoinHandle<Seen>) -> Seen {
    let mut stream = TcpStream::connect(url.trim_start_matches("http://"))?;
    stream.write_all(STOP)?;
    handle
        .join()
        .map_err(|panic| format!("the stand-in panicked: {panic:?}"))?
}

/// The directory key's rights with Secrets update and a right over keys
/// added, as a key widened by hand would hold them.
fn widened() -> Value {
    json!([
        {"group": "Clients", "access_rights": ["read", "create", "update"]},
        {"group": "Secrets", "access_rights": ["read", "update"]},
        {"group": "Users", "access_rights": ["read", "create", "update"]},
        {"group": "AuthProviders", "access_rights": ["read", "create", "update"]},
        {"group": "ApiKeys", "access_rights": ["create"]},
    ])
}

#[test]
fn the_service_is_given_a_key_of_its_own_without_secrets_update_or_key_rights() -> TestResult {
    let (url, handle) = fake_keys(None, OnUpdate::Store, false)?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile(&api, None);
    let (seen, bodies) = stop(&url, handle)?;
    let token = outcome?.ok_or("no token was made for the service")?;
    assert_eq!(token.expose(), TOKEN);
    assert_eq!(
        seen,
        [
            "GET /auth/v1/api_keys HTTP/1.1",
            "POST /auth/v1/api_keys HTTP/1.1",
            "GET /auth/v1/api_keys HTTP/1.1",
        ]
    );
    assert_eq!(bodies.len(), 1, "one key was made");
    let made = &bodies[0];
    assert_eq!(made["name"], "lys_directory");
    let granted = rights(&made["access"])?;
    assert_eq!(
        granted
            .get("Secrets")
            .map(|set| set.iter().map(String::as_str).collect::<Vec<_>>()),
        Some(vec!["read"]),
        "the service may read secrets and never update one"
    );
    assert!(
        !granted.contains_key("ApiKeys"),
        "the service holds no right over keys"
    );
    Ok(())
}

#[test]
fn a_widened_key_is_narrowed_and_a_held_token_is_kept() -> TestResult {
    let (url, handle) = fake_keys(Some(widened()), OnUpdate::Store, false)?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile(&api, Some(TOKEN.as_bytes()));
    let (seen, bodies) = stop(&url, handle)?;
    assert!(outcome?.is_none(), "the service's token is left working");
    assert_eq!(
        seen,
        [
            "GET /auth/v1/api_keys HTTP/1.1",
            "PUT /auth/v1/api_keys/lys_directory HTTP/1.1",
            "GET /auth/v1/api_keys HTTP/1.1",
        ]
    );
    assert_eq!(
        bodies,
        [request()],
        "the key is set to the service's rights"
    );
    Ok(())
}

#[test]
fn an_install_whose_service_held_the_configure_key_renews_the_service_key() -> TestResult {
    let (url, handle) = fake_keys(Some(directory_key_access()), OnUpdate::Store, false)?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile(&api, Some(b"lys_configure$the-install-key"));
    let (seen, _) = stop(&url, handle)?;
    let token = outcome?.ok_or("the configure key was left with the service")?;
    assert_eq!(token.expose(), TOKEN);
    assert_eq!(
        seen,
        [
            "GET /auth/v1/api_keys HTTP/1.1",
            "PUT /auth/v1/api_keys/lys_directory/secret HTTP/1.1",
            "GET /auth/v1/api_keys HTTP/1.1",
        ]
    );
    Ok(())
}

#[test]
fn a_key_that_keeps_other_rights_is_a_read_back_mismatch() -> TestResult {
    let (url, handle) = fake_keys(Some(widened()), OnUpdate::Ignore, false)?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile(&api, Some(TOKEN.as_bytes()));
    stop(&url, handle)?;
    let error = outcome
        .err()
        .ok_or("a key with other rights was accepted")?;
    assert_eq!(error.kind(), ErrorKind::ReadBackMismatch);
    assert!(error.to_string().contains("Secrets/update"), "{error}");
    assert!(error.to_string().contains("ApiKeys/create"), "{error}");
    assert!(error.to_string().contains("issuer administrator"), "{error}");
    Ok(())
}

#[test]
fn a_configure_key_without_key_rights_is_refused_naming_them() -> TestResult {
    let (url, handle) = fake_keys(None, OnUpdate::Store, true)?;
    let api = RauthyApi::new(&url, None)?;
    let outcome = reconcile(&api, None);
    let (seen, bodies) = stop(&url, handle)?;
    let error = outcome.err().ok_or("a forbidden listing was accepted")?;
    assert_eq!(error.kind(), ErrorKind::RauthyForbidden);
    assert!(error.to_string().contains("API key rights"), "{error}");
    assert!(error.to_string().contains("ApiKeys/read"), "{error}");
    assert!(error.to_string().contains("issuer administrator"), "{error}");
    assert!(!error.to_string().contains("upgrading the install grants"), "{error}");
    assert_eq!(seen.len(), 1, "nothing was asked after the refusal");
    assert!(bodies.is_empty(), "nothing was written");
    Ok(())
}

#[test]
fn only_a_token_of_the_directory_key_is_held_as_one() {
    assert!(holds_directory_token(TOKEN.as_bytes()));
    assert!(!holds_directory_token(b"lys_directory$"));
    assert!(!holds_directory_token(b"lys_directoryx$secret"));
    assert!(!holds_directory_token(b"lys_configure$secret"));
}

#[test]
fn missing_directory_rights_name_each_right_and_the_administrator_repair() -> TestResult {
    let access = json!([
        {"group":"Clients","access_rights":["read","create","update"]},
        {"group":"Secrets","access_rights":["read"]},
        {"group":"Users","access_rights":["read"]},
        {"group":"AuthProviders","access_rights":["read","create","update"]}
    ]);
    let (url, handle) = fake_keys(Some(access), OnUpdate::Ignore, false)?;
    let outcome = reconcile(&RauthyApi::new(&url, None)?, Some(TOKEN.as_bytes()));
    stop(&url, handle)?;
    let error = outcome.err().ok_or("missing rights were accepted")?;
    assert_eq!(error.kind(), ErrorKind::ReadBackMismatch);
    for required in ["Users/create", "Users/update", "issuer administrator", "lys_directory"] {
        assert!(error.to_string().contains(required), "{error}");
    }
    Ok(())
}

#[test]
fn malformed_live_rights_refuse_before_any_key_write() -> TestResult {
    for access in [json!({"group":"Users"}), json!([{"group":"Users","access_rights":["read", 1]}]), json!([
        {"group":"Users","access_rights":["read"]},
        {"group":"Users","access_rights":["update"]}
    ])] {
        let (url, handle) = fake_keys(Some(access), OnUpdate::Store, false)?;
        let outcome = reconcile(&RauthyApi::new(&url, None)?, Some(TOKEN.as_bytes()));
        let (seen, bodies) = stop(&url, handle)?;
        assert_eq!(outcome.err().ok_or("malformed rights accepted")?.kind(), ErrorKind::RauthyUnexpected);
        assert_eq!(seen, ["GET /auth/v1/api_keys HTTP/1.1"]);
        assert!(bodies.is_empty());
    }
    Ok(())
}
