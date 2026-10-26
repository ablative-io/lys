//! The review routes: what the agents a person answers for hold now, so the
//! person can decide what each still needs, and the decision to keep one.
//!
//! A grant is due after its written review period since its last keep (or
//! the start of its window), when an agent holds it, the agent is not retired
//! and the grant stands at the moment of asking: no grant on its ancestry is
//! revoked, every grant on it has started and none has ended. The signed-in
//! person is shown the grants of the agents that answer to them, and the root
//! authority is shown every agent's. An agent whose person is not active has
//! no one answering for it, and is listed apart.
//!
//! Dropping a grant is `POST /grants/{id}/revoke`. Keeping one is
//! `POST /reviews/{grant}/keep`, with an operation id and the reviewer's
//! note: only the person the holding agent answers to, or the root
//! authority, keeps it, and only while it stands. The decision is recorded in
//! the review decisions' log before it is answered, under its operation id,
//! so asking it again in the same words answers what was recorded and writes
//! nothing, and the same operation in other words is refused by name.
//! `GET /reviews` shows each due grant's latest decision to keep it.
//!
//! A batch keeps exactly the due grants shown, as one durable act. A keep
//! changes when a check is due, never the grant's permissions or window.

use std::str::FromStr;
use std::sync::{Arc, MutexGuard};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::expiry::within_window;
use lys_identity::grants::revocation::unrevoked;
use lys_identity::grants::{GrantBook, GrantRecord};
use lys_identity::projection::{Projection, Record};
use lys_identity::{AgentId, IdentityId, LifecycleState, OperationId, PersonId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::grant_contract::{GrantView, grant_id};
use crate::grant_sight::{is_root, sees};
use crate::grants::{grant_view, with_grants};
use crate::read_api::{own_person, person_summary};
use crate::read_views::{AgentSummary, PersonSummary};
use crate::reviews_state::{Kept, KeptBatch, period_passed};
use crate::reviews_store::ReviewStore;
use crate::routes::{AppState, signed_in};

/// The review routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/reviews", get(reviews))
        .route("/reviews/{grant}/keep", post(keep))
        .route("/reviews/agents/{agent}/keep", post(keep_agent))
}

/// A decision to keep a grant, as the reviewer sends it.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeepBody {
    operation: String,
    note: String,
}

/// Exactly the grants shown to the reviewer, under one operation.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeepAgentBody {
    operation: String,
    grants: Vec<String>,
}

/// The latest decision to keep a grant.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct KeptView {
    /// The person who kept it.
    pub by: String,
    /// When it was kept, in seconds since the Unix epoch.
    pub at: u64,
    /// The person's words.
    pub note: String,
}

impl From<&Kept> for KeptView {
    fn from(kept: &Kept) -> Self {
        Self {
            by: kept.kept_by.clone(),
            at: kept.at,
            note: kept.note.clone(),
        }
    }
}

/// One grant an agent holds, with the agent and the person who reviews it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DueView {
    /// The grant, as `GET /grants/{id}` answers it.
    pub grant: GrantView,
    /// The agent that holds it.
    pub agent: AgentSummary,
    /// The person the agent answers to, who decides whether it is kept.
    pub reviewer: PersonSummary,
    /// The latest decision to keep the grant, null when it was never kept.
    pub last_kept: Option<KeptView>,
}

/// An agent whose person is not active, so no one answers for it.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct UnansweredView {
    /// The agent.
    pub agent: AgentSummary,
    /// The person it answers to, with the state that leaves it unanswered.
    pub person: PersonSummary,
}

/// The answer of `GET /reviews`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
    /// Whether decisions to keep a grant are recorded here; false when the
    /// configuration names no review decisions' log, and then every
    /// `last_kept` is null.
    pub decisions_recorded: bool,
}

/// An agent in the directory with the person it answers to.
struct Answering<'a> {
    directory: &'a Projection,
    agent: AgentId,
    record: &'a Record,
    person: PersonId,
    person_record: &'a Record,
}

impl Answering<'_> {
    fn agent_summary(&self) -> Result<AgentSummary, ServerError> {
        crate::read_api::agent_summary(self.directory, IdentityId::Agent(self.agent), self.record)
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
        directory,
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

/// The review decisions, settled, or none when the configuration names no
/// log for them.
fn decisions(state: &AppState) -> Result<Option<MutexGuard<'_, ReviewStore>>, ServerError> {
    let Some(store) = state.reviews.as_ref() else {
        return Ok(None);
    };
    let mut store = store
        .lock()
        .map_err(|error| ServerError::ReviewsUnavailable {
            reason: format!("the reviews lock is poisoned: {error}"),
        })?;
    store.settle()?;
    Ok(Some(store))
}

/// The one due rule read by reviews, waiting and the batch act.
fn due(book: &GrantBook, record: &GrantRecord, kept: Option<&Kept>, at: u64, days: u32) -> bool {
    stands(book, record, at) && period_passed(record.grant().window().starts_at(), kept, at, days)
}

async fn reviews(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ReviewView>, ServerError> {
    review_view(&state, &headers).map(Json)
}

/// What `GET /reviews` answers the caller.
pub(crate) fn review_view(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<ReviewView, ServerError> {
    let actor = signed_in(state, headers)?;
    with_grants(state, |judged| {
        let caller = crate::caller_admission::active_caller(judged.directory, &actor)?;
        let pass = crate::routes::admitted_agent(judged.directory, &actor)?;
        let whole = pass || is_root(caller, judged.root);
        let viewer = if whole {
            None
        } else {
            Some(own_person(judged.directory, &actor)?)
        };
        let judged_at = state.sessions.now()?;
        let kept = decisions(state)?;
        let book = judged.grants.book();
        let due = book
            .records()
            .filter(|record| {
                due(
                    book,
                    record,
                    kept.as_ref()
                        .and_then(|store| store.last_for(&record.grant().id().to_string())),
                    judged_at,
                    state.review_period_days,
                )
            })
            .filter_map(|record| {
                answering(judged.directory, record.grant().holder(), viewer).map(|held| {
                    Ok(DueView {
                        grant: grant_view(&judged, caller, record, judged_at),
                        agent: held.agent_summary()?,
                        reviewer: held.person_summary(),
                        last_kept: kept.as_ref().and_then(|store| {
                            store
                                .last_for(&record.grant().id().to_string())
                                .map(KeptView::from)
                        }),
                    })
                })
            })
            .collect::<Result<_, ServerError>>()?;
        let unanswered = judged
            .directory
            .records()
            .filter_map(|(identity, _)| answering(judged.directory, *identity, viewer))
            .filter(|held| held.person_record.state() != LifecycleState::Active)
            .map(|held| {
                Ok(UnansweredView {
                    agent: held.agent_summary()?,
                    person: held.person_summary(),
                })
            })
            .collect::<Result<_, ServerError>>()?;
        Ok(ReviewView {
            scope: if whole { "directory" } else { "personal" }.to_owned(),
            due,
            unanswered,
            revision: judged.grants.revision(),
            judged_at,
            decisions_recorded: kept.is_some(),
        })
    })
}

/// Keep the grant `grant`: only its reviewer or the root authority, and only
/// while it stands. Asked again in the same words it answers what was
/// recorded and writes nothing.
async fn keep(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(grant): Path<String>,
    body: Result<Json<KeepBody>, JsonRejection>,
) -> Result<Json<Kept>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let grant = grant_id(&grant)?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let note = body.note.trim().to_owned();
    with_grants(&state, |judged| {
        let caller = crate::caller_admission::active_caller(judged.directory, &actor)?;
        let pass = crate::routes::admitted_agent(judged.directory, &actor)?;
        let whole = pass || is_root(caller, judged.root);
        let book = judged.grants.book();
        let record = book
            .record(grant)
            .filter(|record| pass || sees(&judged, caller, record))
            .ok_or(ServerError::GrantNotVisible)?;
        let held = answering(judged.directory, record.grant().holder(), None).ok_or_else(|| {
            ServerError::GrantNotDue {
                grant: grant.to_string(),
                why: "it is not held by an agent that answers to a person and is not retired",
            }
        })?;
        if !whole && IdentityId::Person(held.person) != caller {
            return Err(ServerError::ReviewerOnly);
        }
        let at = state.sessions.now()?;
        let kept = Kept {
            grant: grant.to_string(),
            kept_by: caller.to_string(),
            note,
            operation,
            at,
            revision: judged.grants.revision(),
        };
        let mut store = decisions(&state)?.ok_or_else(|| ServerError::ReviewsUnavailable {
            reason: "the configuration names no reviews_dir".to_owned(),
        })?;
        if let Some(recorded) = store.already(&kept)? {
            return Ok(Json(recorded));
        }
        if !stands(book, record, at) {
            return Err(ServerError::GrantNotDue {
                grant: grant.to_string(),
                why: "it does not stand: it or a grant it derives from is revoked, not yet started or ended",
            });
        }
        Ok(Json(store.keep(kept)?))
    })
}

async fn keep_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(agent): Path<String>,
    body: Result<Json<KeepAgentBody>, JsonRejection>,
) -> Result<Json<KeptBatch>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let agent = AgentId::from_str(&agent)?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let grants = body
        .grants
        .iter()
        .map(|id| grant_id(id))
        .collect::<Result<Vec<_>, _>>()?;
    let distinct: std::collections::BTreeSet<_> = grants.iter().collect();
    if grants.is_empty() || distinct.len() != grants.len() {
        return Err(ServerError::RequestMalformed {
            reason: "grants names at least one distinct grant shown to the reviewer".to_owned(),
        });
    }
    with_grants(&state, |judged| {
        let caller = crate::caller_admission::active_caller(judged.directory, &actor)?;
        let pass = crate::routes::admitted_agent(judged.directory, &actor)?;
        let whole = pass || is_root(caller, judged.root);
        let reviewer = caller.to_string();
        let mut store = decisions(&state)?.ok_or_else(|| ServerError::ReviewsUnavailable {
            reason: "the configuration names no reviews_dir".to_owned(),
        })?;
        let shown: Vec<String> = grants.iter().map(ToString::to_string).collect();
        if let Some(recorded) =
            store.already_batch(&operation, &agent.to_string(), &reviewer, &shown)?
        {
            return Ok(Json(recorded));
        }
        let at = state.sessions.now()?;
        let book = judged.grants.book();
        let mut kept = Vec::with_capacity(grants.len());
        for grant in grants {
            let record = book
                .record(grant)
                .filter(|record| pass || sees(&judged, caller, record))
                .ok_or(ServerError::GrantNotVisible)?;
            if record.grant().holder() != IdentityId::Agent(agent) {
                return Err(ServerError::AgentMismatch {
                    grant: grant.to_string(),
                    agent: agent.to_string(),
                });
            }
            let held =
                answering(judged.directory, record.grant().holder(), None).ok_or_else(|| {
                    ServerError::GrantNotDue {
                        grant: grant.to_string(),
                        why: "the holding agent no longer answers to a person",
                    }
                })?;
            if !whole && caller != IdentityId::Person(held.person) {
                return Err(ServerError::ReviewerOnly);
            }
            if !due(
                book,
                record,
                store.last_for(&grant.to_string()),
                at,
                state.review_period_days,
            ) {
                return Err(ServerError::GrantNotDue {
                    grant: grant.to_string(),
                    why: "it does not stand or its review period has not passed since its last check",
                });
            }
            kept.push(Kept {
                grant: grant.to_string(),
                kept_by: reviewer.clone(),
                note: String::new(),
                operation: operation.clone(),
                at,
                revision: judged.grants.revision(),
            });
        }
        store
            .keep_batch(KeptBatch {
                operation,
                agent: agent.to_string(),
                kept,
            })
            .map(Json)
    })
}
