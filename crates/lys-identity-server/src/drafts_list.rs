//! `GET /drafts`: the drafts the signed-in person may decide, each carrying
//! everything a decision needs, read from the directory's projection. It is a
//! question: it records nothing.
//!
//! A draft is listed to exactly the caller who may decide it at
//! `POST /drafts/{id}/approve` or `/refuse`: the signed-in person's own
//! session, when they are the draft's responsible person. No one else is
//! shown another person's drafts, the administrator included, since the
//! decisions admit no one else either.
//!
//! `state` chooses the rows: `waiting`, the default, answers the drafts still
//! awaiting a decision; `decided` those approved, refused or corrected; `all`
//! both. A correction's own change, saved beside the refusal it records,
//! awaits no decision and is not a row of its own: the corrected row names
//! it. The list pages as the other lists do (`list_page`); its search reads
//! the agent's name, the note, the path and the target, and its team filter
//! keeps a draft whose agent or responsible person is a member.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::http::HeaderMap;
use lys_identity::projection::Projection;
use lys_identity::projection::draft::DraftRecord;
use lys_identity::{Actor, IdentityError, IdentityId, PersonId};
use serde::{Deserialize, Serialize};

use crate::drafts_api::{decides, personal};
use crate::error::ServerError;
use crate::list_page::{ListQuery, Page, Totals};
use crate::read_api::{login, own_person, person_record, person_summary};
use crate::read_views::{Login, PersonSummary};
use crate::routes::{AppState, hex, with_directory};

/// Which drafts the list answers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[schema(as = DraftsState)]
pub enum Standing {
    /// The drafts still awaiting a decision.
    #[default]
    Waiting,
    /// The drafts approved, refused or corrected.
    Decided,
    /// Both.
    All,
}

impl Standing {
    fn holds(self, held: &DraftRecord) -> bool {
        match self {
            Self::Waiting => held.is_pending(),
            Self::Decided => !held.is_pending(),
            Self::All => true,
        }
    }

    /// The list a paging cursor belongs to: a cursor of one state is refused under another.
    fn route(self) -> &'static str {
        match self {
            Self::Waiting => "/drafts",
            Self::Decided => "/drafts?state=decided",
            Self::All => "/drafts?state=all",
        }
    }
}

/// The list's query: the members every list shares, and `state`.
#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DraftsQuery {
    /// Case-insensitive text contained in the agent's name, the note, the path or the target.
    pub q: Option<String>,
    /// A team and every descendant in its current nesting tree.
    pub team: Option<String>,
    /// The opaque cursor returned by the preceding page with the same filters and state.
    pub after: Option<String>,
    /// Page size, defaulting to 50 and capped at 200 as every list's is; zero is refused.
    pub limit: Option<usize>,
    /// `waiting` (the default), `decided` or `all`.
    pub state: Option<Standing>,
}

/// The agent that submitted a draft.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DraftAgent {
    /// The agent's id.
    pub id: String,
    /// Its display name, null when the directory no longer holds it.
    pub display_name: Option<String>,
}

/// The resource and action a draft's prepared change names.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DraftTargetView {
    /// The resource kind.
    pub kind: String,
    /// The resource id.
    pub id: String,
    /// The action the change exercises.
    pub action: String,
}

/// The person who decided a draft.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DraftDecider {
    /// The person's id.
    pub id: String,
    /// Their display name.
    pub display_name: String,
}

/// How a draft was decided.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DraftDecision {
    /// The decision's own operation id.
    pub operation: String,
    /// The person whose session decided it, null when that login is no longer bound to a person.
    pub by: Option<DraftDecider>,
    /// The login the decision was made through.
    pub login: Login,
    /// When it was decided, in seconds since the Unix epoch.
    pub at: u64,
    /// Why it was refused or corrected; absent for an approval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The operation an approval reserved for applying the change; null otherwise.
    pub application: Option<String>,
    /// The draft id of a correction's own change; null otherwise.
    pub replacement: Option<String>,
}

/// One draft, with everything a decision on it needs.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DraftView {
    /// The draft's id: the operation it was created under.
    pub id: String,
    /// The agent whose signed request submitted it, null when no agent did.
    pub agent: Option<DraftAgent>,
    /// The person responsible for it, who decides it.
    pub responsible: PersonSummary,
    /// What the prepared change acts on.
    pub target: DraftTargetView,
    /// The prepared request's method.
    pub method: String,
    /// The prepared request's path.
    pub path: String,
    /// The prepared request's body, whole, exactly as submitted.
    pub body: String,
    /// The submitter's note to the person deciding.
    pub note: String,
    /// When it was created, in seconds since the Unix epoch.
    pub created_at: u64,
    /// The SHA-256 of its creation, in hex, which an approval or refusal names.
    pub creation_hash: String,
    /// `waiting`, `approved`, `refused` or `corrected`.
    pub state: &'static str,
    /// The decision, absent while it waits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided: Option<DraftDecision>,
}

/// The answer of `GET /drafts`.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct DraftList {
    /// The drafts, in draft id order.
    pub drafts: Vec<DraftView>,
    /// Counters and continuation, absent when no query was supplied.
    #[serde(flatten)]
    pub page: Option<Totals>,
}

pub(crate) async fn list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<DraftsQuery>, QueryRejection>,
) -> Result<Json<DraftList>, ServerError> {
    let Query(query) = query.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    listed(&state, &headers, query).map(Json)
}

/// What `GET /drafts` answers the caller for `query`.
pub(crate) fn listed(
    state: &AppState,
    headers: &HeaderMap,
    query: DraftsQuery,
) -> Result<DraftList, ServerError> {
    let actor = personal(state, headers)?;
    let standing = query.state.unwrap_or_default();
    let shared = ListQuery {
        q: query.q,
        team: query.team,
        after: query.after,
        limit: query.limit,
    };
    let page = Page::of(shared, standing.route())?;
    let members = page
        .as_ref()
        .map(|page| page.members(state))
        .transpose()?
        .flatten();
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, &actor)?;
        let responsible = person_summary(person, person_record(projection, person)?);
        let rows = projection
            .drafts()
            .filter(|held| {
                held.replacement_of.is_none() && decides(held, person) && standing.holds(held)
            })
            .map(|held| (held.created.operation.to_string(), held));
        let shown = |held: &DraftRecord| view(projection, &responsible, held);
        let Some(page) = page else {
            return Ok(DraftList {
                drafts: rows
                    .map(|(_, held)| shown(held))
                    .collect::<Result<_, _>>()?,
                page: None,
            });
        };
        let person = person.to_string();
        let (drafts, totals) = page.select(
            rows,
            None,
            |(_, held)| {
                let created = &held.created;
                let agent = created.actor.provenance().agent();
                let name = agent
                    .and_then(|agent| projection.record(IdentityId::Agent(agent)))
                    .map_or("", |record| record.profile().display_name());
                let agent = agent.map(|agent| agent.to_string());
                Ok(crate::list_page::member(
                    members.as_ref(),
                    agent.as_deref().unwrap_or(&person),
                    Some(&person),
                ) && page.matches([
                    name,
                    created.note.as_str(),
                    created.path.as_str(),
                    created.target.kind.as_str(),
                    created.target.id.as_str(),
                    created.target.action.as_str(),
                ]))
            },
            |(id, _)| id,
            |(_, held)| shown(held),
        )?;
        Ok(DraftList {
            drafts,
            page: Some(totals),
        })
    })
}

fn decision(projection: &Projection, actor: &Actor, operation: String, at: u64) -> DraftDecision {
    DraftDecision {
        operation,
        by: projection
            .person_for(actor.binding())
            .and_then(|person: PersonId| {
                projection
                    .record(IdentityId::Person(person))
                    .map(|record| DraftDecider {
                        id: person.to_string(),
                        display_name: record.profile().display_name().to_owned(),
                    })
            }),
        login: login(actor.binding()),
        at,
        reason: None,
        application: None,
        replacement: None,
    }
}

fn view(
    projection: &Projection,
    responsible: &PersonSummary,
    held: &DraftRecord,
) -> Result<DraftView, ServerError> {
    let created = &held.created;
    let body = String::from_utf8(created.body.clone()).map_err(|error| {
        ServerError::from(IdentityError::LogUnavailable {
            reason: format!(
                "draft {}'s prepared body is not text: {error}",
                created.operation
            ),
        })
    })?;
    let (state, decision) = if let Some((approved, _)) = &held.approved {
        let mut decided = decision(
            projection,
            &approved.actor,
            approved.operation.to_string(),
            approved.recorded_at,
        );
        decided.application = Some(approved.application.to_string());
        ("approved", Some(decided))
    } else if let Some((refused, _)) = &held.refused {
        let mut decided = decision(
            projection,
            &refused.actor,
            refused.operation.to_string(),
            refused.recorded_at,
        );
        decided.reason = Some(refused.reason.clone());
        ("refused", Some(decided))
    } else if let Some((corrected, _)) = &held.correction {
        let mut decided = decision(
            projection,
            &corrected.actor,
            corrected.operation.to_string(),
            corrected.recorded_at,
        );
        decided.reason = Some(corrected.reason.clone());
        decided.replacement = Some(corrected.corrected.operation.to_string());
        ("corrected", Some(decided))
    } else {
        ("waiting", None)
    };
    Ok(DraftView {
        id: created.operation.to_string(),
        agent: created.actor.provenance().agent().map(|agent| DraftAgent {
            id: agent.to_string(),
            display_name: projection
                .record(IdentityId::Agent(agent))
                .map(|record| record.profile().display_name().to_owned()),
        }),
        responsible: responsible.clone(),
        target: DraftTargetView {
            kind: created.target.kind.clone(),
            id: created.target.id.clone(),
            action: created.target.action.clone(),
        },
        method: created.method.clone(),
        path: created.path.clone(),
        body,
        note: created.note.clone(),
        created_at: created.recorded_at,
        creation_hash: hex(&held.hash),
        state,
        decided: decision,
    })
}
