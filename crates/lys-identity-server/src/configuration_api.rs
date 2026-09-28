//! The administrator reads effective startup settings without credentials, private paths, or a pretend live editor.

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde_json::{Value, json};

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

#[derive(Clone)]
struct StartupSettings {
    session_seconds: u64,
    secure_cookie: bool,
}

/// Configuration as loaded at startup, read only by the administrator.
pub fn routes(config: &Config) -> Router<Arc<AppState>> {
    Router::new()
        .route("/configuration", get(configuration))
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
    state.admission.administrator(&actor)?;
    let issuer = crate::connections_api::origin(state.oidc.issuer(), "sign-in provider")?;
    Ok(Json(json!({
        "source": "startup_configuration",
        "mutable_in_browser": false,
        "sign_in": {
            "provider_origin": issuer,
            "session_seconds": startup.session_seconds,
            "secure_cookie": startup.secure_cookie
        },
        "directory": {"roles_configured": state.roles.is_some()},
        "permissions": {
            "model_version": state.grant_setup.model.version(),
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
