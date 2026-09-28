//! The two decisions on an access request.
//!
//! An approval is two writes, the grant and the decision, in two logs. The
//! approval is kept as an intent before the grant is issued, so whatever
//! stops the service between the two writes, what was begun is known.
//!
//! Every decision starts by reconciling an intent that stands, under the
//! grants' lock. When the grants hold the grant its operation issued, the
//! decision the intent was made for is kept, by its approver and in their
//! words. When the grants hold nothing for the operation, the intent is
//! withdrawn. While the grants cannot say, the request is held and nothing
//! is decided. So a grant is never active beside a request that waits or
//! is declined.

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::grants::{DelegateRequest, GrantId, PassOn, RootRequest, Source, Window};
use lys_identity::{IdentityId, OperationId, PersonId};
use serde::Deserialize;

use crate::error::ServerError;
use crate::grant_contract::{RouteWire, grant_id};
use crate::grant_sight::as_seen_by;
use crate::grants::{Judged, caller};
use crate::requests_api::{
    Weighed, decider, malformed, seen, settled, taken, with_requests, words,
};
use crate::requests_store::{Asked, Decided, Intended, RequestStore};
use crate::requests_views::RequestView;
use crate::routes::AppState;
use crate::session::now;

/// An approval. `source` is the grant the access is lent from, or null for
/// the root authority to issue it to a person.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ApproveBody {
    operation: String,
    route: RouteWire,
    source: Option<String>,
    note: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclineBody {
    note: String,
}

/// The grant the grants hold for the intent's operation, when its event was
/// committed and applied and it is the grant this approval would issue. An
/// operation that names any other event issued nothing for this request.
fn issued(
    judged: &Judged<'_>,
    asked: &Asked,
    weighed: &Weighed,
    intent: &Intended,
) -> Result<Option<GrantId>, ServerError> {
    let operation = OperationId::from_str(&intent.operation)?;
    let issuer = IdentityId::Person(PersonId::from_str(&intent.by)?);
    let source = match &intent.source {
        Some(source) => Source::Grant(grant_id(source)?),
        None => Source::Root,
    };
    let window = Window::new(asked.asked_at, asked.ends_at)?;
    let book = judged.grants.book();
    let Some(index) = book.operation(operation) else {
        return Ok(None);
    };
    Ok(book
        .records()
        .find(|record| record.index() == index)
        .map(|record| record.grant().parts())
        .filter(|parts| {
            parts.issuer == issuer
                && parts.holder == weighed.seeker
                && parts.source == source
                && parts.resource == weighed.resource
                && parts.relation == weighed.relation
                && parts.window == window
        })
        .map(|parts| parts.id))
}

fn held(intent: &Intended) -> ServerError {
    ServerError::RequestHeld {
        request: intent.id.clone(),
        by: intent.by.clone(),
    }
}

/// Settle the intent that stands on the request, if one does, and answer
/// the decision the request has once that is done.
fn reconcile(
    judged: &Judged<'_>,
    store: &mut RequestStore,
    asked: &Asked,
    weighed: &Weighed,
    at: u64,
) -> Result<Option<Decided>, ServerError> {
    if let Some(intent) = store.intent(&asked.id).cloned() {
        if judged.grants.ledger().uncertain().is_some() {
            return Err(held(&intent));
        }
        match issued(judged, asked, weighed, &intent)? {
            Some(grant) => store.decide(Decided {
                id: intent.id,
                by: intent.by,
                approved: true,
                note: intent.note,
                grant: Some(grant.to_string()),
                decided_at: at,
            })?,
            None => store.withdraw(&asked.id, &intent.operation)?,
        }
    }
    Ok(store
        .request(&asked.id)
        .and_then(|(_, decided)| decided.cloned()))
}

pub(crate) async fn approve(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<ApproveBody>, JsonRejection>,
) -> Result<Json<RequestView>, ServerError> {
    crate::spicedb_cancel::judged(Arc::clone(&state), move |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let body = taken(body)?;
        let at = now();
        with_requests(&state, |store| {
            let (asked, _, weighed) = seen(&judged, store, caller, &id, at)?;
            let by = decider(&judged, caller, &weighed)?;
            let decided = reconcile(&judged, store, &asked, &weighed, at)?;
            if let Some(decided) = &decided {
                settled(decided, by, true)?;
                return Ok(Json(weighed.view(
                    &judged,
                    caller,
                    &asked,
                    Some(decided),
                    None,
                )?));
            }
            let note = words("note", &body.note)?;
            let operation = OperationId::from_str(&body.operation)?;
            let source = body.source.as_deref().map(grant_id).transpose()?;
            let window = Window::new(asked.asked_at, asked.ends_at)?;
            if source.is_none() && matches!(weighed.seeker, IdentityId::Agent(_)) {
                return Err(malformed(
                    "an agent's access is lent from a grant a person holds: name the source",
                ));
            }
            store.intend(Intended {
                id: asked.id.clone(),
                by: by.to_string(),
                operation: operation.to_string(),
                source: source.map(|grant| grant.to_string()),
                note: note.clone(),
                intended_at: at,
            })?;
            let recorded = match (source, weighed.seeker) {
                (Some(source), _) => judged.grants.delegate(
                    judged.directory,
                    &DelegateRequest {
                        operation,
                        caller,
                        route: body.route.into(),
                        source,
                        recipient: weighed.seeker,
                        responsible: weighed.responsible,
                        resource: weighed.resource.clone(),
                        relation: weighed.relation.clone(),
                        pass_on: PassOn::UseOnly,
                        window,
                    },
                    at,
                ),
                (None, IdentityId::Person(holder)) => judged.grants.issue_root(
                    judged.directory,
                    &RootRequest {
                        operation,
                        caller,
                        route: body.route.into(),
                        holder,
                        resource: weighed.resource.clone(),
                        relation: weighed.relation.clone(),
                        pass_on: PassOn::UseOnly,
                        window,
                    },
                    at,
                ),
                (None, IdentityId::Agent(_)) => {
                    return Err(malformed(
                        "an agent's access is lent from a grant a person holds: name the source",
                    ));
                }
            };
            let recorded = match recorded {
                Ok(recorded) => recorded,
                Err(refusal) => {
                    reconcile(&judged, store, &asked, &weighed, at)?;
                    return Err(as_seen_by(&judged, caller, refusal));
                }
            };
            let decided = Decided {
                id: asked.id.clone(),
                by: by.to_string(),
                approved: true,
                note,
                grant: Some(recorded.receipt.grant.to_string()),
                decided_at: at,
            };
            store.decide(decided.clone())?;
            Ok(Json(weighed.view(
                &judged,
                caller,
                &asked,
                Some(&decided),
                None,
            )?))
        })
    })
    .await
}

/// Settle the approval being settled on a request, for anyone the request
/// is shown to. It settles an intent that stands and does nothing else: it
/// issues no grant and makes no decision of its own.
pub(crate) async fn settle(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RequestView>, ServerError> {
    crate::spicedb_cancel::judged(Arc::clone(&state), move |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let at = now();
        with_requests(&state, |store| {
            let (asked, _, weighed) = seen(&judged, store, caller, &id, at)?;
            let decided = reconcile(&judged, store, &asked, &weighed, at)?;
            Ok(Json(weighed.view(
                &judged,
                caller,
                &asked,
                decided.as_ref(),
                None,
            )?))
        })
    })
    .await
}

pub(crate) async fn decline(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<DeclineBody>, JsonRejection>,
) -> Result<Json<RequestView>, ServerError> {
    crate::spicedb_cancel::judged(Arc::clone(&state), move |judged| {
        let caller = caller(&state, &headers, judged.directory)?;
        let body = taken(body)?;
        let at = now();
        with_requests(&state, |store| {
            let (asked, _, weighed) = seen(&judged, store, caller, &id, at)?;
            let by = decider(&judged, caller, &weighed)?;
            let decided = reconcile(&judged, store, &asked, &weighed, at)?;
            if let Some(decided) = &decided {
                settled(decided, by, false)?;
                return Ok(Json(weighed.view(
                    &judged,
                    caller,
                    &asked,
                    Some(decided),
                    None,
                )?));
            }
            let decided = Decided {
                id: asked.id.clone(),
                by: by.to_string(),
                approved: false,
                note: words("note", &body.note)?,
                grant: None,
                decided_at: at,
            };
            store.decide(decided.clone())?;
            Ok(Json(weighed.view(
                &judged,
                caller,
                &asked,
                Some(&decided),
                None,
            )?))
        })
    })
    .await
}
