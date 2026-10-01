#![cfg(test)]
//! What one question costs the engine: a schema read is one request, in a
//! scope as outside one, so reading the kinds is one call to `SpiceDB`.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, PoisonError};

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
                .unwrap_or_else(PoisonError::into_inner)
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
        let before = paths.lock().unwrap_or_else(PoisonError::into_inner).len();
        let engine = at(endpoint.clone(), scope);
        assert_eq!(
            engine.kinds()?,
            kinds
                .map(str::to_owned)
                .into_iter()
                .collect::<BTreeSet<_>>(),
            "{scope:?}"
        );
        let heard = paths.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(heard[before..], ["/v1/schema/read"], "{scope:?}");
    }
    Ok(())
}
