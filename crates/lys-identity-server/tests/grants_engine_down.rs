//! A question that cannot be answered is named, never drawn as nobody: with
//! `SpiceDB` answering the service but refusing every relationship read, with
//! a grant recorded that it would not take, and with a revocation the grant
//! log could not record, `/grants/reach` and
//! `/grants/who` answer the named 503 rather than a page with no holders.

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::spicedb::SpiceDbSettings;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// A `SpiceDB` gateway on a local port holding a schema and the
/// relationships written to it, whose relationship reads fail while
/// `broken` is set, except the mirror's own, and whose writes fail while
/// `refusing` is set.
fn engine(broken: Arc<AtomicBool>, refusing: Arc<AtomicBool>) -> Result<String, Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    let schema = Mutex::new(String::new());
    let mut held: Vec<Value> = Vec::new();
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
            let mut text = schema.lock().unwrap_or_else(PoisonError::into_inner);
            let (status, answer) = match path.as_str() {
                "/v1/schema/read" => (200, json!({ "schemaText": *text }).to_string()),
                "/v1/schema/write" => {
                    request["schema"]
                        .as_str()
                        .unwrap_or_default()
                        .clone_into(&mut text);
                    (200, "{}".to_owned())
                }
                "/v1/relationships/read"
                    if request["relationshipFilter"]["resourceType"] != "lys_mirror"
                        && broken.load(Ordering::SeqCst) =>
                {
                    (503, json!({ "message": "the engine is down" }).to_string())
                }
                "/v1/relationships/read" => (200, matching(&held, &request["relationshipFilter"])),
                "/v1/relationships/write" if refusing.load(Ordering::SeqCst) => (
                    503,
                    json!({ "message": "the engine takes no writes" }).to_string(),
                ),
                "/v1/relationships/write" => {
                    written(&mut held, &request["updates"]);
                    (200, json!({ "writtenAt": {} }).to_string())
                }
                _ => (404, "{}".to_owned()),
            };
            drop(text);
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

/// The relationships of `held` that `filter` names, one read answer a line.
fn matching(held: &[Value], filter: &Value) -> String {
    held.iter()
        .filter(|relationship| {
            relationship["resource"]["objectType"] == filter["resourceType"]
                && filter["optionalResourceId"]
                    .as_str()
                    .is_none_or(|id| relationship["resource"]["objectId"] == id)
        })
        .map(|relationship| json!({ "result": { "relationship": relationship } }).to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `held` after the write's `updates`: each touch kept once, each delete gone.
fn written(held: &mut Vec<Value>, updates: &Value) {
    for update in updates.as_array().into_iter().flatten() {
        let relationship = &update["relationship"];
        held.retain(|kept| kept != relationship);
        if update["operation"] == "OPERATION_TOUCH" {
            held.push(relationship.clone());
        }
    }
}

#[tokio::test]
async fn an_unreadable_engine_is_named_by_reach_and_who_never_answered_as_nobody() -> TestResult {
    let broken = Arc::new(AtomicBool::new(false));
    let endpoint = engine(Arc::clone(&broken), Arc::new(AtomicBool::new(false)))?;
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

#[tokio::test]
async fn a_revocation_the_log_could_not_record_is_named_by_reach_and_who() -> TestResult {
    let endpoint = engine(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    )?;
    let key = tempfile::NamedTempFile::new()?;
    std::fs::write(key.path(), "fixture-only")?;
    let settings = SpiceDbSettings {
        endpoint,
        key_file: key.path().to_owned(),
        mirror: "unresolved_fixture".to_owned(),
    };
    let (service, (seeded, grant_log)) =
        Service::start_judging(GRANT_MODEL, Some(settings), |config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
            Ok((seeded, config.grant_log_dir.clone()))
        })
        .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let doc = json!({ "kind": "doc", "id": "1" });
    let root = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "holder": seeded.people[1].id.to_string(), "resource": doc, "relation": "alpha",
        "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
    });
    let (status, issued) = service.post("/grants/roots", Some(&ada), &root).await?;
    assert_eq!(status, 200, "{issued}");
    let grant = issued["grant"].as_str().ok_or("no grant")?;

    let aside = grant_log.with_extension("aside");
    std::fs::rename(&grant_log, &aside)?;
    let revoke = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "reason": "the log cannot take it",
    });
    let (status, answer) = service
        .post(&format!("/grants/{grant}/revoke"), Some(&ada), &revoke)
        .await?;
    assert_eq!(status, 503, "the revocation is not recorded: {answer}");
    assert_eq!(answer["refusal"], "OperationUnresolved", "{answer}");
    let reach = json!({
        "route": "browser",
        "resources": [{ "kind": "doc", "id": "1", "actions": ["read"] }],
    });
    let who = json!({
        "route": "browser", "resource": doc, "action": "read", "page_size": 10, "after": null,
    });
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 503, "{path}: {answer}");
        assert_eq!(answer["refusal"], "OperationUnresolved", "{path}: {answer}");
    }
    std::fs::rename(&aside, &grant_log)?;
    Ok(())
}

#[tokio::test]
async fn a_grant_the_engine_would_not_take_is_named_pending_by_reach_and_who() -> TestResult {
    let refusing = Arc::new(AtomicBool::new(false));
    let endpoint = engine(Arc::new(AtomicBool::new(false)), Arc::clone(&refusing))?;
    let key = tempfile::NamedTempFile::new()?;
    std::fs::write(key.path(), "fixture-only")?;
    let settings = SpiceDbSettings {
        endpoint,
        key_file: key.path().to_owned(),
        mirror: "pending_fixture".to_owned(),
    };
    let (service, seeded) = Service::start_judging(GRANT_MODEL, Some(settings), |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?)
    })
    .await?;
    let ada = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "ada@example.test".to_owned(),
        })
        .await?;
    let doc = json!({ "kind": "doc", "id": "1" });
    refusing.store(true, Ordering::SeqCst);
    let root = json!({
        "operation": OperationId::generate()?.to_string(), "route": "api",
        "holder": seeded.people[1].id.to_string(), "resource": doc, "relation": "alpha",
        "pass_on": { "kind": "use_only" }, "window": { "starts_at": 0, "ends_at": null },
    });
    let (status, answer) = service.post("/grants/roots", Some(&ada), &root).await?;
    assert_eq!(status, 503, "recorded, not yet taken: {answer}");
    assert_eq!(answer["refusal"], "ProjectionPending", "{answer}");
    let reach = json!({
        "route": "browser",
        "resources": [{ "kind": "doc", "id": "1", "actions": ["read"] }],
    });
    let who = json!({
        "route": "browser", "resource": doc, "action": "read", "page_size": 10, "after": null,
    });
    for (path, body) in [("/grants/reach", &reach), ("/grants/who", &who)] {
        let (status, answer) = service.post(path, Some(&ada), body).await?;
        assert_eq!(status, 503, "{path}: {answer}");
        assert_eq!(answer["refusal"], "ProjectionPending", "{path}: {answer}");
    }
    Ok(())
}
