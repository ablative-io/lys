//! Membership pages (ACCESS-006 R3), served at `POST
//! /grants/membership/resources` and `POST /grants/membership/recipients`.
//!
//! A resource page lists the resources of one kind within a workspace that
//! one subject may take an action on. Its candidates are read from indexes,
//! never from the whole book, every placement or the retired history: the
//! resources the subject holds a live grant on (the book's live holder
//! index) and the resources placed under them (the apps' children index). A
//! recipient page lists the subjects who may take an action on one channel:
//! the holders of live grants on the channel and on each parent whose
//! grants reach it (the book's live resource index).
//!
//! Every candidate is then decided by [`decide`], the decision a single
//! membership question gets, at the page's revision: an allowed one is a
//! row, a refused one is counted skipped, and a decision the grants could
//! not make refuses the whole page by its name. A continuation is decided
//! again, so a revocation between pages is seen by the next page and an old
//! cursor carries no permission. Candidates are taken in the stable order
//! of their keys, each once, however many grants reach it.
//!
//! Both bounds are the caller's and neither may pass the operator's
//! ceiling, which is a required setting: without it no page is served.
//! The cursor is a handle the service keeps (`channel_membership_cursors`),
//! bound to the asker and the page's question, so it cannot be forged or
//! moved to another subject, channel, kind, action or workspace.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, Mode, Resource, Route, identity_kind};
use lys_log_store::LeafStore;
use lys_pass::membership::{self, CONTRACT_VERSION, GrantLog, Subject};
use lys_pass::membership_pages::{
    self, MembershipPage, PageBounds, PageOutcome, PageRow, RecipientPageRequest,
    ResourcePageRequest,
};
use rand::RngCore;

use crate::channel_membership::{Named, contract_resource, named, pass_mode, served_log, within};
use crate::channel_membership_counts::{Counts, Membership};
use crate::channel_membership_cursors::{Asked, Continued, HANDLE_BYTES};
use crate::error::ServerError;
use crate::grants::{Decision, Judged, decide, with_grants};
use crate::grants_batch::{asker, mode_of, unanswered};
use crate::routes::{AppState, identity_id};
use crate::session::now;

/// The candidates of one page, each keyed in page order.
type Candidates = BTreeMap<String, ExerciseRequest>;

/// One page request as every page takes it: who asks, the log served, the
/// time, the randomness of a next cursor, and the capability's state.
struct Asking<'a> {
    acting_for: Option<&'a str>,
    served: &'a GrantLog,
    at: u64,
    random: [u8; HANDLE_BYTES],
    membership: &'a Membership,
}

/// What every page request carries besides its question.
struct Common<'a> {
    question: String,
    bounds: PageBounds,
    at_least: u64,
    after: Option<&'a str>,
}

/// Refuse a page before anything is looked up unless the operator's
/// ceiling is configured and `bounds` stays within it.
fn ceiling(membership: &Membership, bounds: PageBounds) -> Result<(), Named> {
    let Some(settings) = membership.settings else {
        return Err((
            membership_pages::PAGES_UNCONFIGURED.to_owned(),
            "this service has no membership page ceiling configured".to_owned(),
        ));
    };
    let most = PageBounds {
        rows: settings.page_rows_max,
        bytes: settings.page_bytes_max,
    };
    membership_pages::within_ceiling(bounds, most)
        .map_err(|refused| (refused.name.to_owned(), refused.reason))
}

/// One candidate's decision as a page row, `None` when refused, or the
/// name of a decision the grants could not make.
fn row<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    asked: &ExerciseRequest,
    (at, at_least): (u64, u64),
    counts: &Counts,
) -> Result<Option<PageRow>, Named> {
    counts.decision();
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

/// The rows of one page: those emitted, how many were skipped, the last
/// key decided and whether a candidate remains.
struct Filled {
    rows: Vec<PageRow>,
    skipped: u64,
    last: Option<String>,
    more: bool,
}

/// Decide `candidates` after `after`, in key order, into at most `bounds`.
fn fill<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    candidates: &Candidates,
    (after, bounds, at, at_least): (Option<&str>, PageBounds, u64, u64),
    counts: &Counts,
) -> Result<Filled, Named> {
    let mut filled = Filled {
        rows: Vec::new(),
        skipped: 0,
        last: None,
        more: false,
    };
    let budget = usize::try_from(bounds.bytes).unwrap_or(usize::MAX);
    let room = usize::try_from(bounds.rows).unwrap_or(usize::MAX);
    let mut spent = 0_usize;
    for (key, asked) in candidates
        .iter()
        .filter(|(key, _)| after.is_none_or(|after| key.as_str() > after))
    {
        let Some(decided) = row(judged, asked, (at, at_least), counts)? else {
            filled.skipped += 1;
            filled.last = Some(key.clone());
            continue;
        };
        let size = serde_json::to_vec(&decided)
            .map_err(|error| {
                (
                    membership::REQUEST_MALFORMED.to_owned(),
                    format!("a page row could not be written: {error}"),
                )
            })?
            .len();
        if size > budget {
            return Err((
                membership_pages::ROW_OVER_BUDGET.to_owned(),
                "one row is larger than the page's byte budget".to_owned(),
            ));
        }
        if filled.rows.len() == room || spent + size > budget {
            filled.more = true;
            break;
        }
        spent += size;
        filled.rows.push(decided);
        filled.last = Some(key.clone());
    }
    Ok(filled)
}

/// The page `candidates` give for `common`, continuing its cursor and
/// keeping the next one; the listing's cursors are released when it
/// completes. Every refusal is the whole page's, by name.
fn answer<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    asking: &Asking<'_>,
    common: &Common<'_>,
    candidates: impl FnOnce(&Judged<'_, S>) -> Result<Candidates, Named>,
) -> Result<MembershipPage, Named> {
    let membership = asking.membership;
    let counts = &membership.counts;
    let revision = judged.grants.revision();
    let asked = Asked {
        asker: asking.acting_for,
        question: &common.question,
        served: asking.served,
        revision,
    };
    let continued = match common.after {
        Some(handle) => Some(membership.cursors()?.take(handle, &asked, asking.at)?),
        None => None,
    };
    let candidates = candidates(&*judged)?;
    let after = continued.as_ref().map(|continued| continued.after.as_str());
    let filled = fill(
        judged,
        &candidates,
        (after, common.bounds, asking.at, common.at_least),
        counts,
    )?;
    let chain = continued
        .as_ref()
        .map(|continued: &Continued| continued.chain);
    let next = match (filled.more, &filled.last) {
        (true, Some(last)) => Some(membership.cursors()?.issue(
            &asked,
            (last, chain),
            asking.random,
            asking.at,
        )?),
        _ => {
            if let Some(chain) = chain {
                membership.cursors()?.complete(chain);
            }
            None
        }
    };
    let returned = u64::try_from(filled.rows.len()).unwrap_or(u64::MAX);
    counts.page(returned, filled.skipped);
    Ok(MembershipPage {
        contract: CONTRACT_VERSION,
        log: asking.served.clone(),
        revision: Some(revision),
        outcome: PageOutcome::Page {
            rows: filled.rows,
            returned,
            skipped: filled.skipped,
            complete: next.is_none(),
            next,
        },
    })
}

/// The resources a subject may act on, gathered from indexes: those it
/// holds a live grant on and those placed under them, of `kind`, in
/// `workspace`, every index entry read counted.
fn held_resources<S: LeafStore>(
    judged: &Judged<'_, S>,
    subject: IdentityId,
    (kind, workspace): (&str, &Resource),
    counts: &Counts,
) -> BTreeSet<Resource> {
    let held = judged.apps.held();
    let mut queue: Vec<Resource> = judged
        .grants
        .book()
        .live_held_by(subject)
        .map(|record| record.grant().resource().clone())
        .collect();
    counts.probe(u64::try_from(queue.len()).unwrap_or(u64::MAX));
    let (mut seen, mut found) = (BTreeSet::new(), BTreeSet::new());
    while let Some(resource) = queue.pop() {
        if !seen.insert(resource.clone()) {
            continue;
        }
        let before = queue.len();
        queue.extend(
            held.children(resource.kind(), resource.id())
                .filter_map(|placed| Resource::new(&placed.child_kind, &placed.child_id).ok()),
        );
        counts.probe(u64::try_from(queue.len() - before).unwrap_or(u64::MAX));
        if resource.kind() == kind {
            found.insert(resource);
        }
    }
    found.retain(|resource| within(judged, resource, workspace, counts));
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

fn question(parts: &impl serde::Serialize) -> Result<String, Named> {
    serde_json::to_string(parts).map_err(|error| {
        (
            membership::REQUEST_MALFORMED.to_owned(),
            format!("the question could not be written: {error}"),
        )
    })
}

fn resources_page<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    asking: &Asking<'_>,
    request: &ResourcePageRequest,
) -> Result<MembershipPage, Named> {
    membership_pages::validate_resources(request, asking.served)
        .map_err(|refused| (refused.name.to_owned(), refused.reason))?;
    ceiling(asking.membership, request.bounds)?;
    let common = Common {
        question: question(&(
            "resources",
            &request.workspace,
            &request.subject,
            &request.kind,
            &request.action,
        ))?,
        bounds: request.bounds,
        at_least: request.at_least,
        after: request.after.as_deref(),
    };
    let counts = &asking.membership.counts;
    answer(judged, asking, &common, |judged| {
        let subject = identity_id(&request.subject.id).map_err(named)?;
        if identity_kind(&subject) != request.subject.kind {
            return Err((
                membership::SUBJECT_KIND_MISMATCH.to_owned(),
                "the subject is not of the kind stated".to_owned(),
            ));
        }
        admit(
            judged,
            asking.acting_for,
            (&request.workspace.kind, &request.kind, &request.action),
        )?;
        let workspace =
            Resource::new(&request.workspace.kind, &request.workspace.id).map_err(named)?;
        let action = Action::new(&request.action).map_err(named)?;
        Ok(
            held_resources(judged, subject, (&request.kind, &workspace), counts)
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
                .collect(),
        )
    })
}

fn recipients_page<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    asking: &Asking<'_>,
    request: &RecipientPageRequest,
) -> Result<MembershipPage, Named> {
    membership_pages::validate_recipients(request, asking.served)
        .map_err(|refused| (refused.name.to_owned(), refused.reason))?;
    ceiling(asking.membership, request.bounds)?;
    let common = Common {
        question: question(&(
            "recipients",
            &request.workspace,
            &request.resource,
            &request.action,
        ))?,
        bounds: request.bounds,
        at_least: request.at_least,
        after: request.after.as_deref(),
    };
    let counts = &asking.membership.counts;
    answer(judged, asking, &common, |judged| {
        admit(
            judged,
            asking.acting_for,
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
        if !within(judged, &resource, &workspace, counts) {
            return Err((
                membership::OUTSIDE_WORKSPACE.to_owned(),
                "the resource is not placed under the named workspace".to_owned(),
            ));
        }
        let action = Action::new(&request.action).map_err(named)?;
        let book = judged.grants.book();
        let holders = judged
            .apps
            .reach(&resource)
            .iter()
            .flat_map(|reached| book.live_on_resource(reached))
            .inspect(|_| counts.probe(1))
            .map(|record| record.grant().holder())
            .collect::<BTreeSet<IdentityId>>();
        Ok(holders
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
            .collect())
    })
}

/// Serve one page: the asker, the log, the time and a next cursor's
/// randomness are fixed before the grants are held, and a refusal is the
/// whole page's, counted.
fn serve<T>(
    state: &AppState,
    headers: &HeaderMap,
    request: &T,
    page: impl FnOnce(&mut Judged<'_>, &Asking<'_>, &T) -> Result<MembershipPage, Named>,
) -> Result<Json<MembershipPage>, ServerError> {
    let acting_for = asker(state, headers)?;
    let served = served_log(state);
    let mut random = [0_u8; HANDLE_BYTES];
    rand::rng().fill_bytes(&mut random);
    let membership = &state.membership;
    membership.counts.call();
    let asking = Asking {
        acting_for: acting_for.as_deref(),
        served: &served,
        at: now(),
        random,
        membership,
    };
    with_grants(state, |mut judged| {
        Ok(Json(page(&mut judged, &asking, request).unwrap_or_else(
            |(name, reason)| {
                membership.counts.refused_page();
                membership_pages::refused_page(&served, &name, &reason)
            },
        )))
    })
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
    serve(&state, &headers, &request, resources_page)
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
    serve(&state, &headers, &request, recipients_page)
}
