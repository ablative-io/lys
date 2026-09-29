//! Complete an issuer callback, preserving the personal session admission check.

use crate::error::ServerError;
use crate::routes::Shared;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Answer {
    code: String,
    state: String,
}

pub(crate) async fn callback(
    State(state): State<Shared>,
    headers: HeaderMap,
    Query(answer): Query<Answer>,
) -> Result<Response, ServerError> {
    let actor = state.oidc.finish(answer.code, &answer.state).await?;
    if wants_page(&headers) {
        let cookie = crate::session_admission::begin(&state, actor)?;
        return Ok((
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, cookie),
                (header::LOCATION, "/".to_owned()),
            ],
        )
            .into_response());
    }
    crate::sign_in::begin_session(&state, &actor)
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
