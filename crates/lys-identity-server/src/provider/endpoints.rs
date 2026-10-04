//! The provider's endpoints: discovery, keys, authorize, the token exchange
//! and userinfo, each refusal answered in OAuth's own words.

use super::{
    Access, Grant, ID_TOKEN_SECONDS, OpenIdProvider, encoded, held, random, same, unavailable,
};
use crate::error::ServerError;
use crate::routes::{AppState, cookie_header, hex, with_directory};
use crate::session::now;
use axum::extract::rejection::FormRejection;
use axum::extract::{Form, Query, RawQuery, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
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
    Ok(Json(provider(&state)?.keys()))
}

/// What a product sends a person to Lys with.
#[derive(Deserialize)]
pub(super) struct Asked {
    client_id: String,
    redirect_uri: String,
    response_type: String,
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
    let client = provider.client(&asked.client_id)?;
    if !client.redirect_uris.contains(&asked.redirect_uri) {
        return Err(ServerError::RedirectUnregistered);
    }
    if asked.response_type != "code" {
        return Err(malformed("a product asks for a code"));
    }
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
                client_id: client.client_id.clone(),
                redirect_uri: asked.redirect_uri.clone(),
                challenge,
                nonce: asked.nonce,
                subject: person.to_string(),
                authenticated_at,
                expires_at: at.saturating_add(provider.code_seconds),
                sign_in_ends_at: session.ends_at,
                used: false,
                issued_access: None,
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

/// A product's token request.
#[derive(Deserialize)]
pub(super) struct Exchange {
    pub(super) grant_type: String,
    pub(super) code: String,
    pub(super) redirect_uri: String,
    pub(super) code_verifier: String,
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

/// An OAuth error answer carrying the refusal by name.
pub(super) fn oauth_refusal(error: &ServerError) -> Response {
    let code = match error {
        ServerError::ClientUnknown => "invalid_client",
        ServerError::CodeUsed
        | ServerError::CodeExpired
        | ServerError::CodeUnknown
        | ServerError::VerifierWrong
        | ServerError::RedirectUnregistered => "invalid_grant",
        ServerError::RequestMalformed { .. }
        | ServerError::BodyTooLarge
        | ServerError::Holding(..) => "invalid_request",
        ServerError::Team(..)
        | ServerError::Budget(..)
        | ServerError::HarnessCatalogueUnreadable { .. }
        | ServerError::Identity(..)
        | ServerError::Grant(..)
        | ServerError::App(..)
        | ServerError::Goal(..)
        | ServerError::Inactive { .. }
        | ServerError::NotSignedIn
        | ServerError::NotAdmitted { .. }
        | ServerError::NoPerson
        | ServerError::SetupRequired
        | ServerError::AgentNotVisible
        | ServerError::GrantNotVisible
        | ServerError::Withheld { .. }
        | ServerError::SessionUnknown
        | ServerError::SignInStateUnknown
        | ServerError::SignInRefused
        | ServerError::SignInThrottled
        | ServerError::RegistrationThrottled
        | ServerError::SecondFactorUnsupported
        | ServerError::SetupClosed
        | ServerError::SetupCodeRefused
        | ServerError::SetupUnavailable { .. }
        | ServerError::AccountRefused { .. }
        | ServerError::SignInFailed { .. }
        | ServerError::ConfigInvalid { .. }
        | ServerError::BootstrapInterrupted { .. }
        | ServerError::SecretsUnavailable { .. }
        | ServerError::SecretsRefused { .. }
        | ServerError::RequestsUnavailable { .. }
        | ServerError::McpRequestsUnavailable { .. }
        | ServerError::McpServerUnknown { .. }
        | ServerError::McpServerHeld { .. }
        | ServerError::McpBeyondRemit { .. }
        | ServerError::RequestUnknown
        | ServerError::RequestDecided { .. }
        | ServerError::RequestHeld { .. }
        | ServerError::RequestReused { .. }
        | ServerError::NetworkUnavailable { .. }
        | ServerError::LoginUnbound
        | ServerError::MachineUnknown
        | ServerError::Machine(..)
        | ServerError::Cord(..)
        | ServerError::SessionsUnavailable { .. }
        | ServerError::MemoryUnavailable { .. }
        | ServerError::ProvisioningUnavailable { .. }
        | ServerError::ProvisioningChanged { .. }
        | ServerError::ProvisioningReused { .. }
        | ServerError::ProfileVersionUnknown { .. }
        | ServerError::ProfileVersionReplaced { .. }
        | ServerError::ProfileNotReviewed { .. }
        | ServerError::CertificatesUnavailable { .. }
        | ServerError::CertificateUnknown { .. }
        | ServerError::CertificateReused { .. }
        | ServerError::CertificateWithdrawn { .. }
        | ServerError::RolesUnavailable { .. }
        | ServerError::RoleUnknown
        | ServerError::RoleVersionUnknown
        | ServerError::HolderUnknown
        | ServerError::RoleReused { .. }
        | ServerError::RoleHeld { .. }
        | ServerError::HoldingOver { .. }
        | ServerError::HoldingChanged
        | ServerError::LaunchRecordMissing
        | ServerError::OperatorRefused { .. }
        | ServerError::AgentPassRefused { .. }
        | ServerError::AgentSignatureRefused { .. }
        | ServerError::AgentNotActive { .. }
        | ServerError::MachineCannotReach { .. }
        | ServerError::MachineRetired
        | ServerError::MachineNotForAgent
        | ServerError::MachineWithoutRuntime
        | ServerError::MachineWithoutRunner
        | ServerError::SkillUnknown { .. }
        | ServerError::PolicyUnrepresentable { .. }
        | ServerError::ModelUnrepresentable { .. }
        | ServerError::HarnessUndeclared { .. }
        | ServerError::LaunchUnrenderable { .. }
        | ServerError::WorkingFolderUnnamed
        | ServerError::McpCredentialInline { .. }
        | ServerError::McpSettingUnrepresentable { .. }
        | ServerError::McpHandleUnsupported { .. }
        | ServerError::RuntimeUnavailable { .. }
        | ServerError::RuntimeSessionUnknown
        | ServerError::RuntimeSessionStarted { .. }
        | ServerError::RuntimeSessionStopped { .. }
        | ServerError::RuntimeReportReused { .. }
        | ServerError::ServiceAccountsUnavailable { .. }
        | ServerError::ServiceAccountUnknown
        | ServerError::ServiceAccountReused { .. }
        | ServerError::ServiceAccountRetired { .. }
        | ServerError::ServiceAccountOwnerRetired { .. }
        | ServerError::PolicyUnavailable { .. }
        | ServerError::AgentHasNoPolicy { .. }
        | ServerError::PolicyVersionConflict { .. }
        | ServerError::PolicyRefused { .. }
        | ServerError::StopsUnavailable { .. }
        | ServerError::StopReused { .. }
        | ServerError::ReviewsUnavailable { .. }
        | ServerError::ReviewerOnly
        | ServerError::GrantNotDue { .. }
        | ServerError::ReviewReused { .. }
        | ServerError::DirectoryUnavailable { .. }
        | ServerError::SignInProvidersUnavailable { .. }
        | ServerError::ProviderUnavailable { .. }
        | ServerError::TokenUnknown
        | ServerError::ProviderRefused { .. }
        | ServerError::SignInProvidersRefused { .. }
        | ServerError::NotPermitted { .. }
        | ServerError::NoLiveSession { .. }
        | ServerError::RunnerAbsent { .. }
        | ServerError::Runner { .. }
        | ServerError::DialRefused { .. }
        | ServerError::DialStale { .. } => "server_error",
    };
    let body = json!({
        "error": code,
        "error_description": error.to_string(),
        "refusal": error.name(),
        "reason": error.to_string(),
    });
    (
        error.status(),
        [(header::CACHE_CONTROL, "no-store")],
        Json(body),
    )
        .into_response()
}

pub(super) async fn token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    form: Result<Form<Exchange>, FormRejection>,
) -> Response {
    match exchange(&state, &headers, form) {
        Ok(answer) => (
            StatusCode::OK,
            [(header::CACHE_CONTROL, "no-store")],
            Json(answer),
        )
            .into_response(),
        Err(error) => oauth_refusal(&error),
    }
}

pub(super) fn exchange(
    state: &AppState,
    headers: &HeaderMap,
    form: Result<Form<Exchange>, FormRejection>,
) -> Result<Value, ServerError> {
    let provider = provider(state)?;
    let Form(form) = form.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    if form.grant_type != "authorization_code" {
        return Err(malformed("a product exchanges an authorization code"));
    }
    let (client_id, secret) = presented(headers, &form).ok_or(ServerError::ClientUnknown)?;
    let client = provider.authenticated(&client_id, &secret)?;
    let at = now();
    let mut codes = held(&provider.codes)?;
    let grant = codes.get_mut(&form.code).ok_or(ServerError::CodeUnknown)?;
    let (subject, nonce, authenticated_at, expires_at, session_id) = {
        if grant.client_id != client.client_id {
            return Err(ServerError::CodeUnknown);
        }
        if grant.used {
            if let Some(key) = &grant.issued_access {
                held(&provider.tokens)?.revoke(key)?;
            }
            return Err(ServerError::CodeUsed);
        }
        if at >= grant.expires_at || at >= grant.sign_in_ends_at {
            return Err(ServerError::CodeExpired);
        }
        if grant.redirect_uri != form.redirect_uri {
            return Err(ServerError::RedirectUnregistered);
        }
        grant.used = true;
        let verified = URL_SAFE_NO_PAD.encode(Sha256::digest(form.code_verifier.as_bytes()));
        if !same(&verified, &grant.challenge) {
            return Err(ServerError::VerifierWrong);
        }
        (
            grant.subject.clone(),
            grant.nonce.clone(),
            grant.authenticated_at,
            grant.sign_in_ends_at,
            grant.session_id.clone(),
        )
    };
    if !state.sessions.is_live(&session_id)? {
        return Err(ServerError::CodeExpired);
    }
    let mut claims = json!({
        "iss": provider.issuer,
        "sub": subject,
        "aud": client.client_id,
        "iat": at,
        "exp": expires_at.min(at.saturating_add(ID_TOKEN_SECONDS)),
        "auth_time": authenticated_at,
    });
    if let Some(nonce) = nonce {
        claims["nonce"] = Value::String(nonce);
    }
    let access = random::<32>()?;
    let lookup = hex(&Sha256::digest(access.as_bytes()));
    {
        let mut tokens = held(&provider.tokens)?;
        tokens.insert(
            lookup.clone(),
            Access {
                session_id,
                subject,
                expires_at,
            },
            at,
        )?;
    }
    grant.issued_access = Some(lookup);
    Ok(json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_in": expires_at.saturating_sub(at),
        "id_token": provider.signed(&claims),
    }))
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
        .ok_or(ServerError::TokenUnknown)?;
    let (subject, session_id) = {
        let tokens = held(&provider.tokens)?;
        let access = tokens.get(&hex(&Sha256::digest(bearer.trim().as_bytes())), now())?;
        (access.subject.clone(), access.session_id.clone())
    };
    if !state.sessions.is_live(&session_id)? {
        return Err(ServerError::TokenUnknown);
    }
    Ok(Json(json!({ "sub": subject })))
}
