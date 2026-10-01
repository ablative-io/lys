#![cfg(test)]
//! DIRECTORY-029 R6: the door's handle-record client, read against a local
//! stub answering the read shape of the door's handle-resolving endpoint. It
//! answers ids and validity, that no record exists, or the refusal of an
//! answer carrying a credential value, and the credentials check built over
//! it answers the same (CONFORMANCE 5.2, 5.3).

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential, check};
use lys_identity_server::routes::door_handles::DoorHandles;

type TestResult = Result<(), Box<dyn Error>>;

/// The value the stub plants in vc-fixture-1's record in one case.
const VALUE_ONE: &str = "fixture-credential-value-1f3a";

/// A local stub of the door's handle read: it answers whichever case the
/// test set, and counts the requests each case answered.
struct Stub {
    address: String,
    case: Arc<Mutex<&'static str>>,
    served: Arc<Mutex<BTreeMap<&'static str, usize>>>,
}

fn answer_for(case: &str) -> (u16, String) {
    match case {
        "active" => (
            200,
            r#"{"holder":"agent-fixture-1","handles":[{"id":"vc-fixture-1","state":"active"}]}"#
                .to_owned(),
        ),
        "revoked" => (
            200,
            r#"{"holder":"agent-fixture-1","handles":[{"id":"vc-fixture-1","state":"revoked"}]}"#
                .to_owned(),
        ),
        "missing" => (404, r#"{"error":"handle_record_missing"}"#.to_owned()),
        _ => (
            200,
            format!(
                r#"{{"holder":"agent-fixture-1","handles":[{{"id":"vc-fixture-1","state":"active","value":"{VALUE_ONE}"}}]}}"#
            ),
        ),
    }
}

impl Stub {
    fn start() -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = format!("http://{}", listener.local_addr()?);
        let case = Arc::new(Mutex::new("active"));
        let served = Arc::new(Mutex::new(BTreeMap::new()));
        let (set, count) = (Arc::clone(&case), Arc::clone(&served));
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut request = Vec::new();
                let mut byte = [0_u8; 1];
                while !request.ends_with(b"\r\n\r\n") {
                    match stream.read(&mut byte) {
                        Ok(1) => request.push(byte[0]),
                        _ => break,
                    }
                }
                let head = String::from_utf8_lossy(&request);
                let asked =
                    head.starts_with("GET /secrets/handles?holder=agent-fixture-1 HTTP/1.1\r\n");
                let current = *set.lock().expect("fixture lock poisoned");
                let (status, body) = if asked {
                    *count
                        .lock()
                        .expect("fixture lock poisoned")
                        .entry(current)
                        .or_insert(0) += 1;
                    answer_for(current)
                } else {
                    (400, String::new())
                };
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                if let Err(error) = stream.write_all(reply.as_bytes()) {
                    eprintln!("the stub's reply was not written: {error}");
                }
            }
        });
        Ok(Self {
            address,
            case,
            served,
        })
    }

    fn set(&self, case: &'static str) {
        *self.case.lock().expect("fixture lock poisoned") = case;
    }

    fn served(&self, case: &str) -> usize {
        self.served
            .lock()
            .expect("fixture lock poisoned")
            .get(case)
            .copied()
            .unwrap_or(0)
    }
}

#[test]
fn the_client_reads_the_doors_handle_records() -> TestResult {
    let stub = Stub::start()?;
    let client = DoorHandles::at(&stub.address)?;
    let mut cases = 0;

    stub.set("active");
    assert_eq!(
        client.handles("agent-fixture-1"),
        HandleAnswer::Held(vec![HeldCredential {
            id: "vc-fixture-1".to_owned(),
            valid: true,
        }])
    );
    assert_eq!(
        check(&client, "agent-fixture-1")?,
        Ok(vec!["vc-fixture-1".to_owned()])
    );
    cases += 1;

    stub.set("revoked");
    assert_eq!(
        client.handles("agent-fixture-1"),
        HandleAnswer::Held(vec![HeldCredential {
            id: "vc-fixture-1".to_owned(),
            valid: false,
        }])
    );
    let refusal = check(&client, "agent-fixture-1")?.expect_err("revoked is not valid");
    assert_eq!(refusal.name(), "virtual_credentials_not_valid");
    cases += 1;

    stub.set("missing");
    assert_eq!(
        client.handles("agent-fixture-1"),
        HandleAnswer::RecordMissing
    );
    let refusal = check(&client, "agent-fixture-1")?.expect_err("no record exists");
    assert_eq!(refusal.name(), "check_record_missing");
    assert!(refusal.to_string().contains("SECRETS-002"), "{refusal}");
    cases += 1;

    stub.set("value");
    let answer = client.handles("agent-fixture-1");
    let refused = answer
        .refusal()
        .ok_or("an answer carrying a value is refused")?;
    assert_eq!(refused.name(), "credential_value_in_answer");
    let words = refused.to_string();
    assert!(
        words.contains("vc-fixture-1") && words.contains("value"),
        "{words}"
    );
    assert!(
        !matches!(answer, HandleAnswer::Held(_)),
        "no credential id is returned"
    );
    let shown = format!("{words} {refused:?} {answer:?}");
    assert!(!shown.contains(VALUE_ONE), "{shown}");
    let refusal = check(&client, "agent-fixture-1")?.expect_err("a value in the answer");
    assert_eq!(refusal.name(), "credential_value_in_answer");
    assert!(!format!("{refusal} {refusal:?}").contains(VALUE_ONE));
    cases += 1;

    assert_eq!(cases, 4);
    for case in ["active", "revoked", "missing", "value"] {
        assert_eq!(stub.served(case), 2, "the {case} case");
    }
    Ok(())
}

#[test]
fn a_client_with_no_door_reads_no_record() {
    assert_eq!(
        DoorHandles::unconfigured().handles("agent-fixture-1"),
        HandleAnswer::RecordMissing
    );
}
