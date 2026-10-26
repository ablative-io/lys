#![cfg(test)]

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::Client;

fn request(
    reader: &mut BufReader<TcpStream>,
) -> Result<Option<bool>, Box<dyn Error + Send + Sync>> {
    let mut length = 0;
    let mut close = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        if line == "\r\n" {
            break;
        }
        if line.to_ascii_lowercase().contains("connection: close") {
            close = true;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length: ") {
            length = value.trim().parse()?;
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(Some(close))
}

#[test]
fn two_answers_use_one_accepted_connection() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let accepts = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&accepts);
    let worker = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut answered = 0;
        while answered < 2 {
            let (socket, _) = listener.accept()?;
            counted.fetch_add(1, Ordering::Relaxed);
            let mut reader = BufReader::new(socket);
            while answered < 2 {
                let Some(close) = request(&mut reader)? else {
                    break;
                };
                reader
                    .get_mut()
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")?;
                answered += 1;
                if close {
                    break;
                }
            }
        }
        Ok(())
    });
    let client = Client::default();
    for _ in 0..2 {
        assert_eq!(
            client
                .post_json(&endpoint, "/read", "fixture-bearer", "{}")?
                .body,
            "{}"
        );
    }
    drop(client);
    worker
        .join()
        .map_err(|p| format!("fixture panicked: {p:?}"))?
        .map_err(|e| e.to_string())?;
    assert_eq!(
        accepts.load(Ordering::Relaxed),
        1,
        "two calls keep their connection"
    );
    Ok(())
}

#[test]
fn a_closed_connection_is_replaced() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let worker = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + Sync>> {
        for _ in 0..2 {
            let (socket, _) = listener.accept()?;
            let mut reader = BufReader::new(socket);
            assert!(request(&mut reader)?.is_some());
            reader
                .get_mut()
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")?;
        }
        Ok(())
    });
    let client = Client::default();
    for _ in 0..2 {
        client.post_json(&endpoint, "/read", "fixture-bearer", "{}")?;
    }
    worker
        .join()
        .map_err(|p| format!("fixture panicked: {p:?}"))?
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn a_non_http_answer_names_the_authority_without_the_bearer() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let worker = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + Sync>> {
        let (socket, _) = listener.accept()?;
        let mut reader = BufReader::new(socket);
        request(&mut reader)?;
        reader.get_mut().write_all(b"not HTTP\r\n\r\n")?;
        Ok(())
    });
    let error = Client::default()
        .post_json(&endpoint, "/read", "fixture-bearer", "{}")
        .expect_err("malformed answer must refuse");
    assert!(error.contains(&endpoint));
    assert!(error.contains("not HTTP"));
    assert!(!error.contains("fixture-bearer"));
    worker
        .join()
        .map_err(|p| format!("fixture panicked: {p:?}"))?
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn an_unresponsive_authority_closes_by_name_without_the_bearer() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let endpoint = listener.local_addr()?.to_string();
    let worker = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + Sync>> {
        let (socket, _) = listener.accept()?;
        let mut reader = BufReader::new(socket);
        request(&mut reader)?;
        Ok(())
    });
    let error = Client::default()
        .post_json(&endpoint, "/read", "fixture-bearer", "{}")
        .expect_err("the authority did not answer");
    worker
        .join()
        .map_err(|p| format!("fixture panicked: {p:?}"))?
        .map_err(|e| e.to_string())?;
    assert!(error.contains(&endpoint));
    assert!(error.contains("could not be reached"));
    assert!(!error.contains("fixture-bearer"));
    Ok(())
}
