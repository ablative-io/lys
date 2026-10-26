//! Complete an issuer callback, preserving the personal session admission check.

use crate::error::ServerError;
use crate::routes::AppState;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct Answer {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub(crate) async fn callback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    answer: Result<Query<Answer>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, ServerError> {
    let Ok(Query(answer)) = answer else {
        return Ok(refused(
            &headers,
            "SignInFailed",
            "Lys could not read the sign-in answer. Try signing in again.",
        ));
    };
    if answer.error.is_some() {
        if let Some(state_id) = answer.state.as_deref() {
            state.oidc.abandon(state_id)?;
        }
        let (name, words) =
            issuer_refusal(answer.error.as_deref(), answer.error_description.as_deref());
        return Ok(refused(&headers, name, words));
    }
    let (Some(code), Some(state_id)) = (answer.code, answer.state) else {
        return Ok(refused(
            &headers,
            "SignInFailed",
            "The sign-in answer was incomplete. Try signing in again.",
        ));
    };
    let actor = match state.oidc.finish(code, &state_id).await {
        Ok(actor) => actor,
        Err(error) if wants_page(&headers) => {
            return Ok(refused(
                &headers,
                &error.name(),
                "Lys could not complete the sign-in. Try signing in again.",
            ));
        }
        Err(error) => return Err(error),
    };
    if wants_page(&headers) {
        let cookie = crate::session_admission::begin(&state, actor).await?;
        return Ok((
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, cookie),
                (header::LOCATION, "/".to_owned()),
            ],
        )
            .into_response());
    }
    crate::sign_in::begin_session(&state, &actor).await
}

fn issuer_refusal(error: Option<&str>, description: Option<&str>) -> (&'static str, &'static str) {
    match (error, description) {
        (_, Some("User not found")) => (
            "NotAdmittedYet",
            "This account has not been added to Lys yet. Ask your administrator to add it.",
        ),
        (Some("Disabled"), _) | (_, Some("User is not enabled")) => (
            "AccountDisabled",
            "This account has been disabled in Lys. Ask your administrator to enable it.",
        ),
        (Some("access_denied"), _) => (
            "SignInCancelled",
            "Sign-in was cancelled or permission was not given. You can try again.",
        ),
        (Some("temporarily_unavailable" | "server_error"), _) => (
            "SignInFailed",
            "The sign-in service could not finish your sign-in. Try again.",
        ),
        _ => (
            "SignInFailed",
            "The sign-in service refused this sign-in. Try signing in again.",
        ),
    }
}

fn refused(headers: &HeaderMap, name: &str, words: &str) -> Response {
    let answer = if wants_page(headers) {
        Html(format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Sign in to Lys</title><main><h1>Sign-in did not finish</h1><p>{words}</p><a href=\"/#/sign-in\">Try again</a></main></html>")).into_response()
    } else {
        Json(serde_json::json!({"refusal": name, "reason": words})).into_response()
    };
    (
        StatusCode::BAD_REQUEST,
        [(header::CACHE_CONTROL, "no-store")],
        answer,
    )
        .into_response()
}

/// Whether the caller is a browser following the sign-in, which is taken to the
/// screens, rather than a program, which is answered the signed-in JSON.
fn wants_page(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| {
            accept
                .split(',')
                .any(|kind| kind.trim().starts_with("text/html"))
        })
}

/// The provider is the one kept in the initiating server-side flight.
pub(crate) struct ProviderLogin {
    pub(crate) actor: lys_identity::Actor,
    pub(crate) provider: String,
}

#[cfg(test)]
#[path = "sign_in_callback_tests.rs"]
mod provider_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_issuer_error_is_a_page_with_a_next_step_and_never_raw_query_text()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut headers = HeaderMap::new();
        headers.insert(header::ACCEPT, "text/html".parse()?);
        let (name, words) = issuer_refusal(Some("access_denied"), Some("private-token<script>"));
        assert_eq!(name, "SignInCancelled");
        let answer = refused(&headers, name, words);
        assert_eq!(answer.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(answer.into_body(), 4096).await?;
        let page = std::str::from_utf8(&bytes)?;
        assert!(page.contains("Sign-in was cancelled"));
        assert!(page.contains("href=\"/#/sign-in\">Try again"));
        assert!(!page.contains("private-token"));
        assert!(!page.contains("missing field"));
        Ok(())
    }

    #[test]
    fn an_error_return_does_not_require_a_code() -> Result<(), Box<dyn std::error::Error>> {
        let answer: Answer = serde_json::from_value(serde_json::json!({
            "error": "access_denied", "error_description": "Permission was not given"
        }))?;
        assert!(answer.code.is_none());
        assert_eq!(answer.error.as_deref(), Some("access_denied"));
        Ok(())
    }
}
