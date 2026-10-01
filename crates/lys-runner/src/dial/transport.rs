//! The bridge's channel to its server: one HTTP/1.1 exchange per
//! connection, over TLS for an `https://` server, verified against the
//! public roots and any authority the bridge is given, and in cleartext
//! only to this machine's own loopback address. The exchange ends when the
//! server closes the connection, never on a clock.

use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream};
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use super::{STALE, failed};
use crate::error::RunnerError;

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
}

/// A response: its status, its headers and its body.
pub(super) struct Response {
    status: u16,
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
        })
    }

    /// Send one request and read its whole response, refused by name when
    /// the server answers anything but 200.
    pub(super) fn send(
        &self,
        method: &str,
        route: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<Response, RunnerError> {
        let path = if self.prefix.is_empty() {
            route.to_owned()
        } else {
            format!("/{}{route}", self.prefix)
        };
        let mut head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n",
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
        let stream = TcpStream::connect(&self.authority)
            .map_err(|error| failed(format!("{} could not be reached: {error}", self.authority)))?;
        let raw = match &self.scheme {
            Scheme::Loopback => exchange(stream, head.as_bytes(), body)?,
            Scheme::Tls { config, name } => {
                let connection = ClientConnection::new(Arc::clone(config), name.clone())
                    .map_err(|error| failed(format!("TLS could not begin: {error}")))?;
                exchange(StreamOwned::new(connection, stream), head.as_bytes(), body)?
            }
        };
        let response = parse(&raw)?;
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
}

/// Write `head` and `body` on `stream` and read to its end.
fn exchange(
    mut stream: impl Read + Write,
    head: &[u8],
    body: &[u8],
) -> Result<Vec<u8>, RunnerError> {
    stream
        .write_all(head)
        .and_then(|()| stream.write_all(body))
        .and_then(|()| stream.flush())
        .map_err(|error| failed(format!("the request was not sent: {error}")))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|error| failed(format!("the answer was not read to its end: {error}")))?;
    Ok(raw)
}

/// An HTTP/1.1 response read to its end.
fn parse(raw: &[u8]) -> Result<Response, RunnerError> {
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| failed("the answer has no header end"))?;
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| failed("the answer has no status"))?;
    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
        .collect();
    let rest = &raw[split + 4..];
    let mut response = Response {
        status,
        headers,
        body: Vec::new(),
    };
    response.body = if response
        .header("transfer-encoding")
        .is_some_and(|coding| coding.eq_ignore_ascii_case("chunked"))
    {
        unchunk(rest)?
    } else {
        rest.to_vec()
    };
    Ok(response)
}

/// A chunked body, joined.
fn unchunk(mut rest: &[u8]) -> Result<Vec<u8>, RunnerError> {
    let mut body = Vec::new();
    loop {
        let end = rest
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| failed("a chunk has no size line"))?;
        let size_text = String::from_utf8_lossy(&rest[..end]).into_owned();
        let size_digits = size_text.split(';').next().map_or("", str::trim);
        let size = usize::from_str_radix(size_digits, 16)
            .map_err(|error| failed(format!("a chunk size does not read: {error}")))?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Ok(body);
        }
        let chunk = rest
            .get(..size)
            .ok_or_else(|| failed("a chunk is shorter than its size"))?;
        body.extend_from_slice(chunk);
        rest = rest
            .get(size..)
            .and_then(|after| after.strip_prefix(b"\r\n"))
            .ok_or_else(|| failed("a chunk does not end its line"))?;
    }
}

#[cfg(test)]
#[path = "../../tests/transport/cases.rs"]
mod tests;
