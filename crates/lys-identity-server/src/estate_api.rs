//! The installed plan is read before approval; applying uses the real loader credential.
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use lys_identity::IdentityId;
use serde::Serialize;
use serde_json::Value;

use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};

#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct EstatePlanAnswer {
    #[schema(value_type = Object)]
    plan: Value,
    loader: String,
}

fn loader(state: &AppState, headers: &HeaderMap) -> Result<(HeaderMap, String), ServerError> {
    let actor = signed_in(state, headers)?;
    crate::routes::administrator(state, &actor)?;
    let owner = with_directory(state, |directory| {
        Ok(crate::read_api::own_person(directory.projection()?, &actor)?.to_string())
    })?;
    let path = state
        .import_credential_file
        .as_ref()
        .ok_or_else(|| ServerError::ConfigInvalid {
            reason: "no installed loader credential".to_owned(),
        })?;
    let credential = crate::import_bootstrap::read(path)?;
    let mut authenticated = HeaderMap::new();
    let mut bearer =
        HeaderValue::from_str(&format!("Bearer {}", credential.trim())).map_err(|_error| {
            ServerError::ConfigInvalid {
                reason: "invalid installed loader credential".to_owned(),
            }
        })?;
    bearer.set_sensitive(true);
    authenticated.insert(header::AUTHORIZATION, bearer);
    let account = crate::grants::with_grants(state, |judged| {
        let caller = crate::service_account_grants::caller(state, &authenticated, &judged)?;
        let IdentityId::ServiceAccount(account) = caller else {
            return Err(ServerError::ServiceAccountUnknown);
        };
        let responsible = judged
            .directory
            .record(caller)
            .and_then(lys_identity::projection::Record::responsible);
        if responsible.map(|id| id.to_string()).as_deref() != Some(owner.as_str()) {
            return Err(ServerError::NotAdmitted {
                reason: "the installed loader must be owned by the approving person",
            });
        }
        Ok(account.to_string())
    })?;
    Ok((authenticated, account))
}

pub(crate) async fn plan(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<EstatePlanAnswer>, ServerError> {
    let (_, account) = loader(&state, &headers)?;
    let bytes =
        std::fs::read(&state.estate_plan_file).map_err(|_error| ServerError::ConfigInvalid {
            reason: "the installed estate plan is absent or unreadable; upgrade the installation"
                .to_owned(),
        })?;
    let plan: Value =
        serde_json::from_slice(&bytes).map_err(|_error| ServerError::ConfigInvalid {
            reason: "the installed estate plan is not JSON".to_owned(),
        })?;
    Ok(Json(EstatePlanAnswer {
        plan,
        loader: account,
    }))
}

pub(crate) async fn apply(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<crate::import_api::ImportDocument>, axum::extract::rejection::JsonRejection>,
) -> Response {
    match loader(&state, &headers) {
        Ok((authenticated, _)) => {
            crate::import_api::import(State(state), authenticated, body).await
        }
        Err(error) => error.into_response(),
    }
}
