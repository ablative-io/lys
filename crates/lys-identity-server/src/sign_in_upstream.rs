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
    unix_second, unreachable_issuer,
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
    continuation: Option<Continuation>,
}

/// Where a sign-in continues once the person is signed in: a bounded
/// authorize request on Lys's own origin, the only target Lys accepts. It is
/// made only by [`Continuation::accepted`], so nothing else can be carried.
#[derive(Clone)]
pub(super) struct Continuation(String);

impl Continuation {
    /// `target` as a continuation, refused by name unless it is a bounded
    /// authorize request on this origin.
    fn accepted(target: String) -> Result<Self, ServerError> {
        if target.len() > 8192
            || !["/oauth/authorize?", "/oauth/mcp/authorize?"]
                .iter()
                .any(|path| target.starts_with(path))
            || !target.is_ascii()
            || target
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
            || target.contains(['#', '\\'])
        {
            return Err(failed(
                "the provider continuation is not a bounded authorize request on this origin",
            ));
        }
        Ok(Self(target))
    }
}

/// A provider sign-in refused once its flight was found: the refusal, and
/// the continuation the flight carried when it may be kept for the browser
/// that answered.
pub(super) struct Refused {
    error: ServerError,
    continuation: Option<Continuation>,
}

impl Refused {
    fn dropping(error: ServerError) -> Self {
        Self {
            error,
            continuation: None,
        }
    }
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
            Ok((actor, _)) => Ok(actor),
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
    ) -> Result<(Actor, Option<Continuation>), Refused> {
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
            return Err(Refused::dropping(ServerError::SignInStateUnknown));
        }
        let continuation = held.continuation.clone();
        match self.settle(oidc, code, upstream, address, held).await {
            Ok(actor) => Ok((actor, continuation)),
            Err(error) => Err(Refused {
                error,
                continuation,
            }),
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
    ) -> Result<Actor, ServerError> {
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
                Ok(checked_at) => accepted(answer, None, checked_at).await,
                Err(error) => Err(error),
            },
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

/// Only recognized refusal words enter the log; upstream bodies can carry
/// credentials, personal details and text that would forge another log line.
fn callback_summary(body: &[u8]) -> String {
    let Ok(value) = serde_json::from_slice::<Value>(body) else {
        return format!("body is not a JSON refusal ({} bytes)", body.len());
    };
    let error = match value.get("error").and_then(Value::as_str) {
        Some(word)
            if matches!(
                word,
                "BadRequest"
                    | "Blocked"
                    | "Connection"
                    | "CSRFTokenError"
                    | "Database"
                    | "DatabaseIo"
                    | "Disabled"
                    | "Encryption"
                    | "Forbidden"
                    | "Internal"
                    | "invalid_grant"
                    | "invalid_target"
                    | "JwtToken"
                    | "JoseError"
                    | "MfaRequired"
                    | "NoSession"
                    | "NotFound"
                    | "PasswordExpired"
                    | "PasswordRefresh"
                    | "PreconditionRequired"
                    | "Scim"
                    | "SessionExpired"
                    | "SessionTimeout"
                    | "Timeout"
                    | "Unauthorized"
                    | "NotAccepted"
            ) =>
        {
            word
        }
        _ => "unrecognized",
    };
    let message = match value.get("message").and_then(Value::as_str) {
        Some(text)
            if text.starts_with("User with email '")
                && text.ends_with("' already exists but is not linked to this provider.") =>
        {
            "existing account is not linked to this provider"
        }
        Some(
            text @ ("User not found"
            | "Invalid value for the Upstream User ID"
            | "Cannot find any user id in the response"
            | "bad provider_id in link cookie"
            | "bad user_id in link cookie"
            | "Invalid E-Mail"
            | "No `email` in ID token claims. This is a mandatory claim"
            | "Callback Code not found - timeout reached?"
            | "Neither `access_token` nor `id_token` existed"),
        ) => text,
        Some(text)
            if text.starts_with("HTTP ")
                && text.contains(" during POST ")
                && text.contains(" for upstream auth provider '") =>
        {
            "upstream token endpoint refused"
        }
        Some(_) => "unrecognized message redacted",
        None => "no string message",
    };
    format!(
        "body error={error}; message={message} ({} bytes)",
        body.len()
    )
}

/// A callback refusal keeps its status even when its body cannot be read.
async fn callback_refusal(mut answer: reqwest::Response) -> ServerError {
    const MOST: usize = 4096;
    let status = answer.status().as_u16();
    let mut body = Vec::new();
    loop {
        match answer.chunk().await {
            Ok(Some(chunk)) => {
                if chunk.len() > MOST - body.len() {
                    return failed(format!(
                        "the provider callback answered {status}; body exceeds {MOST} bytes"
                    ));
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(error) => {
                return failed(format!(
                    "the provider callback answered {status}; body read failed: {}",
                    super::unreached(error)
                ));
            }
        }
    }
    failed(format!(
        "the provider callback answered {status}; {}",
        callback_summary(&body)
    ))
}

/// Send the browser to Lys's sign-in screen naming the refusal, keeping
/// `continuation`, encoded, so signing in there continues where it was going.
fn to_sign_in(error: &ServerError, continuation: Option<&Continuation>) -> Response {
    // The browser is told only the refusal's name; the reason is written to
    // the service's log, so whoever reads it can see why.
    eprintln!("lys-identity-server provider sign-in refused: {error}");
    let mut location = format!("{SIGN_IN_SCREEN}?refused={}", error.name());
    if let Some(Continuation(target)) = continuation {
        location.push_str("&continue=");
        location.push_str(&crate::provider::encoded(target));
    }
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
    continuation: Option<String>,
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
) -> Result<(String, Option<Continuation>), Refused> {
    let (Some(code), Some(upstream)) = (back.code, back.state) else {
        return Err(Refused::dropping(failed(
            "the provider did not sign the person in",
        )));
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
    match crate::session_admission::begin(state, actor).await {
        Ok(cookie) => Ok((cookie, continuation)),
        Err(error) => Err(Refused {
            error,
            continuation,
        }),
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
        Err(refused) => to_sign_in(&refused.error, refused.continuation.as_ref()),
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use axum::http::{StatusCode, header};

    use super::{Continuation, callback_refusal, continuation_target, to_sign_in};
    use crate::error::ServerError;

    /// Where `response` sends the browser.
    fn location(response: &axum::response::Response) -> Result<String, Box<dyn Error>> {
        Ok(response
            .headers()
            .get(header::LOCATION)
            .ok_or("the refusal names no location")?
            .to_str()?
            .to_owned())
    }

    #[test]
    fn a_refused_provider_sign_in_keeps_its_continuation() -> Result<(), Box<dyn Error>> {
        let target = "/oauth/authorize?client_id=notes&redirect_uri=https%3A%2F%2Fapp.example.test%2Fcallback&state=s1";
        let Some(kept) = continuation_target(Some(target.to_owned()))? else {
            return Err("an authorize request on this origin is a continuation".into());
        };
        let response = to_sign_in(&ServerError::SignInStateUnknown, Some(&kept));
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = location(&response)?;
        let carried = location
            .strip_prefix("/#/sign-in?refused=SignInStateUnknown&continue=")
            .ok_or_else(|| format!("the refusal keeps no continuation: {location}"))?;
        assert!(
            !carried.contains(['&', '#', '?', '/']),
            "the continuation is one encoded value: {carried}"
        );
        let read_back = reqwest::Url::parse(&format!("http://lys.test/?continue={carried}"))?;
        let decoded = read_back
            .query_pairs()
            .find(|(name, _)| name == "continue")
            .map(|(_, value)| value.into_owned());
        assert_eq!(decoded.as_deref(), Some(target));
        let without = to_sign_in(&ServerError::SignInStateUnknown, None);
        assert_eq!(location(&without)?, "/#/sign-in?refused=SignInStateUnknown");
        Ok(())
    }

    #[test]
    fn an_unacceptable_continuation_is_refused_by_name_and_never_carried()
    -> Result<(), Box<dyn Error>> {
        let long = format!("/oauth/authorize?{}", "a".repeat(8192));
        for target in [
            "https://elsewhere.example/oauth/authorize?client_id=notes",
            "//elsewhere.example/oauth/authorize?client_id=notes",
            "/oauth/authorize",
            "/#/me",
            "/oauth/authorize?client_id=notes#elsewhere",
            "/oauth/authorize?client_id=notes\\elsewhere",
            "/oauth/authorize?client_id=no tes",
            "/oauth/authorize?client_id=notes\r\nLocation:%20https://elsewhere.example",
            "/oauth/authorize?client_id=n\u{f6}tes",
            long.as_str(),
        ] {
            let error = match continuation_target(Some(target.to_owned())) {
                Ok(_) => return Err(format!("{target:?} was accepted").into()),
                Err(error) => error,
            };
            assert_eq!(error.name(), "SignInFailed", "{target:?}");
            let response = to_sign_in(&error, None);
            let location = location(&response)?;
            assert_eq!(location, "/#/sign-in?refused=SignInFailed", "{target:?}");
        }
        assert!(continuation_target(None)?.is_none());
        let Continuation(kept) =
            Continuation::accepted("/oauth/mcp/authorize?client_id=a".to_owned())?;
        assert_eq!(kept, "/oauth/mcp/authorize?client_id=a");
        Ok(())
    }

    fn answer(status: u16, body: String) -> Result<reqwest::Response, axum::http::Error> {
        Ok(reqwest::Response::from(
            axum::http::Response::builder()
                .status(status)
                .body(reqwest::Body::from(body))?,
        ))
    }

    #[tokio::test]
    async fn an_existing_unlinked_account_keeps_the_status_and_safe_cause()
    -> Result<(), Box<dyn Error>> {
        let body = serde_json::json!({
            "error": "Forbidden",
            "message": "User with email 'private-address' already exists but is not linked to this provider.",
            "access_token": "private-token",
        }).to_string();
        let error = callback_refusal(answer(403, body)?).await;
        assert_eq!(error.name(), "SignInFailed");
        let words = error.to_string();
        assert!(words.contains("403"), "{words}");
        assert!(words.contains("Forbidden"), "{words}");
        assert!(
            words.contains("existing account is not linked to this provider"),
            "{words}"
        );
        assert!(!words.contains("private-address"), "{words}");
        assert!(!words.contains("private-token"), "{words}");
        Ok(())
    }

    #[tokio::test]
    async fn unknown_messages_and_error_values_never_reach_the_log() -> Result<(), Box<dyn Error>> {
        for body in [
            r#"{"error":"private-token","message":"private-token\nforged log line"}"#,
            r#"{"error":"Forbidden","message":"private-token"}"#,
            r#"{"error":{"Forbidden":"private-token"},"message":"private-token"}"#,
            "private-token\nforged log line",
        ] {
            let error = callback_refusal(answer(400, body.to_owned())?).await;
            let words = error.to_string();
            assert!(words.contains("400"), "{words}");
            assert!(!words.contains("private-token"), "{words}");
            assert!(!words.contains('\n'), "{words}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn each_callback_refusal_keeps_its_actual_status() -> Result<(), Box<dyn Error>> {
        for status in [400, 401, 403, 404, 409, 422] {
            let error = callback_refusal(answer(
                status,
                r#"{"error":"Forbidden","message":"User not found"}"#.to_owned(),
            )?)
            .await;
            let words = error.to_string();
            assert_eq!(error.name(), "SignInFailed");
            assert!(words.contains(&status.to_string()), "{words}");
            assert!(words.contains("User not found"), "{words}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn oversized_callback_bodies_are_named_without_logging_their_bytes()
    -> Result<(), Box<dyn Error>> {
        let error = callback_refusal(answer(403, "private-token".repeat(1024))?).await;
        let words = error.to_string();
        assert!(words.contains("403"), "{words}");
        assert!(words.contains("body exceeds 4096 bytes"), "{words}");
        assert!(!words.contains("private-token"), "{words}");
        Ok(())
    }
}
