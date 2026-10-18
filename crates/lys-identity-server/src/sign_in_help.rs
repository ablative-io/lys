//! Public sign-in help exposes only the configured administrator's contact details.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::error::ServerError;
use crate::routes::{AppState, with_directory};

pub(super) async fn contact(State(state): State<Arc<AppState>>) -> Result<Response, ServerError> {
    let contact = match state.admission.administrator_login()? {
        Some(login) => {
            let name = with_directory(&state, |directory| {
                Ok(crate::admission::Admission::administrator_name(
                    directory.projection()?,
                    &login,
                ))
            })?;
            match name {
                Some(name) => {
                    let email = if login.issuer() == state.oidc.issuer() {
                        match state.sign_in_providers.as_ref() {
                            Some(api) => {
                                let email = crate::accounts::email_of(api, login.subject()).await?;
                                Some(crate::accounts::check_email(&email)?.to_owned())
                            }
                            None => None,
                        }
                    } else {
                        None
                    };
                    json!({"name": name, "email": email})
                }
                None => serde_json::Value::Null,
            }
        }
        None => serde_json::Value::Null,
    };
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"administrator": contact})),
    )
        .into_response())
}

/// The providers a person may sign in with, one button each: their id at
/// the issuer, their name and which offered provider each is; and Lys's own
/// origin, the address a person is told to open.
pub(super) async fn offered(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, ServerError> {
    let Some(api) = state.sign_in_providers.as_ref() else {
        return Ok(Json(
            json!({ "providers": [], "origin": state.oidc.public_origin() }),
        ));
    };
    let mut providers: Vec<serde_json::Value> = api
        .listed()
        .await?
        .into_iter()
        .filter(|listed| listed.enabled)
        .map(|listed| {
            json!({
                "id": listed.id,
                "name": listed.name,
                "provider": crate::sign_in_providers::Provider::from_name(&listed.name),
            })
        })
        .collect();
    providers.sort_by_key(|provider| provider["name"].as_str().map(str::to_owned));
    Ok(Json(
        json!({ "providers": providers, "origin": state.oidc.public_origin() }),
    ))
}

#[cfg(test)]
#[path = "sign_in_help_tests.rs"]
mod tests;
