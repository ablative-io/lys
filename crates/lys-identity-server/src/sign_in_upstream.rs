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
//! name, never on a page of the issuer, keeping the continuation the sign-in
//! was started with so signing in there still returns to it. A continuation
//! is carried only once it is one Lys accepts ([`Continuation::accepted`]).

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{Extensions, HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_identity::Actor;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::{
    IssuerSignIn, Opened, SIGN_IN_SCREEN, accepted, failed, query_value, request_fields,
    unix_second, unreachable_issuer,
};
use crate::error::ServerError;
use crate::oidc::Oidc;
use crate::routes::AppState;
use crate::sign_in_callback::ProviderLogin;
use crate::sign_in_refusal_line as refusal_line;

/// Where a person lands once a provider sign-in has finished, on Lys's origin.
const SIGNED_IN: &str = "/#/me";

/// A provider sign-in the service began at the issuer and has not finished.
pub(super) struct Upstream {
    provider: String,
    opened: Opened,
    xsrf: String,
    verifier: String,
    state: String,
    browser: [u8; 32],
    continuation: Option<Continuation>,
}

#[path = "sign_in_continuation.rs"]
mod continuation;
use continuation::{Continuation, Refused, verifier};

impl IssuerSignIn {
    /// Begin a sign-in through the issuer's provider `provider` for the
    /// person at `address`, answering the provider's address to send the
    /// browser to.
    ///
    /// # Errors
    /// Refuses an unknown provider, unavailable or exhausted sign-in pools,
    /// issuer provider flow failures or an invalid returned provider address.
    pub async fn begin_provider(
        &self,
        oidc: &Oidc,
        provider: &str,
        address: IpAddr,
        browser: [u8; 32],
    ) -> Result<String, ServerError> {
        self.begin_provider_to(oidc, provider, address, browser, None)
            .await
    }

    async fn begin_provider_to(
        &self,
        oidc: &Oidc,
        provider: &str,
        address: IpAddr,
        browser: [u8; 32],
        continuation: Option<Continuation>,
    ) -> Result<String, ServerError> {
        let begun = reqwest::Url::parse(&oidc.begin_provider(address)?)
            .map_err(|error| failed(format!("the sign-in start is not an address: {error}")))?;
        let state = query_value(&begun, "state")
            .ok_or_else(|| failed("the sign-in start carries no state"))?;
        match self
            .start_upstream(
                &begun,
                provider,
                address,
                state.clone(),
                browser,
                continuation,
            )
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
        continuation: Option<Continuation>,
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
                provider: provider.to_owned(),
                opened,
                xsrf,
                verifier,
                state,
                browser,
                continuation,
            },
            client_address,
            started_at,
        )?;
        Ok(location)
    }

    /// Finish the provider sign-in in flight under the provider's `upstream`
    /// state with the provider's `code`, answering the actor it signs in.
    ///
    /// # Errors
    /// Refuses an unknown, expired or mismatched provider continuation,
    /// issuer provider or code exchange failures, or an invalid ID token.
    pub async fn finish_provider(
        &self,
        oidc: &Oidc,
        code: &str,
        upstream: &str,
        address: IpAddr,
        browser: [u8; 32],
    ) -> Result<Actor, ServerError> {
        match self
            .finish_provider_to(oidc, code, upstream, address, browser)
            .await
        {
            Ok((login, _)) => Ok(login.actor),
            Err(refused) => Err(refused.error),
        }
    }

    async fn finish_provider_to(
        &self,
        oidc: &Oidc,
        code: &str,
        upstream: &str,
        address: IpAddr,
        browser: [u8; 32],
    ) -> Result<(ProviderLogin, Option<Continuation>), Box<Refused>> {
        let held = self
            .upstream
            .lock()
            .map_err(|error| failed(format!("provider sign-in flights unavailable: {error}")))
            .and_then(|mut flights| flights.take(upstream, Instant::now()))
            .map_err(Refused::dropping)?;
        if !crate::provider_browser::matches(&held.browser, &browser) {
            // Another browser answered: the continuation was asked for by
            // the browser that began, so it is not handed to this one.
            oidc.abandon(&held.state).map_err(Refused::dropping)?;
            return Err(Refused::dropping(ServerError::SignInStateUnknown).into());
        }
        let continuation = held.continuation.clone();
        match self.settle(oidc, code, upstream, address, held).await {
            Ok(actor) => Ok((actor, continuation)),
            Err(refused) => Err(Refused {
                error: refused.error,
                email: refused.email,
                continuation,
            }
            .into()),
        }
    }

    /// Finish the provider sign-in `held` at the issuer, from the browser
    /// that began it, answering the actor it signs in.
    async fn settle(
        &self,
        oidc: &Oidc,
        code: &str,
        upstream: &str,
        address: IpAddr,
        held: Upstream,
    ) -> Result<ProviderLogin, ProviderRefusal> {
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
                Err(callback_refusal(answer).await)
            }
            Ok(answer) => match unix_second() {
                Ok(checked_at) => accepted(answer, None, checked_at).await.map_err(Into::into),
                Err(error) => Err(error.into()),
            },
            Err(error) => Err(error.into()),
        };
        match outcome {
            Ok((code, answered)) if answered == held.state => oidc
                .finish_provider(code, &held.state)
                .await
                .map(|actor| ProviderLogin {
                    actor,
                    provider: held.provider,
                })
                .map_err(Into::into),
            Ok(_) => {
                oidc.abandon(&held.state)?;
                Err(ServerError::SignInStateUnknown.into())
            }
            Err(error) => {
                oidc.abandon(&held.state)?;
                Err(error)
            }
        }
    }
}

/// Send the browser to Lys's sign-in screen naming the refusal, keeping
/// `continuation`, encoded, so signing in there continues where it was going.
fn to_sign_in(error: &ServerError, continuation: Option<&Continuation>) -> Response {
    refusal_line::refused(refusal_line::PROVIDER_START_ROUTE, "", error);
    sent_to_sign_in(error, continuation, None)
}

/// [`to_sign_in`] for a refused provider callback, naming `email`, the
/// address the issuer would not admit, when it gave one.
fn refused_to_sign_in(
    error: &ServerError,
    continuation: Option<&Continuation>,
    email: Option<&str>,
) -> Response {
    // The browser receives the named refusal and masked mismatch addresses.
    // The refusal, the route,
    // the address tried and the refusal's sanitised words are said through
    // the WARN sink into the service's log, so whoever reads it can see who
    // and why (DIRECTORY-099 R2).
    refusal_line::refused(
        refusal_line::CALLBACK_ROUTE,
        email.unwrap_or_default(),
        error,
    );
    sent_to_sign_in(error, continuation, email)
}

fn sent_to_sign_in(
    error: &ServerError,
    continuation: Option<&Continuation>,
    email: Option<&str>,
) -> Response {
    let mut location = format!("{SIGN_IN_SCREEN}?refused={}", error.name());
    if let ServerError::ProviderEmailMismatch {
        provider,
        account,
        held,
    } = error
    {
        for (name, value) in [
            ("provider", provider),
            ("account_email", account),
            ("held_email", held),
        ] {
            location.push('&');
            location.push_str(name);
            location.push('=');
            location.push_str(&crate::provider::encoded(value));
        }
    }
    if let Some(email) = email {
        location.push_str("&email=");
        location.push_str(&crate::provider::encoded(email));
    }
    if let Some(Continuation(target)) = continuation {
        location.push_str("&continue=");
        location.push_str(&crate::provider::encoded(target));
    }
    (StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response()
}

/// The sign-in provider routes under the service's own routes.
pub(super) fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sign-in/providers", get(help::offered))
        .route("/sign-in/help", get(help::contact))
        .route("/sign-in/providers/{id}", get(start))
}

/// The route the issuer's provider callback path is served at, on Lys's
/// own origin, beside the screens rather than under `/api`.
pub fn callback_routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route(super::PROVIDER_CALLBACK_PATH, get(callback))
        .with_state(state)
}

async fn begin(
    state: &AppState,
    extensions: &Extensions,
    id: &str,
    headers: &HeaderMap,
    continuation: Option<Continuation>,
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
        .begin_provider_to(&state.oidc, id, address, digest, continuation)
        .await?;
    Ok((location, cookie))
}

async fn start(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    Path(id): Path<String>,
    headers: HeaderMap,
    asked: Result<Query<Continue>, axum::extract::rejection::QueryRejection>,
) -> Response {
    let target = asked
        .map_err(|error| {
            failed(format!(
                "the provider continuation is malformed: {}",
                error.body_text()
            ))
        })
        .and_then(|Query(asked)| continuation_target(asked.continuation));
    // A continuation Lys does not accept is refused by name and never
    // carried back to the sign-in screen.
    let target = match target {
        Ok(target) => target,
        Err(error) => return to_sign_in(&error, None),
    };
    match begin(&state, &extensions, &id, &headers, target.clone()).await {
        Ok((location, cookie)) => (
            StatusCode::SEE_OTHER,
            [(header::LOCATION, location), (header::SET_COOKIE, cookie)],
        )
            .into_response(),
        Err(error) => to_sign_in(&error, target.as_ref()),
    }
}

#[derive(Deserialize)]
struct Continue {
    #[serde(rename = "continue")]
    continuation: Option<String>,
}

/// A continuation is a bounded request on the same origin, held in the
/// one-use flight rather than trusted from a callback's query or referrer.
fn continuation_target(target: Option<String>) -> Result<Option<Continuation>, ServerError> {
    target.map(Continuation::accepted).transpose()
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
) -> Result<(String, Option<Continuation>), Box<Refused>> {
    let (Some(code), Some(upstream)) = (back.code, back.state) else {
        return Err(Refused::dropping(failed("the provider did not sign the person in")).into());
    };
    let address = state
        .sign_in
        .address(extensions, headers)
        .map_err(Refused::dropping)?;
    let browser = crate::provider_browser::digest(headers).map_err(Refused::dropping)?;
    let (actor, continuation) = state
        .sign_in
        .finish_provider_to(&state.oidc, &code, &upstream, address, browser)
        .await?;
    match crate::session_admission::begin_provider(state, actor).await {
        Ok(cookie) => Ok((cookie, continuation)),
        Err(error) => Err(Refused {
            error,
            email: None,
            continuation,
        }
        .into()),
    }
}

async fn callback(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    Query(back): Query<Back>,
    headers: HeaderMap,
) -> Response {
    match finish(&state, &extensions, back, &headers).await {
        Ok((cookie, continuation)) => (
            StatusCode::SEE_OTHER,
            [
                (header::SET_COOKIE, cookie),
                (
                    header::LOCATION,
                    match continuation {
                        Some(Continuation(target)) => target,
                        None => SIGNED_IN.to_owned(),
                    },
                ),
            ],
        )
            .into_response(),
        Err(refused) => refused_to_sign_in(
            &refused.error,
            refused.continuation.as_ref(),
            refused.email.as_deref(),
        ),
    }
}

#[path = "sign_in_refusals.rs"]
mod refusals;
use refusals::{ProviderRefusal, callback_refusal};

#[path = "sign_in_help.rs"]
mod help;

#[cfg(test)]
#[path = "sign_in_upstream_tests.rs"]
mod tests;
