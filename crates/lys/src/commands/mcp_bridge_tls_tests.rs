//! AGENTS-015 R3 over a real socket: Lys's MCP route served over TLS on loopback,
//! reached through the bridge's own relay. The test's root stands alone on the
//! same verifying path the bridge builds from the system's roots, so what is
//! proved about verification does not depend on the machine's own trust store;
//! a certificate no root on that path issued is refused by name and answered as
//! a JSON-RPC error.
//!
//! These live inside the crate because the one seam, a root added to the trust path,
//! is crate-private and compiled only for tests: nothing outside the crate, and no
//! flag, option or variable, reaches it.

use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use lys_core::keys::Ed25519Identity;
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use zeroize::Zeroizing;

use super::super::tls::{Transport, TrustPath};
use super::super::{endpoint, relay};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const AGENT: &str = "agent-00000000000000000000000000000015";
const TOOLS: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"read"}]}}"#;

/// A root, and a certificate for `localhost` it issued with that certificate's key.
struct Issued {
    root: CertificateDer<'static>,
    leaf: CertificateDer<'static>,
    key: Vec<u8>,
}

fn issue(root_name: &str) -> TestResult<Issued> {
    let mut root_params = CertificateParams::new(Vec::<String>::new())?;
    root_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    root_params
        .distinguished_name
        .push(DnType::CommonName, root_name);
    let root_key = KeyPair::generate()?;
    let root = root_params.self_signed(&root_key)?;
    let leaf_key = KeyPair::generate()?;
    let leaf = CertificateParams::new(vec!["localhost".to_owned()])?
        .signed_by(&leaf_key, &root, &root_key)?;
    Ok(Issued {
        root: root.der().clone(),
        leaf: leaf.der().clone(),
        key: leaf_key.serialize_der(),
    })
}

/// How the test server ends a connection after its answer.
#[derive(Clone, Copy)]
enum Close {
    /// With TLS `close_notify`, as a well-behaved server does.
    Notify,
    /// By closing the socket with no `close_notify`; the answer's length says it is whole.
    Drop,
}

/// Serves one connection per entry of `closes` over TLS on 127.0.0.1, answering each
/// request with [`TOOLS`]; the port, and the thread whose result is each request's
/// head or the reason the connection failed.
fn serve(issued: &Issued, closes: &[Close]) -> TestResult<(u16, JoinHandle<Vec<String>>)> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = Arc::new(
        ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(
                vec![issued.leaf.clone()],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(issued.key.clone())),
            )?,
    );
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    let closes = closes.to_vec();
    let server = thread::spawn(move || {
        closes
            .into_iter()
            .map(|close| {
                answer_one(&listener, &config, close)
                    .unwrap_or_else(|error| format!("failed: {error}"))
            })
            .collect()
    });
    Ok((port, server))
}

fn answer_one(
    listener: &TcpListener,
    config: &Arc<ServerConfig>,
    close: Close,
) -> TestResult<String> {
    let (socket, _peer) = listener.accept()?;
    let mut stream = StreamOwned::new(ServerConnection::new(Arc::clone(config))?, socket);
    let head = read_request(&mut stream)?;
    stream.write_all(
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{TOOLS}",
            TOOLS.len()
        )
        .as_bytes(),
    )?;
    stream.flush()?;
    if matches!(close, Close::Notify) {
        stream.conn.send_close_notify();
        stream.flush()?;
    }
    Ok(head)
}

/// Reads one request's head and its whole body, by its length; the head.
fn read_request(stream: &mut StreamOwned<ServerConnection, TcpStream>) -> TestResult<String> {
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        if let Some(end) = raw.windows(4).position(|window| window == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&raw[..end]).into_owned();
            let length = head
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map_or(Ok(0), |(_, value)| value.trim().parse::<usize>())?;
            if raw.len() >= end + 4 + length {
                return Ok(head);
            }
        }
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            return Err("the bridge closed before its request was whole".into());
        }
        raw.extend_from_slice(&buffer[..read]);
    }
}

fn tools_list(id: u32) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/list"}}"#)
}

fn identity() -> Ed25519Identity {
    Ed25519Identity::from_seed(&Zeroizing::new([15; 32]))
}

/// A tools/list through the bridge is answered by Lys's MCP route over TLS, verified
/// against the test's root alone on the verifying path, whether the server ends
/// with `close_notify` or closes once its answer is whole.
#[test]
fn a_tools_list_through_the_bridge_is_answered_over_verified_tls() -> TestResult {
    let issued = issue("AGENTS-015 test root")?;
    let (port, server) = serve(&issued, &[Close::Notify, Close::Drop])?;
    let endpoint = endpoint(&format!("https://localhost:{port}/api/mcp"))?;
    let transport = Transport::Tls(TrustPath::only(&issued.root)?);
    for id in [1, 2] {
        let answer = relay(&endpoint, &transport, AGENT, &identity(), &tools_list(id))
            .ok_or("the bridge wrote no answer")?;
        assert_eq!(answer, TOOLS, "message {id}");
    }
    let seen = server
        .join()
        .map_err(|_panicked| "the server thread panicked")?;
    assert_eq!(seen.len(), 2, "{seen:?}");
    for head in &seen {
        assert!(head.starts_with("POST /api/mcp HTTP/1.1"), "{head}");
        assert!(
            head.contains(&format!("lys-agent-signature: {AGENT} ")),
            "every message is signed as the agent: {head}"
        );
    }
    Ok(())
}

/// A server presenting a certificate no root on the trust path issued is refused by
/// name, naming the host and the verifier's reason, and the message is answered as a
/// JSON-RPC error with its own id, never dropped.
#[test]
fn a_certificate_the_trust_path_does_not_trust_is_refused_by_name() -> TestResult {
    let trusted = issue("AGENTS-015 trusted root")?;
    let stranger = issue("AGENTS-015 stranger root")?;
    let (port, server) = serve(&stranger, &[Close::Notify])?;
    let endpoint = endpoint(&format!("https://localhost:{port}/api/mcp"))?;
    let transport = Transport::Tls(TrustPath::only(&trusted.root)?);
    let answer = relay(&endpoint, &transport, AGENT, &identity(), &tools_list(7))
        .ok_or("the refusal was dropped")?;
    let answer: serde_json::Value = serde_json::from_str(&answer)?;
    assert_eq!(answer["id"], 7, "{answer}");
    let message = answer["error"]["message"]
        .as_str()
        .ok_or("the answer is not a JSON-RPC error")?;
    assert!(message.starts_with("mcp_tls_refused: "), "{message}");
    assert!(message.contains(&format!("localhost:{port}")), "{message}");
    assert!(message.contains("UnknownIssuer"), "{message}");
    let seen = server
        .join()
        .map_err(|_panicked| "the server thread panicked")?;
    assert!(
        seen.iter().all(|line| line.starts_with("failed: ")),
        "no request reached a server the bridge did not trust: {seen:?}"
    );
    Ok(())
}

/// An address where nothing listens is answered `mcp_unreachable`, naming the address,
/// over https and over loopback http alike.
#[test]
fn an_address_where_nothing_listens_is_answered_mcp_unreachable() -> TestResult {
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
    let issued = issue("AGENTS-015 unused root")?;
    for (url, transport) in [
        (
            format!("https://127.0.0.1:{port}/api/mcp"),
            Transport::Tls(TrustPath::only(&issued.root)?),
        ),
        (
            format!("http://127.0.0.1:{port}/api/mcp"),
            Transport::Loopback,
        ),
    ] {
        let endpoint = endpoint(&url)?;
        let answer = relay(&endpoint, &transport, AGENT, &identity(), &tools_list(3))
            .ok_or("the failure was dropped")?;
        let answer: serde_json::Value = serde_json::from_str(&answer)?;
        let message = answer["error"]["message"].as_str().unwrap_or_default();
        assert!(message.starts_with("mcp_unreachable: "), "{url}: {answer}");
        assert!(
            message.contains(&format!("127.0.0.1:{port}")),
            "{url}: {message}"
        );
    }
    Ok(())
}
