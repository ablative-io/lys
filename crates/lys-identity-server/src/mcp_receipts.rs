//! Every change an agent asks for through MCP is kept as a leaf of the
//! directory log before it is made; a change whose leaf cannot be kept is
//! not made. The agent is the one the MCP message's signature or the run's
//! pass names; a signed message's signature is kept with the leaf as it was
//! sent.
//!
//! The leaf records the ask, not the outcome: the route judges after it is
//! written. So the answer names the leaf as `receipt` only when the route
//! made the change, and as `asked` when the route refused. An ask the route
//! refused is held in memory, and the same ask from the same agent, made
//! again, is relayed under the leaf already kept instead of writing another,
//! so an agent that loops or probes cannot grow the directory log by one leaf
//! per attempt. The hold ends when the ask is made, or when it is pushed out
//! by newer refused asks once [`REFUSED_ASKS_HELD`] are held.

use std::collections::VecDeque;
use std::sync::Mutex;

use axum::http::Method;
use axum::http::request::Parts;
use lys_identity::agent_call::AgentCall;
use lys_identity::projection::Record;
use lys_identity::{Actor, AgentId, IdentityId, Provenance};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::AppState;

#[cfg(test)]
#[path = "mcp_receipts_tests.rs"]
mod tests;

/// How a change made under a grant token is told apart from a run Lys
/// started.
const GRANT_TOKEN: &str = "grant-token";

/// The most refused asks held in memory at once, across every agent; the
/// oldest is let go when one more is held, and that ask, made again, is
/// kept as a fresh leaf.
pub(crate) const REFUSED_ASKS_HELD: usize = 4096;

/// Who a relayed change is kept against, and the evidence kept with it.
pub(crate) struct Witness {
    agent: AgentId,
    provenance: Provenance,
    signature: String,
}

/// One ask as the hold tells it apart: the agent, the method, the path as
/// the log keeps it and the body's digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ask {
    agent: AgentId,
    method: String,
    path: String,
    body_sha256: String,
}

/// The refused asks held, oldest first, each with the view of the leaf it
/// was kept under.
pub(crate) struct RefusedAsks {
    held: Mutex<VecDeque<(Ask, Value)>>,
}

fn unavailable(error: &dyn std::fmt::Display) -> ServerError {
    ServerError::ConfigInvalid {
        reason: format!("the refused-asks hold is unavailable: {error}"),
    }
}

impl RefusedAsks {
    /// An empty hold.
    pub(crate) fn new() -> Self {
        Self {
            held: Mutex::new(VecDeque::new()),
        }
    }

    /// The leaf an identical refused ask was kept under, if one is held.
    pub(crate) fn held(&self, ask: &Ask) -> Result<Option<Value>, ServerError> {
        let held = self.held.lock().map_err(|error| unavailable(&error))?;
        Ok(held
            .iter()
            .find(|(kept, _)| kept == ask)
            .map(|(_, leaf)| leaf.clone()))
    }

    /// Hold `ask`, refused, under `leaf`; one already held is left as it is,
    /// and the oldest is let go once more than [`REFUSED_ASKS_HELD`] would be
    /// held.
    pub(crate) fn refused(&self, ask: Ask, leaf: Value) -> Result<(), ServerError> {
        let mut held = self.held.lock().map_err(|error| unavailable(&error))?;
        if held.iter().any(|(kept, _)| *kept == ask) {
            return Ok(());
        }
        held.push_back((ask, leaf));
        while held.len() > REFUSED_ASKS_HELD {
            held.pop_front();
        }
        Ok(())
    }

    /// `ask` was made: it is no longer held.
    pub(crate) fn made(&self, ask: &Ask) -> Result<(), ServerError> {
        let mut held = self.held.lock().map_err(|error| unavailable(&error))?;
        held.retain(|(kept, _)| kept != ask);
        Ok(())
    }
}

/// A changing call's leaf: the ask it was kept under and whether this call
/// wrote it or found it already kept from the same ask refused before.
pub(crate) struct Kept {
    ask: Ask,
    leaf: Value,
}

impl Kept {
    /// Keep the ask `witness` makes through `method` and `uri` with `body`
    /// before it is relayed: under the leaf an identical refused ask is
    /// already held under, else as a fresh leaf.
    pub(crate) fn before(
        state: &AppState,
        hold: &RefusedAsks,
        witness: Witness,
        method: &Method,
        uri: &axum::http::Uri,
        body: &[u8],
    ) -> Result<Self, ServerError> {
        let ask = Ask {
            agent: witness.agent,
            method: method.as_str().to_owned(),
            path: recorded_path(uri),
            body_sha256: digest(body),
        };
        if let Some(leaf) = hold.held(&ask)? {
            return Ok(Self { ask, leaf });
        }
        let leaf = keep(state, witness, (method, &ask.path, &ask.body_sha256))?;
        Ok(Self { ask, leaf })
    }

    /// Name the leaf in `result` as the route judged: `receipt` when the
    /// change was made, `asked` when it was refused, which also holds the
    /// ask so the same one is not kept again.
    pub(crate) fn answer(
        self,
        hold: &RefusedAsks,
        made: bool,
        result: &mut Value,
    ) -> Result<(), ServerError> {
        if made {
            hold.made(&self.ask)?;
            result["receipt"] = self.leaf;
        } else {
            hold.refused(self.ask, self.leaf.clone())?;
            result["asked"] = self.leaf;
        }
        Ok(())
    }
}

/// The witness of a call before it is relayed: the verified signer of the
/// MCP message, else the connected app its bearer token names, else the
/// agent the run pass names; none for any other caller, whose change is
/// kept by the route it reaches as it always was.
pub(crate) fn witness(
    state: Option<&AppState>,
    parts: &Parts,
    signer: Option<AgentId>,
    app: Option<&(AgentId, String)>,
) -> Result<Option<Witness>, ServerError> {
    if signer.is_none()
        && let Some((agent, client_id)) = app
    {
        return Ok(Some(Witness {
            agent: *agent,
            provenance: Provenance::by_pass(*agent, crate::mcp_oauth::LAUNCH, client_id)?,
            signature: String::new(),
        }));
    }
    if let Some(agent) = signer {
        let signature = parts
            .headers
            .get(crate::agent_signature::HEADER)
            .ok_or(ServerError::AgentSignatureRefused {
                reason: "the verified signature header is missing",
            })?
            .to_str()
            .map_err(|_unread| ServerError::AgentSignatureRefused {
                reason: "the header is not text",
            })?
            .to_owned();
        return Ok(Some(Witness {
            agent,
            provenance: Provenance::by_agent(agent, crate::session::now()),
            signature,
        }));
    }
    if let Some(principal) = parts
        .extensions
        .get::<crate::agent_signature::TokenPrincipal>()
    {
        return Ok(Some(Witness {
            agent: principal.holder,
            provenance: Provenance::by_pass(
                principal.holder,
                GRANT_TOKEN,
                &principal.grant().to_string(),
            )?,
            signature: String::new(),
        }));
    }
    let Some(state) = state else {
        return Ok(None);
    };
    Ok(
        crate::agent_pass::verified(state, &parts.headers)?.map(|(agent, provenance)| Witness {
            agent,
            provenance,
            signature: String::new(),
        }),
    )
}

/// Whether a call asks for a change, and so is kept before it is made.
pub(crate) fn changing(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// The route's path as the log keeps it: a query is kept only as its
/// SHA-256, so nothing a query carries ever sits in the log.
pub(crate) fn recorded_path(uri: &axum::http::Uri) -> String {
    match uri.query() {
        Some(query) => format!("{}?sha256={}", uri.path(), digest(query.as_bytes())),
        None => uri.path().to_owned(),
    }
}

/// The SHA-256 of the body the route was given, in lowercase hex.
pub(crate) fn digest(body: &[u8]) -> String {
    crate::routes::hex(&Sha256::digest(body))
}

/// Keep the asked change as a leaf before it is made and answer the view of
/// the leaf it was kept under.
fn keep(
    state: &AppState,
    witness: Witness,
    (method, path, body_sha256): (&Method, &str, &str),
) -> Result<Value, ServerError> {
    let call = AgentCall::new(method.as_str(), path, body_sha256, &witness.signature)?;
    crate::routes::with_directory(state, |directory| {
        let projection = directory.projection()?;
        let binding = projection
            .record(IdentityId::Agent(witness.agent))
            .and_then(Record::responsible)
            .and_then(|person| projection.record(IdentityId::Person(person)))
            .and_then(|person| person.bindings().first())
            .ok_or(ServerError::NoPerson)?
            .clone();
        let actor = Actor::new(binding, witness.provenance);
        let receipt =
            directory.record_agent_call(actor, witness.agent, call, crate::session::now())?;
        serde_json::to_value(crate::directory_views::receipt_view(&receipt)).map_err(|error| {
            ServerError::ConfigInvalid {
                reason: format!("the receipt could not be rendered: {error}"),
            }
        })
    })
}
