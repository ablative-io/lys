//! A grants section's `SpiceDB` wait ends when its request leaves, through
//! the served transport: a real TCP client against the service as
//! `axum::serve` serves it, its permission engine a listener that accepts
//! and never answers.
//!
//! Nothing here waits on a clock. Each step ends on a socket: an accept, a
//! read of end of file, or a whole answer. The runtime the service runs in
//! is the test's own, so dropping it waits for every grants section that
//! began, and what the listener holds afterwards is all it will ever hold.

use std::error::Error;
use std::io::{ErrorKind, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::spicedb::SpiceDbSettings;

type TestResult = Result<(), Box<dyn Error>>;

/// The service, judging grants in a permission engine that accepts and
/// never answers, and what a test drives it with.
struct Started {
    runtime: tokio::runtime::Runtime,
    service: Service,
    cookie: String,
    address: String,
    spicedb: TcpListener,
    keys: tempfile::TempDir,
}

impl Started {
    fn new() -> Result<Self, Box<dyn Error>> {
        let spicedb = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let keys = tempfile::TempDir::new()?;
        let key_file = keys.path().join("spicedb.key");
        std::fs::write(&key_file, "spicedb-cancel-test-key")?;
        let settings = SpiceDbSettings {
            endpoint: spicedb.local_addr()?.to_string(),
            key_file,
            mirror: "test".to_owned(),
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        let (service, ()) = runtime.block_on(Service::start_judging(
            GRANT_MODEL,
            Some(settings),
            |config| {
                seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
                Ok(())
            },
        ))?;
        let cookie = runtime.block_on(service.sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "administrator@example.test".to_owned(),
        }))?;
        let address = service
            .base
            .strip_prefix("http://")
            .ok_or("the service's base is not http")?
            .to_owned();
        Ok(Self {
            runtime,
            service,
            cookie,
            address,
            spicedb,
            keys,
        })
    }

    /// A connection that has sent `GET /grants`, a route that checks the
    /// grants and so takes the grants lock and opens the grants on first use.
    fn ask(&self) -> Result<TcpStream, Box<dyn Error>> {
        let mut stream = TcpStream::connect(&self.address)?;
        let request = format!(
            "GET /grants HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\n\r\n",
            self.address, self.cookie
        );
        stream.write_all(request.as_bytes())?;
        Ok(stream)
    }

    /// The whole answer to `GET /grants/model`, a route that takes no
    /// lock, over a connection accepted after every one before it.
    fn answered_after(&self) -> Result<String, Box<dyn Error>> {
        let mut stream = TcpStream::connect(&self.address)?;
        let request = format!(
            "GET /grants/model HTTP/1.1\r\nHost: {}\r\nCookie: {}\r\nConnection: close\r\n\r\n",
            self.address, self.cookie
        );
        stream.write_all(request.as_bytes())?;
        let mut answer = String::new();
        stream.read_to_string(&mut answer)?;
        Ok(answer)
    }

    /// The call the permission engine accepts next, read until its request
    /// is whole, so the section that made it is waiting on the answer.
    fn next_call(&self) -> Result<TcpStream, Box<dyn Error>> {
        let (mut call, peer) = self.spicedb.accept()?;
        assert!(peer.ip().is_loopback(), "{peer}");
        let request = read_request(&mut call)?;
        assert!(
            request.starts_with("POST /v1/schema/read HTTP/1.1\r\n"),
            "{request}"
        );
        Ok(call)
    }

    /// Stop the service and drop its runtime, which waits for every grants
    /// section that began, and answer the permission engine's listener.
    fn stopped(self) -> TcpListener {
        drop(self.service);
        drop(self.runtime);
        drop(self.keys);
        self.spicedb
    }
}

/// The whole request `stream` carries: its head and its `Content-Length`
/// body.
fn read_request(stream: &mut TcpStream) -> Result<String, Box<dyn Error>> {
    let mut raw = Vec::new();
    let mut buffer = [0; 1024];
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err("the call closed before its request was whole".into());
        }
        raw.extend_from_slice(&buffer[..count]);
        let text = String::from_utf8_lossy(&raw).into_owned();
        if let Some(split) = text.find("\r\n\r\n") {
            let length = text[..split]
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .ok_or("the call carries no Content-Length")?
                .parse::<usize>()?;
            if raw.len() >= split + 4 + length {
                return Ok(text);
            }
        }
    }
}

/// Whether `call` reads end of file, as it does once the section that made
/// it closed its socket.
fn ends(call: &mut TcpStream) -> Result<bool, Box<dyn Error>> {
    let mut rest = [0; 64];
    Ok(call.read(&mut rest)? == 0)
}

#[test]
fn closing_the_connection_ends_the_spicedb_wait_and_lets_the_grants_lock_go() -> TestResult {
    let started = Started::new()?;
    let first = started.ask()?;
    let mut waiting = started.next_call()?;
    drop(first);
    assert!(
        ends(&mut waiting)?,
        "the call went on after its request left"
    );
    let second = started.ask()?;
    let mut again = started.next_call()?;
    drop(second);
    assert!(
        ends(&mut again)?,
        "the second call went on after its request left"
    );
    let spicedb = started.stopped();
    spicedb.set_nonblocking(true)?;
    match spicedb.accept() {
        Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
        Ok((_, peer)) => Err(format!("a third call came from {peer}").into()),
        Err(error) => Err(error.into()),
    }
}

#[test]
fn a_request_that_left_while_queued_behind_the_grants_lock_calls_no_spicedb() -> TestResult {
    let started = Started::new()?;
    let holding = started.ask()?;
    let mut held = started.next_call()?;
    let queued = started.ask()?;
    drop(queued);
    let answer = started.answered_after()?;
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    drop(holding);
    assert!(
        ends(&mut held)?,
        "the holding call went on after its request left"
    );
    let spicedb = started.stopped();
    spicedb.set_nonblocking(true)?;
    match spicedb.accept() {
        Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(()),
        Ok((_, peer)) => Err(format!("the request that left called SpiceDB from {peer}").into()),
        Err(error) => Err(error.into()),
    }
}
