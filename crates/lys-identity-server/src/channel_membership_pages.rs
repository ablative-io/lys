//! Membership pages (ACCESS-006 R3), served at `POST
//! /grants/membership/resources` and `POST /grants/membership/recipients`.
//!
//! A resource page lists the resources of one kind within a workspace that
//! one subject may take an action on. Its candidates are read from indexes,
//! never from the whole book or every placement: the resources the subject
//! holds a grant on (the book's holder index) and the resources placed under
//! them (the apps' children index). A recipient page lists the subjects who
//! may take an action on one channel: the holders of grants on the channel
//! and on each parent whose grants reach it (the book's resource index).
//!
//! Every candidate is then decided by [`decide`], the decision a single
//! membership question gets, at the page's revision: an allowed one is a
//! row, a refused one is counted skipped, and a decision the grants could
//! not make refuses the whole page by its name. A continuation is decided
//! again, so a revocation between pages is seen by the next page and an old
//! cursor carries no permission. Candidates are taken in the stable order
//! of their keys, each once, however many grants reach it.
//!
//! The cursor is the page's question, grant log and revision, and the last
//! key decided, as unpadded URL-safe base64 of its JSON. It is checked
//! against the request it continues, so it cannot be moved to another
//! subject, channel, kind, action or workspace.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, Mode, Resource, Route, identity_kind};
use lys_log_store::LeafStore;
use lys_pass::membership::{self, CONTRACT_VERSION, GrantLog, Subject};
use lys_pass::membership_pages::{
    self, MembershipPage, PageBounds, PageOutcome, PageRow, RecipientPageRequest,
    ResourcePageRequest,
};
use serde::{Deserialize, Serialize};

use crate::channel_membership::{Named, contract_resource, named, pass_mode, served_log, within};
use crate::error::ServerError;
use crate::grants::{Decision, Judged, decide, with_grants};
use crate::grants_batch::{asker, mode_of, unanswered};
use crate::routes::{AppState, identity_id};
use crate::session::now;

/// A page's continuation key and its candidates, each keyed in page order.
type Bound = (Option<String>, BTreeMap<String, ExerciseRequest>);

/// What a cursor binds: the question, the log, the revision it was issued
/// at and the last key decided.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    question: String,
    log: GrantLog,
    revision: u64,
    after: String,
}

fn encode(cursor: &Cursor) -> Result<String, ServerError> {
    let bytes = serde_json::to_vec(cursor).map_err(|error| ServerError::RequestMalformed {
        reason: format!("the cursor could not be written: {error}"),
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// The last key `after` continues from, refused by name unless it was
/// issued for `question` from `served` at a revision not after `revision`.
fn continued(
    after: Option<&str>,
    question: &str,
    served: &GrantLog,
    revision: u64,
) -> Result<Option<String>, Named> {
    let Some(text) = after else {
        return Ok(None);
    };
    let malformed = |error: &dyn std::fmt::Display| {
        (
            membership_pages::CURSOR_MALFORMED.to_owned(),
            format!("the cursor is not one this service issued: {error}"),
        )
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|error| malformed(&error))?;
    let cursor: Cursor = serde_json::from_slice(&bytes).map_err(|error| malformed(&error))?;
    if &cursor.log != served || cursor.revision > revision {
        return Err((
            membership_pages::CURSOR_RESET.to_owned(),
            "the cursor was issued from another grant log, epoch or later revision".to_owned(),
        ));
    }
    if cursor.question != question {
        return Err((
            membership_pages::CURSOR_FOREIGN.to_owned(),
            "the cursor was issued for another question".to_owned(),
        ));
    }
    Ok(Some(cursor.after))
}

/// One candidate's decision as a page row, `None` when refused, or the
/// name of a decision the grants could not make.
fn row<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    asked: &ExerciseRequest,
    at: u64,
    at_least: u64,
) -> Result<Option<PageRow>, Named> {
    match decide(judged, asked, at, Some(at_least), Decision::Explain) {
        Ok((permit, via)) => Ok(mode_of(judged, permit.grant).map(|mode: Mode| PageRow {
            subject: Subject {
                id: asked.caller.to_string(),
                kind: identity_kind(&asked.caller).to_owned(),
            },
            resource: contract_resource(&asked.resource),
            grant: permit.grant.to_string(),
            scope: contract_resource(&via),
            mode: pass_mode(mode),
        })),
        Err(error) if unanswered(&error) => Err(named(error)),
        Err(_) => Ok(None),
    }
}

/// Decide `candidates` after the cursor's key, in key order, into one page
/// of at most `bounds`, answered at the grants' revision.
fn fill<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    candidates: BTreeMap<String, ExerciseRequest>,
    (question, after): (&str, Option<String>),
    served: &GrantLog,
    bounds: PageBounds,
    (at, at_least): (u64, u64),
) -> Result<MembershipPage, ServerError> {
    let (mut rows, mut skipped, mut spent) = (Vec::new(), 0_u64, 0_usize);
    let mut last = None;
    let mut more = false;
    let budget = usize::try_from(bounds.bytes).unwrap_or(usize::MAX);
    let room = usize::try_from(bounds.rows).unwrap_or(usize::MAX);
    let start = after.as_deref();
    for (key, asked) in candidates
        .iter()
        .filter(|(key, _)| start.is_none_or(|after| key.as_str() > after))
    {
        let decided = match row(judged, asked, at, at_least) {
            Ok(decided) => decided,
            Err((name, reason)) => {
                return Ok(membership_pages::refused_page(served, &name, &reason));
            }
        };
        let Some(decided) = decided else {
            skipped += 1;
            last = Some(key.clone());
            continue;
        };
        let size = serde_json::to_vec(&decided)
            .map_err(|error| ServerError::RequestMalformed {
                reason: format!("a page row could not be written: {error}"),
            })?
            .len();
        if size > budget {
            return Ok(membership_pages::refused_page(
                served,
                membership_pages::ROW_OVER_BUDGET,
                "one row is larger than the page's byte budget",
            ));
        }
        if rows.len() == room || spent + size > budget {
            more = true;
            break;
        }
        spent += size;
        rows.push(decided);
        last = Some(key.clone());
    }
    let revision = judged.grants.revision();
    let next = match (more, last) {
        (true, Some(after)) => Some(encode(&Cursor {
            question: question.to_owned(),
            log: served.clone(),
            revision,
            after,
        })?),
        _ => None,
    };
    Ok(MembershipPage {
        contract: CONTRACT_VERSION,
        log: served.clone(),
        revision: Some(revision),
        outcome: PageOutcome::Page {
            returned: u64::try_from(rows.len()).unwrap_or(u64::MAX),
            rows,
            skipped,
            complete: next.is_none(),
            next,
        },
    })
}

/// The resources a subject may act on, gathered from indexes: those it
/// holds a grant on and those placed under them, of `kind`, in `workspace`.
fn held_resources<S: LeafStore>(
    judged: &Judged<'_, S>,
    subject: IdentityId,
    kind: &str,
    workspace: &Resource,
) -> BTreeSet<Resource> {
    let held = judged.apps.held();
    let mut queue: Vec<Resource> = judged
        .grants
        .book()
        .held_by(subject)
        .map(|record| record.grant().resource().clone())
        .collect();
    let (mut seen, mut found) = (BTreeSet::new(), BTreeSet::new());
    while let Some(resource) = queue.pop() {
        if !seen.insert(resource.clone()) {
            continue;
        }
        queue.extend(
            held.children(resource.kind(), resource.id())
                .filter_map(|placed| Resource::new(&placed.child_kind, &placed.child_id).ok()),
        );
        if resource.kind() == kind {
            found.insert(resource);
        }
    }
    found.retain(|resource| within(judged, resource, workspace));
    found
}

fn admit<S: LeafStore>(
    judged: &Judged<'_, S>,
    acting_for: Option<&str>,
    (workspace, kind, action): (&str, &str, &str),
) -> Result<(), Named> {
    for asked in [workspace, kind] {
        judged.apps.admit_kind(acting_for, asked).map_err(named)?;
    }
    judged.apps.admit_action(kind, action).map_err(named)
}

fn resources_page<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    acting_for: Option<&str>,
    served: &GrantLog,
    request: &ResourcePageRequest,
    at: u64,
) -> Result<MembershipPage, ServerError> {
    let refuse = |(name, reason): Named| Ok(membership_pages::refused_page(served, &name, &reason));
    if let Err(refused) = membership_pages::validate_resources(request, served) {
        return refuse((refused.name.to_owned(), refused.reason));
    }
    let question = serde_json::to_string(&(
        "resources",
        &request.workspace,
        &request.subject,
        &request.kind,
        &request.action,
    ))
    .map_err(|error| ServerError::RequestMalformed {
        reason: error.to_string(),
    })?;
    let bound = (|| -> Result<Bound, Named> {
        let after = continued(
            request.after.as_deref(),
            &question,
            served,
            judged.grants.revision(),
        )?;
        let subject = identity_id(&request.subject.id).map_err(named)?;
        if identity_kind(&subject) != request.subject.kind {
            return Err((
                membership::SUBJECT_KIND_MISMATCH.to_owned(),
                "the subject is not of the kind stated".to_owned(),
            ));
        }
        admit(
            judged,
            acting_for,
            (&request.workspace.kind, &request.kind, &request.action),
        )?;
        let workspace =
            Resource::new(&request.workspace.kind, &request.workspace.id).map_err(named)?;
        let action = Action::new(&request.action).map_err(named)?;
        let candidates = held_resources(judged, subject, &request.kind, &workspace)
            .into_iter()
            .map(|resource| {
                let key = resource.id().to_owned();
                let asked = ExerciseRequest {
                    caller: subject,
                    route: Route::Api,
                    resource,
                    action: action.clone(),
                };
                (key, asked)
            })
            .collect();
        Ok((after, candidates))
    })();
    let (after, candidates) = match bound {
        Ok(bound) => bound,
        Err(refused) => return refuse(refused),
    };
    fill(
        judged,
        candidates,
        (&question, after),
        served,
        request.bounds,
        (at, request.at_least),
    )
}

fn recipients_page<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    acting_for: Option<&str>,
    served: &GrantLog,
    request: &RecipientPageRequest,
    at: u64,
) -> Result<MembershipPage, ServerError> {
    let refuse = |(name, reason): Named| Ok(membership_pages::refused_page(served, &name, &reason));
    if let Err(refused) = membership_pages::validate_recipients(request, served) {
        return refuse((refused.name.to_owned(), refused.reason));
    }
    let question = serde_json::to_string(&(
        "recipients",
        &request.workspace,
        &request.resource,
        &request.action,
    ))
    .map_err(|error| ServerError::RequestMalformed {
        reason: error.to_string(),
    })?;
    let bound = (|| -> Result<Bound, Named> {
        let after = continued(
            request.after.as_deref(),
            &question,
            served,
            judged.grants.revision(),
        )?;
        admit(
            judged,
            acting_for,
            (
                &request.workspace.kind,
                &request.resource.kind,
                &request.action,
            ),
        )?;
        let resource =
            Resource::new(&request.resource.kind, &request.resource.id).map_err(named)?;
        let workspace =
            Resource::new(&request.workspace.kind, &request.workspace.id).map_err(named)?;
        if !within(judged, &resource, &workspace) {
            return Err((
                membership::OUTSIDE_WORKSPACE.to_owned(),
                "the resource is not placed under the named workspace".to_owned(),
            ));
        }
        let action = Action::new(&request.action).map_err(named)?;
        let book = judged.grants.book();
        let candidates = judged
            .apps
            .reach(&resource)
            .iter()
            .flat_map(|reached| book.on_resource(reached))
            .map(|record| record.grant().holder())
            .collect::<BTreeSet<IdentityId>>()
            .into_iter()
            .map(|holder| {
                let asked = ExerciseRequest {
                    caller: holder,
                    route: Route::Api,
                    resource: resource.clone(),
                    action: action.clone(),
                };
                (holder.to_string(), asked)
            })
            .collect();
        Ok((after, candidates))
    })();
    let (after, candidates) = match bound {
        Ok(bound) => bound,
        Err(refused) => return refuse(refused),
    };
    fill(
        judged,
        candidates,
        (&question, after),
        served,
        request.bounds,
        (at, request.at_least),
    )
}

/// A page of the resources one subject may act on within a workspace.
pub async fn resources(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ResourcePageRequest>, JsonRejection>,
) -> Result<Json<MembershipPage>, ServerError> {
    let Json(request) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let acting_for = asker(&state, &headers)?;
    let served = served_log(&state);
    let at = now();
    with_grants(&state, |mut judged| {
        resources_page(&mut judged, acting_for.as_deref(), &served, &request, at).map(Json)
    })
}

/// A page of the subjects who may act on one channel.
pub async fn recipients(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<RecipientPageRequest>, JsonRejection>,
) -> Result<Json<MembershipPage>, ServerError> {
    let Json(request) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let acting_for = asker(&state, &headers)?;
    let served = served_log(&state);
    let at = now();
    with_grants(&state, |mut judged| {
        recipients_page(&mut judged, acting_for.as_deref(), &served, &request, at).map(Json)
    })
}
