//! A prepared request must name the exact resource the reviewer is shown.
use super::{DraftBody, malformed, unhex};
use crate::ServerError;
use lys_identity::draft_event::{Created, RequestEvidence, RequestSignature, Target};
use lys_identity::{Actor, AgentId, IdentityId, OperationId};
use std::str::FromStr;

pub(super) fn creation(
    body: DraftBody,
    actor: Actor,
    reviewer: IdentityId,
    at: u64,
    corrects: Option<OperationId>,
) -> Result<Created, ServerError> {
    let (resource, action) =
        crate::openapi_table::token_scope(&body.method, &body.path).map_err(|error| {
            ServerError::SecretsRefused {
                status: axum::http::StatusCode::CONFLICT,
                refusal: "DraftChangeInvalid".to_owned(),
                reason: format!(
                    "target {}:{} {} names no declared mutation scope for {} {}: {error}",
                    body.target.kind, body.target.id, body.target.action, body.method, body.path
                ),
            }
        })?;
    if body.method == "GET"
        || body.target.kind != resource.kind()
        || body.target.id != resource.id()
        || body.target.action != action.as_str()
    {
        return Err(ServerError::SecretsRefused {
            status: axum::http::StatusCode::CONFLICT,
            refusal: "DraftChangeInvalid".to_owned(),
            reason: format!(
                "target {}:{} {} differs from route scope {}:{} {}",
                body.target.kind,
                body.target.id,
                body.target.action,
                resource.kind(),
                resource.id(),
                action.as_str()
            ),
        });
    }
    Ok(Created {
        operation: OperationId::from_str(&body.operation)?,
        actor,
        recorded_at: at,
        target: Target {
            kind: body.target.kind,
            id: body.target.id,
            action: body.target.action,
        },
        method: body.method,
        path: body.path,
        body: body.body.into_bytes(),
        note: body.note,
        reviewer,
        corrects,
        evidence: None,
        request_signature: None,
    })
}

/// The evidence `agent` signed `signed_body` on `signed_path` under the
/// signature header `text`: what a draft keeps of its agent's own request.
pub(super) fn signed_by(
    agent: AgentId,
    text: &str,
    signed_path: String,
    signed_body: Vec<u8>,
) -> Result<(RequestEvidence, RequestSignature), ServerError> {
    let words: Vec<_> = text.split_ascii_whitespace().collect();
    let [author, signed_at, nonce, signature] = words.as_slice() else {
        return Err(malformed(
            "the verified signature header does not contain four fields",
        ));
    };
    if *author != agent.to_string() {
        return Err(malformed(
            "the verified agent differs from the signature header",
        ));
    }
    let signed_at_ms = signed_at
        .parse()
        .map_err(|error| malformed(format!("the signing time does not read: {error}")))?;
    let payload =
        crate::agent_signature::payload("POST", &signed_path, &signed_body, signed_at_ms, nonce);
    Ok((
        RequestEvidence {
            agent,
            method: "POST".to_owned(),
            path: signed_path,
            body: signed_body,
            signed_at_ms,
            nonce: (*nonce).to_owned(),
            cose_sign1: unhex(signature)?,
        },
        RequestSignature {
            header: text.as_bytes().to_vec(),
            payload,
        },
    ))
}

/// What a decision keeps of who made it: nothing for a person; for an agent
/// (AGENTS-019 R4), the evidence of its own signed MCP message, which every
/// agent's draft leaf carries. A run pass signs nothing, so it decides none.
pub(super) fn decision_evidence(actor: &Actor) -> Result<Option<RequestEvidence>, ServerError> {
    let Some(agent) = actor.provenance().agent() else {
        return Ok(None);
    };
    let (header, message) =
        crate::agent_signature::relayed_signature().ok_or(ServerError::AgentSignatureRefused {
            reason: "an agent decides a draft only through its own signed MCP message",
        })?;
    let path = crate::agent_signature::RELAY_PATH.to_owned();
    Ok(Some(signed_by(agent, &header, path, message.to_vec())?.0))
}
