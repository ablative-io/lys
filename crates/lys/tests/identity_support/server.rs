//! HTTP over `std::net`, written apart from `lys`'s own client so the tests
//! read what the services say through a second implementation, and the
//! polling that waits for them.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use super::TestResult;

/// A response: status and body text.
#[derive(Debug)]
pub struct Reply {
    /// The HTTP status.
    pub status: u16,
    /// The body, de-chunked.
    pub body: String,
}

/// Send one request to `address` (`host:port`) and read the whole response.
pub fn request(
    address: &str,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> TestResult<Reply> {
    let mut stream = TcpStream::connect(address)?;
    stream.set_read_timeout(Some(Duration::from_secs(20)))?;
    let payload = body.unwrap_or_default();
    let mut head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAccept-Encoding: identity\r\nContent-Length: {}\r\n",
        payload.len()
    );
    if body.is_some() {
        head.push_str("Content-Type: application/json\r\n");
    }
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(payload.as_bytes())?;
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw)?;
    parse(&raw)
}

/// Parse a complete HTTP/1.1 response.
pub fn parse(raw: &[u8]) -> TestResult<Reply> {
    let text = String::from_utf8_lossy(raw);
    let (head, rest) = text
        .split_once("\r\n\r\n")
        .ok_or("the response has no header terminator")?;
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .ok_or("the response has no status")?
        .parse()?;
    let chunked = head
        .lines()
        .any(|line| line.to_ascii_lowercase().starts_with("transfer-encoding: chunked"));
    let body = if chunked { dechunk(rest)? } else { rest.to_string() };
    Ok(Reply { status, body })
}

fn dechunk(mut rest: &str) -> TestResult<String> {
    let mut body = String::new();
    loop {
        let (size, after) = rest.split_once("\r\n").ok_or("an unterminated chunk size")?;
        let size = usize::from_str_radix(size.split(';').next().unwrap_or_default().trim(), 16)?;
        if size == 0 {
            return Ok(body);
        }
        body.push_str(after.get(..size).ok_or("a truncated chunk")?);
        rest = after.get(size + 2..).ok_or("a truncated chunk")?;
    }
}

/// Poll `ready` once a second until it answers true or `limit` passes.
pub fn wait_for(what: &str, limit: Duration, mut ready: impl FnMut() -> bool) -> TestResult {
    let started = Instant::now();
    while started.elapsed() < limit {
        if ready() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    Err(format!("{what} was not ready within {}s", limit.as_secs()).into())
}

/// Whether Rauthy at `address` reports its database and cache healthy.
pub fn rauthy_healthy(address: &str) -> bool {
    request(address, "GET", "/auth/v1/health", &[], None).is_ok_and(|reply| {
        reply.status == 200 && reply.body.contains("\"db_healthy\":true") && reply.body.contains("\"cache_healthy\":true")
    })
}

/// Whether SpiceDB's HTTP gateway at `address` reports SERVING.
pub fn spicedb_serving(address: &str) -> bool {
    request(address, "GET", "/healthz", &[], None)
        .is_ok_and(|reply| reply.status == 200 && reply.body.contains("SERVING"))
}
