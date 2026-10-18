//! Make a service account a registrar: its credential is answered once, and
//! the same act sent again names it and answers no credential. Split from
//! `apps_api.rs` so each file stays under 500 lines (ADR-111).

use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::OperationId;
use serde::Deserialize;

use super::{administrator, malformed, service_account_held, with_apps};
use crate::apps_binding::{Registrar, new_secret};
use crate::apps_error::AppError;
use crate::apps_state::Line;
use crate::apps_views::RegistrarIssued;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::session::now;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegistrarBody {
    operation: String,
    service_account: String,
}

/// Make a service account a registrar, answering its credential once.
pub(super) async fn registrar(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<RegistrarBody>, JsonRejection>,
) -> Result<Json<RegistrarIssued>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    with_apps(&state, |apps, projection| {
        let by = administrator(&state, apps, &headers, projection)?;
        if let Some(Line::Registrar(made)) = apps.held().operation(&operation) {
            if made.service_account != body.service_account {
                return Err(AppError::AppOperationReused { operation }.into());
            }
            return Ok(RegistrarIssued {
                service_account: made.service_account,
                credential: None,
            });
        }
        service_account_held(&state, &body.service_account)?;
        let (secret, digest) = new_secret()?;
        apps.keep(Line::Registrar(Registrar {
            operation,
            service_account: body.service_account.clone(),
            secret_sha256: digest,
            by,
            at: now(),
        }))?;
        Ok(RegistrarIssued {
            credential: Some(format!(
                "{}.{}.{secret}",
                crate::apps_binding::REGISTRAR_CREDENTIAL,
                body.service_account
            )),
            service_account: body.service_account,
        })
    })
    .map(Json)
}
