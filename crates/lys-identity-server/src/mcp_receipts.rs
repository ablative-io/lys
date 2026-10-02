//! Every change an agent makes through MCP is kept as a leaf of the
//! directory log, and the agent is answered with the receipt it was kept
//! under. The agent is the one the MCP message's signature or the run's pass
//! names; a signed message's signature is kept with the leaf as it was sent.

use axum::http::request::Parts;
use axum::http::{Method, StatusCode};
use lys_identity::agent_call::AgentCall;
use lys_identity::projection::Record;
use lys_identity::{Actor, AgentId, IdentityId, Provenance};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::routes::AppState;

/// Who a relayed change is kept against, and the evidence kept with it.
pub(crate) struct Witness {
    agent: AgentId,
    provenance: Provenance,
    signature: String,
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
    let Some(state) = state else {
        return Ok(None);
    };
    Ok(crate::agent_pass::verified(state, &parts.headers)?.map(|(agent, provenance)| Witness {
        agent,
        provenance,
        signature: String::new(),
    }))
}

/// Whether a route's answer is a change that is kept.
pub(crate) fn kept(method: &Method, status: StatusCode) -> bool {
    status.is_success() && !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// The SHA-256 of the body the route was given, in lowercase hex.
pub(crate) fn digest(body: &[u8]) -> String {
    crate::routes::hex(&Sha256::digest(body))
}

/// Keep the change as a leaf and answer the receipt it was kept under.
pub(crate) fn keep(
    state: &AppState,
    witness: Witness,
    (method, path, body_sha256, status): (&Method, &str, &str, StatusCode),
) -> Result<Value, ServerError> {
    let call = AgentCall::new(
        method.as_str(),
        path,
        body_sha256,
        status.as_u16(),
        &witness.signature,
    )?;
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
