//! JSON admission for handlers that retain exact bytes for an agent signature.
//! Browser origins come from configured service coordinates, never Host or
//! forwarded headers. Non-browser requests may omit Origin; text/plain cannot
//! replace JSON. Parsing leaves the caller's signed byte buffer unchanged.

use axum::Json;
use axum::http::{HeaderMap, header};
use serde::de::DeserializeOwned;

use crate::error::ServerError;
use crate::routes::AppState;

fn malformed(reason: &str) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.to_owned(),
    }
}

fn check(headers: &HeaderMap, origin: &str) -> Result<(), ServerError> {
    let mut origins = headers.get_all(header::ORIGIN).iter();
    if let Some(given) = origins.next()
        && (origins.next().is_some() || given.to_str().ok() != Some(origin))
    {
        return Err(malformed(
            "the request Origin is not this service's configured origin",
        ));
    }
    let mut types = headers.get_all(header::CONTENT_TYPE).iter();
    let kind = types
        .next()
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase());
    if types.next().is_some()
        || !kind.is_some_and(|kind| {
            kind == "application/json"
                || (kind.starts_with("application/") && kind.ends_with("+json"))
        })
    {
        return Err(malformed(
            "the request must carry one JSON Content-Type (act: send Content-Type: application/json)",
        ));
    }
    Ok(())
}

/// Read JSON from the original signed bytes, after checking media type and Origin.
pub(crate) fn read<T: DeserializeOwned>(
    state: &AppState,
    headers: &HeaderMap,
    bytes: &[u8],
) -> Result<T, ServerError> {
    check(headers, state.oidc.public_origin())?;
    let Json(body) =
        Json::<T>::from_bytes(bytes).map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })?;
    Ok(body)
}

#[cfg(test)]
#[path = "signed_json_tests.rs"]
mod tests;
