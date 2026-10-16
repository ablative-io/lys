//! Both polling lanes must arrive before either server request is answered.

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, mpsc};

#[test]
fn two_next_bridges_are_open_before_either_request_is_answered() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("runner.sock");
    let local = UnixListener::bind(&socket)?;
    let native = std::thread::spawn(move || -> Result<(), String> {
        let mut workers = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = local.accept().map_err(|error| error.to_string())?;
            workers.push(std::thread::spawn(move || -> Result<(), String> {
                let greeting = crate::protocol::Greeting::fresh("0123456789abcdef0123456789abcdef");
                writeln!(stream, "{}", greeting.line()).map_err(|error| error.to_string())?;
                let mut rest = Vec::new();
                stream
                    .read_to_end(&mut rest)
                    .map_err(|error| error.to_string())?;
                if !rest.is_empty() {
                    return Err("an unaccepted poll sent a native request".to_owned());
                }
                Ok(())
            }));
        }
        for worker in workers {
            worker
                .join()
                .map_err(|panic| format!("native fixture panicked: {panic:?}"))??;
        }
        Ok(())
    });
    let tcp = TcpListener::bind("127.0.0.1:0")?;
    let server = format!("http://{}", tcp.local_addr()?);
    let (entered, arrivals) = mpsc::channel();
    let release = Arc::new(Barrier::new(3));
    let released = Arc::clone(&release);
    let remote = std::thread::spawn(move || -> Result<(), String> {
        let mut workers = Vec::new();
        for _ in 0..4 {
            let (stream, _) = tcp.accept().map_err(|error| error.to_string())?;
            let entered = entered.clone();
            let released = Arc::clone(&released);
            workers.push(std::thread::spawn(move || -> Result<(), String> {
                let mut stream = BufReader::new(stream);
                let mut first = String::new();
                stream.read_line(&mut first).map_err(|error| error.to_string())?;
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    stream.read_line(&mut line).map_err(|error| error.to_string())?;
                    if line == "\r\n" { break; }
                    if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse::<usize>().map_err(|error| error.to_string())?;
                    }
                    if line.is_empty() { return Err("fixture request ended before its head".to_owned()); }
                }
                let mut body = vec![0; length];
                stream.read_exact(&mut body).map_err(|error| error.to_string())?;
                let (status, body) = if first.starts_with("GET ") {
                    (200, "test-epoch")
                } else {
                    if !first.contains("/next ") { return Err("the fixture expected a next poll".to_owned()); }
                    entered.send(()).map_err(|error| error.to_string())?;
                    released.wait();
                    (403, "{\"refusal\":\"fixture_finished\"}")
                };
                write!(stream.get_mut(), "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                    .map_err(|error| error.to_string())?;
                stream.get_mut().flush().map_err(|error| error.to_string())
            }));
        }
        for worker in workers {
            worker
                .join()
                .map_err(|panic| format!("remote fixture panicked: {panic:?}"))??;
        }
        Ok(())
    });
    let dial = super::Dial {
        server,
        machine: "machine".to_owned(),
        socket,
        key: Arc::new(lys_core::Ed25519Identity::ephemeral()),
        authority: None,
    };
    let bridge = std::thread::spawn(move || dial.bridge());
    arrivals.recv()?;
    arrivals.recv()?;
    release.wait();
    let refusal = bridge
        .join()
        .map_err(|panic| format!("bridge panicked: {panic:?}"))?
        .err()
        .ok_or("an admission refusal did not end the bridge")?;
    assert!(
        matches!(refusal, crate::RunnerError::Dial { .. }),
        "{refusal}"
    );
    remote
        .join()
        .map_err(|panic| format!("remote fixture panicked: {panic:?}"))??;
    native
        .join()
        .map_err(|panic| format!("native fixture panicked: {panic:?}"))??;
    dir.close()?;
    Ok(())
}

fn request(stream: &mut BufReader<std::net::TcpStream>) -> Result<String, String> {
    let mut first = String::new();
    stream
        .read_line(&mut first)
        .map_err(|error| error.to_string())?;
    let mut length = 0;
    loop {
        let mut line = String::new();
        stream
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if line == "\r\n" {
            break;
        }
        if line.is_empty() {
            return Err("the request ended before its head".to_owned());
        }
        if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
            length = value
                .trim()
                .parse::<usize>()
                .map_err(|error| error.to_string())?;
        }
    }
    let mut body = vec![0; length];
    stream
        .read_exact(&mut body)
        .map_err(|error| error.to_string())?;
    Ok(first)
}

#[test]
fn a_lost_reply_connection_is_not_a_refusal_and_is_not_replayed() -> Result<(), Box<dyn Error>> {
    let tcp = TcpListener::bind("127.0.0.1:0")?;
    let server = format!("http://{}", tcp.local_addr()?);
    let remote = std::thread::spawn(move || -> Result<(), String> {
        for ticket in ["lost", "following"] {
            let (stream, _) = tcp.accept().map_err(|error| error.to_string())?;
            let mut stream = BufReader::new(stream);
            let line = request(&mut stream)?;
            if !line.contains(&format!("/replies/{ticket} ")) {
                return Err("a reply was replayed instead of the next ticket".to_owned());
            }
            if ticket == "following" {
                stream
                    .get_mut()
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                    )
                    .map_err(|error| error.to_string())?;
            }
            // The first request was received, but its outcome cannot be known by the bridge.
        }
        Ok(())
    });
    let signer = super::Dial {
        server,
        machine: "machine".to_owned(),
        socket: std::path::PathBuf::new(),
        key: Arc::new(lys_core::Ed25519Identity::ephemeral()),
        authority: None,
    }
    .signer()?;
    let finished = AtomicBool::new(false);
    let disconnected = AtomicBool::new(false);
    super::post_reply(&signer, "epoch", "lost", "reply", &finished, &disconnected)?;
    assert!(disconnected.load(Ordering::Acquire));
    assert!(!finished.load(Ordering::Acquire));
    super::post_reply(
        &signer,
        "epoch",
        "following",
        "reply",
        &finished,
        &disconnected,
    )?;
    remote
        .join()
        .map_err(|panic| format!("reply fixture panicked: {panic:?}"))??;
    Ok(())
}
