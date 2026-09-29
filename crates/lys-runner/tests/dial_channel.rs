#![cfg(test)]
//! DIRECTORY-050 R1: the dial bridge's channel. To an `https://` server it
//! speaks TLS, verified against the authority it is given, and signs each
//! dial with the machine's key under the server's epoch; a server whose
//! certificate no trusted authority issued is refused by name, and
//! cleartext is refused for any host but this machine's loopback address.
//! The server here is a second party: it is written from the published
//! dial route and signed bytes, and it verifies what the bridge sent. Every
//! wait ends on a connection's answer or its close, never a clock.

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::dial::{
    Dial, EPOCH_HEADER, EPOCH_ROUTE, NONCE_HEADER, SIGNATURE_HEADER, TICKET_HEADER,
    dial_signed_bytes, next_route,
};
use lys_runner::protocol::unhex;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

type TestResult = Result<(), Box<dyn Error>>;

const EPOCH: &str = "0e0e";
const TICKET: &str = "t-1";
const LINE: &str = "the signed request";

/// One request as the server read it.
struct Heard {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Heard {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(named, _)| named.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

fn hear(stream: impl Read) -> Result<Heard, Box<dyn Error>> {
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    reader.read_line(&mut first)?;
    let mut parts = first.split_whitespace();
    let (method, path) = (
        parts.next().ok_or("no method")?.to_owned(),
        parts.next().ok_or("no path")?.to_owned(),
    );
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').ok_or("a header has no colon")?;
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let mut heard = Heard {
        method,
        path,
        headers,
        body: Vec::new(),
    };
    let length: usize = heard.header("content-length").unwrap_or("0").parse()?;
    heard.body = vec![0; length];
    reader.read_exact(&mut heard.body)?;
    Ok(heard)
}

/// What the server heard, once it has answered every connection.
type Serving = std::thread::JoinHandle<Result<Vec<Heard>, String>>;

/// A TLS server answering `connections` requests as the dial routes do,
/// its certificate issued for `localhost` by an authority written to
/// `authority`. It answers what it heard.
fn server(
    connections: usize,
    authority: &std::path::Path,
) -> Result<(u16, Serving), Box<dyn Error>> {
    let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    std::fs::write(authority, issued.cert.pem())?;
    let certificate = CertificateDer::from(issued.cert.der().to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(issued.key_pair.serialize_der()));
    let config = Arc::new(
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(vec![certificate], key)?,
    );
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let serving = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..connections {
            let (tcp, _) = listener.accept().map_err(|error| error.to_string())?;
            let connection =
                ServerConnection::new(Arc::clone(&config)).map_err(|error| error.to_string())?;
            let mut stream = StreamOwned::new(connection, tcp);
            let Ok(request) = hear(&mut stream) else {
                continue;
            };
            let (head, body) = if request.path == EPOCH_ROUTE {
                (String::new(), EPOCH.to_owned())
            } else {
                (format!("{TICKET_HEADER}: {TICKET}\r\n"), LINE.to_owned())
            };
            let answer = format!(
                "HTTP/1.1 200 OK\r\n{head}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(answer.as_bytes())
                .and_then(|()| stream.flush())
                .map_err(|error| error.to_string())?;
            stream.conn.send_close_notify();
            stream.flush().map_err(|error| error.to_string())?;
            requests.push(request);
        }
        Ok(requests)
    });
    Ok((port, serving))
}

fn dial(server: String, key: &Arc<Ed25519Identity>, authority: Option<std::path::PathBuf>) -> Dial {
    Dial {
        server,
        machine: "m-1".to_owned(),
        key: Arc::clone(key),
        socket: std::path::PathBuf::from("/nonexistent/runner.sock"),
        authority,
    }
}

#[test]
fn a_dial_over_tls_is_signed_under_the_servers_epoch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("machine.key"),
    )?);
    let authority = dir.path().join("authority.pem");
    let (port, serving) = server(2, &authority)?;
    let asked =
        dial(format!("https://localhost:{port}"), &key, Some(authority)).next("the greeting")?;
    assert_eq!(asked, (TICKET.to_owned(), LINE.to_owned()));

    let heard = serving
        .join()
        .map_err(|_panicked| "the server panicked")??;
    assert_eq!(heard.len(), 2, "the epoch, then the dial");
    assert_eq!(
        (heard[0].method.as_str(), heard[0].path.as_str()),
        ("GET", EPOCH_ROUTE)
    );
    let next = &heard[1];
    let route = next_route("m-1");
    assert_eq!(
        (next.method.as_str(), next.path.as_str()),
        ("POST", route.as_str())
    );
    assert_eq!(next.body, b"the greeting");
    assert_eq!(next.header(EPOCH_HEADER), Some(EPOCH));
    let nonce = next.header(NONCE_HEADER).ok_or("no nonce")?;
    let signature = unhex(next.header(SIGNATURE_HEADER).ok_or("no signature")?)
        .ok_or("the signature is not hex")?;
    Ed25519Identity::verify(
        &key.public_key_bytes(),
        &dial_signed_bytes("POST", &route, EPOCH, nonce, &next.body),
        &signature,
    )?;
    let other_epoch = dial_signed_bytes("POST", &route, "0f0f", nonce, &next.body);
    assert!(
        Ed25519Identity::verify(&key.public_key_bytes(), &other_epoch, &signature).is_err(),
        "the signature binds the epoch"
    );
    Ok(())
}

#[test]
fn a_server_no_trusted_authority_issued_is_refused_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("machine.key"),
    )?);
    let (port, serving) = server(1, &dir.path().join("authority.pem"))?;
    let refused = dial(format!("https://localhost:{port}"), &key, None).next("the greeting");
    let error = refused.err().ok_or("an unknown authority was trusted")?;
    assert_eq!(error.name(), "runner_dial_failed", "{error}");
    assert!(error.to_string().contains("certificate"), "{error}");
    let heard = serving
        .join()
        .map_err(|_panicked| "the server panicked")??;
    assert!(
        heard.is_empty(),
        "nothing was sent before the server was trusted"
    );
    Ok(())
}

#[test]
fn cleartext_is_refused_for_any_host_but_loopback() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(
        &dir.path().join("machine.key"),
    )?);
    let mut refused = 0;
    for server in [
        "http://lys.example.test:8080",
        "http://192.0.2.7",
        "ftp://localhost",
        "http://localhost.evil.test",
        "http://127.0.0.1.nip.io:80",
        "http://[::1]@evil.test:80",
        "https://[::1]@evil.test",
    ] {
        let error = dial(server.to_owned(), &key, None)
            .next("the greeting")
            .err()
            .ok_or(format!("{server} was dialled"))?;
        assert_eq!(error.name(), "runner_dial_failed", "{error}");
        refused += 1;
    }
    assert_eq!(refused, 7);
    Ok(())
}
