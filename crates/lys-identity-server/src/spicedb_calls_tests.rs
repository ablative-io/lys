#![cfg(test)]
//! What one question costs the engine: a schema read is one request, in a
//! scope as outside one, so reading the kinds is one call to `SpiceDB`.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use super::SpiceDb;

const HELD: &str = "definition grant {}\n\ndefinition lys/b0123456789abcdef/grant {}\n\ndefinition lys/b0123456789abcdef/fixture/doc {}\n\ndefinition fixture/doc {}";

/// Each request's path, in the order the engine heard them.
type Heard = Arc<Mutex<Vec<String>>>;

/// An engine on a local port that answers every schema read with [`HELD`]
/// and records each request's path.
fn engine() -> Result<(String, Heard), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    let paths = Arc::new(Mutex::new(Vec::new()));
    let heard = Arc::clone(&paths);
    std::thread::spawn(move || {
        for socket in listener.incoming().map_while(Result::ok) {
            let mut reader = BufReader::new(socket);
            let mut line = String::new();
            let mut length = 0;
            let mut path = None;
            while reader.read_line(&mut line).is_ok_and(|read| read > 0) {
                let text = line.trim_end().to_owned();
                line.clear();
                if text.is_empty() {
                    break;
                }
                if path.is_none() {
                    path = text.split(' ').nth(1).map(str::to_owned);
                }
                if let Some(value) = text.to_ascii_lowercase().strip_prefix("content-length: ") {
                    length = value.parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            if reader.read_exact(&mut body).is_err() {
                continue;
            }
            heard
                .lock()
                .expect("fixture lock poisoned")
                .push(path.unwrap_or_default());
            let answer = serde_json::json!({ "schemaText": HELD }).to_string();
            let mut socket = reader.into_inner();
            let written = write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            );
            if written.is_err() {
                break;
            }
        }
    });
    Ok((address, paths))
}

fn at(endpoint: String, scope: Option<&str>) -> SpiceDb {
    SpiceDb {
        endpoint,
        key: Arc::from("fixture-only"),
        mirror: "calls_fixture".to_owned(),
        relations: BTreeMap::new(),
        app_kinds: Mutex::new(BTreeMap::new()),
        scope: scope.map(str::to_owned),
        client: Arc::default(),
        post_json: super::plain_post_json,
        tokens: Arc::default(),
        confirmed: Arc::default(),
    }
}

#[test]
fn reading_the_kinds_is_one_schema_read_in_a_scope_and_outside_one() -> Result<(), Box<dyn Error>> {
    let (endpoint, paths) = engine()?;
    let expected = [
        (None, ["fixture.doc"]),
        (Some("lys/b0123456789abcdef"), ["fixture.doc"]),
    ];
    for (scope, kinds) in expected {
        let before = paths.lock().expect("fixture lock poisoned").len();
        let engine = at(endpoint.clone(), scope);
        assert_eq!(
            engine.kinds()?,
            kinds
                .map(str::to_owned)
                .into_iter()
                .collect::<BTreeSet<_>>(),
            "{scope:?}"
        );
        let heard = paths.lock().expect("fixture lock poisoned");
        assert_eq!(heard[before..], ["/v1/schema/read"], "{scope:?}");
    }
    Ok(())
}

fn consistency_engine(written: bool, marker: bool) -> Result<(SpiceDb, Heard), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let heard = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&heard);
    std::thread::spawn(move || {
        for socket in listener.incoming().map_while(Result::ok) {
            let mut reader = BufReader::new(socket);
            let mut line = String::new();
            let mut length = 0;
            let mut path = String::new();
            while reader.read_line(&mut line).is_ok_and(|n| n > 0) {
                if path.is_empty() {
                    line.split(' ')
                        .nth(1)
                        .unwrap_or_default()
                        .clone_into(&mut path);
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length: ") {
                    length = value.trim().parse().unwrap_or(0);
                }
                if line == "\r\n" {
                    break;
                }
                line.clear();
            }
            let mut body = vec![0; length];
            if reader.read_exact(&mut body).is_err() {
                break;
            }
            kept.lock()
                .expect("fixture lock poisoned")
                .push(String::from_utf8(body).expect("fixture JSON"));
            let answer = match path.as_str() {
                "/v1/schema/read" => serde_json::json!({"schemaText": HELD}).to_string(),
                "/v1/relationships/write" => serde_json::json!({"writtenAt":{"token":"write-token"}}).to_string(),
                "/v1/relationships/read" if !marker => String::new(),
                "/v1/relationships/read" => serde_json::json!({"result":{"readAt":{"token":"read-token"},"relationship":{
                    "resource":{"objectType":"lys_mirror","objectId":"calls_fixture"},"relation":"revision",
                    "subject":{"object":{"objectType":"lys_revision","objectId":"1"}}}}}).to_string(),
                "/v1/permissions/check" => serde_json::json!({"permissionship":"PERMISSIONSHIP_HAS_PERMISSION"}).to_string(),
                _ => "{}".to_owned(),
            };
            if write!(
                reader.get_mut(),
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            )
            .is_err()
            {
                break;
            }
        }
    });
    let mut engine = at(endpoint, None);
    engine
        .relations
        .insert("reader".to_owned(), BTreeSet::from(["read".to_owned()]));
    if written {
        lys_identity::grants::RelationshipStore::write(&mut engine, 1, &[], &[])?;
    }
    Ok((engine, heard))
}

#[test]
fn a_write_token_is_used_by_the_next_permission_question() -> Result<(), Box<dyn Error>> {
    let (engine, heard) = consistency_engine(true, true)?;
    let subject =
        lys_identity::IdentityId::Person("person-11111111111111111111111111111111".parse()?);
    assert!(engine.check(
        &lys_identity::grants::Resource::new("fixture.doc", "asked")?,
        &lys_identity::grants::Action::new("read")?,
        subject,
        1
    )?);
    let bodies: Vec<serde_json::Value> = heard
        .lock()
        .expect("fixture lock poisoned")
        .iter()
        .map(|body| serde_json::from_str(body))
        .collect::<Result<_, _>>()?;
    let first = bodies
        .iter()
        .find(|body| body.get("consistency").is_some())
        .ok_or("no consistent question")?;
    assert_eq!(
        first
            .pointer("/consistency/atLeastAsFresh/token")
            .and_then(serde_json::Value::as_str),
        Some("write-token")
    );
    assert!(
        bodies
            .iter()
            .filter(|body| body.get("consistency").is_some())
            .all(|body| body.pointer("/consistency/fullyConsistent").is_none())
    );
    Ok(())
}

#[test]
fn a_restart_learns_one_fully_consistent_token_then_uses_it() -> Result<(), Box<dyn Error>> {
    let (engine, heard) = consistency_engine(false, true)?;
    let subject =
        lys_identity::IdentityId::Person("person-11111111111111111111111111111111".parse()?);
    let resource = lys_identity::grants::Resource::new("fixture.doc", "asked")?;
    let action = lys_identity::grants::Action::new("read")?;
    engine.check(&resource, &action, subject, 1)?;
    engine.check(&resource, &action, subject, 1)?;
    let bodies: Vec<serde_json::Value> = heard
        .lock()
        .expect("fixture lock poisoned")
        .iter()
        .map(|body| serde_json::from_str(body))
        .collect::<Result<_, _>>()?;
    let fully = bodies
        .iter()
        .filter(|body| body.pointer("/consistency/fullyConsistent").is_some())
        .count();
    assert_eq!(
        fully, 1,
        "a restarted revision is read fully consistently only once"
    );
    assert!(
        bodies
            .last()
            .ok_or("no requests")?
            .pointer("/consistency/atLeastAsFresh/token")
            .is_some()
    );
    Ok(())
}

#[test]
fn a_missing_settled_revision_marker_refuses_by_its_existing_name() -> Result<(), Box<dyn Error>> {
    let (engine, _) = consistency_engine(true, false)?;
    let subject =
        lys_identity::IdentityId::Person("person-11111111111111111111111111111111".parse()?);
    let error = engine
        .check(
            &lys_identity::grants::Resource::new("fixture.doc", "asked")?,
            &lys_identity::grants::Action::new("read")?,
            subject,
            1,
        )
        .expect_err("an absent mirror marker must refuse");
    assert!(
        matches!(error, lys_identity::grants::GrantError::PermissionEngineUnavailable { reason } if reason.contains("settled revision 1"))
    );
    Ok(())
}
