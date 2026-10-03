//! Both start routes apply the same current stop limits before their handlers admit new work.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::rejection::PathRejection;
use axum::extract::{MatchedPath, Path, Request, State};
use axum::http::Method;
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use lys_identity::OperationId;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::error::ServerError;
use crate::routes::{AppState, signed_in};

/// Guard the joined routes once, inside authentication and session admission.
pub(crate) fn guarded(api: Router, state: Arc<AppState>) -> Router {
    api.route_layer(from_fn_with_state(state, check))
}

async fn check(
    State(state): State<Arc<AppState>>,
    matched: Option<MatchedPath>,
    parameters: Result<Path<BTreeMap<String, String>>, PathRejection>,
    request: Request,
    next: Next,
) -> Response {
    let path = matched
        .as_ref()
        .map(MatchedPath::as_str)
        .map(|path| path.strip_prefix("/api").unwrap_or(path));
    if request.method() != Method::POST
        || !matches!(
            path,
            Some("/agents/{id}/start" | "/agents/{id}/start-command")
        )
        || state.budgets.is_none()
    {
        return next.run(request).await;
    }
    let agent = match parameters {
        Ok(Path(parameters)) => match parameters.get("id") {
            Some(agent) => agent.clone(),
            None => {
                return ServerError::RequestMalformed {
                    reason: "a start names its agent".to_owned(),
                }
                .into_response();
            }
        },
        Err(error) => {
            return ServerError::RequestMalformed {
                reason: format!("start agent could not be read: {error}"),
            }
            .into_response();
        }
    };
    let actor = match signed_in(&state, request.headers()) {
        Ok(actor) => actor,
        Err(error) => return error.into_response(),
    };
    if let Err(error) = crate::launch_api::start_caller(&state, request.headers(), &actor, &agent) {
        return error.into_response();
    }
    let start_command = path == Some("/agents/{id}/start-command");
    // Read whole: `signed_first.rs`, outside this guard, has already held
    // an unverified caller's body to `UNVERIFIED_BODY_LIMIT`.
    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(error) => {
            return ServerError::RequestMalformed {
                reason: format!("start body could not be read: {error}"),
            }
            .into_response();
        }
    };
    let body: Value = match serde_json::from_slice(&bytes) {
        Ok(body) => body,
        Err(error) => {
            return ServerError::RequestMalformed {
                reason: format!("start body is not JSON: {error}"),
            }
            .into_response();
        }
    };
    let replay = if start_command {
        match body["operation"]
            .as_str()
            .map(str::parse::<OperationId>)
            .transpose()
        {
            Ok(Some(operation)) => {
                crate::launch_api::admitted(&state, &operation.to_string(), &agent)
                    .map(|held| held.is_some())
            }
            Ok(None) => Ok(false),
            Err(error) => Err(error.into()),
        }
    } else {
        Ok(false)
    };
    match replay {
        Err(error) => return error.into_response(),
        Ok(true) => {}
        Ok(false) => {
            if let Err(error) = crate::budgets_enforce::admit_at(
                &state,
                &agent,
                jiff::Timestamp::now().as_millisecond(),
            ) {
                return error.into_response();
            }
        }
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}
