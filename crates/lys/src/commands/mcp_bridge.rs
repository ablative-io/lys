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
//! The key is read once at start from a PEM file or a seat file's `privateKeyPem`, held
//! in memory that is wiped when dropped, and never printed; neither is a signature.

use std::io::{BufRead, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use lys_core::attestation::sign_attestation;
use lys_core::keys::Ed25519Identity;
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;
use crate::identity::loopback_http::{Authority, Request, exchange};

/// The domain every agent request signature is made under (Lys `agent_signature`).
const DOMAIN: &str = "lys-identity/agent-request/v1";
/// PKCS#8 v1 DER for an ed25519 private key, before its 32-byte seed.
const PKCS8_ED25519_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];
/// The most bytes read from a key file.
const KEY_FILE_BYTES: usize = 65_536;

/// Run the bridge until stdin closes.
///
/// # Errors
/// Refuses by name when the URL is not a loopback HTTP URL or the key cannot be read;
/// a message Lys does not answer is answered on stdout as a JSON-RPC error.
pub fn run(url: &str, agent: &str, key_file: &Path) -> CliResult<()> {
    let (authority, path) = endpoint(url)?;
    let identity = read_key(key_file)?;
    // The route verifies the signature over the path the router sees, under `/api`
    // when Lys serves its screens.
    let signed_path = path.strip_prefix("/api").unwrap_or(&path).to_owned();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| refused("mcp_stdin_unreadable", error.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(answer) = relay(&authority, (&path, &signed_path), agent, &identity, &line) {
            writeln!(stdout, "{answer}")
                .and_then(|()| stdout.flush())
                .map_err(|error| refused("mcp_stdout_unwritable", error.to_string()))?;
        }
    }
    Ok(())
}

/// Sign and post one message; the line to answer on stdout, none for a notification.
fn relay(
    authority: &Authority,
    (path, signed_path): (&str, &str),
    agent: &str,
    identity: &Ed25519Identity,
    message: &str,
) -> Option<String> {
    let id = serde_json::from_str::<serde_json::Value>(message)
        .ok()
        .and_then(|value| value.get("id").cloned());
    let body = message.as_bytes();
    let header = signature(identity, agent, signed_path, body);
    let headers: [(&str, &[u8]); 3] = [
        ("content-type", b"application/json"),
        ("accept", b"application/json, text/event-stream"),
        ("lys-agent-signature", header.as_bytes()),
    ];
    let answered = exchange(
        authority,
        &Request {
            method: "POST",
            path,
            headers: &headers,
            body,
        },
    );
    match answered {
        Ok(response) if response.body.is_empty() => id.map(|id| {
            failure(
                &id,
                &format!("Lys answered {} with no body", response.status),
            )
        }),
        Ok(response) => Some(String::from_utf8_lossy(&response.body).trim().to_owned()),
        Err(error) => id.map(|id| failure(&id, &format!("Lys was not reached: {error:?}"))),
    }
}

fn failure(id: &serde_json::Value, reason: &str) -> String {
    serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":reason}})
        .to_string()
}

/// The `lys-agent-signature` header for `POST path` with `body`:
/// `{agent} {millis} {nonce} {attestation hex}`.
fn signature(identity: &Ed25519Identity, agent: &str, path: &str, body: &[u8]) -> String {
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

/// The loopback authority and path of `url`.
fn endpoint(url: &str) -> CliResult<(Authority, String)> {
    let rest = url.strip_prefix("http://").ok_or_else(|| {
        refused(
            "mcp_url_invalid",
            "the Lys MCP URL must be http:// on loopback",
        )
    })?;
    let (host, path) = rest
        .split_once('/')
        .map_or((rest, "/"), |(host, path)| (host, path));
    let authority =
        Authority::parse(host, Some(80)).map_err(|reason| refused("mcp_url_invalid", reason))?;
    let numeric = authority
        .host
        .trim_matches(['[', ']'])
        .parse::<std::net::IpAddr>();
    if !numeric.is_ok_and(|address| address.is_loopback()) {
        return Err(refused(
            "mcp_url_invalid",
            "the Lys MCP URL must name a numeric loopback address",
        ));
    }
    let path = if path == "/" {
        "/".to_owned()
    } else {
        format!("/{path}")
    };
    Ok((authority, path))
}

/// The agent's key, from a PEM file or a seat file whose `privateKeyPem` holds the PEM.
fn read_key(path: &Path) -> CliResult<Ed25519Identity> {
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
    let pem = if text.trim_start().starts_with('{') {
        let seat: serde_json::Value = serde_json::from_str(&text).map_err(|error| {
            unreadable(&format!(
                "the seat file is not JSON at line {}",
                error.line()
            ))
        })?;
        Zeroizing::new(
            seat.get("privateKeyPem")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| unreadable("the seat file has no privateKeyPem"))?
                .to_owned(),
        )
    } else {
        text
    };
    let body: Zeroizing<String> = Zeroizing::new(
        pem.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with("-----"))
            .collect(),
    );
    let der = Zeroizing::new(
        base64::engine::general_purpose::STANDARD
            .decode(body.as_bytes())
            .map_err(|_not_pem| unreadable("the key is not PEM"))?,
    );
    let seed = der
        .strip_prefix(&PKCS8_ED25519_PREFIX)
        .and_then(|seed| <[u8; 32]>::try_from(seed).ok())
        .ok_or_else(|| unreadable("the key is not a PKCS#8 ed25519 private key"))?;
    Ok(Ed25519Identity::from_seed(&Zeroizing::new(seed)))
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
