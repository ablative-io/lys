//! The connected-apps door's four steps: an app registers, the person is
//! asked, the person answers, and the app exchanges the answer for tokens.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use super::consent::{Mode, answer};
use super::{
    APPROVAL_SECONDS, Apps, Asking, CODE_SECONDS, Code, REGISTRATION, UNNAMED, digest,
    encoded, held, malformed, random, redirect_allowed,
};
use crate::error::ServerError;
use crate::mcp_oauth_store::{App, Kind, PersonReads};
use crate::routes::cookie_header;
use crate::session::now;

#[derive(Deserialize)]
pub(super) struct Registration {
    client_name: Option<String>,
    redirect_uris: Vec<String>,
}

pub(super) async fn register(
    State(apps): State<Arc<Apps>>,
    peer: Option<axum::extract::Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    body: Result<Json<Registration>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, ServerError> {
    let Json(asked) = body.map_err(|refused| malformed(&refused.body_text()))?;
    if asked.redirect_uris.is_empty()
        || !asked.redirect_uris.iter().all(|uri| redirect_allowed(uri))
    {
        return Err(malformed(
            "an app registers at least one redirect address, https or http on this machine",
        ));
    }
    let name: String = asked
        .client_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(UNNAMED)
        .chars()
        .filter(|character| !character.is_control())
        .collect();
    let client_id = random()?;
    let issued_at = now();
    {
        held(&apps.store)?.register(
            client_id.clone(),
            App {
                name: name.clone(),
                redirect_uris: asked.redirect_uris.clone(),
                registered_at: issued_at,
                client_host: Some(super::return_page::address(&apps, peer, &headers)?),
            },
            REGISTRATION,
        )?;
    }
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "client_id": client_id,
            "client_id_issued_at": issued_at,
            "client_name": name,
            "redirect_uris": asked.redirect_uris,
            "token_endpoint_auth_method": "none",
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
        })),
    )
        .into_response())
}

#[derive(Deserialize)]
pub(super) struct Asked {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    state: Option<String>,
    resource: Option<String>,
}

pub(super) async fn authorize(
    State(apps): State<Arc<Apps>>,
    headers: HeaderMap,
    peer: Option<axum::extract::Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    asked: Result<Query<Asked>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, ServerError> {
    let Query(asked) = asked.map_err(|refused| malformed(&refused.body_text()))?;
    let name = super::native::known(&apps, &asked.client_id, &asked.redirect_uri)?;
    let native = asked.client_id == super::native::CLIENT;
    if asked.response_type != "code" {
        return Err(malformed("an app asks for a code"));
    }
    if asked
        .resource
        .as_ref()
        .is_some_and(|resource| *resource != apps.resource && !native)
    {
        return Err(malformed("an app asks only for this install's MCP door"));
    }
    let challenge = match (asked.code_challenge, asked.code_challenge_method.as_deref()) {
        (Some(challenge), Some("S256")) if challenge.len() == 43 => challenge,
        _ => return Err(malformed("an app connects with a PKCE S256 challenge")),
    };
    let session = match apps.state.sessions.session(cookie_header(&headers)) {
        Ok(session) => session,
        Err(ServerError::NotSignedIn) => {
            let back = format!("/oauth/mcp/authorize?{}", query.unwrap_or_default());
            let location = format!(
                "{}?continue={}",
                crate::sign_in::SIGN_IN_SCREEN,
                encoded(&back)
            );
            return Ok((StatusCode::SEE_OTHER, [(header::LOCATION, location)]).into_response());
        }
        Err(error) => return Err(error),
    };
    let session_actor = session.actor;
    let person = crate::routes::with_directory(&apps.state, |directory| {
        crate::read_api::own_person(directory.projection()?, &session_actor)
    })?;
    let asking = random()?;
    let at = now();
    {
        let mut held_asking = held(&apps.asking)?;
        held_asking.retain(|_, waiting| waiting.expires_at > at);
        held_asking.insert(
            asking.clone(),
            Asking {
                person,
                client_id: asked.client_id,
                redirect_uri: asked.redirect_uri.clone(),
                challenge,
                state: asked.state,
                expires_at: at.saturating_add(APPROVAL_SECONDS),
                client_host: Some(super::return_page::address(&apps, peer, &headers)?),
            },
        );
    }
    if native {
        return Ok(([(header::CACHE_CONTROL, "no-store"), (header::X_FRAME_OPTIONS, "DENY"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; form-action 'self'")],
            Html(format!("<!doctype html><title>Sign in to Lys CLI</title><h1>Sign in to Lys CLI</h1><p>Your terminal will act as you in Lys. Ending this sign-in ends its authority.</p><form method=\"post\" action=\"/oauth/mcp/consent\"><input type=\"hidden\" name=\"asking\" value=\"{asking}\"><button name=\"decision\" value=\"approve\">Sign in to Lys CLI</button><button name=\"decision\" value=\"refuse\">Cancel</button></form>"))).into_response());
    }
    super::screen::consent_screen(&apps, person, &asking, &name, &asked.redirect_uri)
}

pub(super) async fn consent(
    State(apps): State<Arc<Apps>>,
    headers: HeaderMap,
    peer: Option<axum::extract::Extension<axum::extract::ConnectInfo<std::net::SocketAddr>>>,
    body: axum::body::Bytes,
) -> Result<Response, ServerError> {
    let answer = answer(&body)?;
    if let Some(origin) = headers.get(header::ORIGIN)
        && origin.to_str().ok() != Some(apps.origin.as_str())
    {
        return Err(malformed("an approval is answered from Lys's own page"));
    }
    let session = apps.state.sessions.session(cookie_header(&headers))?;
    let asking = match super::consent::take(&apps, &answer.asking, &session)? {
        super::consent::Approval::Ready(asking) => asking,
        super::consent::Approval::Refused(response) => return Ok(response),
    };
    let state = asking.state.clone().unwrap_or_default();
    let with_state = |mut pairs: Vec<(&'static str, String)>| {
        if asking.state.is_some() {
            pairs.push(("state", state.clone()));
        }
        pairs
    };
    let mut expires_at = None;
    let pairs = match answer.decision.as_str() {
        "refuse" => with_state(vec![("error", "access_denied".to_owned())]),
        "approve" => {
            let name = super::native::known(&apps, &asking.client_id, &asking.redirect_uri)?;
            let person = crate::routes::with_directory(&apps.state, |directory| {
                crate::read_api::own_person(directory.projection()?, &session.actor)
            })?;
            let person_reads = if answer.mode == Mode::PersonReads {
                if !answer.grants.is_empty() || asking.client_id == super::native::CLIENT {
                    return Err(malformed(
                        "Person reads is an app's read-only choice, without agent permissions",
                    ));
                }
                if session.actor.provenance().method() != lys_identity::AuthMethod::Oidc {
                    return Err(malformed("Person reads needs your browser sign-in"));
                }
                Some(PersonReads {
                    person: person.to_string(),
                    issuer: session.actor.binding().issuer().to_owned(),
                    subject: session.actor.binding().subject().to_owned(),
                    authenticated_at: session.actor.provenance().authenticated_at().ok_or_else(
                        || malformed("Person reads needs your browser sign-in instant"),
                    )?,
                })
            } else {
                None
            };
            let agent = if asking.client_id == super::native::CLIENT {
                if !answer.grants.is_empty() {
                    return Err(malformed("the native client uses its person's authority"));
                }
                format!("cli-session:{}", session.id)
            } else {
                let picked = crate::mcp_oauth_grants::chosen(&apps.state, person, &answer.grants)?;
                let agent = apps.agent_for(&asking.client_id, &name, &session.actor)?;
                if person_reads.is_none() {
                    crate::mcp_oauth_grants::pass_on(&apps.state, person, &agent, &picked)?;
                }
                agent
            };
            let code = random()?;
            let at = apps.state.sessions.now()?;
            expires_at = Some(at.saturating_add(CODE_SECONDS));
            {
                let mut codes = held(&apps.codes)?;
                codes.retain(|_, held_code| held_code.expires_at > at);
                codes.insert(
                    code.clone(),
                    Code {
                        agent,
                        client_id: asking.client_id.clone(),
                        redirect_uri: asking.redirect_uri.clone(),
                        challenge: asking.challenge.clone(),
                        expires_at: at.saturating_add(CODE_SECONDS),
                        person_reads,
                    },
                );
            }
            with_state(vec![("code", code)])
        }
        _ => return Err(malformed("an approval is answered approve or refuse")),
    };
    let pairs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect();
    super::return_page::finish(&apps, &asking, super::return_page::address(&apps, peer, &headers)?, &pairs, expires_at)
}

#[derive(Deserialize)]
pub(super) struct Exchange {
    grant_type: String,
    client_id: String,
    code: Option<String>,
    code_verifier: Option<String>,
    redirect_uri: Option<String>,
    refresh_token: Option<String>,
}

/// An OAuth token refusal, in the shape every OAuth client reads.
fn refused(error: &str, description: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        [(header::CACHE_CONTROL, "no-store")],
        Json(json!({"error": error, "error_description": description})),
    )
        .into_response()
}

pub(super) async fn token(
    State(apps): State<Arc<Apps>>,
    form: Result<Form<Exchange>, axum::extract::rejection::FormRejection>,
) -> Response {
    let Ok(Form(asked)) = form else {
        return refused("invalid_request", "a token request is a form");
    };
    let issued = match asked.grant_type.as_str() {
        "authorization_code" => exchange(&apps, &asked),
        "refresh_token" => refresh(&apps, &asked),
        _ => {
            return refused(
                "unsupported_grant_type",
                "authorization_code or refresh_token",
            );
        }
    };
    match issued {
        Ok(response) => response,
        Err(Refusal::Grant(description)) => refused("invalid_grant", description),
        Err(Refusal::Server(error)) => {
            tracing::error!("a connected app's token could not be issued: {error}");
            error.into_response()
        }
    }
}

enum Refusal {
    Grant(&'static str),
    Server(ServerError),
}

impl From<ServerError> for Refusal {
    fn from(error: ServerError) -> Self {
        Self::Server(error)
    }
}

fn exchange(apps: &Apps, asked: &Exchange) -> Result<Response, Refusal> {
    let (Some(code), Some(verifier), Some(redirect_uri)) =
        (&asked.code, &asked.code_verifier, &asked.redirect_uri)
    else {
        return Err(Refusal::Grant(
            "a code exchange carries its code, verifier and redirect address",
        ));
    };
    let held_code = held(&apps.codes)?
        .remove(code)
        .ok_or(Refusal::Grant("the code is unknown or was used"))?;
    if held_code.expires_at <= now() {
        return Err(Refusal::Grant("the code is past its instant"));
    }
    if held_code.client_id != asked.client_id || held_code.redirect_uri != *redirect_uri {
        return Err(Refusal::Grant(
            "the code was not given to this app at this address",
        ));
    }
    if URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes())) != held_code.challenge {
        return Err(Refusal::Grant("the verifier does not answer the challenge"));
    }
    Ok(apps.issue(
        &held_code.agent,
        &held_code.client_id,
        None,
        held_code.person_reads.as_ref(),
    )?)
}

fn refresh(apps: &Apps, asked: &Exchange) -> Result<Response, Refusal> {
    let Some(token) = &asked.refresh_token else {
        return Err(Refusal::Grant("a refresh carries its refresh token"));
    };
    let spent = digest(token);
    let (agent, person_reads) = {
        let store = held(&apps.store)?;
        let issued = store
            .token(&spent, Kind::Refresh, now())
            .ok_or(Refusal::Grant(
                "the refresh token is unknown, used or past its instant",
            ))?;
        if issued.client_id != asked.client_id {
            return Err(Refusal::Grant(
                "the refresh token was not given to this app",
            ));
        }
        (issued.agent.clone(), issued.person_reads.clone())
    };
    Ok(apps.issue(
        &agent,
        &asked.client_id,
        Some(&spent),
        person_reads.as_ref(),
    )?)
}

#[cfg(test)]
#[path = "mcp_oauth_native_tests.rs"]
mod native_tests;
