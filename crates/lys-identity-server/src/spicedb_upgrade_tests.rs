//! Upgrade starts from the old two-principal schema at a private HTTP fixture.
//! The real SpiceDb::open path must write the new subject without losing old kinds.
use super::{Model, SpiceDb, SpiceDbSettings};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

#[test]
fn opening_an_old_model_writes_service_account_subject_and_preserves_resources()
-> Result<(), Box<dyn Error>> {
    let old = "definition person {}\ndefinition agent {}\ndefinition grant {\n relation holder: person | agent\n}\ndefinition service_account {\n relation viewer: grant#holder\n permission view = viewer\n}\ndefinition directory {\n relation viewer: grant#holder\n permission view = viewer\n}\ndefinition fixture/doc {\n relation viewer: grant#holder\n permission view = viewer\n}";
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    listener.set_nonblocking(true)?;
    let fixture = std::thread::spawn(move || -> Result<String, String> {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            let (mut socket, _) = match listener.accept() {
                Ok(accepted) => accepted,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if std::time::Instant::now() >= deadline {
                        return Err("upgrade never wrote its schema".into());
                    }
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                Err(error) => return Err(error.to_string()),
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .map_err(|e| e.to_string())?;
            let mut raw = Vec::new();
            let split = loop {
                let mut byte = [0];
                socket.read_exact(&mut byte).map_err(|e| e.to_string())?;
                raw.push(byte[0]);
                if raw.ends_with(b"\r\n\r\n") {
                    break raw.len();
                }
                if raw.len() > 16384 {
                    return Err("request header too long".into());
                }
            };
            let head = String::from_utf8(raw.clone()).map_err(|e| e.to_string())?;
            let len: usize = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_owned)
                })
                .ok_or("no length")?
                .trim()
                .parse::<usize>()
                .map_err(|e| e.to_string())?;
            raw.resize(split + len, 0);
            socket
                .read_exact(&mut raw[split..])
                .map_err(|e| e.to_string())?;
            let request: Value =
                serde_json::from_slice(&raw[split..]).map_err(|e| e.to_string())?;
            let written = head.starts_with("POST /v1/schema/write ");
            let body = if written {
                json!({})
            } else {
                json!({"schemaText":old})
            }
            .to_string();
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .map_err(|e| e.to_string())?;
            if written {
                return request["schema"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or("no schema".into());
            }
        }
    });
    let temp = tempfile::tempdir()?;
    let key = temp.path().join("key");
    std::fs::write(&key, "fixture-only")?;
    let model = Model::new(
        1,
        [(
            lys_identity::grants::Relation::new("viewer")?,
            [lys_identity::grants::Action::new("view")?].into(),
        )],
    )?;
    let opened = SpiceDb::open(
        &SpiceDbSettings {
            endpoint: address,
            key_file: key,
            mirror: "upgrade_fixture".to_owned(),
        },
        &model,
    );
    let schema = fixture.join().map_err(|_| "fixture panicked")??;
    opened?;
    assert!(schema.contains("service_account with unexpired"));
    assert_eq!(schema.matches("definition service_account {").count(), 1);
    assert!(schema.contains("definition directory {"));
    assert!(schema.contains("definition fixture/doc {"));
    assert!(schema.contains("person | person with unexpired"));
    Ok(())
}
