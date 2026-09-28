//! `POST /_lys/signature`: the broker signs with a signing key it holds, for
//! the holder of a handle on that key, and answers the signature only.
//!
//! The request is JSON naming the key, the purpose and the purpose's typed
//! members, and carries the handle's signed presentation in the headers
//! every handle route reads, bound to this very request. Every level of it
//! denies unknown members, so a request carrying a digest to sign or bytes
//! it composed does not parse, and is refused 400 before any admission. The
//! bytes signed are built by the broker from the members (see
//! [`lys_secrets::Broker::sign_for`]). The answer is the `COSE_Sign1`
//! signature and the key's public key in hex; the seed is never in it.
//!
//! The route's name is kept apart from the `sign` command, with which a
//! holder signs its own presentation; the key that signs a presentation
//! stays with the presenter.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_secrets::{AgentRequest, SecretsError, Signable, Signing, SigningPurpose, to_hex};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::callers::refused;
use crate::serve::{MAX_BODY, Shared, on_broker, signed};

/// A signature asked for: the signing key by name, the purpose, and the
/// purpose's members, read once the purpose is known.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureAsked {
    key: String,
    purpose: String,
    members: Value,
}

/// The members of an `agent_request`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentRequestMembers {
    method: String,
    path: String,
    body_digest: String,
    signed_at_ms: u64,
    nonce: String,
}

fn malformed(reason: String) -> (StatusCode, String) {
    refused(&SecretsError::Encoding {
        context: "signature request",
        reason,
    })
}

/// The thing `purpose` names to sign, read from `members`.
fn signable(purpose: &str, members: Value) -> Result<Signable, (StatusCode, String)> {
    match SigningPurpose::parse(purpose).map_err(|error| refused(&error))? {
        SigningPurpose::AgentRequest => {
            let members: AgentRequestMembers = serde_json::from_value(members)
                .map_err(|error| malformed(error.to_string()))?;
            AgentRequest::new(
                &members.method,
                &members.path,
                &members.body_digest,
                members.signed_at_ms,
                &members.nonce,
            )
            .map(Signable::AgentRequest)
            .map_err(|error| refused(&error))
        }
    }
}

/// Signs what the request names, for the holder whose presentation it
/// carries, and answers the signature and the public key.
pub async fn signature(State(shared): State<Arc<Shared>>, request: Request) -> Response {
    match signed_for(&shared, request).await {
        Ok(answer) => answer,
        Err(refusal) => refusal.into_response(),
    }
}

async fn signed_for(
    shared: &Arc<Shared>,
    request: Request,
) -> Result<Response, (StatusCode, String)> {
    let (parts, body) = request.into_parts();
    let body = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|error| malformed(error.to_string()))?;
    let asked: SignatureAsked =
        serde_json::from_slice(&body).map_err(|error| malformed(error.to_string()))?;
    let signable = signable(&asked.purpose, asked.members)?;
    let (token, presentation) =
        signed(&parts, &body).map_err(|(_status, error)| refused(&error))?;
    let key = asked.key;
    let made = on_broker(shared, move |broker| {
        broker.sign_for(&token, &presentation, &key, &signable)
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))?;
    Ok(match made {
        Signing::Made(made) => Json(json!({
            "signature": to_hex(&made.cose),
            "public_key": to_hex(&made.public_key),
            "uses_left": made.uses_left,
        }))
        .into_response(),
        Signing::Retried { outcome } => (
            StatusCode::CONFLICT,
            format!("already presented; first outcome: {outcome}\n"),
        )
            .into_response(),
    })
}
