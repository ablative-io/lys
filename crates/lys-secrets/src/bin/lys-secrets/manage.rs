//! Routes a screen changes a secret through, as the identity its caller
//! speaks for (see `callers`): a secret's scope and who it may be handed
//! to, both as its owner and each under an operation id the caller made
//! once for that change, those two settings as they stand with the last
//! operation id applied, where a handle's revocation stands, and a handle
//! ended by the person its holder acts for, under an operation id made once
//! for that ending. The
//! presentation is bound to the request's body, so a signed change cannot
//! be replayed with another body.

use std::sync::{Arc, PoisonError};

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use lys_secrets::{
    HandleEnded, HandleId, OwnerChanged, Recipients, Scope, SecretsError, UpstreamRevocation,
};
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
    /// The id the caller made once for this change.
    operation: Option<String>,
}

/// Whether the change was applied now, or answered from an earlier one.
fn repeated(changed: &OwnerChanged) -> bool {
    matches!(changed, OwnerChanged::Repeated { .. })
}

/// Sets a secret's scope, as its owner, once per operation id.
pub async fn scope(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = change::<ScopeChange>(&shared, request).await?;
    let scope = Scope::parse(&asked.scope).map_err(|error| refused(&error))?;
    let target = scope.target();
    let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let changed = broker
        .set_scope_via(
            &who.identity,
            &asked.secret,
            scope,
            who.via.as_deref(),
            asked.operation.as_deref(),
        )
        .map_err(|error| refused(&error))?;
    Ok(Json(json!({
        "secret": asked.secret,
        "scope": target,
        "operation": asked.operation,
        "repeated": repeated(&changed),
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipientsChange {
    secret: String,
    /// `anyone` or `people_only`.
    recipients: Recipients,
    /// The id the caller made once for this change.
    operation: Option<String>,
}

/// Sets who a secret may be handed to, as its owner, once per operation id.
pub async fn recipients(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = change::<RecipientsChange>(&shared, request).await?;
    let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let changed = broker
        .set_recipients_via(
            &who.identity,
            &asked.secret,
            asked.recipients,
            who.via.as_deref(),
            asked.operation.as_deref(),
        )
        .map_err(|error| refused(&error))?;
    Ok(Json(json!({
        "secret": asked.secret,
        "recipients": asked.recipients.label(),
        "operation": asked.operation,
        "repeated": repeated(&changed),
    })))
}

/// The query of a settings read: the secret, percent-decoded.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsAsked {
    secret: String,
}

/// A secret's scope, who it may be handed to, and the operation id of the
/// last owner change applied, as they stand, when the caller may discover
/// it. A screen reads this to settle a change whose
/// answer it never received.
pub async fn settings(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, _) = request.into_parts();
    let Query(asked) = Query::<SettingsAsked>::try_from_uri(&parts.uri)
        .map_err(|error| malformed("query", error.body_text()))?;
    if asked.secret.is_empty() {
        return Err(malformed("query", "no secret named".to_owned()));
    }
    let who = caller(&shared, &parts, &[])?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let settings = broker
        .settings(&who.identity, &asked.secret)
        .map_err(|error| refused(&error))?;
    Ok(Json(json!({
        "secret": asked.secret,
        "scope": settings.scope.map(|scope| scope.target()),
        "recipients": settings.recipients.label(),
        "last_operation": settings.last_operation,
    })))
}

/// The query of a revocation read: the handle, percent-decoded.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RevocationAsked {
    handle: String,
}

/// Where the revocation of the handle named by the `handle` query member
/// stands, when the caller may discover its secret.
pub async fn revocation(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (parts, _) = request.into_parts();
    let Query(asked) = Query::<RevocationAsked>::try_from_uri(&parts.uri)
        .map_err(|error| malformed("query", error.body_text()))?;
    let handle = asked.handle;
    if handle.is_empty() {
        return Err(malformed("query", "no handle named".to_owned()));
    }
    let who = caller(&shared, &parts, &[])?;
    let broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let state = broker
        .revocation_state_as(&who.identity, &HandleId::from_text(&handle))
        .map_err(|error| refused(&error))?;
    let (upstream, reason) = upstream_label(&state.upstream);
    Ok(Json(json!({
        "handle": handle,
        "stopped_here": state.stopped_here,
        "upstream": upstream,
        "upstream_reason": reason,
    })))
}

/// The provider's part of a revocation as a screen reads it: its state and
/// the reason of an unconfirmed one.
pub fn upstream_label(upstream: &UpstreamRevocation) -> (&'static str, Option<String>) {
    match upstream {
        UpstreamRevocation::NotAsked => ("not_asked", None),
        UpstreamRevocation::Unconfirmed(reason) => ("unconfirmed", Some(reason.clone())),
        UpstreamRevocation::Confirmed => ("confirmed", None),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ending {
    /// The id of the handle to end.
    handle: String,
    /// The id the caller made once for this ending.
    operation: Option<String>,
}

/// Ends a handle, as the person its holder acts for, once per operation id,
/// with every handle lent on from it. The provider's part of the revocation
/// is answered as it stands and is not asked here.
pub async fn drop_handle(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked) = change::<Ending>(&shared, request).await?;
    if asked.handle.is_empty() {
        return Err(malformed("request body", "no handle named".to_owned()));
    }
    let id = HandleId::from_text(&asked.handle);
    let mut broker = shared.broker.lock().unwrap_or_else(PoisonError::into_inner);
    let answered = broker
        .end_handle(
            &who.identity,
            &id,
            who.via.as_deref(),
            asked.operation.as_deref(),
        )
        .map_err(|error| refused(&error))?;
    let (outcome, ended) = match answered {
        HandleEnded::Ended { ended } => ("ended", ended),
        HandleEnded::Repeated { ended } => ("repeated", ended),
        HandleEnded::AlreadyEnded => ("already_ended", Vec::new()),
    };
    let state = broker
        .revocation_state(&id)
        .map_err(|error| refused(&error))?;
    let (upstream, reason) = upstream_label(&state.upstream);
    Ok(Json(json!({
        "handle": asked.handle,
        "operation": asked.operation,
        "outcome": outcome,
        "ended": ended,
        "stopped_here": state.stopped_here,
        "upstream": upstream,
        "upstream_reason": reason,
    })))
}
