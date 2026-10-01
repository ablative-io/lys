//! Administrator reporting changes are admitted before their atomic directory operation.

use std::str::FromStr;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::{AgentId, OperationId};
use serde::Deserialize;

use crate::directory_views::{AgentRegistered, receipt_view};
use crate::error::ServerError;
use crate::reporting_views::{ReportsToChanged, ResponsibilityChanged, edge};
use crate::routes::{Shared, identity_id, signed_in, with_directory};

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = ReportsToBody)]
pub(crate) struct ReportsToBody {
    operation: String,
    reports_to: String,
}

pub(crate) fn registered(
    answer: &lys_identity::directory::reporting::Registered,
) -> Result<AgentRegistered, ServerError> {
    Ok(AgentRegistered {
        agent: answer.agent.to_string(),
        responsible: answer.responsible.to_string(),
        reports_to: edge(answer.reports_to)?,
        receipt: receipt_view(&answer.receipt),
    })
}

pub(crate) async fn change(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ReportsToBody>,
) -> Result<Json<ReportsToChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let agent = AgentId::from_str(&id)?;
    let operation = OperationId::from_str(&body.operation)?;
    let target = identity_id(&body.reports_to)?;
    with_directory(&state, |directory| {
        let answer =
            directory.change_reports_to(actor, operation, agent, target, crate::session::now())?;
        let receipt = receipt_view(&answer.receipt);
        let responsibility =
            (answer.responsible_from != answer.responsible_to).then(|| ResponsibilityChanged {
                receipt: receipt.clone(),
                from: answer.responsible_from.to_string(),
                to: answer.responsible_to.to_string(),
            });
        Ok(Json(ReportsToChanged {
            reports_to: edge(answer.reports_to)?,
            responsible: answer.responsible_to.to_string(),
            receipt,
            responsibility,
        }))
    })
}
