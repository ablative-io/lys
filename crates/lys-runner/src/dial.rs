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
//! ends the bridge, by name, and never the runner or a session it holds.

pub mod agent;
mod dispatch;
pub mod join;
pub mod mcp;
mod transport;

use std::path::PathBuf;
use std::sync::Arc;

use lys_core::Ed25519Identity;

use crate::error::RunnerError;
use crate::protocol::{Answer, hex, nonce, reply_line};
use transport::{Channel, Response};

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
pub fn next_route(machine: &str) -> String {
    format!("/runner/dial/{machine}/next")
}

/// The route a machine posts the reply to a ticket on.
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

/// The signed side of a bridge, shared by the threads that post replies.
struct Signer {
    channel: Channel,
    machine: String,
    key: Arc<Ed25519Identity>,
}

impl Dial {
    /// Relay every request the server has for this machine, until the
    /// server or the runner cannot be reached, or the server refuses the
    /// machine, which is answered by name.
    pub fn bridge(&self) -> Result<(), RunnerError> {
        let signer = Arc::new(self.signer()?);
        let mut epoch = signer.epoch()?;
        let mut dispatch = dispatch::Dispatch::new();
        loop {
            let mut connection = crate::client::connect(&self.socket)?;
            let greeting = connection.greeting_line()?;
            let (ticket, line) = match signer.next(&epoch, &greeting) {
                Err(RunnerError::DialStale { reason }) => {
                    crate::error::said(&format!("{STALE}: {reason}: reading the new epoch"));
                    epoch = signer.epoch()?;
                    continue;
                }
                taken => taken?,
            };
            let control = control_request(&line);
            let posted = Arc::clone(&signer);
            let posting_epoch = epoch.clone();
            let posting_ticket = ticket.clone();
            if let Err(error) = dispatch.submit(control, move || {
                let reply = connection
                    .exchange(&line)
                    .unwrap_or_else(|error| reply_line(Answer::refusal(&error)));
                if let Err(error) = posted.reply(&posting_epoch, &posting_ticket, &reply) {
                    crate::error::said(&format!("a reply was not delivered: {error}"));
                }
            }) {
                signer.reply(&epoch, &ticket, &reply_line(Answer::refusal(&error)))?;
            }
        }
    }

    /// The next request for this machine and its ticket, asked with the
    /// greeting `greeting` of a connection to its runner.
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
        let ticket = answer
            .header(TICKET_HEADER)
            .ok_or_else(|| failed("the server named no ticket"))?
            .to_owned();
        let line = String::from_utf8(answer.body)
            .map_err(|error| failed(format!("the request is not text: {error}")))?;
        Ok((ticket, line))
    }

    fn reply(&self, epoch: &str, ticket: &str, reply: &str) -> Result<(), RunnerError> {
        self.post(
            &reply_route(&self.machine, ticket),
            epoch,
            reply.trim_end().as_bytes(),
        )
        .map(drop)
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
