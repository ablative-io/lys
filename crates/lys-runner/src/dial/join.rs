//! Joining a server: a computer the server names, holding the connection
//! code its administrator was given once, sends the code and the public
//! half of the computer's own key, and is answered the public key every
//! request to its runner is signed with. The private key never leaves the
//! computer, and the code is sent in the request's body alone, never in its
//! address.
//!
//! The server is reached as a bridge reaches it, over TLS verified against
//! the public roots and any authority given. An address another computer's
//! runner cannot dial is refused by name before anything is sent: this
//! computer's own loopback address, and anything that is not `https://`,
//! since cleartext is spoken only to this computer's loopback address.

use std::path::Path;

use serde_json::{Value, json};

use super::failed;
use super::transport::{Channel, host, loopback};
use crate::error::RunnerError;
use crate::protocol::unhex;

/// The route a computer joins on.
pub const JOIN_ROUTE: &str = "/runner/join";

/// The refusal a server address another computer cannot reach is given.
pub const UNREACHABLE: &str = "runner_join_unreachable";

/// Refuse, by name, a server address a runner on another computer cannot
/// dial: one that is not `https://`, or one on this computer's own loopback
/// address.
pub fn reachable(server: &str) -> Result<(), RunnerError> {
    let Some(rest) = server.strip_prefix("https://") else {
        return Err(RunnerError::refused(
            UNREACHABLE,
            format!(
                "{server} is not an https:// address, and a runner on another computer connects to Lys only over https: give the https address Lys is served at"
            ),
        ));
    };
    let authority = rest
        .split_once('/')
        .map_or(rest, |(authority, _)| authority);
    if loopback(host(authority)) {
        return Err(RunnerError::refused(
            UNREACHABLE,
            format!(
                "{server} is this computer's own loopback address, which reaches no other computer: give the address Lys is served at"
            ),
        ));
    }
    Ok(())
}

/// What a computer joins with.
pub struct Join<'a> {
    /// The server's address: `https://host:port` and any path prefix.
    pub server: &'a str,
    /// A certificate authority, in PEM, trusted for the server beside the
    /// public roots.
    pub authority: Option<&'a Path>,
    /// The computer's id, as the server names it.
    pub machine: &'a str,
    /// The connection code the server gave for it.
    pub code: &'a str,
    /// The computer's own Ed25519 public key.
    pub key: [u8; 32],
}

impl Join<'_> {
    /// Join, answering the server's runner public key; refused by name
    /// when the address cannot be dialled, the server cannot be reached, or
    /// the server refuses the code.
    pub fn join(&self) -> Result<[u8; 32], RunnerError> {
        reachable(self.server)?;
        let channel = Channel::for_server(self.server, self.authority)?;
        let body = json!({
            "machine": self.machine,
            "code": self.code,
            "key": crate::protocol::hex(&self.key),
        })
        .to_string();
        let answer = channel.send(
            "POST",
            JOIN_ROUTE,
            &[("content-type", "application/json")],
            body.as_bytes(),
        );
        let answer: Value = serde_json::from_slice(&answer?.body).map_err(|error| {
            failed(format!(
                "the server's answer to the join is not JSON: {error}"
            ))
        })?;
        if answer["machine"] != self.machine {
            return Err(failed(format!(
                "the server's answer to the join names another computer: {}",
                answer["machine"]
            )));
        }
        answer["server_key"]
            .as_str()
            .and_then(unhex)
            .and_then(|bytes| <[u8; 32]>::try_from(bytes).ok())
            .ok_or_else(|| {
                failed("the server's answer to the join carries no runner key as 64 hexadecimal characters")
            })
    }
}
