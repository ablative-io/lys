//! The product drafts a signed-in person may see: the administrator sees
//! every one; anyone else those they hold, those they answer for as the
//! grant's responsible person, and those they may decide because they may
//! grant the draft's action.

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

fn view(record: &ProductDraftRecord) -> ProductDraftView {
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
    crate::grants::with_grants(state, |mut judged| {
        let person = crate::read_api::own_person(judged.directory, &actor)?;
        let administrator = state.admission.is_administrator(judged.directory, &actor)?;
        let records: Vec<ProductDraftRecord> = judged.directory.product_drafts().cloned().collect();
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
    })
}

/// One `page` of the product drafts the signed-in caller may see, with
/// how many they may see in all and the cursor of the page after it.
pub(super) fn person_list(
    state: &AppState,
    headers: &HeaderMap,
    page: &Page,
) -> Result<ProductDraftList, ServerError> {
    let seen = seen(state, headers)?;
    let rows = seen
        .iter()
        .map(|record| (record.created.operation.to_string(), record));
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
