//! The dial bridge: how a runner on another machine is reached. The server
//! never dials a runner on another machine; the machine dials the server.
//!
//! The bridge opens a connection to the machine's runner on its local Unix
//! socket and takes the runner's greeting, then asks the server for the
//! next request for its machine, carrying that greeting. The server answers
//! when it has a request: the ask ends on the server's answer or when either
//! side leaves, never on a clock. The server signs the request over the
//! greeting's runner and challenge, so it is good on that one connection
//! alone, where the runner verifies it like any other; the runner's reply is
//! posted back.
//!
//! Every ask and every reply is signed with the machine's own key, the key
//! the machine's record names, over [`DIAL_DOMAIN`], the method, the route,
//! the server's epoch, a nonce and the body. The epoch is made fresh each
//! time the server starts and is read at [`EPOCH_ROUTE`]; the server holds
//! every nonce of its epoch, so a dial is admitted once, and a dial captured
//! under an earlier epoch is refused `runner_dial_stale`. A bridge told its
//! epoch is stale reads the new one and asks again.
//!
//! The channel is TLS for an `https://` server, verified against the public
//! roots and any authority the bridge is given; `http://` is spoken only to
//! this machine's own loopback address, where nothing crosses a network.
//! Any other `http://` server is refused by name.
//!
//! The bridge is a process of its own, so a server that cannot be reached
//! is retried without replaying a request; an admission refusal ends the bridge,
//! by name, and never the runner or a session it holds.

pub mod agent;
mod dispatch;
pub mod join;
pub mod mcp;
pub mod programs;
mod transport;

use std::path::PathBuf;
use std::sync::Arc;

use lys_core::Ed25519Identity;

use crate::error::RunnerError;
use crate::protocol::{hex, nonce};
use transport::{Channel, Response};

/// The existing verified transport, usable by the native CLI without another TLS dependency.
pub struct HttpClient(Channel);

/// A bounded HTTP answer; response bodies are never logged by the transport.
pub struct HttpResponse {
    /// The HTTP status.
    pub status: u16,
    /// The whole bounded response body.
    pub body: Vec<u8>,
}

impl HttpClient {
    /// Open verified TLS, or HTTP on loopback only.
    ///
    /// # Errors
    /// Refuses an invalid or insecure server address.
    pub fn new(server: &str) -> Result<Self, RunnerError> {
        Channel::for_server(server, None).map(Self)
    }

    /// Send once, without retries or ambient credentials.
    ///
    /// # Errors
    /// Reports malformed requests and uncertain transport outcomes by name.
    pub fn send(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<HttpResponse, RunnerError> {
        let response = self.0.send_response(method, path, headers, body)?;
        Ok(HttpResponse {
            status: response.status,
            body: response.body,
        })
    }
}

/// The domain every dial signature is made under.
pub const DIAL_DOMAIN: &str = "lys/runner-dial/v1";

/// The header carrying a dial request's nonce.
pub const NONCE_HEADER: &str = "x-lys-machine-nonce";

/// The header carrying a dial request's signature.
pub const SIGNATURE_HEADER: &str = "x-lys-machine-signature";

/// The header carrying the server epoch a dial request was signed under.
pub const EPOCH_HEADER: &str = "x-lys-server-epoch";

/// The header the server names a relayed request's ticket in.
pub const TICKET_HEADER: &str = "x-lys-runner-ticket";

/// The route the server's epoch is read at.
pub const EPOCH_ROUTE: &str = "/runner/dial/epoch";

/// The refusal a server gives a dial signed under an epoch not its own.
pub const STALE: &str = "runner_dial_stale";

/// The bytes a dial signature is over. The epoch and nonce are
/// hexadecimal, so no field can carry the newline that ends the one before
/// it, and the body is last.
#[must_use]
pub fn dial_signed_bytes(
    method: &str,
    route: &str,
    epoch: &str,
    nonce: &str,
    body: &[u8],
) -> Vec<u8> {
    let mut bytes = format!("{DIAL_DOMAIN}\n{method}\n{route}\n{epoch}\n{nonce}\n").into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

/// The route a machine asks for its next request on.
#[must_use]
pub fn next_route(machine: &str) -> String {
    format!("/runner/dial/{machine}/next")
}

/// The route the machine identity a computer's join made asks for a pass
/// to an app on, signed as every dial is, by the key that join recorded.
#[must_use]
pub fn pass_route(machine: &str) -> String {
    format!("/runner/dial/{machine}/pass")
}

/// The route a machine posts the reply to a ticket on.
#[must_use]
pub fn reply_route(machine: &str, ticket: &str) -> String {
    format!("/runner/dial/{machine}/replies/{ticket}")
}

/// What a bridge dials with.
pub struct Dial {
    /// The server's base address, `https://host:port` and any path prefix;
    /// `http://` only for this machine's own loopback address.
    pub server: String,
    /// The machine's id, as the server's records name it.
    pub machine: String,
    /// The machine's own key.
    pub key: Arc<Ed25519Identity>,
    /// The machine's runner's socket.
    pub socket: PathBuf,
    /// A certificate authority, in PEM, trusted for the server beside the
    /// public roots.
    pub authority: Option<PathBuf>,
}

pub(crate) fn failed(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::Dial {
        reason: what.to_string(),
    }
}

mod bridge;
mod reconnect;

/// The signed side of a bridge, shared by the threads that post replies.
struct Signer {
    channel: Channel,
    machine: String,
    key: Arc<Ed25519Identity>,
}

impl Dial {
    /// Relay requests until the local runner ends or the server refuses this machine.
    /// A lost server connection reads a fresh epoch and reconnects without replaying a request.
    ///
    /// # Errors
    /// - `RunnerError::Dial` if server/TLS setup, epoch or request decoding fails.
    /// - `RunnerError::Unreachable` if the local runner socket fails or closes.
    /// - `RunnerError::DialStale` if an epoch request or unrecovered signed exchange is refused as stale.
    /// - `RunnerError::Refused` if the outbox, local controls, worker or reconnect state fails, or the server refuses admission.
    pub fn bridge(&self) -> Result<(), RunnerError> {
        bridge::run(self)
    }

    /// The next request for this machine and its ticket, asked with the
    /// greeting `greeting` of a connection to its runner.
    ///
    /// # Errors
    /// - `RunnerError::Dial` if server/TLS setup fails, or the epoch or request is not text.
    /// - `RunnerError::Unreachable` if server transport fails or closes, or an answer carries no ticket.
    /// - `RunnerError::DialStale` if the server refuses the signed epoch.
    /// - `RunnerError::Refused` if the server returns a named refusal or transport state cannot be held.
    pub fn next(&self, greeting: &str) -> Result<(String, String), RunnerError> {
        let signer = self.signer()?;
        let epoch = signer.epoch()?;
        signer.next(&epoch, greeting)
    }

    fn signer(&self) -> Result<Signer, RunnerError> {
        Ok(Signer {
            channel: Channel::for_server(&self.server, self.authority.as_deref())?,
            machine: self.machine.clone(),
            key: Arc::clone(&self.key),
        })
    }
}

/// The ticket a long poll was answered with. An answer without one did not
/// come from Lys's dial: a restarting server, or something in front of it,
/// answered. That is a lost connection, so the bridge reconnects rather than
/// ending while its runner keeps serving.
fn ticket(answer: &Response) -> Result<String, RunnerError> {
    answer
        .header(TICKET_HEADER)
        .map(str::to_owned)
        .ok_or_else(|| RunnerError::Unreachable {
            reason: format!(
                "the server answered HTTP {} without a ticket ({}, {} bytes): something other than Lys's dial answered",
                answer.status,
                answer.header("content-type").unwrap_or("no content type"),
                answer.body.len()
            ),
        })
}

fn control_request(line: &str) -> bool {
    let Ok(request) = serde_json::from_str::<crate::protocol::Request>(line) else {
        return false;
    };
    match serde_json::from_str::<crate::protocol::Act>(&request.act) {
        Ok(
            crate::protocol::Act::Status { .. }
            | crate::protocol::Act::ControlStatus { .. }
            | crate::protocol::Act::End { .. },
        ) => true,
        Ok(crate::protocol::Act::Operate { operation }) => {
            matches!(operation.request, crate::operations::OperationRequest::Stop)
        }
        Ok(_) | Err(_) => false,
    }
}

impl Signer {
    /// The server's epoch.
    fn epoch(&self) -> Result<String, RunnerError> {
        let answer = self.channel.send("GET", EPOCH_ROUTE, &[], b"")?;
        let epoch = String::from_utf8(answer.body)
            .map_err(|error| failed(format!("the epoch is not text: {error}")))?;
        Ok(epoch.trim().to_owned())
    }

    fn next(&self, epoch: &str, greeting: &str) -> Result<(String, String), RunnerError> {
        let answer = self.post(&next_route(&self.machine), epoch, greeting.as_bytes())?;
        let ticket = ticket(&answer)?;
        let line = String::from_utf8(answer.body)
            .map_err(|error| failed(format!("the request is not text: {error}")))?;
        Ok((ticket, line))
    }

    fn reply(&self, epoch: &str, ticket: &str, reply: &str) -> Result<(), RunnerError> {
        let route = reply_route(&self.machine, ticket);
        let body = reply.trim_end().as_bytes();
        match self.post(&route, epoch, body) {
            Err(RunnerError::DialStale { .. }) => self.post(&route, &self.epoch()?, body).map(drop),
            answer => answer.map(drop),
        }
    }

    fn post(&self, route: &str, epoch: &str, body: &[u8]) -> Result<Response, RunnerError> {
        let nonce = nonce();
        let signature = hex(&self
            .key
            .sign(&dial_signed_bytes("POST", route, epoch, &nonce, body)));
        self.channel.send(
            "POST",
            route,
            &[
                (EPOCH_HEADER, epoch),
                (NONCE_HEADER, &nonce),
                (SIGNATURE_HEADER, &signature),
            ],
            body,
        )
    }
}

#[cfg(test)]
mod control_status_tests {
    #[test]
    fn current_control_status_uses_the_reserved_dial_slot() -> Result<(), Box<dyn std::error::Error>>
    {
        let request = serde_json::json!({"version":crate::protocol::Greeting::fresh("fixture").version,"runner":"fixture","challenge":"challenge", "act":"{\"act\":\"control_status\",\"session\":\"session\"}","signature":"signature"});
        assert!(super::control_request(&serde_json::to_string(&request)?));
        Ok(())
    }
}

#[cfg(test)]
mod ticketless_tests;
