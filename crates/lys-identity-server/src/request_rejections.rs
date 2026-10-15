//! Extractor refusals use the same named JSON envelope as handler refusals.
use axum::body::to_bytes;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

pub(crate) async fn named(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let status = response.status();
    let plain = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/plain"));
    if !plain
        || !matches!(
            status,
            StatusCode::BAD_REQUEST
                | StatusCode::UNPROCESSABLE_ENTITY
                | StatusCode::UNSUPPORTED_MEDIA_TYPE
        )
    {
        return response;
    }
    let reason = match to_bytes(response.into_body(), 65_536).await {
        Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
            Ok(reason) => reason,
            Err(error) => {
                format!("the request was rejected, but its reason was not UTF-8: {error}")
            }
        },
        Err(error) => {
            format!("the request was rejected, but its reason could not be read: {error}")
        }
    };
    let mut response = crate::error::ServerError::RequestMalformed { reason }.into_response();
    *response.status_mut() = status;
    response
}
