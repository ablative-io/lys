//! `lys mcp`: an agent's own MCP connection to Lys, over stdio.
//!
//! Lys's `/mcp` route already admits a message the agent signed
//! (`lys-agent-signature`), verified over the message exactly as it arrives, and relays
//! each call to its HTTP route as that agent, so the call carries the agent's own grants
//! and meets Lys's own refusals. An MCP client such as Claude Code cannot sign each
//! message, and the person's OAuth sign-in is the only other way in: every agent call
//! answered `NotSignedIn` (12 Oct 2026). This bridge reads one JSON-RPC message a line on
//! stdin, signs it with the agent's key, posts it, and writes the answer a line on stdout.
//! It grants nothing: what a call may do is decided by the route, exactly as over HTTP.
//!
//! Lys is reached from any device over `https://`, its certificate verified against
//! this system's trust roots with nothing that turns the check off, or over `http://`
//! on a numeric loopback address (AGENTS-015 R3). A message Lys does not answer, a TLS
//! refusal or an address that cannot be reached, is said on stderr by name and answered
//! to the MCP client as a JSON-RPC error for that message.
//!
//! The key is read once at start from a PEM file or a seat file's `privateKeyPem`, held
//! in memory that is wiped when dropped, and never printed; neither is a signature. A
//! seat file is read by `lys_core::seat_file`, the one reader `lys agent enrol` writes
//! for and an app's door shares (AGENTS-017 R5). `lys agent enrol` signs and posts
//! through the same endpoint, signature and transport as this bridge.

use std::io::{BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use lys_core::attestation::sign_attestation;
use lys_core::keys::Ed25519Identity;
use lys_core::seat_file::{SeatFile, identity_from_pem};
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;
use crate::identity::loopback_http::{Authority, Failure, Request};

#[path = "mcp_bridge_tls.rs"]
pub(crate) mod tls;

use tls::{Transport, TrustPath, Unanswered, post};

/// The domain every agent request signature is made under (Lys `agent_signature`).
const DOMAIN: &str = "lys-identity/agent-request/v1";
/// The most bytes read from a key file.
const KEY_FILE_BYTES: usize = 65_536;
/// The port of an `https://` URL that names none.
const HTTPS_DEFAULT_PORT: u16 = 443;
/// The port of an `http://` URL that names none.
const HTTP_DEFAULT_PORT: u16 = 80;

/// Where the bridge posts: Lys's MCP route, as the URL names it.
pub(crate) struct Endpoint {
    /// The host and port.
    pub(crate) authority: Authority,
    /// The path posted to, as the URL names it.
    pub(crate) path: String,
    /// The path the signature is made over: the router's path, `/api` stripped.
    pub(crate) signed_path: String,
    /// Whether the URL is `https://`, reached over verified TLS.
    pub(crate) secure: bool,
}

/// Run the bridge until stdin closes.
///
/// # Errors
/// Refuses by name when the URL is neither `https://` nor `http://` on a loopback
/// address (`mcp_url_invalid`, `mcp_url_insecure`), when the system's trust roots
/// cannot be read (`mcp_trust_roots_unreadable`), or when the key cannot be read; a
/// message Lys does not answer is answered on stdout as a JSON-RPC error.
pub fn run(url: &str, agent: &str, key_file: &Path) -> CliResult<()> {
    let endpoint = endpoint(url)?;
    let identity = read_key(key_file)?;
    let transport = if endpoint.secure {
        Transport::Tls(TrustPath::system()?)
    } else {
        Transport::Loopback
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| refused("mcp_stdin_unreadable", error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(answer) = relay(&endpoint, &transport, agent, &identity, &line) {
            writeln!(stdout, "{answer}")
                .and_then(|()| stdout.flush())
                .map_err(|error| refused("mcp_stdout_unwritable", error.to_string()))?;
        }
    }
    Ok(())
}

/// Sign and post one message; the line to answer on stdout, none for a notification.
/// A failure is said on stderr by name, and answered as a JSON-RPC error when the
/// message has an id.
fn relay(
    endpoint: &Endpoint,
    transport: &Transport,
    agent: &str,
    identity: &Ed25519Identity,
    message: &str,
) -> Option<String> {
    let id = serde_json::from_str::<serde_json::Value>(message)
        .ok()
        .and_then(|value| value.get("id").cloned());
    let body = message.as_bytes();
    let header = signature(identity, agent, &endpoint.signed_path, body);
    let headers: [(&str, &[u8]); 3] = [
        ("content-type", b"application/json"),
        ("accept", b"application/json, text/event-stream"),
        ("lys-agent-signature", header.as_bytes()),
    ];
    let answered = post(
        transport,
        &endpoint.authority,
        &Request {
            method: "POST",
            path: &endpoint.path,
            headers: &headers,
            body,
        },
    );
    let reason = match answered {
        Ok(response) if response.body.is_empty() => {
            // A notification is accepted with no body; a request with an id is owed one.
            id.as_ref()?;
            format!(
                "mcp_answer_empty: Lys answered {} with no body",
                response.status
            )
        }
        Ok(response) => return Some(String::from_utf8_lossy(&response.body).trim().to_owned()),
        Err(unanswered) => named(&unanswered),
    };
    eprintln!("{reason}");
    id.map(|id| failure(&id, &reason))
}

/// The failure, by name: `mcp_tls_refused`, `mcp_unreachable`, `mcp_answer_lost` or
/// `mcp_answer_malformed`, with its detail (an empty answer is `mcp_answer_empty`).
pub(crate) fn named(unanswered: &Unanswered) -> String {
    match unanswered {
        Unanswered::TlsRefused(detail) => format!("mcp_tls_refused: {detail}"),
        Unanswered::Exchange(Failure::Unreachable(detail)) => {
            format!("mcp_unreachable: {detail}")
        }
        Unanswered::Exchange(Failure::Uncertain(detail)) => {
            format!(
                "mcp_answer_lost: the message may have reached Lys and no whole answer came back: {detail}"
            )
        }
        Unanswered::Exchange(Failure::Malformed(detail)) => {
            format!("mcp_answer_malformed: {detail}")
        }
    }
}

fn failure(id: &serde_json::Value, reason: &str) -> String {
    serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":reason}})
        .to_string()
}

/// The `lys-agent-signature` header for `POST path` with `body`:
/// `{agent} {millis} {nonce} {attestation hex}`.
pub(crate) fn signature(
    identity: &Ed25519Identity,
    agent: &str,
    path: &str,
    body: &[u8],
) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        });
    let mut nonce = [0_u8; 16];
    rand::rng().fill_bytes(&mut nonce);
    let nonce = hex(&nonce);
    let payload = format!(
        "{DOMAIN}\nPOST\n{path}\n{}\n{millis}\n{nonce}",
        hex(&Sha256::digest(body))
    );
    let attestation = sign_attestation(payload.as_bytes(), identity).to_cose_bytes();
    format!("{agent} {millis} {nonce} {}", hex(&attestation))
}

/// Lys's MCP route as `url` names it: `https://` on any host, or `http://` on a
/// numeric loopback address.
pub(crate) fn endpoint(url: &str) -> CliResult<Endpoint> {
    let (secure, rest) = if let Some(rest) = url.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("http://") {
        (false, rest)
    } else {
        return Err(refused(
            "mcp_url_invalid",
            "the Lys MCP URL must be https://<lys host>/api/mcp, or http:// on a loopback address",
        ));
    };
    let (host, path) = rest
        .split_once('/')
        .map_or((rest, "/"), |(host, path)| (host, path));
    let default_port = if secure {
        HTTPS_DEFAULT_PORT
    } else {
        HTTP_DEFAULT_PORT
    };
    let authority = Authority::parse(host, Some(default_port))
        .map_err(|reason| refused("mcp_url_invalid", reason))?;
    let loopback = authority
        .host
        .parse::<std::net::IpAddr>()
        .is_ok_and(|address| address.is_loopback());
    if !secure && !loopback {
        return Err(refused(
            "mcp_url_insecure",
            "the Lys MCP URL must be https:// off this machine; http:// is only for a numeric loopback address such as 127.0.0.1",
        ));
    }
    let path = if path == "/" {
        "/".to_owned()
    } else {
        format!("/{path}")
    };
    // The route verifies the signature over the path the router sees, under `/api`
    // when Lys serves its screens.
    let signed_path = path.strip_prefix("/api").unwrap_or(&path).to_owned();
    Ok(Endpoint {
        authority,
        path,
        signed_path,
        secure,
    })
}

/// The agent's key, from a PEM file or a seat file whose `privateKeyPem` holds the PEM.
pub(crate) fn read_key(path: &Path) -> CliResult<Ed25519Identity> {
    let unreadable = |why: &str| {
        refused(
            "mcp_key_unreadable",
            format!(
                "the agent key file {} could not be read: {why}",
                path.display()
            ),
        )
    };
    let text = Zeroizing::new(
        std::fs::read_to_string(path).map_err(|error| unreadable(&error.kind().to_string()))?,
    );
    if text.len() > KEY_FILE_BYTES {
        return Err(unreadable("the file is too large"));
    }
    let read = if text.trim_start().starts_with('{') {
        SeatFile::from_json(&text)
            .and_then(|seat| seat.identity())
            .map_err(|refusal| unreadable(&refusal.to_string()))?
    } else {
        identity_from_pem(&text).map_err(|refusal| unreadable(&refusal.to_string()))?
    };
    Ok(read)
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 15)],
            ]
        })
        .map(char::from)
        .collect()
}

#[cfg(test)]
#[path = "mcp_bridge_tests.rs"]
mod tests;
