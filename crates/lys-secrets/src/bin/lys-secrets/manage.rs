//! Routes a screen changes a secret through, as the identity its caller
//! speaks for (see `callers`): a secret's scope and who it may be handed to, both
//! as its owner, and where a handle's revocation stands. The presentation
//! is bound to the request's body, so a signed change cannot be replayed
//! with another body.

use std::sync::{Arc, PoisonError};

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use lys_secrets::{HandleId, Recipients, Scope, SecretsError, UpstreamRevocation};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::callers::{Caller, caller, refused};
use crate::serve::{MAX_BODY, Shared};

type Answer = Result<Json<Value>, (StatusCode, String)>;

fn malformed(context: &'static str, reason: String) -> (StatusCode, String) {
    refused(&SecretsError::Encoding { context, reason })
}

/// The caller and the JSON body of a signed change.
async fn change<T: DeserializeOwned>(
    shared: &Shared,
    request: Request,
) -> Result<(Caller, T), (StatusCode, String)> {
    let (parts, body) = request.into_parts();
    let body: Bytes = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|error| malformed("request body", error.to_string()))?;
    let who = caller(shared, &parts, &body)?;
    let asked = serde_json::from_slice(&body)
        .map_err(|error| malformed("request body", error.to_string()))?;
    Ok((who, asked))
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
    let (who, asked) = change::<ScopeChange>(&shared, request).await?;
    let scope = Scope::parse(&asked.scope).map_err(|error| refused(&error))?;
    let target = scope.target();
    let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    broker
        .set_scope_via(&who.identity, &asked.secret, scope, who.via.as_deref())
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
    let (who, asked) = change::<RecipientsChange>(&shared, request).await?;
    let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    broker
        .set_recipients_via(
            &who.identity,
            &asked.secret,
            asked.recipients,
            who.via.as_deref(),
        )
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
    let who = caller(&shared, &parts, &[])?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let state = broker
        .revocation_state_as(&who.identity, &HandleId::from_text(&handle))
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
