//! Who a screen route acts for. Either a handle's holder, with a
//! presentation signed over that very request, or a person a trusted screen
//! service vouches for, with the service's signature over that very
//! request. Either way the route answers as that one identity.

use std::sync::{Arc, PoisonError};

use axum::http::StatusCode;
use axum::http::request::Parts;
use lys_secrets::{OnBehalf, SecretsError, request_digest};

use crate::files::now_ms;
use crate::serve::{Shared, on_broker, signed};

/// The identity a screen route acts for, and the service that vouched for
/// it when one did.
pub struct Caller {
    pub identity: String,
    pub via: Option<String>,
}

/// The status a refusal answers with: a name the caller may not discover
/// as not found, a malformed request as bad, a refusal of the caller's
/// authority as forbidden, and a failure of the broker itself as internal.
pub fn refused(error: &SecretsError) -> (StatusCode, String) {
    let status = match error {
        SecretsError::SecretUnknown { .. }
        | SecretsError::HandleUnknown
        | SecretsError::NotFound => StatusCode::NOT_FOUND,
        SecretsError::InvalidScope { .. }
        | SecretsError::InvalidName { .. }
        | SecretsError::Encoding { .. }
        | SecretsError::OwnerChange(_)
        | SecretsError::PresentationInvalid { .. }
        | SecretsError::OperationIdTooShort { .. } => StatusCode::BAD_REQUEST,
        SecretsError::Lending(_)
        | SecretsError::Revocation(_)
        | SecretsError::Service(_)
        | SecretsError::PermissionDenied { .. }
        | SecretsError::NoRelation { .. }
        | SecretsError::RelationRemoved { .. }
        | SecretsError::HandleExpired { .. }
        | SecretsError::HandleDropped { .. }
        | SecretsError::HandleWrongIdentity { .. }
        | SecretsError::PresentationReplayed { .. }
        | SecretsError::PresentationStale { .. }
        | SecretsError::OperationIdReused { .. }
        | SecretsError::LeaseExhausted { .. }
        | SecretsError::LeaseWindowClosed { .. } => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, format!("{}: {error}\n", error.name()))
}

fn header<'a>(parts: &'a Parts, name: &str) -> Option<&'a str> {
    parts
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
}

/// The caller of the request whose head is `parts` and whose body is
/// `body`. A request carrying `lys-service` is a screen service's; any
/// other is a handle's.
pub async fn caller(
    shared: &Arc<Shared>,
    parts: &Parts,
    body: &[u8],
) -> Result<Caller, (StatusCode, String)> {
    if header(parts, "lys-service").is_some() {
        return on_behalf(shared, parts, body);
    }
    let (token, presentation) = signed(parts, body).map_err(|(_status, error)| refused(&error))?;
    let identity = on_broker(shared, move |broker| broker.caller(&token, &presentation))
        .await
        .map_err(|error| refused(&error))?
        .map_err(|error| refused(&error))?;
    Ok(Caller {
        identity,
        via: None,
    })
}

fn on_behalf(shared: &Shared, parts: &Parts, body: &[u8]) -> Result<Caller, (StatusCode, String)> {
    let missing = |name: &str| {
        refused(&SecretsError::Encoding {
            context: "screen service headers",
            reason: format!("no {name} header"),
        })
    };
    let value = |name: &'static str| header(parts, name).ok_or_else(|| missing(name));
    let request = request_digest(
        parts.method.as_str(),
        parts
            .uri
            .path_and_query()
            .map_or("/", |whole| whole.as_str()),
        body,
    )
    .map_err(|error| refused(&error))?;
    let asked = OnBehalf::from_wire(
        [
            value("lys-service")?,
            value("lys-on-behalf-of")?,
            value("lys-operation")?,
            value("lys-signed-at")?,
            value("lys-service-signature")?,
        ],
        request,
    )
    .map_err(|error| refused(&error))?;
    let trusted = shared.layout.services().map_err(|error| refused(&error))?;
    let mut window = shared.window.lock().unwrap_or_else(PoisonError::into_inner);
    let identity = window
        .admit(&trusted, &asked, now_ms())
        .map_err(|error| refused(&error))?;
    Ok(Caller {
        identity,
        via: Some(asked.service),
    })
}
