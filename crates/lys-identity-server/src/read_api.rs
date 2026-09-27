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
//! null until it does. Each route answers its type from `read_views`.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::projection::{Projection, Record};
use lys_identity::{Actor, AgentId, IdentityId, LifecycleState, LoginBinding, PersonId};

use crate::error::ServerError;
use crate::read_views::{
    AgentSummary, AgentView, Login, MeView, PeopleView, PersonSummary, PersonView, Provenance,
};
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

fn login(binding: &LoginBinding) -> Login {
    Login {
        provider: binding.issuer().to_owned(),
        subject: binding.subject().to_owned(),
    }
}

fn person_summary(id: PersonId, record: &Record) -> PersonSummary {
    PersonSummary {
        id: id.to_string(),
        display_name: record.profile().display_name().to_owned(),
        state: record.state().to_string(),
    }
}

/// Every agent in the directory, grouped by the person it answers to, in one pass.
fn agents_by_person(projection: &Projection) -> HashMap<PersonId, Vec<AgentSummary>> {
    let mut grouped: HashMap<PersonId, Vec<AgentSummary>> = HashMap::new();
    for (id, record) in projection.records() {
        if let (IdentityId::Agent(agent), Some(person)) = (id, record.responsible()) {
            grouped.entry(person).or_default().push(AgentSummary {
                id: agent.to_string(),
                display_name: record.profile().display_name().to_owned(),
                state: record.state().to_string(),
            });
        }
    }
    grouped
}

/// The agents answering to one person, read from the records alone.
fn agents_of(projection: &Projection, person: PersonId) -> Vec<AgentSummary> {
    projection
        .records()
        .filter_map(|(id, record)| match (id, record.responsible()) {
            (IdentityId::Agent(agent), Some(owner)) if owner == person => Some(AgentSummary {
                id: agent.to_string(),
                display_name: record.profile().display_name().to_owned(),
                state: record.state().to_string(),
            }),
            _ => None,
        })
        .collect()
}

fn person_view(id: PersonId, record: &Record, agents: Vec<AgentSummary>) -> PersonView {
    let PersonSummary {
        id: text,
        display_name,
        state,
    } = person_summary(id, record);
    PersonView {
        id: text,
        display_name,
        state,
        agents,
    }
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
) -> Result<Json<MeView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let record = person_record(projection, person)?;
        Ok(Json(MeView {
            person: person_summary(person, record),
            signed_in: login(actor.binding()),
            sign_in_identities: record.bindings().iter().map(login).collect(),
            service_accounts: Vec::new(),
        }))
    })
}

async fn own_people(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<PeopleView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let record = person_record(projection, person)?;
        let agents = agents_of(projection, person);
        Ok(Json(PeopleView {
            scope: "personal".to_owned(),
            people: vec![person_view(person, record, agents)],
        }))
    })
}

async fn every_person(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<PeopleView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let mut grouped = agents_by_person(projection);
        let people = projection
            .records()
            .filter_map(|(id, record)| match id {
                IdentityId::Person(person) => {
                    let agents = grouped.remove(person).unwrap_or_default();
                    Some(person_view(*person, record, agents))
                }
                IdentityId::Agent(_) => None,
            })
            .collect();
        Ok(Json(PeopleView {
            scope: "directory".to_owned(),
            people,
        }))
    })
}

async fn own_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AgentView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let agent = AgentId::from_str(&id)?;
    with_directory(&state, |directory| {
        agent_json(directory, agent, |projection| {
            Ok(Scope::Person(own_person(projection, &actor)?))
        })
    })
}

async fn any_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<AgentView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let agent = AgentId::from_str(&id)?;
    with_directory(&state, |directory| {
        agent_json(directory, agent, |_| Ok(Scope::Directory))
    })
}

/// One agent as JSON: the projection is settled once, the scope read from it,
/// and the registration receipt looked up after the view is built.
fn agent_json(
    directory: &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
    agent: AgentId,
    scope_of: impl FnOnce(&Projection) -> Result<Scope, ServerError>,
) -> Result<Json<AgentView>, ServerError> {
    let projection = directory.projection()?;
    let scope = scope_of(projection)?;
    let mut view = agent_view(projection, agent, scope)?;
    view.provenance.registration = view
        .provenance
        .events
        .first()
        .and_then(|index| directory.receipt_at(*index))
        .map(receipt_json);
    Ok(Json(view))
}

/// One agent with its person, state and provenance, if `scope` may see it.
fn agent_view(
    projection: &Projection,
    agent: AgentId,
    scope: Scope,
) -> Result<AgentView, ServerError> {
    let record = projection
        .record(IdentityId::Agent(agent))
        .ok_or(ServerError::AgentNotVisible)?;
    let person = record.responsible().ok_or(ServerError::AgentNotVisible)?;
    if matches!(scope, Scope::Person(own) if own != person) {
        return Err(ServerError::AgentNotVisible);
    }
    let person_record = person_record(projection, person)?;
    Ok(AgentView {
        id: agent.to_string(),
        display_name: record.profile().display_name().to_owned(),
        person: person_summary(person, person_record),
        needs_new_person: person_record.state() == LifecycleState::Retired,
        role: None,
        version: None,
        state: record.state().to_string(),
        provenance: Provenance {
            registered_by: login(record.registered_by()),
            events: record.events().to_vec(),
            registration: None,
        },
    })
}
