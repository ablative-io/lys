//! Sign-in through a provider, Google, GitHub or Microsoft, started from a
//! button on Lys's sign-in page and finished on Lys's own origin.
//!
//! The service opens a session at the issuer exactly as a password sign-in
//! does, then asks the issuer to start the provider's sign-in with a PKCE
//! challenge of its own. The issuer answers the provider's address; that is
//! the one address outside Lys a browser is sent to, and a person passes
//! through the provider's own consent page only. The provider sends the
//! person back to the issuer's public address, which is Lys's origin, at
//! the issuer's provider callback path: Lys serves that path, and the
//! service finishes the provider's sign-in at the issuer from the server,
//! with the cookies, request token, cross-site token and PKCE verifier it
//! kept since the start. The issuer's answer is then finished as `/callback`
//! finishes one, and the person lands on a Lys page signed in.
//!
//! Invariants: the provider's address is refused unless it sends the person
//! back to Lys's own callback address, so a provider is never registered
//! with an address that is not Lys's; a provider sign-in in flight is used
//! once; and every refusal lands the person on Lys's sign-in screen with its
//! name, never on a page of the issuer.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Path, Query, State};
use axum::http::{Extensions, HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_identity::Actor;
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    IssuerSignIn, Opened, SIGN_IN_SCREEN, accepted, failed, query_value, request_fields,
    unreachable_issuer,
};
use crate::error::ServerError;
use crate::oidc::Oidc;
use crate::routes::AppState;
use crate::sign_in_providers::Provider;

/// Where a person lands once a provider sign-in has finished, on Lys's origin.
const SIGNED_IN: &str = "/#/me";

/// A provider sign-in the service began at the issuer and has not finished.
pub(super) struct Upstream {
    opened: Opened,
    xsrf: String,
    verifier: String,
    state: String,
    browser: [u8; 32],
}

/// A PKCE verifier: 32 bytes from the secure random source, base64url.
fn verifier() -> Result<String, ServerError> {
    let mut bytes = [0u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| failed(format!("the secure random source failed: {error}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

impl IssuerSignIn {
    /// Begin a sign-in through the issuer's provider `provider` for the
    /// person at `address`, answering the provider's address to send the
    /// browser to.
    pub async fn begin_provider(
        &self,
        oidc: &Oidc,
        provider: &str,
        address: IpAddr,
        browser: [u8; 32],
    ) -> Result<String, ServerError> {
        let begun = reqwest::Url::parse(&oidc.begin_provider(address)?)
            .map_err(|error| failed(format!("the sign-in start is not an address: {error}")))?;
        let state = query_value(&begun, "state")
            .ok_or_else(|| failed("the sign-in start carries no state"))?;
        match self
            .start_upstream(&begun, provider, address, state.clone(), browser)
            .await
        {
            Ok(location) => Ok(location),
            Err(error) => {
                oidc.abandon(&state)?;
                Err(error)
            }
        }
    }

    async fn start_upstream(
        &self,
        begun: &reqwest::Url,
        provider: &str,
        address: IpAddr,
        state: String,
        browser: [u8; 32],
    ) -> Result<String, ServerError> {
        let client_address = address;
        let started_at = Instant::now();
        let address = address.to_string();
        let mut opened = self.open(begun, &address).await?;
        let verifier = verifier()?;
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut request = request_fields(begun);
        request.insert("email".to_owned(), Value::Null);
        request.insert("pow".to_owned(), json!(opened.pow));
        request.insert("provider_id".to_owned(), json!(provider));
        request.insert("pkce_challenge".to_owned(), json!(challenge));
        let answer = self
            .post_in(
                "/providers/login",
                &opened,
                &address,
                &Value::Object(request),
            )
            .await?;
        match answer.status().as_u16() {
            202 => {}
            404 => return Err(failed("that sign-in provider is not set up")),
            429 => return Err(ServerError::SignInThrottled),
            other => {
                return Err(failed(format!(
                    "the sign-in service answered {other} to the start of a provider sign-in"
                )));
            }
        }
        opened.jar.keep(answer.headers());
        let location = answer
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
            .ok_or_else(|| failed("the sign-in service named no provider address"))?;
        let xsrf = answer
            .text()
            .await
            .map_err(unreachable_issuer)?
            .trim()
            .to_owned();
        let provider_url = reqwest::Url::parse(&location)
            .map_err(|error| failed(format!("the provider's address is not one: {error}")))?;
        if query_value(&provider_url, "redirect_uri").as_deref() != Some(self.callback.as_str()) {
            return Err(failed(format!(
                "the provider would send the person back to an address that is not Lys's own {} (act: set the sign-in service's public address to Lys's origin)",
                self.callback
            )));
        }
        let upstream = query_value(&provider_url, "state")
            .ok_or_else(|| failed("the provider's address carries no state"))?;
        let mut held = self
            .upstream
            .lock()
            .map_err(|error| failed(format!("provider sign-in flights unavailable: {error}")))?;
        held.insert(
            upstream,
            Upstream {
                opened,
                xsrf,
                verifier,
                state,
                browser,
            },
            client_address,
            started_at,
        )?;
        Ok(location)
    }

    /// Finish the provider sign-in in flight under the provider's `upstream`
    /// state with the provider's `code`, answering the actor it signs in.
    pub async fn finish_provider(
        &self,
        oidc: &Oidc,
        code: &str,
        upstream: &str,
        address: IpAddr,
        browser: [u8; 32],
    ) -> Result<Actor, ServerError> {
        let held = self
            .upstream
            .lock()
            .map_err(|error| failed(format!("provider sign-in flights unavailable: {error}")))?
            .take(upstream, Instant::now())?;
        if !crate::provider_browser::matches(&held.browser, &browser) {
            oidc.abandon(&held.state)?;
            return Err(ServerError::SignInStateUnknown);
        }
        let body = json!({
            "state": upstream,
            "code": code,
            "xsrf_token": held.xsrf,
            "pkce_verifier": held.verifier,
        });
        let outcome = match self
            .post_in(
                "/providers/callback",
                &held.opened,
                &address.to_string(),
                &body,
            )
            .await
        {
            Ok(answer) if answer.status().is_client_error() && answer.status() != 429 => {
                Err(failed("the sign-in through the provider was not accepted"))
            }
            Ok(answer) => accepted(&answer),
            Err(error) => Err(error),
        };
        match outcome {
            Ok((code, answered)) if answered == held.state => oidc.finish(code, &held.state).await,
            Ok(_) => {
                oidc.abandon(&held.state)?;
                Err(ServerError::SignInStateUnknown)
            }
            Err(error) => {
                oidc.abandon(&held.state)?;
                Err(error)
            }
        }
    }
}

/// Send the browser to Lys's sign-in screen naming the refusal.
fn to_sign_in(error: &ServerError) -> Response {
    let location = format!("{SIGN_IN_SCREEN}?refused={}", error.name());
    (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
}

/// The sign-in provider routes under the service's own routes.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sign-in/providers", get(offered))
        .route("/sign-in/providers/{id}", get(start))
}

/// The route the issuer's provider callback path is served at, on Lys's
/// own origin, beside the screens rather than under `/api`.
pub fn callback_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(super::PROVIDER_CALLBACK_PATH, get(callback))
        .with_state(state)
}

/// The providers a person may sign in with, one button each: their id at
/// the issuer, their name and which offered provider each is.
async fn offered(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ServerError> {
    let Some(api) = state.sign_in_providers.as_ref() else {
        return Ok(Json(json!({ "providers": [] })));
    };
    let mut providers: Vec<Value> = api
        .listed()
        .await?
        .into_iter()
        .filter(|listed| listed.enabled)
        .map(|listed| {
            json!({
                "id": listed.id,
                "name": listed.name,
                "provider": Provider::from_name(&listed.name),
            })
        })
        .collect();
    providers.sort_by_key(|provider| provider["name"].as_str().map(str::to_owned));
    Ok(Json(json!({ "providers": providers })))
}

async fn begin(
    state: &AppState,
    extensions: &Extensions,
    id: &str,
    headers: &HeaderMap,
) -> Result<(String, String), ServerError> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err(failed("that sign-in provider is not set up"));
    }
    let address = state.sign_in.address(extensions, headers)?;
    let (cookie, digest) =
        crate::provider_browser::begin(state.sign_in.callback().starts_with("https://"))?;
    let location = state
        .sign_in
        .begin_provider(&state.oidc, id, address, digest)
        .await?;
    Ok((location, cookie))
}

async fn start(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    match begin(&state, &extensions, &id, &headers).await {
        Ok((location, cookie)) => (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, location), (header::SET_COOKIE, cookie)],
        )
            .into_response(),
        Err(error) => to_sign_in(&error),
    }
}

/// What a provider sends the person back with.
#[derive(Deserialize)]
struct Back {
    code: Option<String>,
    state: Option<String>,
}

async fn finish(
    state: &AppState,
    extensions: &Extensions,
    back: Back,
    headers: &HeaderMap,
) -> Result<String, ServerError> {
    let (Some(code), Some(upstream)) = (back.code, back.state) else {
        return Err(failed("the provider did not sign the person in"));
    };
    let address = state.sign_in.address(extensions, headers)?;
    let actor = state
        .sign_in
        .finish_provider(
            &state.oidc,
            &code,
            &upstream,
            address,
            crate::provider_browser::digest(headers)?,
        )
        .await?;
    crate::session_admission::begin(state, actor).await
}

async fn callback(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    Query(back): Query<Back>,
    headers: HeaderMap,
) -> Response {
    match finish(&state, &extensions, back, &headers).await {
        Ok(cookie) => (
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, cookie),
                (header::LOCATION, SIGNED_IN.to_owned()),
            ],
        )
            .into_response(),
        Err(error) => to_sign_in(&error),
    }
}
