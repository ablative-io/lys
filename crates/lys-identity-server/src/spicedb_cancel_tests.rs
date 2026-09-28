#![cfg(test)]
//! A `SpiceDB` call ends on its answer, on a close, or on its request
//! leaving, and never on a clock: each test below ends on a socket, a pipe
//! or a thread's return, and none waits on time.

use std::error::Error;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::os::fd::OwnedFd;
use std::sync::Arc;
use std::thread::JoinHandle;

use lys_identity::grants::GrantError;
use rustix::event::{PollFd, PollFlags};
use rustix::io::Errno;
use rustix::net::{AddressFamily, SocketType};

use super::{Cancel, LEFT, Leaving, current, scope, still_asked};
use crate::spicedb_http::{Answer, post_json};

type TestResult = Result<(), Box<dyn Error>>;

/// A listener's thread, answering the request it read.
type Served = JoinHandle<Result<String, String>>;

const BODY: &str = "{\"schemaText\":\"definition person {}\"}";

fn joined<T>(handle: JoinHandle<T>) -> Result<T, Box<dyn Error>> {
    handle
        .join()
        .map_err(|unwound| format!("a test thread unwound: {unwound:?}").into())
}

/// The whole request `stream` carries: its head and its `Content-Length`
/// body, read until they are in.
fn read_request(stream: &mut TcpStream) -> Result<String, Box<dyn Error>> {
    let mut raw = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err("the request closed before it was whole".into());
        }
        raw.extend_from_slice(&buffer[..count]);
        let text = String::from_utf8_lossy(&raw).into_owned();
        if let Some(split) = text.find("\r\n\r\n") {
            let length = text[..split]
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .ok_or("the request carries no Content-Length")?
                .parse::<usize>()?;
            if raw.len() >= split + 4 + length {
                return Ok(text);
            }
        }
    }
}

/// A call to `authority` on its own thread, inside `cancel`'s scope.
fn asking(authority: String, cancel: Arc<Cancel>) -> JoinHandle<Result<Answer, String>> {
    std::thread::spawn(move || {
        scope(&cancel, || {
            post_json(&authority, "/v1/schema/read", "key", "{}")
        })
    })
}

/// A listener that accepts one call, reads it whole and answers 200 with
/// [`BODY`], answering the request it read.
fn answering() -> Result<(String, Served), Box<dyn Error>> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let authority = listener.local_addr()?.to_string();
    let served = std::thread::spawn(move || {
        let (mut stream, peer) = listener.accept().map_err(|error| error.to_string())?;
        if !peer.ip().is_loopback() {
            return Err(format!("{peer} is not the call"));
        }
        let request = read_request(&mut stream).map_err(|error| error.to_string())?;
        let answer = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{BODY}",
            BODY.len()
        );
        stream
            .write_all(answer.as_bytes())
            .map_err(|error| error.to_string())?;
        Ok(request)
    });
    Ok((authority, served))
}

/// Whether `socket` is writable now: polled, with no timeout, beside a pipe
/// that is already readable, so the poll returns at once and says only
/// what the socket is.
fn writable_now(socket: &OwnedFd) -> Result<bool, Box<dyn Error>> {
    let (ready, mark) = rustix::pipe::pipe()?;
    rustix::io::write(&mark, &[1])?;
    let mut watched = [
        PollFd::new(socket, PollFlags::OUT),
        PollFd::new(&ready, PollFlags::IN),
    ];
    rustix::event::poll(&mut watched, None)?;
    Ok(watched[0].revents().contains(PollFlags::OUT))
}

/// A non-blocking socket whose connect to `address` has begun.
fn probe(address: SocketAddr) -> Result<OwnedFd, Box<dyn Error>> {
    let socket = rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None)?;
    rustix::io::ioctl_fionbio(&socket, true)?;
    match rustix::net::connect(&socket, &address) {
        Ok(()) | Err(Errno::INPROGRESS) => Ok(socket),
        Err(error) => Err(error.into()),
    }
}

#[test]
fn a_cancel_ends_a_call_spicedb_never_answers_and_closes_its_socket() -> TestResult {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let authority = listener.local_addr()?.to_string();
    let cancel = Cancel::new()?;
    let asked = Arc::clone(&cancel);
    let call = asking(authority, asked);
    let (mut accepted, peer) = listener.accept()?;
    assert!(peer.ip().is_loopback(), "{peer}");
    let request = read_request(&mut accepted)?;
    assert!(
        request.starts_with("POST /v1/schema/read HTTP/1.1\r\n"),
        "{request}"
    );
    let canceller = Arc::clone(&cancel);
    joined(std::thread::spawn(move || canceller.cancel()))?;
    assert_eq!(joined(call)?, Err(LEFT.to_owned()));
    let mut after = [0; 16];
    assert_eq!(
        accepted.read(&mut after)?,
        0,
        "the call's socket was not closed"
    );
    Ok(())
}

#[test]
fn a_cancel_ends_a_connect_still_in_progress() -> TestResult {
    let listening = rustix::net::socket(AddressFamily::INET, SocketType::STREAM, None)?;
    rustix::net::bind(&listening, &SocketAddr::from((Ipv4Addr::LOCALHOST, 0)))?;
    rustix::net::listen(&listening, 1)?;
    let listener = TcpListener::from(listening);
    let address = listener.local_addr()?;
    let mut probes = Vec::new();
    for _ in 0..16 {
        probes.push(probe(address)?);
    }
    let cancel = Cancel::new()?;
    let asked = Arc::clone(&cancel);
    let authority = address.to_string();
    let call = asking(authority, asked);
    let behind = probe(address)?;
    assert!(
        !writable_now(&behind)?,
        "the backlog is not full, so a connect is not held in progress"
    );
    assert!(
        !call.is_finished(),
        "the call ended before it was cancelled"
    );
    cancel.cancel();
    assert_eq!(joined(call)?, Err(LEFT.to_owned()));
    assert_eq!(probes.len(), 16);
    Ok(())
}

#[test]
fn cancelling_again_from_two_threads_and_dropping_the_guard_never_blocks() -> TestResult {
    let cancel = Cancel::new()?;
    let guard = Leaving(Arc::clone(&cancel));
    let other = Arc::clone(&cancel);
    scope(&cancel, || -> TestResult {
        let second = std::thread::spawn(move || {
            other.cancel();
            other.cancel();
        });
        cancel.cancel();
        joined(second)?;
        Ok(())
    })?;
    drop(guard);
    assert!(cancel.cancelled());
    assert_eq!(
        rustix::io::ioctl_fionread(cancel.wake())?,
        1,
        "only the first cancel writes to the pipe"
    );
    Ok(())
}

#[test]
fn a_nested_scope_puts_back_the_outer_cancel_when_it_returns_and_when_it_unwinds() -> TestResult {
    let (authority, served) = answering()?;
    let outer = Cancel::new()?;
    let inner = Cancel::new()?;
    let seen = scope(&outer, || -> Result<Answer, Box<dyn Error>> {
        let answered = scope(&inner, || {
            post_json(&authority, "/v1/schema/read", "key", "{}")
        })?;
        let restored = current().ok_or("the outer scope's cancel is gone")?;
        assert!(Arc::ptr_eq(&restored, &outer));
        let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            scope(&inner, || -> u8 {
                std::panic::resume_unwind(Box::new("unwinding"))
            })
        }));
        assert!(unwound.is_err());
        let restored = current().ok_or("the outer scope's cancel is gone after an unwind")?;
        assert!(Arc::ptr_eq(&restored, &outer));
        Ok(answered)
    })?;
    assert_eq!(seen.status, 200);
    assert!(current().is_none());
    joined(served)??;
    Ok(())
}

#[test]
fn a_call_after_its_request_left_opens_no_socket() -> TestResult {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    listener.set_nonblocking(true)?;
    let authority = listener.local_addr()?.to_string();
    let cancel = Cancel::new()?;
    cancel.cancel();
    let answered = scope(&cancel, || {
        post_json(&authority, "/v1/schema/read", "key", "{}")
    });
    assert_eq!(answered, Err(LEFT.to_owned()));
    match listener.accept() {
        Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
        Ok((_, peer)) => Err(format!("the listener accepted {peer}").into()),
        Err(error) => Err(error.into()),
    }
}

#[test]
fn a_call_with_no_scope_reads_the_answer() -> TestResult {
    let (authority, served) = answering()?;
    assert!(current().is_none());
    let answered = post_json(&authority, "/v1/schema/read", "key", "{}")?;
    assert_eq!(
        answered,
        Answer {
            status: 200,
            body: BODY.to_owned()
        }
    );
    let request = joined(served)??;
    assert!(
        request.contains("Authorization: Bearer key\r\n"),
        "{request}"
    );
    Ok(())
}

#[test]
fn a_refused_connect_names_the_address() -> TestResult {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
    let authority = listener.local_addr()?.to_string();
    drop(listener);
    let refused = post_json(&authority, "/v1/schema/read", "key", "{}");
    match refused {
        Err(reason) if reason.starts_with(&format!("{authority} could not be reached")) => Ok(()),
        other => Err(format!("a refused port answered {other:?}").into()),
    }
}

#[test]
fn a_section_whose_request_left_is_refused_as_soon_as_it_asks() -> TestResult {
    assert!(still_asked().is_ok(), "a start, with no scope, is refused");
    let cancel = Cancel::new()?;
    scope(&cancel, || -> TestResult {
        assert!(still_asked().is_ok(), "a request still waiting is refused");
        cancel.cancel();
        match still_asked() {
            Err(GrantError::PermissionEngineUnavailable { reason }) if reason == LEFT => Ok(()),
            other => Err(format!("a request that left was answered {other:?}").into()),
        }
    })
}
