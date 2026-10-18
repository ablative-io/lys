//! Both polling lanes must arrive before either server request is answered.

use std::error::Error;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, mpsc};

/// How the fixture's two next-poll replies are ordered against the bridge's end.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Order {
    /// Each worker writes its reply as soon as the barrier releases it.
    Free,
    /// The second next-poll reply is written only after the bridge has ended, so it
    /// always meets a lane the bridge closed when the other lane was refused
    /// (AGENTS-011).
    SecondAfterBridgeEnd,
}

#[test]
fn two_next_bridges_are_open_before_either_request_is_answered() -> Result<(), Box<dyn Error>> {
    two_next_bridges(Order::Free)
}

#[test]
fn a_next_reply_written_after_the_bridge_ended_is_the_bridge_s_close_not_the_fixture_s_failure()
-> Result<(), Box<dyn Error>> {
    two_next_bridges(Order::SecondAfterBridgeEnd)
}

/// Writes a next-poll reply to a lane the bridge may already have closed: the
/// bridge, refused on the other lane, closes this one, and a write refused for
/// that reason is the behaviour under test, not the fixture's failure.
fn write_next_reply(stream: &mut std::net::TcpStream, body: &str) -> Result<(), String> {
    let written = write!(
        stream,
        "HTTP/1.1 403 Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .and_then(|()| stream.flush());
    match written {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::BrokenPipe | ErrorKind::ConnectionReset
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn two_next_bridges(order: Order) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let socket = dir.path().join("runner.sock");
    let local = UnixListener::bind(&socket)?;
    let native = std::thread::spawn(move || -> Result<(), String> { serve_native(&local) });
    let tcp = TcpListener::bind("127.0.0.1:0")?;
    let server = format!("http://{}", tcp.local_addr()?);
    let (entered, arrivals) = mpsc::channel();
    let release = Arc::new(Barrier::new(3));
    let released = Arc::clone(&release);
    let bridge_ended = Arc::new(Barrier::new(2));
    let after_end = Arc::clone(&bridge_ended);
    let replies = Arc::new(AtomicUsize::new(0));
    let remote = std::thread::spawn(move || -> Result<(), String> {
        let mut workers = Vec::new();
        for _ in 0..4 {
            let (stream, _) = tcp.accept().map_err(|error| error.to_string())?;
            let entered = entered.clone();
            let released = Arc::clone(&released);
            let after_end = Arc::clone(&after_end);
            let replies = Arc::clone(&replies);
            workers.push(std::thread::spawn(move || -> Result<(), String> {
                let mut stream = BufReader::new(stream);
                let first = request(&mut stream)?;
                if first.starts_with("GET ") {
                    return write!(
                        stream.get_mut(),
                        "HTTP/1.1 200 Fixture\r\nContent-Length: 10\r\nConnection: close\r\n\r\ntest-epoch"
                    )
                    .and_then(|()| stream.get_mut().flush())
                    .map_err(|error| error.to_string());
                }
                if !first.contains("/next ") {
                    return Err("the fixture expected a next poll".to_owned());
                }
                entered.send(()).map_err(|error| error.to_string())?;
                released.wait();
                if order == Order::SecondAfterBridgeEnd && replies.fetch_add(1, Ordering::AcqRel) == 1 {
                    after_end.wait();
                }
                write_next_reply(stream.get_mut(), "{\"refusal\":\"fixture_finished\"}")
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
    if order == Order::SecondAfterBridgeEnd {
        bridge_ended.wait();
    }
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

fn serve_native(local: &UnixListener) -> Result<(), String> {
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
}
