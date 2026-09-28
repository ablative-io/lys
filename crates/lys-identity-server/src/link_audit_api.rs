//! The authenticated link-audit receiver endpoint over lys-identity's core (R4),
//! and the source's question of which person holds a login.
//!
//! The person a login belongs to is read from the directory's bindings by the
//! login's exact issuer and subject. It is never inferred from an email
//! address, and an issuer's own user id is never taken to be a person id. Only
//! the configured link-audit source may ask, and a login no person holds is
//! refused as `LoginUnbound`.
//!
//! The source is admitted to both routes in one of two ways. A session whose
//! login is the configured link-audit source login is admitted, and the actor
//! recorded is that login by the method OIDC. A request carrying the
//! `lys-agent-signature` header is admitted when the signature stands, as
//! `agent_signature` checks it over the method `POST`, the route's path and
//! the exact bytes of the body, and the person responsible for the signing
//! agent holds the configured link-audit source login. The actor recorded is
//! then that person, named by that login, by the method agent signature, and
//! the event keeps the agent's id. No token of any other kind is taken.
//!
//! By either way, the person who holds the link-audit source login must not
//! be suspended or retired: such a person answers for no request, their own
//! or their agent's.
//!
//! A request carrying both a session cookie and the signature header is
//! judged by the signature alone: the session neither admits it nor names
//! its actor.
//!
//! The path an agent signs is the path this router serves the route at,
//! `/link-audit` or `/link-audit/person`, as it is for every signed agent
//! request: where the service mounts its routes under a prefix, the prefix is
//! not part of what is signed.
//!
//! The caller is admitted before the body is read, and the body is read from
//! the bytes that were signed. A body that is not JSON, or not the members
//! the route asks for and no others, is refused `RequestMalformed`.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Uri, header};
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::{Actor, LinkChange, LinkObservation, LoginBinding, PersonId, Provenance};
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::agent_signature::signed_agent;
use crate::directory_views::{LinkAuditPerson, ReceiptAnswer, receipt_view};
use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// The link-audit routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/link-audit", post(deliver))
        .route("/link-audit/person", post(holder))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// The actor a POST of `body` to `path` is admitted as: the person
/// responsible for the agent that signed it, or else the signed-in source.
fn admitted(
    state: &AppState,
    headers: &HeaderMap,
    path: &str,
    body: &[u8],
) -> Result<Actor, ServerError> {
    let signed = with_directory(state, |directory| {
        let directory = directory.projection()?;
        let Some(agent) = signed_agent(state, directory, headers, ("POST", path, body))? else {
            return Ok(None);
        };
        let login = state.admission.link_audit_agent(directory, agent)?;
        Ok(Some(Actor::new(
            login.clone(),
            Provenance::by_agent(agent, now()),
        )))
    })?;
    if let Some(actor) = signed {
        return Ok(actor);
    }
    let source = signed_in(state, headers)?;
    state.admission.link_audit_source(&source)?;
    with_directory(state, |directory| {
        state.admission.link_audit_holder(directory.projection()?)
    })?;
    Ok(source)
}

/// Whether the request says its body is JSON.
fn says_json(headers: &HeaderMap) -> bool {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|kind| kind.trim().to_ascii_lowercase())
        .is_some_and(|kind| {
            kind == "application/json"
                || (kind.starts_with("application/") && kind.ends_with("+json"))
        })
}

/// The members `body` carries, which must be the ones `T` names and no others.
fn read<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> Result<T, ServerError> {
    if !says_json(headers) {
        return Err(malformed(
            "the request does not say its body is JSON (act: send it with `Content-Type: application/json`)",
        ));
    }
    let Json(members) =
        Json::<T>::from_bytes(body).map_err(|refused| malformed(refused.body_text()))?;
    Ok(members)
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = LinkAuditAsked)]
pub(crate) struct Asked {
    issuer: String,
    subject: String,
}

async fn holder(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<LinkAuditPerson>, ServerError> {
    admitted(&state, &headers, uri.path(), &bytes)?;
    let asked: Asked = read(&headers, &bytes)?;
    let login = LoginBinding::new(&asked.issuer, &asked.subject)
        .map_err(|refused| malformed(refused.to_string()))?;
    with_directory(&state, |directory| {
        let person = directory
            .projection()?
            .person_for(&login)
            .ok_or(ServerError::LoginUnbound)?;
        Ok(Json(LinkAuditPerson {
            person: person.to_string(),
        }))
    })
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = LinkAuditDelivery)]
pub(crate) struct Delivery {
    person: String,
    source_operation_id: String,
    change: String,
    issuer: String,
    subject: String,
    observer: String,
    observed_at: u64,
}

async fn deliver(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<ReceiptAnswer>, ServerError> {
    let source = admitted(&state, &headers, uri.path(), &bytes)?;
    let body: Delivery = read(&headers, &bytes)?;
    let change = match body.change.as_str() {
        "linked" => LinkChange::Linked,
        "unlinked" => LinkChange::Unlinked,
        other => {
            return Err(malformed(format!("{other} is not linked or unlinked")));
        }
    };
    let person = PersonId::from_str(&body.person)?;
    let observation = LinkObservation::new(
        &body.source_operation_id,
        change,
        LoginBinding::new(&body.issuer, &body.subject)?,
        &body.observer,
        body.observed_at,
    )?;
    with_directory(&state, |directory| {
        let receipt = directory.accept_link_audit(source, person, observation, now())?;
        Ok(Json(ReceiptAnswer {
            receipt: receipt_view(&receipt),
        }))
    })
}
