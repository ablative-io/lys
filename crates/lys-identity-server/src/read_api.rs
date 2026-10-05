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
//! `GET /authority` answers anyone, signed in or not, with how access is
//! managed and `build`: the commit this service was built from, stamped at
//! build time, followed by `; dirty` when the tree built had changes, or
//! `not built from a git commit` when it had no commit to read. It is never
//! empty and never invented.
//!
//! `GET /me` lists the signed-in person's own service accounts that are not
//! retired, read from the service accounts' log (`service_accounts_store`),
//! and none when the configuration names no service accounts. The directory
//! records no roles or role versions yet, so `role` and `version` answer
//! null until it does. Each route answers its type from `read_views`.

use std::collections::{BTreeSet, HashMap};
use std::ops::Bound;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::projection::{Projection, Record};
use lys_identity::{Actor, AgentId, IdentityId, LoginBinding, PersonId};

use crate::admission::AUTHORITY;
use crate::directory_views::receipt_json;
use crate::error::ServerError;
use crate::read_views::{
    AgentSummary, AgentView, Login, MeView, PeopleView, PersonSummary, PersonView, Provenance,
};
use crate::routes::{AppState, signed_in, with_directory};

#[cfg(test)]
#[path = "read_api_index_tests.rs"]
mod index_tests;

/// The commit this service was built from, as `build.rs` stamped it.
pub const BUILD: &str = env!("LYS_BUILD");

/// The read routes: the authority, the personal views and the
/// administrator's wider view.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/authority", get(authority))
        .route("/me", get(me))
        .route("/people", get(own_people))
        .route("/agents/{id}", get(own_agent))
        .route("/directory/people", get(every_person))
        .route("/directory/agents/{id}", get(any_agent))
}

/// How access is managed, and the build answering.
async fn authority() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "authority": AUTHORITY, "build": BUILD }))
}

/// Whose records a view may show.
#[derive(Clone, Copy)]
enum Scope {
    /// Only this person's.
    Person(PersonId),
    /// Every person's, for the administrator.
    Directory,
}

pub(crate) fn login(binding: &LoginBinding) -> Login {
    Login {
        provider: binding.issuer().to_owned(),
        subject: binding.subject().to_owned(),
    }
}

pub(crate) fn person_summary(id: PersonId, record: &Record) -> PersonSummary {
    PersonSummary {
        id: id.to_string(),
        display_name: record.profile().display_name().to_owned(),
        state: record.state().to_string(),
    }
}

/// Every agent in the directory, grouped by the person it answers to, in one pass.
fn agents_by_person(
    projection: &Projection,
) -> Result<HashMap<PersonId, Vec<AgentSummary>>, ServerError> {
    let mut grouped: HashMap<PersonId, Vec<AgentSummary>> = HashMap::new();
    for (id, record) in projection.records() {
        if let (IdentityId::Agent(agent), Some(person)) = (id, record.responsible()) {
            grouped.entry(person).or_default().push(agent_summary(
                projection,
                IdentityId::Agent(*agent),
                record,
            )?);
        }
    }
    Ok(grouped)
}

pub(crate) fn agent_summary(
    projection: &Projection,
    id: IdentityId,
    record: &Record,
) -> Result<AgentSummary, ServerError> {
    #[cfg(test)]
    VIEW_BUILDS.with(|count| count.set(count.get() + 1));
    let reporting = crate::reporting_read::read(projection, record)?;
    Ok(AgentSummary {
        id: id.to_string(),
        display_name: record.profile().display_name().to_owned(),
        state: record.state().to_string(),
        reports_to: reporting.target,
        accountable: reporting.accountable,
        gap: reporting.gap,
    })
}

/// The agents answering to one person, selected by the maintained index.
fn agents_of(projection: &Projection, person: PersonId) -> Result<Vec<AgentSummary>, ServerError> {
    agents_of_observing(projection, person, |_| {})
}

fn agents_of_observing(
    projection: &Projection,
    person: PersonId,
    mut visited: impl FnMut(IdentityId),
) -> Result<Vec<AgentSummary>, ServerError> {
    projection
        .agents_of(person)
        .map(|entry| {
            let (id, record) = entry?;
            visited(*id);
            agent_summary(projection, *id, record)
        })
        .collect()
}

#[cfg(test)]
thread_local! {
    static VIEW_BUILDS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn person_view(id: PersonId, record: &Record, agents: Vec<AgentSummary>) -> PersonView {
    #[cfg(test)]
    VIEW_BUILDS.with(|count| count.set(count.get() + 1));
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

pub(crate) fn person_record(projection: &Projection, id: PersonId) -> Result<&Record, ServerError> {
    projection.record(IdentityId::Person(id)).ok_or_else(|| {
        ServerError::from(lys_identity::IdentityError::IdentityUnknown {
            identity: id.to_string(),
        })
    })
}

/// The person the signed-in `actor`'s login is bound to, or `NoPerson`.
pub(crate) fn own_person(projection: &Projection, actor: &Actor) -> Result<PersonId, ServerError> {
    match crate::caller_admission::active_caller(projection, actor)? {
        IdentityId::Person(person) => Ok(person),
        IdentityId::Agent(_) | IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => {
            Err(ServerError::NoPerson)
        }
    }
}

async fn me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MeView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let person = match crate::caller_admission::own_account_person(projection, &actor) {
            Err(ServerError::NoPerson) => {
                return Err(match state.admission.configured_administrator(&actor) {
                    Ok(()) => ServerError::SetupRequired,
                    Err(ServerError::NotAdmitted { .. }) => ServerError::NoPerson,
                    Err(error) => error,
                });
            }
            answer => answer?,
        };
        let record = person_record(projection, person)?;
        Ok(Json(MeView {
            person: person_summary(person, record),
            signed_in: login(actor.binding()),
            sign_in_identities: record.bindings().iter().map(login).collect(),
            service_accounts: crate::service_accounts_api::owned_by(&state, person)?,
        }))
    })
}

async fn own_people(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: crate::list_page::Input,
) -> Result<Json<PeopleView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let page = crate::list_page::Page::read(query, "/people")?;
        let members = page
            .as_ref()
            .map(|page| page.members(&state))
            .transpose()?
            .flatten();
        Ok(Json(personal_people(
            projection,
            person,
            page.as_ref(),
            members.as_ref(),
        )?))
    })
}

async fn every_person(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: crate::list_page::Input,
) -> Result<Json<PeopleView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    with_directory(&state, |directory| {
        let projection = directory.projection()?;
        let page = crate::list_page::Page::read(query, "/directory/people")?;
        let Some(page) = page else {
            let mut grouped = agents_by_person(projection)?;
            let people = projection
                .records()
                .filter_map(|(id, record)| match id {
                    IdentityId::Person(person) => Some(person_view(
                        *person,
                        record,
                        grouped.remove(person).unwrap_or_default(),
                    )),
                    IdentityId::Agent(_)
                    | IdentityId::ServiceAccount(_)
                    | IdentityId::Connector(_) => None,
                })
                .collect();
            return Ok(Json(PeopleView {
                scope: "directory".to_owned(),
                people,
                page: None,
                agents_total: None,
            }));
        };
        let members = page.members(&state)?;
        let filtered = page.filtered();
        let mut agents_total = 0;
        let after = if filtered {
            Bound::Unbounded
        } else {
            page.after()
        };
        let rows = projection.people(after);
        let (people, totals) = page.select(
            rows,
            (!filtered).then_some(projection.people_count()),
            |(_, person)| {
                if !filtered {
                    return Ok(true);
                }
                let record = person_record(projection, *person)?;
                let agents = projection
                    .agents_of(*person)
                    .collect::<Result<Vec<_>, _>>()?;
                let in_team = crate::list_page::member(members.as_ref(), &person.to_string(), None)
                    || agents.iter().any(|(id, _)| {
                        crate::list_page::member(members.as_ref(), &id.to_string(), None)
                    });
                let matched = in_team
                    && page.matches(
                        std::iter::once(record.profile().display_name()).chain(
                            agents
                                .iter()
                                .map(|(_, record)| record.profile().display_name()),
                        ),
                    );
                if matched {
                    agents_total += projection.person_agents_count(*person);
                }
                Ok(matched)
            },
            |(id, _)| id,
            |(_, person)| {
                Ok(person_view(
                    *person,
                    person_record(projection, *person)?,
                    agents_of(projection, *person)?,
                ))
            },
        )?;
        if !filtered {
            agents_total = projection.people_agents_count();
        }
        Ok(Json(PeopleView {
            scope: "directory".to_owned(),
            people,
            page: Some(totals),
            agents_total: Some(agents_total),
        }))
    })
}

fn personal_people(
    projection: &Projection,
    person: PersonId,
    page: Option<&crate::list_page::Page>,
    members: Option<&BTreeSet<String>>,
) -> Result<PeopleView, ServerError> {
    let record = person_record(projection, person)?;
    let Some(page) = page else {
        return Ok(PeopleView {
            scope: "personal".to_owned(),
            people: vec![person_view(person, record, agents_of(projection, person)?)],
            page: None,
            agents_total: None,
        });
    };
    let id = person.to_string();
    let filtered = page.filtered();
    let mut agents_total = 0;
    let (people, totals) = page.select(
        std::iter::once(id.as_str()),
        (!filtered).then_some(1),
        |_| {
            if !filtered {
                return Ok(true);
            }
            let agents = projection
                .agents_of(person)
                .collect::<Result<Vec<_>, _>>()?;
            let in_team = crate::list_page::member(members, &id, None)
                || agents
                    .iter()
                    .any(|(agent, _)| crate::list_page::member(members, &agent.to_string(), None));
            let matched = in_team
                && page.matches(
                    std::iter::once(record.profile().display_name()).chain(
                        agents
                            .iter()
                            .map(|(_, record)| record.profile().display_name()),
                    ),
                );
            if matched {
                agents_total = projection.person_agents_count(person);
            }
            Ok(matched)
        },
        |id| id,
        |_| Ok(person_view(person, record, agents_of(projection, person)?)),
    )?;
    if !filtered {
        agents_total = projection.person_agents_count(person);
    }
    Ok(PeopleView {
        scope: "personal".to_owned(),
        people,
        page: Some(totals),
        agents_total: Some(agents_total),
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
    crate::routes::administrator(&state, &actor)?;
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
    let first = view.provenance.events.first().copied();
    view.provenance.registration = match first {
        Some(index) => directory
            .receipt_at(index)?
            .as_ref()
            .map(receipt_json)
            .transpose()?,
        None => None,
    };
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
    let reporting = crate::reporting_read::read(projection, record)?;
    Ok(AgentView {
        id: agent.to_string(),
        display_name: record.profile().display_name().to_owned(),
        person: person_summary(person, person_record),
        needs_new_person: reporting.gap.is_some(),
        reports_to: reporting.target,
        accountable: reporting.accountable,
        gap: reporting.gap,
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
