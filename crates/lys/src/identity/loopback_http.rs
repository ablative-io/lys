//! One HTTP/1.1 exchange with a service on this machine, shared by the
//! Rauthy admin client and the readiness checks.
//!
//! An address is `host:port` or `http://host[:port]`, with an IPv6 host in
//! brackets; the port is the scheme's default only when it is absent. Every
//! address the host resolves to is tried in turn, so a name that resolves to
//! `::1` first still reaches a service published only on `127.0.0.1`. A
//! response is read to the end and its status line parsed field by field.
//!
//! No exchange is bounded by a clock. A refused or reset connection is an
//! error at once, naming the address and the cause; a peer that accepts and
//! never answers is waited on for as long as it is alive, because a stuck
//! service is found by its signal, never cut off by a wait in seconds. A
//! peer that closes before its answer is whole is an error naming how many
//! bytes arrived.

use std::fmt;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use zeroize::Zeroizing;

/// The port of a plain `http://` address that names none.
pub const HTTP_DEFAULT_PORT: u16 = 80;

/// A host and the port it is reached on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    /// The host, without IPv6 brackets.
    pub host: String,
    /// The port.
    pub port: u16,
}

impl Authority {
    /// Parses `host:port` or `[v6]:port`; `default_port` stands in for an
    /// absent port, and with none an absent port is refused.
    pub fn parse(authority: &str, default_port: Option<u16>) -> Result<Self, &'static str> {
        let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
            let (host, after) = rest
                .split_once(']')
                .ok_or("an IPv6 host is closed with ]")?;
            let port = match after {
                "" => None,
                other => Some(other.strip_prefix(':').ok_or("expected ]:port")?),
            };
            (host, port)
        } else {
            match authority.split_once(':') {
                Some((host, port)) if !port.contains(':') => (host, Some(port)),
                Some(_) => return Err("an IPv6 host is written in brackets"),
                None => (authority, None),
            }
        };
        if host.is_empty() {
            return Err("expected a host");
        }
        let port = match port {
            Some(text) => text
                .parse::<u16>()
                .ok()
                .filter(|port| *port > 0)
                .ok_or("the port is not a number from 1 to 65535")?,
            None => default_port.ok_or("expected host:port")?,
        };
        Ok(Self {
            host: host.to_string(),
            port,
        })
    }

    /// Parses `http://host[:port]`, with or without a closing slash.
    pub fn from_http_url(url: &str) -> Result<Self, &'static str> {
        let authority = url
            .strip_prefix("http://")
            .map(|rest| rest.strip_suffix('/').unwrap_or(rest))
            .ok_or("expected http://host:port")?;
        Self::parse(authority, Some(HTTP_DEFAULT_PORT))
    }

    /// Opens a connection, trying every address the host resolves to.
    pub fn connect(&self) -> Result<TcpStream, String> {
        let addresses: Vec<SocketAddr> = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|error| format!("{self}: {error}"))?
            .collect();
        if addresses.is_empty() {
            return Err(format!("{self} resolves to no address"));
        }
        connect_any(&addresses)
    }
}

impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            write!(f, "[{}]:{}", self.host, self.port)
        } else {
            write!(f, "{}:{}", self.host, self.port)
        }
    }
}

/// Connects to the first of `addresses` that accepts. A refusal is known
/// at once and names every address tried with its cause.
pub fn connect_any(addresses: &[SocketAddr]) -> Result<TcpStream, String> {
    let mut refused = Vec::with_capacity(addresses.len());
    for address in addresses {
        match TcpStream::connect(address) {
            Ok(stream) => return Ok(stream),
            Err(error) => refused.push(format!("{address}: {error}")),
        }
    }
    Err(refused.join("; "))
}

/// One request. Header values may carry a credential, so the request is
/// assembled in memory that is wiped when dropped.
pub struct Request<'a> {
    /// The method.
    pub method: &'a str,
    /// The path and query.
    pub path: &'a str,
    /// Extra headers, by name and value.
    pub headers: &'a [(&'a str, &'a [u8])],
    /// The body, sent with its length.
    pub body: &'a [u8],
}

/// A whole response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The status code.
    pub status: u16,
    /// The body, de-chunked.
    pub body: Vec<u8>,
}

/// Why an exchange produced no response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// No connection was made; the request was never sent.
    Unreachable(String),
    /// The request may have been sent and no whole response arrived.
    Uncertain(String),
    /// A response arrived that is not a well-formed HTTP/1.x response, or
    /// the peer closed before it was whole; the detail says how many bytes
    /// arrived.
    Malformed(String),
}

/// Sends `request` to `authority` and reads the whole response.
pub fn exchange(authority: &Authority, request: &Request<'_>) -> Result<Response, Failure> {
    let mut stream = authority.connect().map_err(Failure::Unreachable)?;
    let mut raw = Zeroizing::new(Vec::new());
    raw.extend_from_slice(
        format!(
            "{} {} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n",
            request.method, request.path
        )
        .as_bytes(),
    );
    for (name, value) in request.headers {
        raw.extend_from_slice(name.as_bytes());
        raw.extend_from_slice(b": ");
        raw.extend_from_slice(value);
        raw.extend_from_slice(b"\r\n");
    }
    raw.extend_from_slice(format!("Content-Length: {}\r\n\r\n", request.body.len()).as_bytes());
    raw.extend_from_slice(request.body);
    stream
        .write_all(&raw)
        .and_then(|()| stream.flush())
        .map_err(|error| Failure::Uncertain(format!("writing the request: {error}")))?;
    let mut answer = Vec::new();
    let read = stream.read_to_end(&mut answer);
    let arrived = answer.len();
    read.map_err(|error| {
        Failure::Uncertain(format!("reading the response after {arrived} bytes: {error}"))
    })?;
    parse_response(&answer).map_err(|detail| {
        Failure::Malformed(format!("{detail}: the peer closed after {arrived} bytes"))
    })
}

/// The status code of an HTTP/1.x status line: the version, one space and
/// exactly three digits, then the end or a space and the reason.
pub fn parse_status_line(line: &str) -> Result<u16, &'static str> {
    let rest = line
        .strip_prefix("HTTP/1.1 ")
        .or_else(|| line.strip_prefix("HTTP/1.0 "))
        .ok_or("the answer is not HTTP/1.x")?;
    let (code, reason) = rest.split_at_checked(3).ok_or("the status is cut short")?;
    if !code.bytes().all(|byte| byte.is_ascii_digit())
        || !(reason.is_empty() || reason.starts_with(' '))
    {
        return Err("the status is not three digits");
    }
    code.parse::<u16>()
        .ok()
        .filter(|status| (100..=599).contains(status))
        .ok_or("the status is outside 100 to 599")
}

/// Parses a whole response: status line, headers, and the body by its
/// declared length or chunked encoding.
pub fn parse_response(raw: &[u8]) -> Result<Response, &'static str> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("the response was cut short")?;
    let head = std::str::from_utf8(&raw[..split])
        .ok()
        .ok_or("the head is not text")?;
    let rest = &raw[split + 4..];
    let mut lines = head.split("\r\n");
    let status = parse_status_line(lines.next().unwrap_or_default())?;
    let mut chunked = false;
    let mut length = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or("a header has no colon")?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") && value.eq_ignore_ascii_case("chunked") {
            chunked = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            length = Some(
                value
                    .parse::<usize>()
                    .ok()
                    .ok_or("the length is not a number")?,
            );
        }
    }
    let body = if chunked {
        dechunk(rest).ok_or("the chunked body was cut short")?
    } else if let Some(length) = length {
        rest.get(..length).ok_or("the body was cut short")?.to_vec()
    } else {
        rest.to_vec()
    };
    Ok(Response { status, body })
}

fn dechunk(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line_end = rest.windows(2).position(|window| window == b"\r\n")?;
        let size_text = std::str::from_utf8(&rest[..line_end]).ok()?;
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}

#[cfg(test)]
#[path = "loopback_http_tests.rs"]
mod tests;
