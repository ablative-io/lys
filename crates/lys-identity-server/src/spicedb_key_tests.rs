//! Engine scopes share the credential loaded before serving requests.

use super::scope::blocks;
use super::{Model, SpiceDb, SpiceDbConnection, SpiceDbEngine, SpiceDbSettings};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn model() -> Result<Model, Box<dyn Error>> {
    Ok(Model::new(
        1,
        [(
            lys_identity::grants::Relation::new("viewer")?,
            [lys_identity::grants::Action::new("view")?].into(),
        )],
    )?)
}

/// The stand-in engine, holding `schema` at the start. It refuses to delete
/// the relationships of a definition its schema does not hold, as `SpiceDB`
/// does. With `rival`, another writer of the same engine takes the scope
/// `rival` out of the schema just after a read shows it, as a writer whose
/// own schema write was composed from an older read does.
fn serve(listener: &TcpListener, mut schema: String, rival: Option<&str>) -> Result<usize, String> {
    let mut calls = 0;
    loop {
        let (mut socket, _) = listener.accept().map_err(|error| error.to_string())?;
        let mut raw = Vec::new();
        loop {
            let mut byte = [0];
            socket
                .read_exact(&mut byte)
                .map_err(|error| error.to_string())?;
            raw.push(byte[0]);
            if raw.ends_with(b"\r\n\r\n") {
                break;
            }
            if raw.len() > 16384 {
                return Err("request header too long".to_owned());
            }
        }
        let header = String::from_utf8(raw).map_err(|error| error.to_string())?;
        if header == "fixture complete\r\n\r\n" {
            return Ok(calls);
        }
        if !header
            .to_ascii_lowercase()
            .lines()
            .any(|line| line == "authorization: bearer fixture-only")
        {
            return Err("unexpected fixture credential".to_owned());
        }
        let length = header
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("content-length: ")
                    .map(str::to_owned)
            })
            .ok_or("no body length")?
            .parse::<usize>()
            .map_err(|error| error.to_string())?;
        let mut body = vec![0; length];
        socket
            .read_exact(&mut body)
            .map_err(|error| error.to_string())?;
        let request: Value = serde_json::from_slice(&body).map_err(|error| error.to_string())?;
        let (status, answer) = if header.starts_with("POST /v1/schema/read ") {
            let shown = json!({"schemaText": schema});
            if let Some(rival) = rival {
                schema = without(&schema, rival);
            }
            ("200 OK", shown)
        } else if header.starts_with("POST /v1/schema/write ") {
            schema = request["schema"].as_str().ok_or("no schema")?.to_owned();
            ("200 OK", json!({}))
        } else if header.starts_with("POST /v1/relationships/delete ") {
            let kind = request["relationshipFilter"]["resourceType"]
                .as_str()
                .ok_or("no resource type")?;
            if blocks(&schema).iter().any(|block| block.name == kind) {
                ("200 OK", json!({}))
            } else {
                ("400 Bad Request", unknown(kind))
            }
        } else {
            return Err("unexpected fixture request".to_owned());
        };
        let body = answer.to_string();
        write!(
            socket,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .map_err(|error| error.to_string())?;
        calls += 1;
    }
}

/// The schema `schema` without the scope `scope`'s names.
fn without(schema: &str, scope: &str) -> String {
    blocks(schema)
        .into_iter()
        .filter(|block| !block.name.starts_with(&format!("{scope}/")))
        .map(|block| block.text)
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `SpiceDB`'s refusal of a request naming the definition `kind` it does
/// not hold, as the engine at v1.56.2 answers it.
fn unknown(kind: &str) -> Value {
    json!({
        "code": 9,
        "message": format!("object definition `{kind}` not found"),
        "details": [{
            "@type": "type.googleapis.com/google.rpc.ErrorInfo",
            "reason": "ERROR_REASON_UNKNOWN_DEFINITION",
            "domain": "authzed.com",
            "metadata": {"definition_name": kind},
        }],
    })
}

#[test]
fn scratch_open_and_cleanup_reuse_the_key_after_its_file_disappears() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let key_file = temporary.path().join("key");
    std::fs::write(&key_file, "fixture-only")?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let worker = std::thread::spawn(move || serve(&listener, String::new(), None));
    let result = (|| -> Result<(), Box<dyn Error>> {
        let settings = SpiceDbSettings {
            endpoint: endpoint.clone(),
            key_file: key_file.clone(),
            mirror: "fixture".to_owned(),
        };
        let model = model()?;
        let connection = SpiceDbConnection::load(&settings)?;
        let engine = SpiceDb::open_connected(&connection, &model)?;
        std::fs::remove_file(&key_file)?;
        let scratch = SpiceDb::open_scratch(&connection, &model, "lys/bench_one")?;
        scratch.kinds()?;
        SpiceDb::remove_scratch(&connection, "lys/bench_one")?;
        SpiceDb::open_scratch(&connection, &model, "lys/bench_two")?;
        if SpiceDb::clear_scratch(&connection)? != 1 {
            return Err("expected one scratch scope".into());
        }
        engine.kinds()?;
        Ok(())
    })();
    let stopped = TcpStream::connect(&endpoint)
        .and_then(|mut socket| socket.write_all(b"fixture complete\r\n\r\n"));
    let calls = worker
        .join()
        .map_err(|panic| format!("fixture panicked: {panic:?}"))??;
    stopped?;
    result?;
    assert!(calls > 3);
    Ok(())
}

/// A scratch scope another writer of the engine takes away between the
/// removal's read and its deletes is already removed: the removal answers
/// done, and no scratch scope is left.
#[test]
fn a_scratch_scope_another_writer_took_away_is_removed_without_refusal()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let key_file = temporary.path().join("key");
    std::fs::write(&key_file, "fixture-only")?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let left = "definition lys/bench_rival/person {}\n\n\
                definition lys/bench_rival/page {\n\trelation reader: lys/bench_rival/person\n}";
    let worker =
        std::thread::spawn(move || serve(&listener, left.to_owned(), Some("lys/bench_rival")));
    let result = (|| -> Result<(), Box<dyn Error>> {
        let connection = SpiceDbConnection::load(&SpiceDbSettings {
            endpoint: endpoint.clone(),
            key_file: key_file.clone(),
            mirror: "fixture".to_owned(),
        })?;
        SpiceDb::remove_scratch(&connection, "lys/bench_rival")?;
        if SpiceDb::clear_scratch(&connection)? != 0 {
            return Err("a scratch scope was left".into());
        }
        Ok(())
    })();
    let stopped = TcpStream::connect(&endpoint)
        .and_then(|mut socket| socket.write_all(b"fixture complete\r\n\r\n"));
    worker
        .join()
        .map_err(|panic| format!("fixture panicked: {panic:?}"))??;
    stopped?;
    result
}

#[test]
fn explicit_reload_replaces_the_key_without_changing_existing_connections()
-> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let settings = SpiceDbSettings {
        endpoint: "127.0.0.1:1".to_owned(),
        key_file: temporary.path().join("key"),
        mirror: "fixture".to_owned(),
    };
    std::fs::write(
        &settings.key_file,
        "SPICEDB_GRPC_PRESHARED_KEY=first-fixture\n",
    )?;
    let original = SpiceDbConnection::load(&settings)?;
    let shared = original.clone();
    assert!(std::sync::Arc::ptr_eq(&original.key, &shared.key));
    assert!(!format!("{original:?}").contains("first-fixture"));
    std::fs::write(&settings.key_file, "second-fixture\n")?;
    let reloaded = SpiceDbConnection::load(&settings)?;
    assert_eq!(original.key.as_ref(), "first-fixture");
    assert_eq!(reloaded.key.as_ref(), "second-fixture");
    assert!(!format!("{reloaded:?}").contains("second-fixture"));
    std::fs::write(&settings.key_file, " \n")?;
    assert!(matches!(
        SpiceDbConnection::load(&settings),
        Err(lys_identity::grants::GrantError::PermissionEngineUnavailable { reason })
            if reason.contains("key file is empty")
    ));
    std::fs::remove_file(&settings.key_file)?;
    assert!(matches!(
        SpiceDbConnection::load(&settings),
        Err(lys_identity::grants::GrantError::PermissionEngineUnavailable { reason })
            if reason.contains("key file") && reason.contains("could not be read")
    ));
    Ok(())
}

#[test]
fn the_engine_reads_its_key_once_on_first_use() -> Result<(), Box<dyn Error>> {
    let temporary = tempfile::tempdir()?;
    let settings = SpiceDbSettings {
        endpoint: "127.0.0.1:1".to_owned(),
        key_file: temporary.path().join("key"),
        mirror: "fixture".to_owned(),
    };
    let unread = SpiceDbEngine::new(settings.clone());
    assert_eq!(unread.endpoint(), "127.0.0.1:1");
    assert!(matches!(
        unread.connection(),
        Err(lys_identity::grants::GrantError::PermissionEngineUnavailable { .. })
    ));
    std::fs::write(&settings.key_file, "first-fixture\n")?;
    let engine = SpiceDbEngine::new(settings.clone());
    let first = engine.connection()?;
    std::fs::remove_file(&settings.key_file)?;
    let again = engine.connection()?;
    assert!(std::sync::Arc::ptr_eq(&first.key, &again.key));
    assert_eq!(again.key.as_ref(), "first-fixture");
    Ok(())
}
