//! `GET /waiting`: everything waiting on the signed-in caller, in one list.
//!
//! Four parts, each read by the function its own route reads it with, so the
//! list can never disagree with the pages it stands for:
//!
//! - `request`: the requests `GET /requests` answers `waiting` that the caller
//!   may decide, who may decide being worked out from the grants that stand now;
//! - `agent_draft`: the drafts `GET /drafts` answers waiting on the caller;
//! - `product_draft`: the product drafts waiting on an approval the caller may
//!   give: never the holder's or the responsible person's own, never one the
//!   caller approved already, and only where the caller may grant its action;
//! - `review`: the grants `GET /reviews` answers due whose reviewer is the caller.
//!
//! Every entry is one the caller may decide; what they may only watch is not
//! listed. A `request` entry carries the request exactly as `GET /requests`
//! answers it and an `agent_draft` entry the draft exactly as `GET /drafts`
//! answers it, the very views those lists built the entry from. An app action
//! carries its product draft; a review row carries one agent's due grants.
//! One read serves the count and every decision in its row. Entries are oldest
//! first by when they were asked, equal instants in
//! kind and then id order. A grant records no issue time of its own, so a
//! review was asked when the grant was last kept, else when its window opened.
//!
//! A part the caller may not read is one `{kind, refusal, reason}` line ahead of
//! the entries, in place of that part's entries: never dropped, never read as
//! nothing. The route answers the signed-in caller's own list and no one else's;
//! a caller not signed in is refused outright.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::error::ServerError;
use crate::list_page::ListQuery;
use crate::routes::{AppState, signed_in, with_directory};

/// The waiting route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/waiting", get(waiting))
}

pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    vec![(
        lys_openapi::Method::Get,
        "/waiting",
        None,
        Some(api.schema::<WaitingList>()),
    )]
}

/// Who asked for an entry.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[schema(as = WaitingAsker)]
pub struct Asker {
    /// Their id: a person or an agent.
    pub id: String,
    /// Their display name, null when the directory does not show them.
    pub name: Option<String>,
}

/// One thing waiting on the caller's decision.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WaitingEntry {
    /// `request`, `agent_draft`, `product_draft` or `review`.
    pub kind: &'static str,
    /// The request's, draft's or holding agent's id.
    pub id: String,
    /// Who asked: the asker, the drafting agent, the drafting holder or the holding agent.
    pub asked_by: Asker,
    /// What it is, in one sentence as its own list says it.
    pub about: String,
    /// When it was asked, RFC 3339 in UTC.
    pub asked_at: String,
    /// Whether the caller may decide it: always true, as nothing else is listed.
    pub can_decide: bool,
    /// A `request` entry's request, exactly as `GET /requests` answers it; absent on every other kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Box<crate::requests_views::RequestView>>,
    /// An `agent_draft` entry's draft, exactly as `GET /drafts` answers it; absent on every other kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<Box<crate::drafts_list::DraftView>>,
    /// An app action, exactly as `GET /product-drafts` answers it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_draft: Option<Box<crate::product_drafts::wire::ProductDraftView>>,
    /// One agent's due grants, exactly the ids a Keep all act sends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<Box<WaitingReview>>,
}

/// One agent's checks, counted as one line waiting on the person.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WaitingReview {
    /// The agent whose access is checked.
    pub agent: crate::read_views::AgentSummary,
    /// Each due grant, in grant id order.
    pub grants: Vec<WaitingGrant>,
    /// How many grants this one row checks.
    pub count: usize,
}

/// What the reviewer needs to recognise one grant and its last check.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WaitingGrant {
    /// The exact id a keep will name.
    pub id: String,
    /// The access level.
    pub relation: String,
    /// The resource in the list's words.
    pub resource: String,
    /// Last keep, else the start of the window, in Unix seconds.
    pub last_check: u64,
}

/// A part the caller may not read, in place of its entries.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct WaitingRefused {
    /// The part: `request`, `agent_draft`, `product_draft` or `review`.
    pub kind: &'static str,
    /// The refusal its own route answers, by name.
    pub refusal: String,
    /// Why, in the refusal's words.
    pub reason: String,
}

/// One line of the list: an entry, or a part that could not be read.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum WaitingItem {
    /// Something the caller may decide.
    Entry(WaitingEntry),
    /// A part the caller may not read.
    Refused(WaitingRefused),
}

/// The answer of `GET /waiting`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct WaitingList {
    /// The refused parts, then the entries oldest first.
    pub waiting: Vec<WaitingItem>,
}

/// An entry with the second it was asked at, which orders it.
pub(crate) struct Dated {
    pub(crate) at: u64,
    pub(crate) entry: WaitingEntry,
}

/// The refused parts, then the entries oldest first, ties in kind and id order.
pub(crate) fn ordered(mut entries: Vec<Dated>, refused: Vec<WaitingRefused>) -> Vec<WaitingItem> {
    entries.sort_by(|left, right| {
        (left.at, left.entry.kind, &left.entry.id).cmp(&(
            right.at,
            right.entry.kind,
            &right.entry.id,
        ))
    });
    refused
        .into_iter()
        .map(WaitingItem::Refused)
        .chain(
            entries
                .into_iter()
                .map(|dated| WaitingItem::Entry(dated.entry)),
        )
        .collect()
}

async fn waiting(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<WaitingList>, ServerError> {
    signed_in(&state, &headers)?;
    let mut entries = Vec::new();
    let mut refused = Vec::new();
    for (kind, part) in [
        ("request", requests(&state, &headers)),
        ("agent_draft", agent_drafts(&state, &headers)),
        ("product_draft", product_drafts(&state, &headers)),
        ("review", reviews(&state, &headers)),
    ] {
        match part {
            Ok(found) => entries.extend(found),
            Err(error) => refused.push(WaitingRefused {
                kind,
                refusal: error.name(),
                reason: error.to_string(),
            }),
        }
    }
    Ok(Json(WaitingList {
        waiting: ordered(entries, refused),
    }))
}

/// `seconds` since the Unix epoch as RFC 3339 in UTC.
fn rfc3339(seconds: u64) -> Result<String, ServerError> {
    i64::try_from(seconds)
        .ok()
        .and_then(|seconds| jiff::Timestamp::from_second(seconds).ok())
        .map(|instant| instant.to_string())
        .ok_or_else(|| ServerError::ClockUnavailable {
            reason: format!("{seconds} seconds since the epoch is not a representable instant"),
        })
}

fn dated(
    kind: &'static str,
    id: String,
    asked_by: Asker,
    about: String,
    at: u64,
) -> Result<Dated, ServerError> {
    Ok(Dated {
        at,
        entry: WaitingEntry {
            kind,
            id,
            asked_by,
            about,
            asked_at: rfc3339(at)?,
            can_decide: true,
            request: None,
            draft: None,
            product_draft: None,
            review: None,
        },
    })
}

/// A resource in the words the lists use: "project logo", "the people directory".
fn resource_words(kind: &str, id: &str) -> String {
    if kind == "directory" {
        format!("the {id} directory")
    } else {
        format!("{kind} {id}")
    }
}

/// "Sam asks to edit project logo"; a relation with no plain word: "Sam asks for auditor access to project logo".
fn asks(name: Option<&str>, relation: &str, resource: &str) -> String {
    let who = name.unwrap_or("Someone outside your view");
    match relation {
        "viewer" => format!("{who} asks to view {resource}"),
        "editor" => format!("{who} asks to edit {resource}"),
        "owner" => format!("{who} asks to own {resource}"),
        other => format!("{who} asks for {other} access to {resource}"),
    }
}

fn requests(state: &AppState, headers: &HeaderMap) -> Result<Vec<Dated>, ServerError> {
    let list = crate::requests_api::listed(state, headers, Ok(Query(ListQuery::default())))?;
    list.requests
        .into_iter()
        .filter(|request| request.state == "waiting" && request.can_decide)
        .map(|request| {
            let about = asks(
                request.asked_by_name.as_deref(),
                &request.relation,
                &resource_words(&request.resource.kind, &request.resource.id),
            );
            let mut dated = dated(
                "request",
                request.id.clone(),
                Asker {
                    id: request.asked_by.clone(),
                    name: request.asked_by_name.clone(),
                },
                about,
                request.asked_at,
            )?;
            dated.entry.request = Some(Box::new(request));
            Ok(dated)
        })
        .collect()
}

fn agent_drafts(state: &AppState, headers: &HeaderMap) -> Result<Vec<Dated>, ServerError> {
    let list =
        crate::drafts_list::listed(state, headers, crate::drafts_list::DraftsQuery::default())?;
    list.drafts
        .into_iter()
        .map(|draft| {
            let asked_by = match &draft.agent {
                Some(agent) => Asker {
                    id: agent.id.clone(),
                    name: agent.display_name.clone(),
                },
                None => Asker {
                    id: draft.responsible.id.clone(),
                    name: Some(draft.responsible.display_name.clone()),
                },
            };
            let who = asked_by
                .name
                .as_deref()
                .unwrap_or("An agent outside your view");
            let about = if draft.note.trim().is_empty() {
                format!("{who} prepared something for you to approve")
            } else {
                format!(
                    "{who} prepared something for you to approve: {}",
                    draft.note.trim()
                )
            };
            let mut dated = dated(
                "agent_draft",
                draft.id.clone(),
                asked_by,
                about,
                draft.created_at,
            )?;
            dated.entry.draft = Some(Box::new(draft));
            Ok(dated)
        })
        .collect()
}

fn product_drafts(state: &AppState, headers: &HeaderMap) -> Result<Vec<Dated>, ServerError> {
    crate::product_drafts::waiting_on(state, headers)?
        .into_iter()
        .map(|(draft, name, at)| {
            let who = name.as_deref().unwrap_or("Someone outside your view");
            let about = format!(
                "{who} asks to {} {} through {}",
                draft.target.action,
                resource_words(&draft.target.kind, &draft.target.id),
                draft.app
            );
            let mut dated = dated(
                "product_draft",
                draft.id.clone(),
                Asker {
                    id: draft.holder.clone(),
                    name,
                },
                about,
                at,
            )?;
            dated.entry.product_draft = Some(Box::new(draft));
            Ok(dated)
        })
        .collect()
}

fn reviews(state: &AppState, headers: &HeaderMap) -> Result<Vec<Dated>, ServerError> {
    let view = crate::reviews_api::review_view(state, headers)?;
    let actor = signed_in(state, headers)?;
    let person = with_directory(state, |directory| {
        crate::read_api::own_person(directory.projection()?, &actor)
    })?
    .to_string();
    let mut groups: BTreeMap<String, WaitingReview> = BTreeMap::new();
    for due in view.due.into_iter().filter(|due| due.reviewer.id == person) {
        let last_check = due
            .last_kept
            .as_ref()
            .map_or(due.grant.window.starts_at, |kept| kept.at);
        let group = groups
            .entry(due.agent.id.clone())
            .or_insert_with(|| WaitingReview {
                agent: due.agent,
                grants: Vec::new(),
                count: 0,
            });
        group.grants.push(WaitingGrant {
            id: due.grant.id,
            relation: due.grant.relation,
            resource: resource_words(&due.grant.resource.kind, &due.grant.resource.id),
            last_check,
        });
        group.count += 1;
    }
    groups
        .into_values()
        .map(|group| {
            let at = group
                .grants
                .iter()
                .map(|grant| grant.last_check)
                .min()
                .ok_or_else(|| ServerError::ReviewsUnavailable {
                    reason: "a review row has no grants".to_owned(),
                })?;
            let about = format!(
                "Check the access {} holds: {} grants",
                group.agent.display_name, group.count
            );
            let mut dated = dated(
                "review",
                group.agent.id.clone(),
                Asker {
                    id: group.agent.id.clone(),
                    name: Some(group.agent.display_name.clone()),
                },
                about,
                at,
            )?;
            dated.entry.review = Some(Box::new(group));
            Ok(dated)
        })
        .collect()
}
