//! Approval is the execution signal; the admitted router owns every act and refusal.
use crate::ServerError;
use crate::routes::{AppState, with_directory};
use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, header};
use lys_identity::draft_event::{DraftEvent, Executed, RefusedOnExecution};
use lys_identity::projection::draft::DraftRecord;
use lys_identity::{Actor, OperationId};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock, Weak};

/// The admitted router and the signal-driven exclusion of simultaneous approvals.
#[derive(Default)]
pub struct Executor {
    router: OnceLock<Weak<Router>>,
    pub(crate) gate: tokio::sync::Mutex<()>,
}
impl Executor {
    pub(crate) fn attach(&self, router: Weak<Router>) -> Result<(), ServerError> {
        self.router
            .set(router)
            .map_err(|_| fault("the draft router was attached twice"))
    }
}

tokio::task_local! {static APPROVER:Actor;}
pub(crate) fn actor() -> Option<Actor> {
    APPROVER.try_with(Clone::clone).ok()
}
fn fault(reason: &str) -> ServerError {
    ServerError::ConfigInvalid {
        reason: reason.to_owned(),
    }
}

/// The route's recorded answer to the prepared request.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Outcome {
    /// The admitted route carried out the act.
    Executed {
        /// Answered HTTP status.
        status: u16,
        /// Directory receipt for the act.
        receipt_index: u64,
    },
    /// The admitted route refused the act.
    RefusedOnExecution {
        /// The route's refusal name.
        refusal: String,
        /// The route's refusal words.
        reason: String,
    },
}
pub(crate) fn outcome(held: &DraftRecord) -> Option<Outcome> {
    held.executed
        .as_ref()
        .map(|(e, _)| Outcome::Executed {
            status: e.status,
            receipt_index: e.receipt_index,
        })
        .or_else(|| {
            held.refused_on_execution
                .as_ref()
                .map(|(e, _)| Outcome::RefusedOnExecution {
                    refusal: e.refusal.clone(),
                    reason: e.reason.clone(),
                })
        })
}

pub(crate) async fn apply(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    held: &DraftRecord,
) -> Result<Outcome, ServerError> {
    if let Some(outcome) = outcome(held) {
        return Ok(outcome);
    }
    let approved = &held
        .approved
        .as_ref()
        .ok_or_else(|| fault("the draft has no approval"))?
        .0;
    let actor = approved
        .actor
        .clone()
        .with_draft(lys_identity::provenance::DraftProvenance {
            draft: held.created.operation,
            application: approved.application,
            drafted_by: held.created.actor.provenance().agent(),
        });
    let bytes = match serde_json::from_slice::<serde_json::Value>(&held.created.body) {
        Ok(serde_json::Value::Object(mut object)) => {
            if object.contains_key("operation") {
                object.insert(
                    "operation".into(),
                    serde_json::Value::String(approved.application.to_string()),
                );
            }
            serde_json::to_vec(&object)
                .map_err(|_| fault("the prepared body cannot be rendered"))?
        }
        _ => held.created.body.clone(),
    };
    let mut request = Request::builder()
        .method(held.created.method.as_str())
        .uri(&held.created.path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(cookie) = headers.get(header::COOKIE) {
        request = request.header(header::COOKIE, cookie);
    }
    let request = request
        .body(Body::from(bytes.clone()))
        .map_err(|_| fault("the prepared request cannot be dispatched"))?;
    let router = state
        .draft_execution
        .router
        .get()
        .and_then(Weak::upgrade)
        .ok_or_else(|| fault("the admitted draft router is absent"))?;
    let router = router.as_ref().clone();
    let response = APPROVER
        .scope(
            actor.clone(),
            crate::mcp_endpoint::dispatch(router, request),
        )
        .await;
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|_| fault("the applied route's answer could not be read"))?;
    let outcome = if status.is_success() {
        let receipt_index = with_directory(state, |directory| {
            if let Some(index) = directory.projection()?.operation(approved.application) {
                return Ok(index);
            }
            let call = lys_identity::agent_call::AgentCall::new(
                &held.created.method,
                &held.created.path,
                &crate::mcp_receipts::digest(&bytes),
                "",
            )?;
            Ok(directory
                .record_draft_call(actor.clone(), call, crate::session::now())?
                .coordinate()
                .index)
        })?;
        Outcome::Executed {
            status: status.as_u16(),
            receipt_index,
        }
    } else {
        let answer: serde_json::Value = serde_json::from_slice(&body)
            .map_err(|_| fault("the applied route did not answer a named JSON refusal"))?;
        let refusal = answer["refusal"]
            .as_str()
            .ok_or_else(|| fault("the applied route's refusal has no name"))?;
        let reason = answer["reason"]
            .as_str()
            .ok_or_else(|| fault("the applied route's refusal has no words"))?;
        Outcome::RefusedOnExecution {
            refusal: refusal.to_owned(),
            reason: reason.to_owned(),
        }
    };
    with_directory(state, |directory| {
        let operation = OperationId::generate()?;
        let recorded_at = crate::session::now();
        let draft = held.created.operation;
        let draft_hash = held.hash;
        let application = approved.application;
        let event = match &outcome {
            Outcome::Executed {
                status,
                receipt_index,
            } => DraftEvent::Executed(Arc::new(Executed {
                operation,
                actor: actor.clone(),
                recorded_at,
                draft,
                draft_hash,
                application,
                status: *status,
                receipt_index: *receipt_index,
            })),
            Outcome::RefusedOnExecution { refusal, reason } => {
                DraftEvent::RefusedOnExecution(Arc::new(RefusedOnExecution {
                    operation,
                    actor,
                    recorded_at,
                    draft,
                    draft_hash,
                    application,
                    refusal: refusal.clone(),
                    reason: reason.clone(),
                }))
            }
        };
        directory.record_draft(event)?;
        Ok(())
    })?;
    Ok(outcome)
}
