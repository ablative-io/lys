//! Password sign-in on Lys's own page. The browser posts an email and a
//! password to this service and never reaches the issuer: the service runs
//! the issuer's authorization for its own client server-side, then finishes
//! it exactly as `/callback` does, through `Oidc::finish`, so the ID token is
//! validated the same way whichever page the person used.
//!
//! The issuer's authorization is carried in the issuer's own three steps,
//! each over the issuer's loopback address: the start of the authorization,
//! which opens a session in the issuer and names its request token in the
//! page it answers; a proof-of-work challenge, which the service answers; and
//! the credentials, with the session, the request token and the answered
//! challenge, which the issuer answers with the address it would send a
//! browser to, carrying the code. That address is read here and never sent
//! to a browser.
//!
//! Every step carries the person's own address in `X-Forwarded-For`, so the
//! issuer counts failed sign-ins per person rather than for the service
//! (the issuer trusts that header from the service's address alone). A
//! request whose address the service cannot tell is refused, never sent
//! under the service's own address.
//!
//! Invariants: a wrong email and a wrong password are one refusal,
//! `SignInRefused`, whose words name neither; an account that asks for a
//! second factor is refused `SecondFactorUnsupported`, because a passkey is
//! bound to the origin the browser sees and the browser never sees the
//! issuer's; the password is never logged, never kept and never part of an
//! error.

use crate::sign_in_flights::Flights;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};

use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, State};
use axum::http::{Extensions, HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::Actor;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::admission::AUTHORITY;
use crate::config::Config;
use crate::directory_views::SignedInView;
use crate::error::ServerError;
use crate::oidc::Oidc;
use crate::routes::AppState;
use crate::sessions_api::SessionLogin;

#[path = "sign_in_upstream.rs"]
mod upstream;
pub use upstream::callback_routes;

/// The User-Agent the service names itself by to the issuer.
pub const USER_AGENT: &str = concat!("lys-identity-server/", env!("CARGO_PKG_VERSION"));

/// The header the person's own address is carried to the issuer in.
pub const FORWARDED_FOR: &str = "x-forwarded-for";

/// The header the issuer's session request token is presented in.
const REQUEST_TOKEN: &str = "x-csrf-token";

/// The template the issuer's authorization page names its request token in.
const REQUEST_TOKEN_TEMPLATE: &str = "tpl_csrf_token";

/// Where on Lys's own origin the sign-in screen is.
pub const SIGN_IN_SCREEN: &str = "/#/sign-in";

fn failed(reason: impl Into<String>) -> ServerError {
    ServerError::SignInFailed {
        reason: reason.into(),
    }
}

/// The sign-in service could not be reached; its address is never said,
/// because the refusal is read in a browser.
fn unreachable_issuer(error: reqwest::Error) -> ServerError {
    failed(format!(
        "the sign-in service could not be reached: {}",
        error.without_url()
    ))
}

/// The issuer's sign-in API, over its loopback address, and the sign-ins
/// through a provider it has begun and not finished.
pub struct IssuerSignIn {
    api: String,
    trusted_proxies: Vec<IpAddr>,
    callback: String,
    http: reqwest::Client,
    upstream: Mutex<Flights<upstream::Upstream>>,
    attempts: crate::sign_in_attempts::Attempts,
    pub(crate) setup_attempts: crate::sign_in_attempts::Attempts,
}

/// One person's sign-in: what they typed, and where they are.
pub struct Attempt<'a> {
    /// The email typed.
    pub email: &'a str,
    /// The password typed. Never printed.
    pub password: &'a str,
    /// The person's own address.
    pub address: IpAddr,
}

/// The cookies the issuer set during one sign-in, sent back to it and to
/// nothing else.
#[derive(Default)]
struct Jar(Vec<(String, String)>);

impl Jar {
    fn keep(&mut self, headers: &reqwest::header::HeaderMap) {
        for value in headers.get_all(reqwest::header::SET_COOKIE) {
            let Some((name, value)) = value
                .to_str()
                .ok()
                .and_then(|text| text.split(';').next())
                .and_then(|pair| pair.split_once('='))
            else {
                continue;
            };
            let name = name.trim().to_owned();
            self.0.retain(|(held, _)| held != &name);
            if !value.trim().is_empty() {
                self.0.push((name, value.trim().to_owned()));
            }
        }
    }

    fn header(&self) -> String {
        self.0
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// The text of the issuer page's `<template>` named `id`.
pub fn template<'a>(page: &'a str, id: &str) -> Option<&'a str> {
    let open = format!("<template id=\"{id}\">");
    let start = page.find(&open)?.checked_add(open.len())?;
    let rest = page.get(start..)?;
    let end = rest.find("</template>")?;
    rest.get(..end).map(str::trim)
}

fn leading_zero_bits(hash: &[u8]) -> u32 {
    let mut bits = 0;
    for byte in hash {
        if *byte != 0 {
            return bits + byte.leading_zeros();
        }
        bits += 8;
    }
    bits
}

/// Answer the issuer's proof-of-work `challenge`: the challenge followed by
/// the first counter whose SHA-256 over the whole begins with as many zero
/// bits as the challenge's difficulty. The challenge is
/// `version:difficulty:expires:salt:challenge:`, version 1 and the
/// difficulty two digits.
pub fn solve(challenge: &str) -> Result<String, ServerError> {
    if !challenge.starts_with("1:") || !challenge.ends_with(':') {
        return Err(failed(
            "the sign-in service's challenge is not one this service answers",
        ));
    }
    let difficulty = challenge
        .get(2..4)
        .and_then(|digits| digits.parse::<u32>().ok())
        .filter(|difficulty| (10..99).contains(difficulty))
        .ok_or_else(|| failed("the sign-in service's challenge names no difficulty"))?;
    let mut counter: u64 = 0;
    loop {
        let answer = format!("{challenge}{counter}");
        if leading_zero_bits(&Sha256::digest(answer.as_bytes())) >= difficulty {
            return Ok(answer);
        }
        counter = counter
            .checked_add(1)
            .ok_or_else(|| failed("the challenge has no answer below 2^64"))?;
    }
}

/// The value of the query pair `name` in `url`.
fn query_value(url: &reqwest::Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

/// The path the issuer sends a person back to from a sign-in provider, on
/// its public address, which is Lys's own origin.
pub const PROVIDER_CALLBACK_PATH: &str = "/auth/v1/providers/callback";

/// Lys's own origin: the origin of the service's `redirect_url`.
pub fn lys_origin(config: &Config) -> Result<String, ServerError> {
    let url = reqwest::Url::parse(&config.redirect_url)
        .map_err(|error| failed(format!("the redirect address is not one: {error}")))?;
    Ok(url.origin().ascii_serialization())
}

/// The address a sign-in provider sends a person back to: Lys's own origin,
/// the origin of the service's `redirect_url`, with the issuer's provider
/// callback path. It is the address a provider is registered with.
pub fn provider_callback(redirect_url: &str) -> Result<String, ServerError> {
    let url = reqwest::Url::parse(redirect_url)
        .map_err(|error| failed(format!("the redirect address is not one: {error}")))?;
    Ok(format!(
        "{}{PROVIDER_CALLBACK_PATH}",
        url.origin().ascii_serialization()
    ))
}

/// A session the issuer opened for one sign-in: its cookies, its request
/// token, and an answered proof-of-work challenge.
struct Opened {
    jar: Jar,
    token: String,
    pow: String,
}

/// The authorization request `begun` names, as the issuer's sign-in steps
/// carry it.
fn request_fields(begun: &reqwest::Url) -> serde_json::Map<String, Value> {
    let value = |name: &str| query_value(begun, name);
    let fields = json!({
        "client_id": value("client_id"),
        "redirect_uri": value("redirect_uri"),
        "scopes": value("scope").map(|scope| {
            scope.split(' ').map(str::to_owned).collect::<Vec<_>>()
        }),
        "state": value("state"),
        "nonce": value("nonce"),
        "code_challenge": value("code_challenge"),
        "code_challenge_method": value("code_challenge_method"),
    });
    match fields {
        Value::Object(fields) => fields,
        _ => serde_json::Map::new(),
    }
}

/// The code and state the issuer's accepting answer sends the browser back
/// with, or the answer's refusal by name.
fn accepted(answer: &reqwest::Response) -> Result<(String, String), ServerError> {
    match answer.status().as_u16() {
        202 => {
            let location = answer
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| failed("the sign-in service accepted without an answer"))?;
            let back = reqwest::Url::parse(location).map_err(|error| {
                failed(format!("the sign-in answer is not an address: {error}"))
            })?;
            let code = query_value(&back, "code")
                .ok_or_else(|| failed("the sign-in answer carries no code"))?;
            let state = query_value(&back, "state")
                .ok_or_else(|| failed("the sign-in answer carries no state"))?;
            Ok((code, state))
        }
        200 => Err(ServerError::SecondFactorUnsupported),
        429 => Err(ServerError::SignInThrottled),
        400..=499 => Err(ServerError::SignInRefused),
        other => Err(failed(format!(
            "the sign-in service answered {other} to the sign-in"
        ))),
    }
}

impl IssuerSignIn {
    /// The issuer's sign-in API at `api`, its base without a trailing
    /// slash, sending people back from a sign-in provider to `callback`.
    pub fn new(api: String, callback: String) -> Result<Self, ServerError> {
        // The issuer refuses a sign-in that names no User-Agent; the service
        // names itself.
        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(USER_AGENT)
            .build()
            .map_err(|error| failed(error.to_string()))?;
        Ok(Self {
            api,
            trusted_proxies: Vec::new(),
            callback,
            http,
            upstream: Mutex::new(Flights::default()),
            attempts: crate::sign_in_attempts::Attempts::default(),
            setup_attempts: crate::sign_in_attempts::Attempts::default(),
        })
    }

    /// The issuer's sign-in API as `config` names it.
    pub fn configured(config: &Config) -> Result<Self, ServerError> {
        let mut service = Self::new(
            config.sign_in_api(),
            provider_callback(&config.redirect_url)?,
        )?;
        service.trusted_proxies.clone_from(&config.trusted_proxies);
        Ok(service)
    }

    pub(crate) fn address(
        &self,
        extensions: &Extensions,
        headers: &HeaderMap,
    ) -> Result<IpAddr, ServerError> {
        crate::sign_in_address::resolve(person_address(extensions)?, headers, &self.trusted_proxies)
    }

    /// The address a sign-in provider sends a person back to.
    pub fn callback(&self) -> &str {
        &self.callback
    }

    /// Sign `attempt` in through the issuer for this service's own client,
    /// answering the actor its validated ID token names.
    pub async fn password(&self, oidc: &Oidc, attempt: &Attempt<'_>) -> Result<Actor, ServerError> {
        self.attempts.admit(attempt.address, 10)?;
        let begun = reqwest::Url::parse(&oidc.begin(attempt.address)?)
            .map_err(|error| failed(format!("the sign-in start is not an address: {error}")))?;
        let state = query_value(&begun, "state")
            .ok_or_else(|| failed("the sign-in start carries no state"))?;
        match self.carry(&begun, attempt).await {
            Ok((code, answered)) if answered == state => oidc.finish(code, &state).await,
            Ok(_) => {
                oidc.abandon(&state);
                Err(ServerError::SignInStateUnknown)
            }
            Err(error) => {
                oidc.abandon(&state);
                Err(error)
            }
        }
    }

    /// Open a session at the issuer for the authorization `begun` names:
    /// the start of the authorization and an answered challenge.
    async fn open(&self, begun: &reqwest::Url, address: &str) -> Result<Opened, ServerError> {
        let mut start = reqwest::Url::parse(&format!("{}/oidc/authorize", self.api))
            .map_err(|error| failed(format!("the sign-in address is not one: {error}")))?;
        start.set_query(begun.query());
        let page = self
            .http
            .get(start)
            .header(reqwest::header::ACCEPT, "text/html")
            .header(FORWARDED_FOR, address)
            .send()
            .await
            .map_err(unreachable_issuer)?;
        let mut jar = Jar::default();
        jar.keep(page.headers());
        let status = page.status();
        let body = page.text().await.map_err(unreachable_issuer)?;
        if !status.is_success() {
            return Err(failed(format!(
                "the sign-in service answered {status} to the start of a sign-in"
            )));
        }
        let token = template(&body, REQUEST_TOKEN_TEMPLATE)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| failed("the sign-in service's start named no request token"))?
            .to_owned();
        let challenge = self
            .http
            .post(format!("{}/pow", self.api))
            .header(FORWARDED_FOR, address)
            .send()
            .await
            .map_err(unreachable_issuer)?
            .text()
            .await
            .map_err(unreachable_issuer)?;
        let challenge = challenge.trim().to_owned();
        let pow = tokio::task::spawn_blocking(move || solve(&challenge))
            .await
            .map_err(|error| failed(format!("the challenge could not be answered: {error}")))??;
        Ok(Opened { jar, token, pow })
    }

    /// POST `body` to `path` at the issuer inside the session `opened`.
    async fn post_in(
        &self,
        path: &str,
        opened: &Opened,
        address: &str,
        body: &Value,
    ) -> Result<reqwest::Response, ServerError> {
        self.http
            .post(format!("{}{path}", self.api))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(reqwest::header::COOKIE, opened.jar.header())
            .header(REQUEST_TOKEN, &opened.token)
            .header(FORWARDED_FOR, address)
            .body(body.to_string())
            .send()
            .await
            .map_err(unreachable_issuer)
    }

    /// Carry the authorization `begun` names through the issuer's three
    /// steps, answering the code and the state the issuer's answer carries.
    async fn carry(
        &self,
        begun: &reqwest::Url,
        attempt: &Attempt<'_>,
    ) -> Result<(String, String), ServerError> {
        let address = attempt.address.to_string();
        let opened = self.open(begun, &address).await?;
        let mut credentials = request_fields(begun);
        credentials.insert("email".to_owned(), json!(attempt.email));
        credentials.insert("password".to_owned(), json!(attempt.password));
        credentials.insert("pow".to_owned(), json!(opened.pow));
        let answer = self
            .post_in(
                "/oidc/authorize",
                &opened,
                &address,
                &Value::Object(credentials),
            )
            .await?;
        accepted(&answer)
    }
}

/// The person's own address, as the connection to this service names it.
pub fn person_address(extensions: &Extensions) -> Result<IpAddr, ServerError> {
    extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(address)| address.ip())
        .ok_or_else(|| {
            failed("the service cannot tell the person's address, so the sign-in is not sent")
        })
}

/// What the sign-in screen posts. Never printed.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct SignInBody {
    email: String,
    password: String,
}

/// The sign-in routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", get(login))
        .route("/sign-in", post(sign_in))
        .merge(upstream::routes())
}

/// The address people are sent to sign in: Lys's own sign-in screen.
async fn login() -> Response {
    (StatusCode::SEE_OTHER, [(header::LOCATION, SIGN_IN_SCREEN)]).into_response()
}

/// Sign the person in with `actor` and answer the signed-in body with the
/// session cookie.
pub(crate) fn begin_session(state: &AppState, actor: &Actor) -> Result<Response, ServerError> {
    let body = SignedInView {
        signed_in: SessionLogin {
            issuer: actor.binding().issuer().to_owned(),
            subject: actor.binding().subject().to_owned(),
        },
        authority: AUTHORITY.to_owned(),
    };
    let cookie = crate::session_admission::begin(state, actor.clone())?;
    Ok(([(header::SET_COOKIE, cookie)], Json(body)).into_response())
}

async fn sign_in(
    State(state): State<Arc<AppState>>,
    extensions: Extensions,
    headers: HeaderMap,
    body: Result<Json<SignInBody>, JsonRejection>,
) -> Result<Response, ServerError> {
    let Json(body) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let email = body.email.trim();
    if email.is_empty() || body.password.is_empty() {
        return Err(ServerError::SignInRefused);
    }
    let attempt = Attempt {
        email,
        password: &body.password,
        address: state.sign_in.address(&extensions, &headers)?,
    };
    let actor = state.sign_in.password(&state.oidc, &attempt).await?;
    begin_session(&state, &actor)
}

#[cfg(test)]
#[path = "sign_in_tests.rs"]
mod tests;
