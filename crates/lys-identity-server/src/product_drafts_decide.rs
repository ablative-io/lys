//! Deciding and closing a product draft. A signed-in person who may grant
//! the draft's action approves or refuses it: the administrator, whose
//! authority every grant traces to, or a person holding a live grant that
//! reaches the target and lets them pass that action on (ADR-135's walk up
//! the chain). The holder and the grant's responsible person never decide
//! their own draft. By draft closes on one approval, by two on two distinct
//! approvers. The app's connector then closes an approved draft once.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::grants::{Action, GrantId, Resource};
use lys_identity::log::Coordinate;
use lys_identity::product_draft_event::{
    Approved, Executed, ProductDraftEvent, Refused, RefusedOnExecution, close_operation,
};
use lys_identity::projection::product_draft::ProductDraftRecord;
use lys_identity::{Actor, IdentityError, IdentityId, OperationId, PersonId};

use super::wire::{
    ExecutedBody, ProductApproveBody, ProductClosed, ProductDecision, ProductRefuseBody,
    RefusedOnExecutionBody,
};
use super::{digest32, malformed, reaches, with_connector};
use crate::error::ServerError;
use crate::error_product_draft::ProductDraftError;
use crate::grants::{Judged, with_directory_grants};
use crate::routes::{AppState, hex};
use crate::session::now;

fn refused(reason: &'static str) -> ServerError {
    ProductDraftError::ApproverRefused { reason }.into()
}

/// Whether `person` may grant `held`'s action on its target.
pub(super) fn may_grant(
    judged: &mut Judged<'_>,
    person: PersonId,
    held: &ProductDraftRecord,
) -> bool {
    if person == judged.root {
        return true;
    }
    let target = &held.created.target;
    let (Ok(resource), Ok(action)) = (
        Resource::new(&target.kind, &target.id),
        Action::new(&target.action),
    ) else {
        return false;
    };
    let candidates: Vec<GrantId> = judged
        .grants
        .book()
        .held_by(IdentityId::Person(person))
        .filter(|record| {
            record
                .grant()
                .pass_on()
                .actions()
                .is_some_and(|actions| actions.contains(&action))
        })
        .map(|record| record.grant().id())
        .collect();
    let at = now();
    candidates.into_iter().any(|grant| {
        reaches(
            judged,
            IdentityId::Person(person),
            grant,
            (&resource, &action),
            at,
        )
    })
}

/// The decision `make` builds, judged for the signed-in person: refused
/// `DraftHashMismatch` unless `request_digest` names the words they decide,
/// and refused unless they may decide the draft, unless the operation is already
/// recorded, in which case the record answers the retry as first made.
fn judged_decision(
    judged: &mut Judged<'_>,
    actor: &Actor,
    (draft, operation): (OperationId, OperationId),
    request_digest: [u8; 32],
    make: impl FnOnce(PersonId, [u8; 32]) -> ProductDraftEvent,
) -> Result<ProductDraftEvent, ServerError> {
    let person = crate::read_api::own_person(judged.directory, actor)?;
    let held = judged
        .directory
        .product_draft(draft)
        .cloned()
        .ok_or_else(|| IdentityError::DraftNotFound {
            draft: draft.to_string(),
        })?;
    if held.created.request_digest != request_digest {
        return Err(IdentityError::DraftHashMismatch.into());
    }
    if judged.directory.operation(operation).is_none() {
        if held.created.holder == IdentityId::Person(person) || held.created.responsible == person {
            return Err(refused(
                "the draft's holder and the grant's responsible person do not decide their own draft",
            ));
        }
        if held
            .approvals
            .iter()
            .any(|(approval, _)| approval.approver == person)
        {
            return Err(refused(
                "this person has approved the draft already; by two needs a second, distinct approver",
            ));
        }
        if !may_grant(judged, person, &held) {
            return Err(refused(
                "only a person who may grant the draft's action decides it",
            ));
        }
    }
    Ok(make(person, held.hash))
}

fn state_of(record: &ProductDraftRecord) -> &'static str {
    if record.refused.is_some() {
        "refused"
    } else if record.is_approved() {
        "approved"
    } else {
        "waiting"
    }
}

fn decision_answer(
    directory: &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
    event: ProductDraftEvent,
) -> Result<Json<ProductDecision>, ServerError> {
    let operation = event.operation();
    let Some((draft, _)) = event.decision() else {
        return Err(malformed("a decision names its draft"));
    };
    let coordinate = directory.record_product_draft(event)?;
    let record = directory
        .projection()?
        .product_draft(draft)
        .ok_or_else(|| IdentityError::DraftNotFound {
            draft: draft.to_string(),
        })?;
    Ok(Json(ProductDecision {
        draft: draft.to_string(),
        operation: operation.to_string(),
        state: state_of(record).to_owned(),
        approvals: record.approvals.len(),
        needed: record.needed(),
        index: coordinate.index,
        tree_size: coordinate.tree_size,
        leaf_hash: hex(&coordinate.leaf_hash),
    }))
}

/// One person's approval: by draft is approved by it, by two on the second
/// distinct approver.
pub(super) async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    bytes: Bytes,
) -> Result<Json<ProductDecision>, ServerError> {
    let actor = crate::drafts_api::personal(&state, &headers)?;
    let body: ProductApproveBody = crate::signed_json::read(&state, &headers, &bytes)?;
    let digest = digest32(&body.request_digest, "request_digest")?;
    let draft = OperationId::from_str(&id)?;
    let operation = OperationId::from_str(&body.operation)?;
    with_directory_grants(
        &state,
        |mut judged| {
            judged_decision(
                &mut judged,
                &actor,
                (draft, operation),
                digest,
                |approver, hash| {
                    ProductDraftEvent::Approved(Arc::new(Approved {
                        operation,
                        actor: actor.clone(),
                        approver,
                        recorded_at: now(),
                        draft,
                        draft_hash: hash,
                    }))
                },
            )
        },
        decision_answer,
    )
}

/// One person's refusal, with the reason; the draft is closed unapproved.
pub(super) async fn refuse(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    bytes: Bytes,
) -> Result<Json<ProductDecision>, ServerError> {
    let actor = crate::drafts_api::personal(&state, &headers)?;
    let body: ProductRefuseBody = crate::signed_json::read(&state, &headers, &bytes)?;
    if body.reason.trim().is_empty() {
        return Err(malformed("a refusal says why"));
    }
    let digest = digest32(&body.request_digest, "request_digest")?;
    let draft = OperationId::from_str(&id)?;
    let operation = OperationId::from_str(&body.operation)?;
    with_directory_grants(
        &state,
        |mut judged| {
            judged_decision(
                &mut judged,
                &actor,
                (draft, operation),
                digest,
                |approver, hash| {
                    ProductDraftEvent::Refused(Arc::new(Refused {
                        operation,
                        actor: actor.clone(),
                        approver,
                        recorded_at: now(),
                        draft,
                        draft_hash: hash,
                        reason: body.reason,
                    }))
                },
            )
        },
        decision_answer,
    )
}

/// Record the draft's one close for the connector's own app. The same close
/// asked again is answered as first recorded; another close is refused
/// `product_draft_closed`, and a close of a draft not approved
/// `product_draft_not_approved`.
fn close(
    state: &AppState,
    headers: &HeaderMap,
    id: &str,
    make: impl FnOnce(&str, OperationId, [u8; 32]) -> (ProductDraftEvent, &'static str),
) -> Result<Json<ProductClosed>, ServerError> {
    let draft = OperationId::from_str(id)?;
    with_connector(state, headers, |app, directory| {
        let held = directory
            .projection()?
            .product_draft(draft)
            .ok_or_else(|| IdentityError::DraftNotFound {
                draft: draft.to_string(),
            })?;
        if held.created.app != app {
            return Err(ProductDraftError::NotYourApp {
                caller: app.to_owned(),
                app: held.created.app.clone(),
            }
            .into());
        }
        let (event, state) = make(app, draft, held.hash);
        let coordinate: Coordinate = match directory.record_product_draft(event) {
            Ok(coordinate) => coordinate,
            Err(IdentityError::OperationReused { .. }) => {
                return Err(ProductDraftError::Closed {
                    draft: draft.to_string(),
                }
                .into());
            }
            Err(IdentityError::DraftNotPending { .. }) => {
                return Err(ProductDraftError::NotApproved {
                    draft: draft.to_string(),
                }
                .into());
            }
            Err(error) => return Err(error.into()),
        };
        Ok(Json(ProductClosed {
            draft: draft.to_string(),
            state: state.to_owned(),
            index: coordinate.index,
            tree_size: coordinate.tree_size,
            leaf_hash: hex(&coordinate.leaf_hash),
        }))
    })
}

/// The product executed the approved draft and committed this receipt.
pub(super) async fn executed(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ExecutedBody>, JsonRejection>,
) -> Result<Json<ProductClosed>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let receipt_digest = digest32(&body.receipt_digest, "receipt_digest")?;
    close(&state, &headers, &id, |app, draft, hash| {
        (
            ProductDraftEvent::Executed(Arc::new(Executed {
                operation: close_operation(draft),
                recorded_at: now(),
                draft,
                draft_hash: hash,
                app: app.to_owned(),
                receipt_digest,
            })),
            "executed",
        )
    })
}

/// The product refused the approved draft when it came to execute it.
pub(super) async fn refused_on_execution(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<RefusedOnExecutionBody>, JsonRejection>,
) -> Result<Json<ProductClosed>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    if body.refusal.trim().is_empty() || body.reason.trim().is_empty() {
        return Err(malformed("a refusal on execution has a name and a reason"));
    }
    close(&state, &headers, &id, |app, draft, hash| {
        (
            ProductDraftEvent::RefusedOnExecution(Arc::new(RefusedOnExecution {
                operation: close_operation(draft),
                recorded_at: now(),
                draft,
                draft_hash: hash,
                app: app.to_owned(),
                refusal: body.refusal,
                reason: body.reason,
            })),
            "refused_on_execution",
        )
    })
}
