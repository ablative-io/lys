//! Startup metadata remains read-only; the administrator versions the organisation zone.

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::routes::{AppState, signed_in};

#[derive(Clone)]
struct StartupSettings {
    session_seconds: u64,
    secure_cookie: bool,
}

/// Configuration as loaded at startup, read only by the administrator.
pub fn routes(config: &Config) -> Router<Arc<AppState>> {
    Router::new()
        .route("/configuration", get(configuration).put(set_zone))
        .layer(Extension(StartupSettings {
            session_seconds: config.session_seconds,
            secure_cookie: config.secure_cookie,
        }))
}

async fn configuration(
    State(state): State<Arc<AppState>>,
    Extension(startup): Extension<StartupSettings>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let issuer = crate::connections_api::origin(state.oidc.issuer(), "sign-in provider")?;
    let organisation = organisation(&state)?;
    // Each model a kept call named that the table has no row for; with no
    // budgets store there are no kept calls to name one.
    let undeclared: Vec<String> = if state.budgets.is_some() {
        crate::budgets_api::with_budgets(&state, |store| {
            Ok(store
                .held()
                .index
                .models()
                .iter()
                .filter(|model| !organisation.model_windows.contains_key(*model))
                .cloned()
                .collect())
        })?
    } else {
        Vec::new()
    };
    Ok(Json(json!({
        "source": "startup_configuration",
        "mutable_in_browser": false,
        "organisation": organisation,
        "models_undeclared": undeclared,
        "sign_in": {
            "provider_origin": issuer,
            "session_seconds": startup.session_seconds,
            "secure_cookie": startup.secure_cookie
        },
        "directory": {"roles_configured": state.roles.is_some()},
        "permissions": {
            "model_version": state.grant_setup.model()?.version(),
            "projection": if state.grant_setup.spicedb.is_some() { "spicedb" } else { "local" }
        },
        "secrets": {"configured": state.secrets.is_some()},
        "runtimes": {
            "machines_configured": state.network.is_some(),
            "provisioning_configured": state.provisioning.is_some()
        },
        "storage": {
            "directory_format": "signed_leaf_log",
            "grant_format": "signed_leaf_log",
            "requests_configured": state.requests.is_some()
        }
    })))
}

/// Read the one settled organisation zone without using a per-request host default.
pub(crate) fn organisation(
    state: &AppState,
) -> Result<crate::configuration_store::Zone, ServerError> {
    let mut store = state.configuration.lock().map_err(|error| {
        ServerError::Budget(BudgetError::ConfigurationUnavailable {
            reason: format!("organisation setting lock poisoned: {error}"),
        })
    })?;
    store.settle()?;
    Ok(store.zone().clone())
}

/// Only the administrator changes the organisation setting at the version read.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct ZoneBody {
    zone: String,
    version: u64,
    /// The whole table of declared context windows, by model: a window in
    /// tokens, or null for a model declared as side work. Absent leaves the
    /// table as held.
    #[serde(default)]
    #[schema(value_type = Object)]
    model_windows: Option<crate::budgets_feed::Windows>,
}

async fn set_zone(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ZoneBody>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<crate::configuration_store::Zone>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let Json(body) = body.map_err(|error| {
        ServerError::Budget(BudgetError::BudgetRefused {
            refusal: "ConfigurationMalformed",
            words: error.body_text(),
        })
    })?;
    let by = crate::routes::with_directory(&state, |directory| {
        if crate::routes::admitted_agent(directory.projection()?, &actor)? {
            return crate::caller_admission::active_caller(directory.projection()?, &actor)
                .map(|identity| identity.to_string());
        }
        crate::read_api::own_person(directory.projection()?, &actor)
            .map(|person| person.to_string())
    })?;
    let mut store = state.configuration.lock().map_err(|error| {
        ServerError::Budget(BudgetError::ConfigurationUnavailable {
            reason: format!("organisation setting lock poisoned: {error}"),
        })
    })?;
    store
        .set(body.zone, body.model_windows, body.version, by)
        .map(Json)
}
