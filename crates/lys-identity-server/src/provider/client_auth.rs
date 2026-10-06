//! The token exchange's client authentication, judged before anything else of
//! the request, the grant type among it (DIRECTORY-081).
//!
//! A secret that begins `lys-client.` is one of the app's virtual client
//! credentials, and only the secrets broker can confirm it: the app is judged
//! from the apps' record first (approved, and not retired, so a retired app's
//! credential is refused `app_retired` before the broker is asked), its live
//! credentials, the identity holding its custody and its approved secret's
//! digest are read under the apps lock, the lock is released, and then the
//! broker is asked. A credential the broker does not confirm is
//! `credential_refused`; a broker that cannot answer is `SecretsUnavailable`,
//! by name, and such a secret is never judged against the approved digest
//! instead. Any other secret is judged against the approved digest, as it
//! always was.

use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde::Deserialize;
use serde_json::json;

use super::endpoints::{Exchange, apps, presented};
use crate::apps_binding::{sign_in_app, sign_in_secret};
use crate::apps_error::AppError;
use crate::error::ServerError;
use crate::error_provider::ProviderError;
use crate::routes::AppState;

/// The prefix every virtual client credential carries.
const APP_CLIENT_PREFIX: &str = "lys-client.";

/// The broker's refusal of a credential it does not confirm.
const BROKER_REFUSED: &str = "AppClientCredentialRefused";

/// What the broker answers when it confirms a credential.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Confirmed {
    app: String,
    credential_id: String,
}

/// The app a token request authenticates as, or the refusal.
pub(super) async fn authenticated(
    state: &AppState,
    headers: &HeaderMap,
    form: &Exchange,
) -> Result<String, ServerError> {
    let (client_id, secret) =
        presented(headers, form).ok_or(ServerError::Provider(ProviderError::ClientUnknown))?;
    let refused = |reason| ServerError::from(AppError::CredentialRefused { reason });
    let (app, owner, live, body) = {
        let mut apps = apps(state)?;
        apps.settle()?;
        if !secret.starts_with(APP_CLIENT_PREFIX) {
            let app = sign_in_secret(apps.held(), &client_id, &secret)?;
            return Ok(app.registered.app.clone());
        }
        let app = sign_in_app(apps.held(), &client_id)?;
        let live = app.live_client_credentials();
        let owner = app
            .client_custody_owner()
            .filter(|_owner| !live.is_empty())
            .ok_or_else(|| refused("the app holds no live client credential"))?;
        let digest = app
            .approved
            .as_ref()
            .map(|approved| approved.client.secret_sha256.clone())
            .ok_or_else(|| refused("no approved app holds that client id"))?;
        let body = json!({
            "app": app.registered.app,
            "presented": secret,
            "live": live,
            "secret_sha256": digest,
        })
        .to_string();
        (app.registered.app.clone(), owner, live, body)
    };
    let answer = crate::secrets_api::ask_as(
        state,
        &owner,
        Method::POST,
        "/_lys/apps/client",
        Bytes::from(body),
    )
    .await
    .map_err(|error| match error {
        ServerError::SecretsRefused { refusal, .. } if refusal == BROKER_REFUSED => {
            refused("no live client credential of this app holds that secret")
        }
        ServerError::SecretsRefused {
            status,
            refusal,
            reason,
        } => ServerError::SecretsUnavailable {
            reason: format!(
                "the secrets broker could not confirm the app's client credential; it answered {status}: {refusal}: {reason}"
            ),
        },
        other => other,
    })?;
    let confirmed: Confirmed =
        serde_json::from_value(answer).map_err(|error| ServerError::SecretsUnavailable {
            reason: format!("the secrets broker's confirmation could not be read: {error}"),
        })?;
    if confirmed.app != app || !live.contains(&confirmed.credential_id) {
        return Err(ServerError::SecretsUnavailable {
            reason: "the secrets broker confirmed a credential the apps' record does not hold live"
                .to_owned(),
        });
    }
    Ok(app)
}
