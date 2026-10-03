//! The signature crosses the process boundary only after the lease settles.

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use lys_secrets::{SecretsError, Used, request_digest};

use crate::serve::{MAX_BODY, Shared, ask, on_broker, signed_for};

fn refusal(status: StatusCode, error: &SecretsError) -> Response {
    (
        status,
        axum::Json(serde_json::json!({"refusal":error.name(), "reason":error.to_string()})),
    )
        .into_response()
}

pub(crate) async fn sign(
    State(shared): State<Arc<Shared>>,
    Path(secret): Path<String>,
    request: Request,
) -> Response {
    match sign_request(&shared, secret, request).await {
        Ok(response) => response,
        Err((status, error)) => refusal(status, &error),
    }
}

async fn sign_request(
    shared: &Arc<Shared>,
    secret: String,
    request: Request,
) -> Result<Response, (StatusCode, SecretsError)> {
    let internal = |error| (StatusCode::INTERNAL_SERVER_ERROR, error);
    let bad = |error| (StatusCode::BAD_REQUEST, error);
    let (parts, body) = request.into_parts();
    // Guarded: read before the caller is known, as the caller's signature covers it.
    let payload = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|error| {
            bad(SecretsError::Encoding {
                context: "signing payload",
                reason: error.to_string(),
            })
        })?;
    let digest = request_digest("SIGN", &secret, &payload).map_err(bad)?;
    let (token, presentation) = signed_for(&parts, digest)?;
    let (token, asks) = on_broker(shared, move |broker| {
        let asks = broker.asks_for(&token);
        (token, asks)
    })
    .await
    .map_err(internal)?;
    let checked = ask(shared, &asks).await.map_err(internal)?;
    let signed = on_broker(shared, move |broker| {
        broker.sign_use_for_checked(&token, &presentation, &secret, &payload, 0, &checked)
    })
    .await
    .map_err(internal)?
    .map_err(|error| (StatusCode::FORBIDDEN, error))?;
    match signed {
        Used::Forwarded { answer, .. } => Ok((
            StatusCode::OK,
            [("content-type", "application/cose")],
            answer.as_bytes().to_vec(),
        )
            .into_response()),
        Used::Retried { outcome } => Ok((
            StatusCode::CONFLICT,
            axum::Json(serde_json::json!({"refusal":"SigningAlreadyPresented","reason":outcome})),
        )
            .into_response()),
    }
}
