//! The administrator's configured integrations, without secrets or invented health claims.

use std::sync::Arc;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use openidconnect::reqwest::Url;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// Read the installation's integrations. This is not an upstream or client registry.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/connections", get(connections))
}

/// Only an origin may leave the service: never URL credentials, paths or queries.
pub(crate) fn origin(value: &str, name: &str) -> Result<String, ServerError> {
    let url = Url::parse(value).map_err(|error| ServerError::ConfigInvalid {
        reason: format!("the {name} endpoint cannot be shown: its URL is invalid: {error}"),
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(ServerError::ConfigInvalid {
            reason: format!("the {name} endpoint cannot be shown: HTTP or HTTPS is required"),
        });
    }
    Ok(url.origin().ascii_serialization())
}

async fn connections(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    state.admission.administrator(&actor)?;
    let issuer = origin(state.oidc.issuer(), "sign-in provider")?;
    let projection = state
        .grant_setup
        .spicedb
        .as_ref()
        .map(|settings| {
            // The gateway contract takes host:port, not a URL or a credential.
            origin(
                &format!("http://{}", settings.endpoint),
                "permission engine",
            )
        })
        .transpose()?;
    let broker = state
        .secrets
        .as_ref()
        .map(|broker| origin(broker.endpoint(), "secrets broker"))
        .transpose()?;
    Ok(Json(json!({
        "connections": [
            {"id": "sign_in", "name": "Sign-in provider", "purpose": "Authenticates people signing in to Lys.", "state": "configured", "endpoint": issuer},
            {"id": "permissions", "name": "Permission engine", "purpose": "Checks the access represented by Lys grants.", "state": if projection.is_some() { "configured" } else { "unconfigured" }, "endpoint": projection},
            {"id": "secrets", "name": "Secrets broker", "purpose": "Provides secret access on behalf of the signed-in person.", "state": if broker.is_some() { "configured" } else { "unconfigured" }, "endpoint": broker}
        ],
        "health_checked": false
    })))
}

#[cfg(test)]
mod tests {
    use super::origin;

    #[test]
    fn endpoint_origin_never_carries_credentials_or_private_url_members()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            origin(
                "https://user:password@example.test:8443/private?token=secret#fragment",
                "test"
            )?,
            "https://example.test:8443"
        );
        Ok(())
    }

    #[test]
    fn refused_endpoint_never_echoes_its_value() {
        for endpoint in [
            "secret-invalid-url",
            "file:///secret-path",
            "javascript:secret",
        ] {
            let answer = origin(endpoint, "test");
            assert!(answer.is_err());
            if let Err(error) = answer {
                assert!(!error.to_string().contains("secret"));
                assert!(error.to_string().contains("test"));
            }
        }
    }
}
