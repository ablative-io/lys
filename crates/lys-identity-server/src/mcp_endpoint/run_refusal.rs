//! Run credentials are checked before ingress can turn their refusal into sign-in.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde_json::json;

use crate::error::ServerError;
use crate::routes::AppState;

pub(crate) fn run_refusals(router: Router, state: Arc<AppState>) -> Router {
    router.layer(axum::middleware::from_fn_with_state(state, answer))
}

async fn answer(State(state): State<Arc<AppState>>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let credential = if headers.contains_key(crate::agent_pass::HEADER) {
        "pass"
    } else if headers.contains_key("lys-seat") {
        return crate::signed_first::refuse(
            request,
            refusal(
                "seat_unverifiable",
                "the run's seat has no run pass to verify",
                "seat",
            ),
        )
        .await;
    } else {
        return next.run(request).await;
    };
    if let Err(error) = crate::agent_pass::verified(&state, headers) {
        let (name, words, credential) = match &error {
            ServerError::AgentPassRefused { reason } if reason.starts_with("the run's seat ") => {
                ("seat_unverifiable", reason.as_str(), "seat")
            }
            ServerError::AgentPassRefused { reason }
                if reason == "the pass is unknown or its run has ended" =>
            {
                ("pass_unknown_or_ended", reason.as_str(), credential)
            }
            ServerError::AgentPassRefused { reason } => {
                ("pass_refused", reason.as_str(), credential)
            }
            _ => return crate::signed_first::refuse(request, error.into_response()).await,
        };
        let response = refusal(name, words, credential);
        return crate::signed_first::refuse(request, response).await;
    }
    let response = next.run(request).await;
    if response.status() != StatusCode::UNAUTHORIZED {
        return response;
    }
    let bytes = match axum::body::to_bytes(response.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return unreadable(format!("the MCP refusal body could not be read: {error}"));
        }
    };
    let body: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(body) => body,
        Err(error) => return unreadable(format!("the MCP refusal is not JSON: {error}")),
    };
    let (Some(name), Some(reason)) = (body["refusal"].as_str(), body["reason"].as_str()) else {
        return unreadable("the MCP refusal names no refusal or reason".to_owned());
    };
    let words = reason
        .strip_prefix(name)
        .and_then(|text| text.strip_prefix(": "))
        .unwrap_or(reason);
    refusal(name, words, credential)
}

fn unreadable(reason: String) -> Response {
    ServerError::Runner {
        refusal: "mcp_refusal_unreadable".to_owned(),
        words: reason,
    }
    .into_response()
}

fn refusal(name: &str, words: &str, credential: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "refusal": name, "words": words, "credential": credential
        })),
    )
        .into_response()
}
