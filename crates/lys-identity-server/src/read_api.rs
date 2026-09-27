//! The read-only views the identity screens are drawn from.
//!
//! Every route here reads the directory's projection and none of them writes.
//! The personal views answer only the signed-in person's own records: the
//! person is the one the signed-in login is bound to, and a login bound to no
//! person is refused `NoPerson`. The configured administrator's wider view is
//! a separate set of routes under `/directory`, admitted as every other
//! administrator act is. A refusal names itself and carries nothing of another
//! person's records: an agent that is not the caller's is refused exactly as
//! an agent the directory does not hold.
//!
//! The directory records no service accounts, roles or role versions yet, so
//! `service_accounts` answers an empty list and `role` and `version` answer
//! null until it does.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::projection::{Projection, Record};
use lys_identity::{Actor, AgentId, IdentityId, LifecycleState, LoginBinding, PersonId};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, receipt_json, signed_in, with_directory};

/// The read routes: the personal views and the administrator's wider view.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(me))
        .route("/people", get(own_people))
        .route("/agents/{id}", get(own_agent))
        .route("/directory/people", get(every_person))
        .route("/directory/agents/{id}", get(any_agent))
}

/// Whose records a view may show.
#[derive(Clone, Copy)]
enum Scope {
    /// Only this person's.
    Person(PersonId),
    /// Every person's, for the administrator.
    Directory,
}

fn login_json(binding: &LoginBinding) -> Value {
    json!({ "provider": binding.issuer(), "subject": binding.subject() })
}

fn person_summary(id: PersonId, record: &Record) -> Value {
    json!({
        "id": id.to_string(),
        "display_name": record.profile().display_name(),
        "state": record.state().to_string(),
    })
}

fn agents_of(projection: &Projection, person: PersonId) -> Vec<Value> {
    projection
        .records()
        .filter_map(|(id, record)| match id {
            IdentityId::Agent(agent) if record.responsible() == Some(person) => Some(json!({
                "id": agent.to_string(),
                "display_name": record.profile().display_name(),
                "state": record.state().to_string(),
            })),
            IdentityId::Agent(_) | IdentityId::Person(_) => None,
        })
        .collect()
}

fn person_with_agents(projection: &Projection, id: PersonId, record: &Record) -> Value {
    let mut person = person_summary(id, record);
    person["agents"] = Value::Array(agents_of(projection, id));
    person
}

fn person_record(projection: &Projection, id: PersonId) -> Result<&Record, ServerError> {
    projection.record(IdentityId::Person(id)).ok_or_else(|| {
        ServerError::from(lys_identity::IdentityError::IdentityUnknown {
            identity: id.to_string(),
        })
    })
}

/// The person the signed-in `actor`'s login is bound to, or `NoPerson`.
fn own_person(projection: &Projection, actor: &Actor) -> Result<PersonId, ServerError> {
    projection
        .person_for(actor.binding())
        .ok_or(ServerError::NoPerson)
}

async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let record = person_record(projection, person)?;
        Ok(Json(json!({
            "person": person_summary(person, record),
            "signed_in": login_json(actor.binding()),
            "sign_in_identities": record.bindings().iter().map(login_json).collect::<Vec<_>>(),
            "service_accounts": Vec::<Value>::new(),
        })))
    })
}

async fn own_people(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let record = person_record(projection, person)?;
        Ok(Json(json!({
            "scope": "personal",
            "people": [person_with_agents(projection, person, record)],
        })))
    })
}

async fn every_person(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let people = projection
            .records()
            .filter_map(|(id, record)| match id {
                IdentityId::Person(person) => Some(person_with_agents(projection, *person, record)),
                IdentityId::Agent(_) => None,
            })
            .collect::<Vec<_>>();
        Ok(Json(json!({ "scope": "directory", "people": people })))
    })
}

async fn own_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id)?;
    with_directory(&state, |directory| {
        let person = own_person(directory.projection()?, &actor)?;
        agent_view(directory, agent, Scope::Person(person))
    })
}

async fn any_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let agent = AgentId::from_str(&id)?;
    with_directory(&state, |directory| {
        agent_view(directory, agent, Scope::Directory)
    })
}

/// One agent with its person, state and provenance, if `scope` may see it.
fn agent_view(
    directory: &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
    agent: AgentId,
    scope: Scope,
) -> Result<Json<Value>, ServerError> {
    let projection = directory.projection()?;
    let record = projection
        .record(IdentityId::Agent(agent))
        .ok_or(ServerError::AgentNotVisible)?;
    let person = record.responsible().ok_or(ServerError::AgentNotVisible)?;
    if matches!(scope, Scope::Person(own) if own != person) {
        return Err(ServerError::AgentNotVisible);
    }
    let person_record = person_record(projection, person)?;
    let mut view = json!({
        "id": agent.to_string(),
        "display_name": record.profile().display_name(),
        "person": person_summary(person, person_record),
        "needs_new_person": person_record.state() == LifecycleState::Retired,
        "role": Value::Null,
        "version": Value::Null,
        "state": record.state().to_string(),
        "provenance": {
            "registered_by": login_json(record.registered_by()),
            "events": record.events(),
        },
    });
    let registered_at = record.events().first().copied();
    let registration = registered_at
        .and_then(|index| directory.receipt_at(index))
        .map_or(Value::Null, receipt_json);
    view["provenance"]["registration"] = registration;
    Ok(Json(view))
}
