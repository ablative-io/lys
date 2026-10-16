//! A prepared request must name the exact resource the reviewer is shown.
use super::DraftBody;
use crate::ServerError;
use lys_identity::draft_event::{Created, Target};
use lys_identity::{Actor, IdentityId, OperationId};
use std::str::FromStr;

pub(super) fn creation(
    body: DraftBody,
    actor: Actor,
    reviewer: IdentityId,
    at: u64,
    corrects: Option<OperationId>,
) -> Result<Created, ServerError> {
    let (resource, action) =
        crate::openapi_table::token_scope(&body.method, &body.path).map_err(|error| {
            ServerError::SecretsRefused {
                status: axum::http::StatusCode::CONFLICT,
                refusal: "DraftChangeInvalid".to_owned(),
                reason: format!(
                    "target {}:{} {} names no declared mutation scope for {} {}: {error}",
                    body.target.kind, body.target.id, body.target.action, body.method, body.path
                ),
            }
        })?;
    if body.method == "GET"
        || body.target.kind != resource.kind()
        || body.target.id != resource.id()
        || body.target.action != action.as_str()
    {
        return Err(ServerError::SecretsRefused {
            status: axum::http::StatusCode::CONFLICT,
            refusal: "DraftChangeInvalid".to_owned(),
            reason: format!(
                "target {}:{} {} differs from route scope {}:{} {}",
                body.target.kind,
                body.target.id,
                body.target.action,
                resource.kind(),
                resource.id(),
                action.as_str()
            ),
        });
    }
    Ok(Created {
        operation: OperationId::from_str(&body.operation)?,
        actor,
        recorded_at: at,
        target: Target {
            kind: body.target.kind,
            id: body.target.id,
            action: body.target.action,
        },
        method: body.method,
        path: body.path,
        body: body.body.into_bytes(),
        note: body.note,
        reviewer,
        corrects,
        evidence: None,
        request_signature: None,
    })
}
