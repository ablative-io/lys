//! An app's virtual client credentials, for a trusted screen service only
//! (DIRECTORY-081): issued once to an administrator, confirmed at the token
//! exchange, and ended on revocation or retirement. The app's sealed client
//! secret never leaves the broker, and a credential's value is answered only
//! by the issue that made it.
use std::sync::Arc;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use lys_secrets::{Secret, SecretsError};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::callers::{Caller, caller, refused};
use crate::serve::{MAX_BODY, Shared, on_broker};

type Answer = Result<Json<Value>, (StatusCode, String)>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Issue {
    app: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authenticate {
    app: String,
    presented: String,
    live: Vec<String>,
    secret_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct End {
    app: String,
    credential_ids: Vec<String>,
    why: String,
}

/// The trusted screen service's caller and its body, read whole, or the
/// refusal. Anything but a screen service is refused.
pub(crate) async fn asked<T: DeserializeOwned>(
    shared: &Arc<Shared>,
    request: Request,
) -> Result<(Caller, T), (StatusCode, String)> {
    let (parts, body): (Parts, _) = request.into_parts();
    // Guarded: read before the caller is known, as the caller's signature covers it.
    let body: Bytes = axum::body::to_bytes(body, MAX_BODY)
        .await
        .map_err(|_error| {
            (
                StatusCode::BAD_REQUEST,
                "RequestMalformed: unreadable body".to_owned(),
            )
        })?;
    let who = caller(shared, &parts, &body).await?;
    if who.via.is_none() {
        return Err((
            StatusCode::FORBIDDEN,
            "NotAdmitted: a trusted screen service is required".to_owned(),
        ));
    }
    let asked = serde_json::from_slice(&body).map_err(|_error| {
        (
            StatusCode::BAD_REQUEST,
            "RequestMalformed: invalid app client credential request".to_owned(),
        )
    })?;
    Ok((who, asked))
}

/// Issues an app a virtual client credential, as the person the service
/// names, answering its value this once.
pub async fn issue(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked): (Caller, Issue) = asked(&shared, request).await?;
    on_broker(&shared, move |broker| {
        let issued = broker.issue_app_client(&asked.app, &who.identity)?;
        let value = String::from_utf8(issued.value.expose().to_vec()).map_err(|error| {
            SecretsError::Encoding {
                context: "app client credential",
                reason: error.to_string(),
            }
        })?;
        Ok::<_, SecretsError>(Json(json!({
            "app": asked.app,
            "credential_id": issued.credential_id,
            "owner": issued.owner,
            "client_secret_sha256": issued.client_secret_sha256,
            "value": value,
        })))
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}

/// Confirms a presented credential for the token exchange, answering its id.
pub async fn authenticate(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (_who, asked): (Caller, Authenticate) = asked(&shared, request).await?;
    on_broker(&shared, move |broker| {
        let presented = Secret::from_slice(asked.presented.as_bytes());
        let credential_id = broker.authenticate_app_client(
            &asked.app,
            &presented,
            &asked.live,
            &asked.secret_sha256,
        )?;
        Ok::<_, SecretsError>(Json(
            json!({"app": asked.app, "credential_id": credential_id}),
        ))
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}

/// Ends an app's credentials, answering the ones ended now.
pub async fn end(State(shared): State<Arc<Shared>>, request: Request) -> Answer {
    let (who, asked): (Caller, End) = asked(&shared, request).await?;
    on_broker(&shared, move |broker| {
        let ended =
            broker.end_app_clients(&asked.app, &asked.credential_ids, &who.identity, &asked.why)?;
        Ok::<_, SecretsError>(Json(json!({"app": asked.app, "ended": ended})))
    })
    .await
    .map_err(|error| refused(&error))?
    .map_err(|error| refused(&error))
}
