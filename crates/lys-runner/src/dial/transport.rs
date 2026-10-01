//! Bounded HTTP responses over persistent long-poll and control connections.
//! TLS verifies the public roots and any supplied authority; cleartext stays
//! on loopback. Kernel keepalive reports a lost peer without a polling thread.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use super::{STALE, failed};
use crate::error::RunnerError;

const BODY_LIMIT: usize = 1_048_576;
const HEADER_LIMIT: usize = 16_384;

enum Stream {
    Plain(TcpStream),
    Tls(Box<StreamOwned<ClientConnection, TcpStream>>),
}

impl Read for Stream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(bytes),
            Self::Tls(stream) => stream.read(bytes),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(bytes),
            Self::Tls(stream) => stream.write(bytes),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Tls(stream) => stream.flush(),
        }
    }
}

/// How the server is reached.
enum Scheme {
    /// TLS, as `https://`.
    Tls {
        config: Arc<ClientConfig>,
        name: ServerName<'static>,
    },
    /// Cleartext, to this machine's loopback address only.
    Loopback,
}

/// The server a bridge dials, and how.
pub(super) struct Channel {
    scheme: Scheme,
    authority: String,
    prefix: String,
    poll: Mutex<Option<BufReader<Stream>>>,
    control: Mutex<Option<BufReader<Stream>>>,
}

/// A response: its status, its headers and its body.
pub(super) struct Response {
    pub(super) status: u16,
    headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl Response {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(named, _)| named.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// The host `authority` names, without its port.
fn host(authority: &str) -> &str {
    match authority.strip_prefix('[') {
        Some(bracketed) => bracketed
            .split_once(']')
            .map_or(bracketed, |(host, _)| host),
        None => authority
            .rsplit_once(':')
            .map_or(authority, |(host, _port)| host),
    }
}

fn loopback(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// The public roots, and the certificates in `authority` when given.
fn roots(authority: Option<&Path>) -> Result<RootCertStore, RunnerError> {
    let mut roots: RootCertStore = webpki_roots::TLS_SERVER_ROOTS.iter().cloned().collect();
    if let Some(path) = authority {
        let named = |error: &dyn std::fmt::Display| {
            failed(format!(
                "the authority {} does not read: {error}",
                path.display()
            ))
        };
        let mut added = 0_usize;
        for certificate in CertificateDer::pem_file_iter(path).map_err(|error| named(&error))? {
            roots
                .add(certificate.map_err(|error| named(&error))?)
                .map_err(|error| named(&error))?;
            added += 1;
        }
        if added == 0 {
            return Err(named(&"it holds no certificate"));
        }
    }
    Ok(roots)
}

impl Channel {
    /// The channel to `server`, refused by name when it is neither
    /// `https://` nor `http://` to this machine's loopback address.
    pub(super) fn for_server(server: &str, authority: Option<&Path>) -> Result<Self, RunnerError> {
        let (tls, rest) = if let Some(rest) = server.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = server.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(failed(format!(
                "the server's address {server} is not https://host:port"
            )));
        };
        let (authority_text, prefix) = rest.split_once('/').unwrap_or((rest, ""));
        if authority_text.contains('@') {
            return Err(failed(format!(
                "the server's address {server} names a user: the host checked is the host dialled, so none is taken"
            )));
        }
        let host = host(authority_text);
        let scheme = if tls {
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let config = ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(|error| failed(format!("TLS could not be set up: {error}")))?
                .with_root_certificates(roots(authority)?)
                .with_no_client_auth();
            let name = ServerName::try_from(host.to_owned())
                .map_err(|error| failed(format!("{host} is not a server name: {error}")))?;
            Scheme::Tls {
                config: Arc::new(config),
                name,
            }
        } else if loopback(host) {
            Scheme::Loopback
        } else {
            return Err(failed(format!(
                "cleartext http:// is spoken only to this machine's loopback address, and {host} is not it: dial https://"
            )));
        };
        Ok(Self {
            scheme,
            authority: authority_text.to_owned(),
            prefix: prefix.trim_end_matches('/').to_owned(),
            poll: Mutex::new(None),
            control: Mutex::new(None),
        })
    }

    /// Send one request and read its whole response, refused by name when
    /// the server answers anything but 200.
    pub(super) fn path(&self, route: &str) -> String {
        if self.prefix.is_empty() {
            route.to_owned()
        } else {
            format!("/{}{route}", self.prefix)
        }
    }
    pub(super) fn send(
        &self,
        method: &str,
        route: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<Response, RunnerError> {
        let response = self.send_response(method, route, headers, body)?;
        if response.status == 200 {
            return Ok(response);
        }
        let words = String::from_utf8_lossy(&response.body).into_owned();
        let refusal = serde_json::from_str::<serde_json::Value>(&words)
            .ok()
            .and_then(|value| value["refusal"].as_str().map(str::to_owned));
        if refusal.as_deref() == Some(STALE) {
            return Err(RunnerError::DialStale { reason: words });
        }
        Err(failed(format!(
            "the server answered {}: {words}",
            response.status
        )))
    }
    pub(super) fn send_response(
        &self,
        method: &str,
        route: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<Response, RunnerError> {
        let path = self.path(route);
        validate_request(method, &path, headers)?;
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: keep-alive\r\n",
            self.authority,
            body.len()
        );
        for (name, value) in headers {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let lane = if route.starts_with("/runner/dial/") && route.ends_with("/next") {
            &self.poll
        } else {
            &self.control
        };
        let mut connection_guard = lane
            .lock()
            .map_err(|error| failed(format!("the connection lock is poisoned: {error}")))?;
        let mut stream = match connection_guard.take() {
            Some(stream) => stream,
            None => self.connect()?,
        };
        // A failed exchange is never replayed: a POST may already be applied.
        let (response, reusable) = exchange(&mut stream, head.as_bytes(), body)?;
        if reusable {
            *connection_guard = Some(stream);
        }
        drop(connection_guard);
        Ok(response)
    }

    fn connect(&self) -> Result<BufReader<Stream>, RunnerError> {
        let stream = TcpStream::connect(&self.authority)
            .map_err(|error| failed(format!("{} could not be reached: {error}", self.authority)))?;
        keepalive(&stream)?;
        let stream = match &self.scheme {
            Scheme::Loopback => Stream::Plain(stream),
            Scheme::Tls { config, name } => {
                let connection = ClientConnection::new(Arc::clone(config), name.clone())
                    .map_err(|error| failed(format!("TLS could not begin: {error}")))?;
                Stream::Tls(Box::new(StreamOwned::new(connection, stream)))
            }
        };
        Ok(BufReader::new(stream))
    }
}

fn keepalive(stream: &TcpStream) -> Result<(), RunnerError> {
    rustix::net::sockopt::set_socket_keepalive(stream, true)
        .and_then(|()| rustix::net::sockopt::set_tcp_keepidle(stream, Duration::from_secs(60)))
        .and_then(|()| rustix::net::sockopt::set_tcp_keepintvl(stream, Duration::from_secs(10)))
        .and_then(|()| rustix::net::sockopt::set_tcp_keepcnt(stream, 3))
        .map_err(|error| RunnerError::refused("runner_dial_keepalive_failed", error.to_string()))
}

fn token(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
}

pub(super) fn validate_request(
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
) -> Result<(), RunnerError> {
    if !token(method)
        || !path.starts_with('/')
        || path.bytes().any(|byte| byte <= b' ' || byte == 127)
    {
        return Err(failed("the request method or path is invalid"));
    }
    for (name, value) in headers {
        if !token(name)
            || value.bytes().any(|byte| byte < b' ' || byte == 127)
            || ["host", "content-length", "transfer-encoding", "connection"]
                .iter()
                .any(|reserved| name.eq_ignore_ascii_case(reserved))
        {
            return Err(failed("the request header is invalid or overrides framing"));
        }
    }
    Ok(())
}

fn exchange(
    stream: &mut BufReader<Stream>,
    head: &[u8],
    body: &[u8],
) -> Result<(Response, bool), RunnerError> {
    stream
        .get_mut()
        .write_all(head)
        .and_then(|()| stream.get_mut().write_all(body))
        .and_then(|()| stream.get_mut().flush())
        .map_err(|error| failed(format!("the request was not sent: {error}")))?;
    read_response(stream)
}

fn too_large() -> RunnerError {
    RunnerError::refused(
        "runner_dial_response_too_large",
        "the response exceeds 1048576 bytes",
    )
}

fn line(stream: &mut impl BufRead, remaining: &mut usize) -> Result<String, RunnerError> {
    let mut bytes = Vec::new();
    stream
        .take(u64::try_from(*remaining).map_err(|error| failed(error.to_string()))? + 1)
        .read_until(b'\n', &mut bytes)
        .map_err(|error| failed(format!("response metadata was not read: {error}")))?;
    if bytes.len() > *remaining {
        return Err(RunnerError::refused(
            "runner_dial_headers_too_large",
            "response metadata exceeds 16384 bytes",
        ));
    }
    *remaining -= bytes.len();
    let bytes = bytes
        .strip_suffix(b"\r\n")
        .ok_or_else(|| failed("response metadata has no complete line"))?;
    String::from_utf8(bytes.to_vec())
        .map_err(|error| failed(format!("response metadata is not UTF-8: {error}")))
}

fn header(value: &str) -> Result<(String, String), RunnerError> {
    let (name, value) = value
        .split_once(':')
        .ok_or_else(|| failed("a response header has no colon"))?;
    if !token(name)
        || value
            .bytes()
            .any(|byte| (byte < b' ' && byte != b'\t') || byte == 127)
    {
        return Err(failed("a response header is invalid"));
    }
    Ok((name.to_owned(), value.trim().to_owned()))
}

fn response_head(
    stream: &mut impl BufRead,
    remaining: &mut usize,
) -> Result<(Response, bool), RunnerError> {
    let first = line(stream, remaining)?;
    let mut parts = first.splitn(3, ' ');
    let version = parts
        .next()
        .ok_or_else(|| failed("the answer has no HTTP version"))?;
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(failed("the answer has an unsupported HTTP version"));
    }
    let code = parts
        .next()
        .ok_or_else(|| failed("the answer has no status"))?;
    let status = code
        .parse::<u16>()
        .map_err(|error| failed(format!("the status is invalid: {error}")))?;
    if code.len() != 3 || !(200..=599).contains(&status) {
        return Err(failed("the answer has an unsupported status"));
    }
    let mut headers = Vec::new();
    loop {
        let value = line(stream, remaining)?;
        if value.is_empty() {
            break;
        }
        headers.push(header(&value)?);
    }
    let response = Response {
        status,
        headers,
        body: Vec::new(),
    };
    let mut reusable = version == "HTTP/1.1";
    for (_, value) in response
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("connection"))
    {
        if value
            .split(',')
            .any(|word| word.trim().eq_ignore_ascii_case("keep-alive"))
        {
            reusable = true;
        }
    }
    if response
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("connection"))
        .any(|(_, value)| {
            value
                .split(',')
                .any(|word| word.trim().eq_ignore_ascii_case("close"))
        })
    {
        reusable = false;
    }
    Ok((response, reusable))
}

fn framing(response: &Response) -> Result<(Option<usize>, bool), RunnerError> {
    let mut length = None;
    let mut chunked = false;
    for (name, value) in &response.headers {
        if name.eq_ignore_ascii_case("content-length") {
            if length.is_some()
                || value.is_empty()
                || !value.bytes().all(|byte| byte.is_ascii_digit())
            {
                return Err(failed("the content length is repeated or invalid"));
            }
            let given = value
                .parse::<usize>()
                .map_err(|error| failed(format!("the content length is invalid: {error}")))?;
            if given > BODY_LIMIT {
                return Err(too_large());
            }
            length = Some(given);
        }
        if name.eq_ignore_ascii_case("transfer-encoding") {
            if chunked || !value.eq_ignore_ascii_case("chunked") {
                return Err(failed("the transfer encoding is repeated or unsupported"));
            }
            chunked = true;
        }
    }
    if chunked && length.is_some() {
        return Err(failed("the answer has ambiguous body framing"));
    }
    Ok((length, chunked))
}

fn read_response(stream: &mut impl BufRead) -> Result<(Response, bool), RunnerError> {
    let mut remaining = HEADER_LIMIT;
    let (mut response, mut reusable) = response_head(stream, &mut remaining)?;
    let (length, chunked) = framing(&response)?;
    response.body = if response.status == 204 || response.status == 304 {
        if chunked || length.is_some_and(|length| length != 0) {
            return Err(failed("a bodyless response declares a body"));
        }
        Vec::new()
    } else if chunked {
        chunks(stream, &mut remaining)?
    } else if let Some(length) = length {
        let mut body = vec![0; length];
        stream
            .read_exact(&mut body)
            .map_err(|error| failed(format!("the response body is incomplete: {error}")))?;
        body
    } else {
        if reusable {
            return Err(failed("a persistent response has no body framing"));
        }
        reusable = false;
        let mut body = Vec::new();
        stream
            .take(1_048_577)
            .read_to_end(&mut body)
            .map_err(|error| failed(format!("the response body was not read: {error}")))?;
        if body.len() > BODY_LIMIT {
            return Err(too_large());
        }
        body
    };
    Ok((response, reusable))
}

fn chunks(stream: &mut impl BufRead, remaining: &mut usize) -> Result<Vec<u8>, RunnerError> {
    let mut body = Vec::new();
    loop {
        let value = line(stream, remaining)?;
        let digits = value.split(';').next().unwrap_or_default();
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(failed("a chunk size is invalid"));
        }
        let size = usize::from_str_radix(digits, 16)
            .map_err(|error| failed(format!("a chunk size is invalid: {error}")))?;
        if size > BODY_LIMIT - body.len() {
            return Err(too_large());
        }
        if size == 0 {
            loop {
                let trailer = line(stream, remaining)?;
                if trailer.is_empty() {
                    return Ok(body);
                }
                let (name, _) = header(&trailer)?;
                if ["content-length", "transfer-encoding", "connection"]
                    .iter()
                    .any(|reserved| name.eq_ignore_ascii_case(reserved))
                {
                    return Err(failed("a trailer overrides response framing"));
                }
            }
        }
        let start = body.len();
        body.resize(start + size, 0);
        stream
            .read_exact(&mut body[start..])
            .map_err(|error| failed(format!("a chunk is incomplete: {error}")))?;
        let mut ending = [0; 2];
        stream
            .read_exact(&mut ending)
            .map_err(|error| failed(format!("a chunk ending is incomplete: {error}")))?;
        if ending != *b"\r\n" {
            return Err(failed("a chunk does not end its line"));
        }
    }
}

#[cfg(test)]
fn parse(raw: &[u8]) -> Result<Response, RunnerError> {
    read_response(&mut std::io::Cursor::new(raw)).map(|(response, _)| response)
}

#[cfg(test)]
#[path = "../../tests/transport/cases.rs"]
mod tests;
