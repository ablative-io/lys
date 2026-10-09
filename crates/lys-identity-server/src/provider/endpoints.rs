//! The provider's endpoints: discovery, keys, authorize, the token exchange
//! and userinfo, each refusal answered in OAuth's own words.
//!
//! Locks. The provider's own two are taken in one order, codes then tokens,
//! never the other way round (`exchange.rs`). The apps store
//! (`admitted_client`, `name_granted`), the directory (`display_name`, the
//! person at authorize) and the sessions are each taken and released on their
//! own, and never under a provider guard. No lock is held while the token
//! exchange waits on the secrets broker (`client_auth`).

use super::refusal::oauth_refusal;
use super::{Grant, Kind, OpenIdProvider, encoded, held, random, unavailable};
use crate::apps_binding::sign_in_redirect;
use crate::apps_error::AppError;
use crate::error::ServerError;
use crate::error_provider::ProviderError;
use crate::routes::{AppState, cookie_header, hex, with_directory};
use crate::session::now;
use axum::extract::rejection::FormRejection;
use axum::extract::{Form, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_identity::{IdentityId, PersonId};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// The provider's routes, at Lys's own origin beside the screens.
pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/oauth/authorize", get(authorize))
        .route("/oauth/token", post(token))
        .route("/oauth/userinfo", get(userinfo))
        .route("/oauth/jwks", get(jwks))
        .route("/oauth/jwks/rotate", post(super::keys::rotate))
        .with_state(state)
}

pub(super) fn provider(state: &AppState) -> Result<&OpenIdProvider, ServerError> {
    state
        .provider
        .as_ref()
        .ok_or_else(|| unavailable("the configuration names no provider"))
}

pub(super) async fn discovery(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ServerError> {
    Ok(Json(provider(&state)?.discovery()))
}

pub(super) async fn jwks(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ServerError> {
    Ok(Json(provider(&state)?.keys(now())?))
}

/// What a product sends a person to Lys with.
#[derive(Deserialize)]
pub(super) struct Asked {
    client_id: String,
    redirect_uri: String,
    response_type: String,
    scope: Option<String>,
    state: Option<String>,
    nonce: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
}

pub(super) fn malformed(reason: &str) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.to_owned(),
    }
}

/// An approved app admitted as a client at one request: its id, and whether
/// its sign-in settings give it the person's name.
pub(super) struct Admitted {
    pub(super) app: String,
    pub(super) profile: bool,
}

/// The approved app `client_id` names, judged from the apps' record as it
/// stands at this request: its approval, `redirect` against the addresses its
/// sign-in settings list, and whether those settings give it the name. The
/// token exchange's client is authenticated before this (`client_auth`).
/// Nothing is kept between requests, so an approval, a retirement or a
/// changed setting is in force at the next one. The apps lock is taken and
/// released here, before any provider lock.
pub(super) fn admitted_client(
    state: &AppState,
    client_id: &str,
    redirect: &str,
) -> Result<Admitted, ServerError> {
    let mut apps = apps(state)?;
    apps.settle()?;
    let app = sign_in_redirect(apps.held(), client_id, redirect)?;
    Ok(Admitted {
        app: app.registered.app.clone(),
        profile: app
            .sign_in
            .as_ref()
            .is_some_and(|settings| settings.profile),
    })
}

pub(super) fn apps(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, crate::apps_store::AppStore>, ServerError> {
    state.apps.lock().map_err(|error| {
        ServerError::from(AppError::AppsUnavailable {
            reason: format!("the apps lock is poisoned: {error}"),
        })
    })
}

/// Whether the app's sign-in settings give it the person's name at this
/// request, read from the apps' record and kept nowhere.
fn name_granted(state: &AppState, app: &str) -> Result<bool, ServerError> {
    let mut apps = apps(state)?;
    apps.settle()?;
    Ok(apps
        .app(app)
        .and_then(|app| app.sign_in.as_ref())
        .is_some_and(|settings| settings.profile))
}

/// The scopes a product asks for, judged word by word: `openid` must be among
/// them, `profile` is granted only by the app's sign-in settings, and a word
/// Lys does not serve is refused by name. Answers whether the name was asked
/// for and granted.
fn scopes(asked: Option<&str>, app: &str, profile: bool) -> Result<bool, ServerError> {
    let asked = asked.unwrap_or_default();
    let mut openid = false;
    let mut name = false;
    for scope in asked.split_whitespace() {
        match scope {
            "openid" => openid = true,
            "profile" if profile => name = true,
            "profile" => {
                return Err(ServerError::Provider(ProviderError::ScopeNotGranted {
                    scope: scope.to_owned(),
                    app: app.to_owned(),
                }));
            }
            _ => {
                return Err(ServerError::Provider(ProviderError::ScopeUnknown {
                    scope: scope.to_owned(),
                }));
            }
        }
    }
    if !openid {
        return Err(malformed("a product asks for the openid scope"));
    }
    Ok(name)
}

/// The person's display name as the directory holds it now, when it holds
/// one that is not blank.
pub(super) fn display_name(
    state: &AppState,
    person: PersonId,
) -> Result<Option<String>, ServerError> {
    with_directory(state, |directory| {
        Ok(directory
            .projection()?
            .record(IdentityId::Person(person))
            .map(|record| record.profile().display_name().trim().to_owned())
            .filter(|name| !name.is_empty()))
    })
}

pub(super) async fn authorize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
    asked: Result<Query<Asked>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, ServerError> {
    let provider = provider(&state)?;
    let Query(asked) = asked.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    let admitted = admitted_client(&state, &asked.client_id, &asked.redirect_uri)?;
    if asked.response_type != "code" {
        return Err(malformed("a product asks for a code"));
    }
    let profile = scopes(asked.scope.as_deref(), &admitted.app, admitted.profile)?;
    let challenge = match (asked.code_challenge, asked.code_challenge_method.as_deref()) {
        (Some(challenge), Some("S256")) if !challenge.is_empty() => challenge,
        _ => return Err(malformed("a product signs in with a PKCE S256 challenge")),
    };
    let session = match state.sessions.session(cookie_header(&headers)) {
        Ok(session) => session,
        Err(ServerError::NotSignedIn) => {
            let back = format!("/oauth/authorize?{}", query.unwrap_or_default());
            let screen = crate::sign_in::SIGN_IN_SCREEN;
            let location = format!("{screen}?continue={}", encoded(&back));
            return Ok((StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response());
        }
        Err(error) => return Err(error),
    };
    let person = with_directory(&state, |directory| {
        crate::read_api::own_person(directory.projection()?, &session.actor)
    })?;
    let authenticated_at = session
        .actor
        .provenance()
        .authenticated_at()
        .ok_or(ServerError::NotSignedIn)?;
    let code = random::<32>()?;
    let at = now();
    {
        let mut codes = held(&provider.codes)?;
        codes.retain(|_, grant| grant.expires_at > at);
        codes.insert(
            code.clone(),
            Grant {
                session_id: session.id,
                client_id: admitted.app,
                redirect_uri: asked.redirect_uri.clone(),
                challenge,
                nonce: asked.nonce,
                subject: person.to_string(),
                person,
                profile,
                authenticated_at,
                expires_at: at.saturating_add(provider.code_seconds),
                sign_in_ends_at: session.ends_at,
                used: false,
                replayed: false,
                issued_access: None,
                issued_refresh: None,
            },
        );
    }
    let mut back = asked.redirect_uri;
    back.push(if back.contains('?') { '&' } else { '?' });
    back.push_str("code=");
    back.push_str(&encoded(&code));
    if let Some(given) = asked.state {
        back.push_str("&state=");
        back.push_str(&encoded(&given));
    }
    Ok((StatusCode::SEE_OTHER, [(header::LOCATION, back)]).into_response())
}

/// A product's token request. Only the grant type is required to read it:
/// the client is authenticated before the grant is judged, so a request for
/// any grant with a refused credential is refused as that.
#[derive(Deserialize)]
pub(super) struct Exchange {
    pub(super) grant_type: String,
    #[serde(default)]
    pub(super) code: String,
    #[serde(default)]
    pub(super) redirect_uri: String,
    #[serde(default)]
    pub(super) code_verifier: String,
    /// The refresh token a `refresh_token` grant presents (ACCESS-002 R2).
    #[serde(default)]
    pub(super) refresh_token: String,
    pub(super) client_id: Option<String>,
    pub(super) client_secret: Option<String>,
}

/// The client id and secret a token request presents, in its Basic
/// authorization or in its form.
pub(super) fn presented(headers: &HeaderMap, form: &Exchange) -> Option<(String, String)> {
    let basic = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Basic "))
        .and_then(|encoded| STANDARD.decode(encoded.trim()).ok())
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|pair| {
            pair.split_once(':')
                .map(|(id, secret)| (id.to_owned(), secret.to_owned()))
        });
    basic.or_else(|| form.client_id.clone().zip(form.client_secret.clone()))
}

pub(super) async fn token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    form: Result<Form<Exchange>, FormRejection>,
) -> Response {
    let answer = match form {
        Ok(Form(form)) => match super::client_auth::authenticated(&state, &headers, &form).await {
            Ok(client) => super::issue::exchange(&state, &form, &client),
            Err(error) => Err(error),
        },
        Err(refused) => Err(ServerError::RequestMalformed {
            reason: refused.body_text(),
        }),
    };
    match answer {
        Ok(answer) => (
            StatusCode::OK,
            [(header::CACHE_CONTROL, "no-store")],
            Json(answer),
        )
            .into_response(),
        Err(error) => oauth_refusal(&error),
    }
}

pub(super) async fn userinfo(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ServerError> {
    let provider = provider(&state)?;
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ServerError::Provider(ProviderError::TokenUnknown))?;
    let (subject, session_id, app, profile) = {
        let tokens = held(&provider.tokens)?;
        let access = tokens.get(&hex(&Sha256::digest(bearer.trim().as_bytes())), now())?;
        // A refresh token is no access token: it answers only the token route.
        if access.kind == Some(Kind::Refresh) {
            return Err(ServerError::Provider(ProviderError::TokenUnknown));
        }
        (
            access.subject.clone(),
            access.session_id.clone(),
            access.app.clone(),
            access.profile,
        )
    };
    if !state.sessions.is_live(&session_id)? {
        return Err(ServerError::Provider(ProviderError::TokenUnknown));
    }
    let mut answer = json!({ "sub": subject });
    // The name follows the setting: read again at this request, given only
    // while the token's scope asked for it and the app is still set to have it.
    if profile && name_granted(&state, &app)? {
        let person = subject.parse::<PersonId>().map_err(ServerError::Identity)?;
        if let Some(name) = display_name(&state, person)? {
            answer["name"] = Value::String(name);
        }
    }
    Ok(Json(answer))
}
