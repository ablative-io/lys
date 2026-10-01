//! Engine scopes share the credential loaded before serving requests.

use super::{Model, SpiceDb, SpiceDbSettings};
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

fn serve(listener: &TcpListener) -> Result<usize, String> {
    let mut schema = String::new();
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
        let answer = if header.starts_with("POST /v1/schema/read ") {
            json!({"schemaText": schema})
        } else if header.starts_with("POST /v1/schema/write ") {
            schema = request["schema"].as_str().ok_or("no schema")?.to_owned();
            json!({})
        } else if header.starts_with("POST /v1/relationships/delete ") {
            json!({})
        } else {
            return Err("unexpected fixture request".to_owned());
        };
        let body = answer.to_string();
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .map_err(|error| error.to_string())?;
        calls += 1;
    }
}

#[test]
fn scratch_open_and_cleanup_reuse_the_key_after_its_file_disappears() -> Result<(), Box<dyn Error>>
{
    let temporary = tempfile::tempdir()?;
    let key_file = temporary.path().join("key");
    std::fs::write(&key_file, "fixture-only")?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let worker = std::thread::spawn(move || serve(&listener));
    let result = (|| -> Result<(), Box<dyn Error>> {
        let settings = SpiceDbSettings {
            endpoint: endpoint.clone(),
            key_file: key_file.clone(),
            mirror: "fixture".to_owned(),
        };
        let model = model()?;
        let engine = SpiceDb::open(&settings, &model)?;
        std::fs::remove_file(&key_file)?;
        let scratch = SpiceDb::open_scratch(&settings, &model, "lys/bench_one")?;
        scratch.kinds()?;
        SpiceDb::remove_scratch(&settings, "lys/bench_one")?;
        SpiceDb::open_scratch(&settings, &model, "lys/bench_two")?;
        if SpiceDb::clear_scratch(&settings)? != 1 {
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
