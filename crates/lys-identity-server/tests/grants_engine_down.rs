//! A permission engine that cannot be read is named, never drawn as nobody:
//! with `SpiceDB` answering the service but refusing every relationship
//! read, `/grants/reach` and `/grants/who` answer the named 503 rather than
//! a page with no holders.

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::spicedb::SpiceDbSettings;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// A `SpiceDB` gateway on a local port holding a schema and no
/// relationships, whose relationship reads fail while `broken` is set.
fn engine(broken: Arc<AtomicBool>) -> Result<String, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    let schema = Mutex::new(String::new());
    std::thread::spawn(move || {
        for socket in listener.incoming().map_while(Result::ok) {
            let mut reader = BufReader::new(socket);
            let mut line = String::new();
            let mut length = 0;
            let mut path = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 0) {
                let text = line.trim_end().to_owned();
                line.clear();
                if text.is_empty() {
                    break;
                }
                if path.is_empty() {
                    text.split(' ')
                        .nth(1)
                        .unwrap_or_default()
                        .clone_into(&mut path);
                }
                if let Some(value) = text.to_ascii_lowercase().strip_prefix("content-length: ") {
                    length = value.parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            if reader.read_exact(&mut body).is_err() {
                break;
            }
            let request: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
            let mut held = schema.lock().unwrap_or_else(PoisonError::into_inner);
            let (status, answer) = match path.as_str() {
                "/v1/schema/read" => (200, json!({ "schemaText": *held }).to_string()),
                "/v1/schema/write" => {
                    request["schema"]
                        .as_str()
                        .unwrap_or_default()
                        .clone_into(&mut held);
                    (200, "{}".to_owned())
                }
                "/v1/relationships/read"
                    if request["relationshipFilter"]["resourceType"] != "lys_mirror"
                        && broken.load(Ordering::SeqCst) =>
                {
                    (503, json!({ "message": "the engine is down" }).to_string())
                }
                "/v1/relationships/read" => (200, String::new()),
                "/v1/relationships/write" => (200, json!({ "writtenAt": {} }).to_string()),
                _ => (404, "{}".to_owned()),
            };
            drop(held);
            let mut socket = reader.into_inner();
            let written = write!(
                socket,
                "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            );
            if written.is_err() {
                break;
            }
        }
    });
    Ok(address)
}

#[tokio::test]
async fn an_unreadable_engine_is_named_by_reach_and_who_never_answered_as_nobody() -> TestResult {
    let broken = Arc::new(AtomicBool::new(false));
    let endpoint = engine(Arc::clone(&broken))?;
    let key = tempfile::NamedTempFile::new()?;
    std::fs::write(key.path(), "fixture-only")?;
    let settings = SpiceDbSettings {
        endpoint,
        key_file: key.path().to_owned(),
        mirror: "engine_down_fixture".to_owned(),
    };
    let (service, _) = Service::start_judging(GRANT_MODEL, Some(settings), |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?)
    })
    .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let reach = json!({
        "route": "browser",
        "resources": [{ "kind": "doc", "id": "1", "actions": ["read"] }],
    });
    let who = json!({
        "route": "browser", "resource": { "kind": "doc", "id": "1" }, "action": "read",
        "page_size": 10, "after": null,
    });
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 200, "{path} while the engine answers: {answer}");
    }
    broken.store(true, Ordering::SeqCst);
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 503, "{path}: {answer}");
        assert_eq!(
            answer["refusal"], "PermissionEngineUnavailable",
            "{path}: {answer}"
        );
    }
    Ok(())
}
