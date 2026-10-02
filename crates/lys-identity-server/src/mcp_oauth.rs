//! Apps outside Lys, in a browser or on a desktop, connect to Lys's
//! MCP door the way the MCP authorization specification asks: they find the
//! door's protected-resource document, find Lys as its authorization server,
//! register themselves, and send the person to Lys to approve them. A
//! person's approval makes the app an agent that answers to that person, so
//! the app holds only the grants that agent is given and never the person's
//! own authority. Its tokens are bearer tokens for that agent alone, kept
//! only as digests, an access token for an hour and a refresh token that is
//! spent when it is used.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_identity::{AgentId, IdentityId, OperationId, Profile, Transition};
use rand::TryRngCore;
use rand::rngs::OsRng;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::mcp_oauth_store::{Issued, Kind, Store};
use crate::routes::{AppState, with_directory};
use crate::session::now;

const ACCESS_SECONDS: u64 = 3600;
const REFRESH_SECONDS: u64 = 30 * 24 * 3600;
const CODE_SECONDS: u64 = 120;
const APPROVAL_SECONDS: u64 = 600;
const APPS_MAX: usize = 1000;
const NAME_MAX: usize = 64;
/// The words a connected app is known by when it gave no name.
const UNNAMED: &str = "Connected app";
/// How a connected app's acts are told apart from a run Lys started.
pub(crate) const LAUNCH: &str = "connected-app";

/// An approval waiting for the person's answer.
struct Asking {
    session_id: String,
    client_id: String,
    redirect_uri: String,
    challenge: String,
    state: Option<String>,
    expires_at: u64,
}

/// A code an approval answered with, exchanged once for tokens.
struct Code {
    agent: String,
    client_id: String,
    redirect_uri: String,
    challenge: String,
    expires_at: u64,
}

/// The connected-apps door.
pub(crate) struct Apps {
    state: Arc<AppState>,
    origin: String,
    resource: String,
    store: Mutex<Store>,
    asking: Mutex<HashMap<String, Asking>>,
    codes: Mutex<HashMap<String, Code>>,
}

fn held<T>(lock: &Mutex<T>) -> Result<MutexGuard<'_, T>, ServerError> {
    lock.lock().map_err(|error| ServerError::ConfigInvalid {
        reason: format!("the connected-apps door is unavailable: {error}"),
    })
}

fn malformed(reason: &str) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.to_owned(),
    }
}

fn random() -> Result<String, ServerError> {
    let mut bytes = [0_u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| ServerError::ConfigInvalid {
            reason: format!("no randomness for a connected-app token: {error}"),
        })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn digest(token: &str) -> String {
    crate::routes::hex(&Sha256::digest(token.as_bytes()))
}

/// Percent-encode everything but the unreserved characters.
fn encoded(text: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(char::from(DIGITS[usize::from(byte >> 4)]));
            out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
    }
    out
}

fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// An app's redirect address: https anywhere, or http on this machine only.
fn redirect_allowed(uri: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(uri) else {
        return false;
    };
    if url.fragment().is_some() || uri.len() > 2048 {
        return false;
    }
    match url.scheme() {
        "https" => url.host_str().is_some(),
        "http" => matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")),
        _ => false,
    }
}

fn back_to(redirect_uri: &str, pairs: &[(&str, &str)]) -> Result<Response, ServerError> {
    let mut url = reqwest::Url::parse(redirect_uri).map_err(|_unread| ServerError::RedirectUnregistered)?;
    {
        let mut query = url.query_pairs_mut();
        for (name, value) in pairs {
            query.append_pair(name, value);
        }
    }
    Ok((StatusCode::SEE_OTHER, [(header::LOCATION, url.to_string())]).into_response())
}

impl Apps {
    /// The door for the install whose directory log is at `log_dir`, whose
    /// origin is `origin` and whose MCP door answers at `mcp_path`.
    pub(crate) fn open(
        state: Arc<AppState>,
        log_dir: &std::path::Path,
        origin: &str,
        mcp_path: &str,
    ) -> Result<Arc<Self>, ServerError> {
        let folder = log_dir.parent().ok_or_else(|| ServerError::ConfigInvalid {
            reason: "the directory log has no containing folder for connected apps".to_owned(),
        })?;
        Ok(Arc::new(Self {
            state,
            origin: origin.to_owned(),
            resource: format!("{origin}{mcp_path}"),
            store: Mutex::new(Store::open(&folder.join("connected-apps.json"))?),
            asking: Mutex::default(),
            codes: Mutex::default(),
        }))
    }

    /// Where an app finds the door's protected-resource document.
    pub(crate) fn metadata_address(&self) -> String {
        format!("{}/.well-known/oauth-protected-resource", self.origin)
    }

    /// The agent and app a connected app's bearer token names; none when the
    /// request carries no bearer token. A bearer token that is not a live
    /// connected-app access token is refused.
    pub(crate) fn bearer(&self, headers: &HeaderMap) -> Result<Option<(AgentId, String)>, ServerError> {
        let Some(value) = headers.get(header::AUTHORIZATION) else {
            return Ok(None);
        };
        let text = value.to_str().map_err(|_unread| ServerError::TokenUnknown)?;
        let Some(token) = text.strip_prefix("Bearer ") else {
            return Ok(None);
        };
        let store = held(&self.store)?;
        let issued = store
            .token(&digest(token.trim()), Kind::Access, now())
            .ok_or(ServerError::TokenUnknown)?;
        let agent = AgentId::from_str(&issued.agent).map_err(|_unread| ServerError::TokenUnknown)?;
        Ok(Some((agent, issued.client_id.clone())))
    }

    fn issue(
        &self,
        agent: &str,
        client_id: &str,
        spent: Option<&str>,
    ) -> Result<Response, ServerError> {
        let access = random()?;
        let refresh = random()?;
        let at = now();
        let made = |kind, seconds| Issued {
            agent: agent.to_owned(),
            client_id: client_id.to_owned(),
            kind,
            expires_at: at.saturating_add(seconds),
        };
        held(&self.store)?.issue(
            vec![
                (digest(&access), made(Kind::Access, ACCESS_SECONDS)),
                (digest(&refresh), made(Kind::Refresh, REFRESH_SECONDS)),
            ],
            spent,
            at,
        )?;
        Ok((
            [(header::CACHE_CONTROL, "no-store")],
            Json(json!({
                "access_token": access,
                "token_type": "Bearer",
                "expires_in": ACCESS_SECONDS,
                "refresh_token": refresh,
            })),
        )
            .into_response())
    }

    /// The agent an approval makes for the app, or the one an earlier
    /// approval by the same person made.
    fn agent_for(&self, client_id: &str, name: &str, actor: &lys_identity::Actor) -> Result<String, ServerError> {
        let person = with_directory(&self.state, |directory| {
            crate::read_api::own_person(directory.projection()?, actor)
        })?;
        let person_key = person.to_string();
        if let Some(agent) = held(&self.store)?.connection(client_id, &person_key) {
            return Ok(agent.to_owned());
        }
        let agent = with_directory(&self.state, |directory| {
            let at = now();
            let (agent, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                Profile::new(name)?,
                at,
            )?;
            directory.transition(
                actor.clone(),
                OperationId::generate()?,
                IdentityId::Agent(agent),
                Transition::Activate,
                "",
                at,
            )?;
            Ok(agent.to_string())
        })?;
        held(&self.store)?.connect(client_id, &person_key, agent.clone())?;
        Ok(agent)
    }
}

/// The door's routes, served at the origin's root.
pub(crate) fn routes(apps: Arc<Apps>) -> Router {
    Router::new()
        .route("/.well-known/oauth-protected-resource", get(resource))
        .route("/.well-known/oauth-protected-resource/mcp", get(resource))
        .route("/.well-known/oauth-protected-resource/api/mcp", get(resource))
        .route("/.well-known/oauth-authorization-server", get(server))
        .route("/oauth/mcp/register", post(register))
        .route("/oauth/mcp/authorize", get(authorize))
        .route("/oauth/mcp/consent", post(consent))
        .route("/oauth/mcp/token", post(token))
        .with_state(apps)
}

/// `router` with every refusal to an unauthenticated caller naming where
/// the door's protected-resource document is.
pub(crate) fn challenged(router: Router, metadata: String) -> Router {
    router.layer(axum::middleware::map_response(move |mut response: Response| {
        let metadata = metadata.clone();
        async move {
            if response.status() == StatusCode::UNAUTHORIZED
                && let Ok(value) = format!("Bearer resource_metadata=\"{metadata}\"").parse()
            {
                response.headers_mut().insert(header::WWW_AUTHENTICATE, value);
            }
            response
        }
    }))
}

async fn resource(State(apps): State<Arc<Apps>>) -> Json<serde_json::Value> {
    Json(json!({
        "resource": apps.resource,
        "authorization_servers": [apps.origin],
        "bearer_methods_supported": ["header"],
        "resource_name": "Lys",
    }))
}

async fn server(State(apps): State<Arc<Apps>>) -> Json<serde_json::Value> {
    let origin = &apps.origin;
    Json(json!({
        "issuer": origin,
        "authorization_endpoint": format!("{origin}/oauth/mcp/authorize"),
        "token_endpoint": format!("{origin}/oauth/mcp/token"),
        "registration_endpoint": format!("{origin}/oauth/mcp/register"),
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
    }))
}

#[path = "mcp_oauth_flow.rs"]
mod flow;
use flow::{authorize, consent, register, token};

#[cfg(test)]
#[path = "mcp_oauth_tests.rs"]
mod tests;
