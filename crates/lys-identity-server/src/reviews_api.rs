//! The review route: what the agents a person answers for hold now, so the
//! person can decide what each still needs.
//!
//! A grant is due for review when an agent holds it, the agent is not retired
//! and the grant stands at the moment of asking: no grant on its ancestry is
//! revoked, every grant on it has started and none has ended. The signed-in
//! person is shown the grants of the agents that answer to them, and the root
//! authority is shown every agent's. An agent whose person is not active has
//! no one answering for it, and is listed apart.
//!
//! The route reads and never writes. Dropping a grant is
//! `POST /grants/{id}/revoke`. A decision to keep a grant is not recorded yet,
//! so the answer carries no decisions and no due date.

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::grants::expiry::within_window;
use lys_identity::grants::revocation::unrevoked;
use lys_identity::grants::{GrantBook, GrantRecord};
use lys_identity::projection::{Projection, Record};
use lys_identity::{AgentId, IdentityId, LifecycleState, PersonId};
use serde::Serialize;

use crate::error::ServerError;
use crate::grant_contract::GrantView;
use crate::grant_sight::is_root;
use crate::grants::with_grants;
use crate::read_api::{own_person, person_summary};
use crate::read_views::{AgentSummary, PersonSummary};
use crate::routes::{AppState, signed_in};
use crate::session::now;

/// The review route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/reviews", get(reviews))
}

/// One grant an agent holds, with the agent and the person who reviews it.
#[derive(Debug, Clone, Serialize)]
pub struct DueView {
    /// The grant, as `GET /grants/{id}` answers it.
    pub grant: GrantView,
    /// The agent that holds it.
    pub agent: AgentSummary,
    /// The person the agent answers to, who decides whether it is kept.
    pub reviewer: PersonSummary,
}

/// An agent whose person is not active, so no one answers for it.
#[derive(Debug, Clone, Serialize)]
pub struct UnansweredView {
    /// The agent.
    pub agent: AgentSummary,
    /// The person it answers to, with the state that leaves it unanswered.
    pub person: PersonSummary,
}

/// The answer of `GET /reviews`.
#[derive(Debug, Clone, Serialize)]
pub struct ReviewView {
    /// `personal` for the signed-in person's own agents, `directory` for the root authority's view of every agent.
    pub scope: String,
    /// Every grant due for review, in grant id order.
    pub due: Vec<DueView>,
    /// Every agent with no one answering for it, in identifier order.
    pub unanswered: Vec<UnansweredView>,
    /// The grants' revision the answer was read at.
    pub revision: u64,
    /// The moment the grants were judged to stand, in seconds since the Unix epoch.
    pub judged_at: u64,
}

/// An agent in the directory with the person it answers to.
struct Answering<'a> {
    agent: AgentId,
    record: &'a Record,
    person: PersonId,
    person_record: &'a Record,
}

impl Answering<'_> {
    fn agent_summary(&self) -> AgentSummary {
        AgentSummary {
            id: self.agent.to_string(),
            display_name: self.record.profile().display_name().to_owned(),
            state: self.record.state().to_string(),
        }
    }

    fn person_summary(&self) -> PersonSummary {
        person_summary(self.person, self.person_record)
    }
}

/// The agent `identity` with its person, when it is an agent that is not
/// retired and `viewer` may review it: every agent for `None`, else only the
/// agents answering to that person.
fn answering(
    directory: &Projection,
    identity: IdentityId,
    viewer: Option<PersonId>,
) -> Option<Answering<'_>> {
    let IdentityId::Agent(agent) = identity else {
        return None;
    };
    let record = directory.record(identity)?;
    let person = record.responsible()?;
    let person_record = directory.record(IdentityId::Person(person))?;
    let shown = viewer.is_none_or(|viewer| viewer == person);
    (shown && record.state() != LifecycleState::Retired).then_some(Answering {
        agent,
        record,
        person,
        person_record,
    })
}

/// Whether the grant stands at `at`: nothing on its ancestry is revoked, and
/// `at` is within every window on it.
pub(crate) fn stands(book: &GrantBook, record: &GrantRecord, at: u64) -> bool {
    book.lineage(record.grant().id()).is_ok_and(|lineage| {
        unrevoked(book, &lineage).is_ok() && within_window(book, &lineage, at).is_ok()
    })
}

async fn reviews(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ReviewView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    with_grants(&state, |judged| {
        let person = own_person(judged.directory, &actor)?;
        let whole = is_root(IdentityId::Person(person), judged.root);
        let viewer = (!whole).then_some(person);
        let judged_at = now();
        let book = judged.grants.book();
        let due = book
            .records()
            .filter(|record| stands(book, record, judged_at))
            .filter_map(|record| {
                let held = answering(judged.directory, record.grant().holder(), viewer)?;
                Some(DueView {
                    grant: GrantView::from(record),
                    agent: held.agent_summary(),
                    reviewer: held.person_summary(),
                })
            })
            .collect();
        let unanswered = judged
            .directory
            .records()
            .filter_map(|(identity, _)| answering(judged.directory, *identity, viewer))
            .filter(|held| held.person_record.state() != LifecycleState::Active)
            .map(|held| UnansweredView {
                agent: held.agent_summary(),
                person: held.person_summary(),
            })
            .collect();
        Ok(Json(ReviewView {
            scope: if whole { "directory" } else { "personal" }.to_owned(),
            due,
            unanswered,
            revision: judged.grants.revision(),
            judged_at,
        }))
    })
}
