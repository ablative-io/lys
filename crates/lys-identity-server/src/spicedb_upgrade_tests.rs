#![cfg(test)]
//! What open writes to the engine's schema, held two ways. The upgrade
//! starts from the old two-principal schema at a private HTTP fixture: the
//! real `SpiceDb::open` path must write the new subject without losing old
//! kinds. And `FIXED` is held against the grant and mirror schemas: it names
//! every definition either declares, and nothing that neither does.
use super::{Model, SpiceDb, SpiceDbSettings};
use serde_json::{Value, json};
use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[test]
fn opening_an_old_model_writes_service_account_subject_and_preserves_resources()
-> Result<(), Box<dyn Error>> {
    let old = "definition person {}\ndefinition agent {}\ndefinition grant {\n relation holder: person | agent\n}\ndefinition service_account {\n relation viewer: grant#holder\n permission view = viewer\n}\ndefinition directory {\n relation viewer: grant#holder\n permission view = viewer\n}\ndefinition machine {\n relation viewer: grant#holder\n permission view = viewer\n}\ndefinition fixture/doc {\n relation viewer: grant#holder\n permission view = viewer\n}";
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
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?.to_string();
    let fixture = std::thread::spawn(move || -> Result<String, String> {
        let mut schema = None;
        loop {
            let (mut socket, _) = listener.accept().map_err(|error| error.to_string())?;
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
            if head == "fixture complete\r\n\r\n" {
                return schema
                    .ok_or_else(|| "upgrade returned without writing its schema".to_owned());
            }
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
                if schema.is_some() {
                    return Err("upgrade wrote its schema more than once".to_owned());
                }
                schema = Some(request["schema"].as_str().ok_or("no schema")?.to_owned());
            }
        }
    });
    let opened = SpiceDb::open(
        &SpiceDbSettings {
            endpoint: address.clone(),
            key_file: key,
            mirror: "upgrade_fixture".to_owned(),
        },
        &model,
    );
    // The synchronous call has returned: no further request belongs to it.
    // Wake the blocking accept even when the upgrade wrote no schema.
    let completed = TcpStream::connect(&address)
        .and_then(|mut socket| socket.write_all(b"fixture complete\r\n\r\n"));
    let schema = fixture
        .join()
        .map_err(|panic| format!("fixture panicked: {panic:?}"))??;
    completed?;
    opened?;
    assert!(schema.contains("service_account with unexpired"));
    assert_eq!(schema.matches("definition service_account {").count(), 1);
    assert!(schema.contains("definition directory {"));
    assert!(schema.contains("definition fixture/doc {"));
    assert!(schema.contains("person | person with unexpired"));
    // A machine is both the computer a grant is on and, since ACCESS-005, a
    // holder: one definition carries both, as a person's does.
    assert!(schema.contains("machine | machine with unexpired"));
    for kind in ["person", "agent", "machine"] {
        assert_eq!(schema.matches(&format!("definition {kind} {{")).count(), 1);
        assert!(schema.contains(&format!(
            "definition {kind} {{\n  relation viewer: grant#holder\n  permission view = viewer\n}}"
        )), "{schema}");
    }
    Ok(())
}

/// FIXED names exactly the definitions the engine keeps beside the resource
/// kinds: every one in lys-identity's grant schema and the mirror's schema, so
/// none is read back from the engine as a resource kind and written a second
/// time, and nothing else, so no real resource kind is hidden from `kinds()`.
#[test]
fn fixed_names_every_definition_of_the_grant_and_mirror_schemas_and_nothing_else() {
    use std::collections::BTreeSet;
    let defined: BTreeSet<&str> = [lys_identity::grants::SCHEMA, super::MIRROR_SCHEMA]
        .iter()
        .flat_map(|schema| schema.lines())
        .filter_map(|line| line.trim_start().strip_prefix("definition "))
        .filter_map(|rest| rest.split([' ', '{']).next())
        .collect();
    assert!(
        defined.contains("grant") && defined.contains("lys_mirror"),
        "both schemas are read: {defined:?}"
    );
    let fixed: BTreeSet<&str> = super::FIXED.into_iter().collect();
    assert_eq!(
        fixed.len(),
        super::FIXED.len(),
        "FIXED names each definition once"
    );
    let unnamed: Vec<&str> = defined.difference(&fixed).copied().collect();
    assert!(
        unnamed.is_empty(),
        "definitions that FIXED in spicedb.rs does not name: {unnamed:?}"
    );
    let stray: Vec<&str> = fixed.difference(&defined).copied().collect();
    assert!(
        stray.is_empty(),
        "names in FIXED that neither schema defines: {stray:?}"
    );
}
