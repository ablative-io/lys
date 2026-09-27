//! Routes a screen changes a secret through, as the identity its signed
//! handle speaks for: a secret's scope and who it may be handed to, both
//! as its owner, and where a handle's revocation stands. The presentation
//! is bound to the request's body, so a signed change cannot be replayed
//! with another body.

use std::sync::{Arc, PoisonError};

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use lys_secrets::{HandleId, Recipients, Scope, SecretsError, UpstreamRevocation};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::serve::{MAX_BODY, Shared, signed};

type Answer = Result<Json<Value>, (StatusCode, String)>;

/// The status a refusal answers with: a name the caller may not discover
/// as not found, a malformed request as bad, a refusal of the caller's
/// authority as forbidden, and a failure of the broker itself as internal.
fn refused(error: &SecretsError) -> (StatusCode, String) {
    let status = match error {
        SecretsError::SecretUnknown { .. }
        | SecretsError::HandleUnknown
        | SecretsError::NotFound => StatusCode::NOT_FOUND,
        SecretsError::InvalidScope { .. }
        | SecretsError::InvalidName { .. }
        | SecretsError::Encoding { .. }
        | SecretsError::PresentationInvalid { .. }
        | SecretsError::OperationIdTooShort { .. } => StatusCode::BAD_REQUEST,
        SecretsError::Lending(_)
        | SecretsError::Revocation(_)
        | SecretsError::PermissionDenied { .. }
        | SecretsError::NoRelation { .. }
        | SecretsError::RelationRemoved { .. }
        | SecretsError::HandleExpired { .. }
        | SecretsError::HandleDropped { .. }
        | SecretsError::HandleWrongIdentity { .. }
        | SecretsError::PresentationReplayed { .. }
        | SecretsError::PresentationStale { .. }
        | SecretsError::OperationIdReused { .. }
        | SecretsError::LeaseExhausted { .. }
        | SecretsError::LeaseWindowClosed { .. } => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, format!("{}: {error}\n", error.name()))
}

fn malformed(context: &'static str, reason: String) -> (StatusCode, String) {
    refused(&SecretsError::Encoding { context, reason })
}

/// The identity the request's signed handle speaks for, checked against
/// the presentation over `body`. Counts no use of the handle.
fn caller(shared: &Shared, parts: &Parts, body: &[u8]) -> Result<String, (StatusCode, String)> {
    let (token, presentation) = signed(parts, body).map_err(|(_status, error)| refused(&error))?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    broker
        .caller(&token, &presentation)
        .map_err(|error| refused(&error))
}

/// The caller and the JSON body of a signed change.
async fn change<T: DeserializeOwned>(
    shared: &Shared,
    request: Request,
) -> Result<(String, T), (StatusCode, String)> {
    let (parts, body) = request.into_parts();
    let body: Bytes = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|error| malformed("request body", error.to_string()))?;
    let identity = caller(shared, &parts, &body)?;
    let asked = serde_json::from_slice(&body)
        .map_err(|error| malformed("request body", error.to_string()))?;
    Ok((identity, asked))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeChange {
    secret: String,
    /// `personal:<person>`, `team:<name>` or `organisation:<name>`.
    scope: String,
}

/// Sets a secret's scope, as its owner.
pub async fn scope(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (identity, asked) = change::<ScopeChange>(&shared, request).await?;
    let scope = Scope::parse(&asked.scope).map_err(|error| refused(&error))?;
    let target = scope.target();
    let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    broker
        .set_scope(&identity, &asked.secret, scope)
        .map_err(|error| refused(&error))?;
    Ok(Json(json!({ "secret": asked.secret, "scope": target })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipientsChange {
    secret: String,
    /// `anyone` or `people_only`.
    recipients: Recipients,
}

/// Sets who a secret may be handed to, as its owner.
pub async fn recipients(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (identity, asked) = change::<RecipientsChange>(&shared, request).await?;
    let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    broker
        .set_recipients(&identity, &asked.secret, asked.recipients)
        .map_err(|error| refused(&error))?;
    Ok(Json(
        json!({ "secret": asked.secret, "recipients": asked.recipients.label() }),
    ))
}

/// Where the revocation of the handle named by the `handle` query member
/// stands, when the caller may discover its secret.
pub async fn revocation(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, _body) = request.into_parts();
    let handle = parts
        .uri
        .query()
        .and_then(|query| {
            query
                .split('&')
                .find_map(|pair| pair.strip_prefix("handle="))
        })
        .filter(|handle| !handle.is_empty())
        .ok_or_else(|| malformed("query", "no handle named".to_owned()))?
        .to_owned();
    let identity = caller(&shared, &parts, &[])?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let state = broker
        .revocation_state_as(&identity, &HandleId::from_text(&handle))
        .map_err(|error| refused(&error))?;
    let (upstream, reason) = match state.upstream {
        UpstreamRevocation::NotAsked => ("not_asked", None),
        UpstreamRevocation::Unconfirmed(reason) => ("unconfirmed", Some(reason)),
        UpstreamRevocation::Confirmed => ("confirmed", None),
    };
    Ok(Json(json!({
        "handle": handle,
        "stopped_here": state.stopped_here,
        "upstream": upstream,
        "upstream_reason": reason,
    })))
}
