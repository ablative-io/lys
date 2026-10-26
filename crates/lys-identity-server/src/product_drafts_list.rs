//! The product drafts a signed-in person may see: the administrator sees
//! every one; anyone else those they hold, those they answer for as the
//! grant's responsible person, and those they may decide because they may
//! grant the draft's action.
//!
//! Own-person readings filter by holder and recorded responsibility before
//! paging so administrator authority cannot widen a connection's consent.

use lys_identity::IdentityId;
use lys_identity::projection::product_draft::{Closed, ProductDraftRecord};

use super::wire::{ApprovalView, ExecutionView, ProductDraftList, ProductDraftView, ProductTarget};
use crate::error::ServerError;
use crate::list_page::Page;
use crate::routes::{AppState, hex};
use axum::http::HeaderMap;

fn state_of(record: &ProductDraftRecord) -> &'static str {
    match &record.closed {
        Some(Closed::Executed(..)) => "executed",
        Some(Closed::RefusedOnExecution(..)) => "refused_on_execution",
        None if record.refused.is_some() => "refused",
        None if record.is_approved() => "approved",
        None => "waiting",
    }
}

pub(super) fn view(record: &ProductDraftRecord) -> ProductDraftView {
    let created = &record.created;
    let request_digest = hex(&created.request_digest);
    let execution = record.closed.as_ref().map(|closed| match closed {
        Closed::Executed(event, _) => ExecutionView::Executed {
            request_digest: request_digest.clone(),
            receipt_digest: hex(&event.receipt_digest),
        },
        Closed::RefusedOnExecution(event, _) => ExecutionView::RefusedOnExecution {
            request_digest: request_digest.clone(),
            refusal: event.refusal.clone(),
            reason: event.reason.clone(),
        },
    });
    ProductDraftView {
        id: created.operation.to_string(),
        app: created.app.clone(),
        grant: created.grant.to_string(),
        target: ProductTarget {
            kind: created.target.kind.clone(),
            id: created.target.id.clone(),
            action: created.target.action.clone(),
        },
        request_digest,
        words: created.words.clone(),
        state: state_of(record).to_owned(),
        mode: created.mode.as_str().to_owned(),
        holder: created.holder.to_string(),
        responsible: created.responsible.to_string(),
        approvals: record
            .approvals
            .iter()
            .map(|(approval, _)| ApprovalView {
                by: approval.approver.to_string(),
                at: approval.recorded_at,
            })
            .collect(),
        execution,
    }
}

/// Every product draft the signed-in caller may see, in draft id order.
pub(super) fn seen(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Vec<ProductDraftRecord>, ServerError> {
    let actor = crate::routes::signed_in(state, headers)?;
    crate::grants_reads::with_read_grants(
        state,
        None,
        crate::session::now(),
        |judged| exercises(judged, &actor),
        |mut judged| {
            let person = crate::read_api::own_person(judged.directory, &actor)?;
            let administrator = state.admission.is_administrator(judged.directory, &actor)?;
            let records: Vec<ProductDraftRecord> =
                judged.directory.product_drafts().cloned().collect();
            let mut seen = Vec::new();
            for record in records {
                if administrator
                    || record.created.holder == IdentityId::Person(person)
                    || record.created.responsible == person
                    || super::decide::may_grant(&mut judged, person, &record)
                {
                    seen.push(record);
                }
            }
            Ok(seen)
        },
    )
}

/// The product drafts waiting on an approval the signed-in person may give
/// now, each with its holder's display name: not yet approved, refused or
/// closed; neither held by the person nor answered for by them; not approved
/// by them already; and on an action they may grant. These are exactly the
/// drafts whose approval `POST /product-drafts/{id}/approve` would take from them.
pub(super) fn deciding(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Vec<(ProductDraftRecord, Option<String>)>, ServerError> {
    let actor = crate::routes::signed_in(state, headers)?;
    crate::grants_reads::with_read_grants(
        state,
        None,
        crate::session::now(),
        |judged| exercises(judged, &actor),
        |mut judged| {
            let person = crate::read_api::own_person(judged.directory, &actor)?;
            let open: Vec<ProductDraftRecord> = judged
                .directory
                .product_drafts()
                .filter(|record| {
                    record.closed.is_none()
                        && record.refused.is_none()
                        && !record.is_approved()
                        && record.created.holder != IdentityId::Person(person)
                        && record.created.responsible != person
                        && !record
                            .approvals
                            .iter()
                            .any(|(approval, _)| approval.approver == person)
                })
                .cloned()
                .collect();
            let mut waiting = Vec::new();
            for record in open {
                if super::decide::may_grant(&mut judged, person, &record) {
                    let name = judged
                        .directory
                        .record(record.created.holder)
                        .map(|held| held.profile().display_name().to_owned());
                    waiting.push((record, name));
                }
            }
            Ok(waiting)
        },
    )
}

/// The exercise of each product draft's action the grants are asked about
/// for the signed-in person before the drafts are read.
fn exercises(
    judged: &mut crate::grants::Judged<'_>,
    actor: &lys_identity::Actor,
) -> Result<Vec<lys_identity::grants::ExerciseRequest>, ServerError> {
    let person = crate::read_api::own_person(judged.directory, actor)?;
    judged
        .directory
        .product_drafts()
        .map(|held| {
            let target = &held.created.target;
            Ok(lys_identity::grants::ExerciseRequest {
                caller: IdentityId::Person(person),
                route: lys_identity::grants::Route::Api,
                resource: lys_identity::grants::Resource::new(&target.kind, &target.id)?,
                action: lys_identity::grants::Action::new(&target.action)?,
            })
        })
        .collect()
}

/// One `page` of the product drafts the signed-in caller may see, with
/// how many they may see in all and the cursor of the page after it.
pub(super) fn person_list(
    state: &AppState,
    headers: &HeaderMap,
    page: &Page,
) -> Result<ProductDraftList, ServerError> {
    if crate::mcp_endpoint::reading_actor().is_some() {
        return own_person_list(state, headers, page);
    }
    let seen = seen(state, headers)?;
    page_records(seen.iter(), page)
}

/// The signed-in person's held and responsible product drafts, filtered before
/// paging. Administrator authority never widens this own-record reading.
pub(crate) fn own_person_list(
    state: &AppState,
    headers: &HeaderMap,
    page: &Page,
) -> Result<ProductDraftList, ServerError> {
    let actor = crate::routes::signed_in(state, headers)?;
    crate::routes::with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = crate::read_api::own_person(projection, &actor)?;
        let own = projection.product_drafts().filter(|record| {
            record.created.holder == IdentityId::Person(person)
                || record.created.responsible == person
        });
        page_records(own, page)
    })
}

fn page_records<'a>(
    records: impl Iterator<Item = &'a ProductDraftRecord>,
    page: &Page,
) -> Result<ProductDraftList, ServerError> {
    let rows = records.map(|record| (record.created.operation.to_string(), record));
    let (drafts, totals) = page.select(
        rows,
        None,
        |_| Ok(true),
        |(id, _)| id,
        |(_, record)| Ok(view(record)),
    )?;
    Ok(ProductDraftList {
        drafts,
        next: totals.next,
        total: totals.total,
    })
}
