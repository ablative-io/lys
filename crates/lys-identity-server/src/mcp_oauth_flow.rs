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

use super::{
    APPROVAL_SECONDS, Apps, Asking, CODE_SECONDS, Code, NAME_MAX, REGISTRATION, UNNAMED, back_to,
    digest, encoded, escaped, held, malformed, random, redirect_allowed,
};
use crate::error::ServerError;
use crate::mcp_oauth_store::{App, Kind};
use crate::routes::cookie_header;
use crate::session::now;

#[derive(Deserialize)]
pub(super) struct Registration {
    client_name: Option<String>,
    redirect_uris: Vec<String>,
}

pub(super) async fn register(
    State(apps): State<Arc<Apps>>,
    body: Result<Json<Registration>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, ServerError> {
    let Json(asked) = body.map_err(|refused| malformed(&refused.body_text()))?;
    if asked.redirect_uris.is_empty()
        || asked.redirect_uris.len() > 10
        || !asked.redirect_uris.iter().all(|uri| redirect_allowed(uri))
    {
        return Err(malformed(
            "an app registers 1 to 10 redirect addresses, https or http on this machine",
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
        .take(NAME_MAX)
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
    axum::extract::RawQuery(query): axum::extract::RawQuery,
    asked: Result<Query<Asked>, axum::extract::rejection::QueryRejection>,
) -> Result<Response, ServerError> {
    let Query(asked) = asked.map_err(|refused| malformed(&refused.body_text()))?;
    let name = {
        let store = held(&apps.store)?;
        let app = store
            .app(&asked.client_id)
            .ok_or(ServerError::RedirectUnregistered)?;
        if !app.redirect_uris.contains(&asked.redirect_uri) {
            return Err(ServerError::RedirectUnregistered);
        }
        app.name.clone()
    };
    if asked.response_type != "code" {
        return Err(malformed("an app asks for a code"));
    }
    if asked
        .resource
        .as_ref()
        .is_some_and(|resource| *resource != apps.resource)
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
    let session_actor = session.actor.clone();
    let asking = random()?;
    let at = now();
    {
        let mut held_asking = held(&apps.asking)?;
        held_asking.retain(|_, waiting| waiting.expires_at > at);
        held_asking.insert(
            asking.clone(),
            Asking {
                session_id: session.id,
                client_id: asked.client_id,
                redirect_uri: asked.redirect_uri.clone(),
                challenge,
                state: asked.state,
                expires_at: at.saturating_add(APPROVAL_SECONDS),
            },
        );
    }
    let person = crate::routes::with_directory(&apps.state, |directory| {
        crate::read_api::own_person(directory.projection()?, &session_actor)
    })?;
    let mut choices = String::new();
    for offer in crate::mcp_oauth_grants::offers(&apps.state, person)? {
        choices.push_str("<label><input type=\"checkbox\" name=\"grant\" value=\"");
        choices.push_str(&escaped(&offer.key));
        choices.push_str("\"> ");
        choices.push_str(&escaped(&offer.words()));
        choices.push_str("</label><br>");
    }
    let choices = if choices.is_empty() {
        "<p>You hold nothing you can pass on to an agent yet, so it starts with no permissions.</p>"
            .to_owned()
    } else {
        format!("<p>Choose what it may do for you:</p><p>{choices}</p>")
    };
    let name = escaped(&name);
    let site = escaped(
        &reqwest::Url::parse(&asked.redirect_uri)
            .ok()
            .and_then(|url| url.host_str().map(str::to_owned))
            .unwrap_or_default(),
    );
    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::X_FRAME_OPTIONS, "DENY"),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'unsafe-inline'; form-action 'self' https: http:"),
        ],
        Html(format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>Connect {name} to Lys</title>\
<style>body{{font-family:system-ui,sans-serif;max-width:32rem;margin:4rem auto;padding:0 1.5rem;line-height:1.5;color:#1d1d1f}}\
button{{font:inherit;padding:.6rem 1.2rem;margin-right:.75rem;border-radius:.5rem;border:1px solid #888;background:#fff;cursor:pointer}}\
button[value=approve]{{background:#1d1d1f;color:#fff;border-color:#1d1d1f}}</style></head><body>\
<h1>Connect {name}?</h1>\
<p>{name}, at {site}, is asking to act for you in Lys.</p>\
<p>If you connect it, Lys adds it as one of your agents. It can do only what you choose here, \
and you can take that away or remove it at any time from your agents in Lys.</p>\
<form method=\"post\" action=\"/oauth/mcp/consent\">\
<input type=\"hidden\" name=\"asking\" value=\"{asking}\">{choices}\
<button type=\"submit\" name=\"decision\" value=\"approve\">Connect {name}</button>\
<button type=\"submit\" name=\"decision\" value=\"refuse\">Don't connect</button>\
</form></body></html>"
        )),
    )
        .into_response())
}

/// The approval form's answer: which approval, the decision, and each
/// permission chosen, read in the order the form sent them.
struct Answer {
    asking: String,
    decision: String,
    grants: Vec<String>,
}

fn answer(body: &[u8]) -> Result<Answer, ServerError> {
    let text = std::str::from_utf8(body).map_err(|_unread| malformed("an approval is a form"))?;
    let url = reqwest::Url::parse(&format!("http://form.invalid/?{text}"))
        .map_err(|_unread| malformed("an approval is a form"))?;
    let (mut asking, mut decision, mut grants) = (None, None, Vec::new());
    for (name, value) in url.query_pairs() {
        match name.as_ref() {
            "asking" => asking = Some(value.into_owned()),
            "decision" => decision = Some(value.into_owned()),
            "grant" => grants.push(value.into_owned()),
            _ => return Err(malformed("an approval carries only its own fields")),
        }
    }
    Ok(Answer {
        asking: asking.ok_or_else(|| malformed("an approval names what it answers"))?,
        decision: decision.ok_or_else(|| malformed("an approval carries its decision"))?,
        grants,
    })
}

pub(super) async fn consent(
    State(apps): State<Arc<Apps>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Response, ServerError> {
    let answer = answer(&body)?;
    if let Some(origin) = headers.get(header::ORIGIN)
        && origin.to_str().ok() != Some(apps.origin.as_str())
    {
        return Err(malformed("an approval is answered from Lys's own page"));
    }
    let session = apps.state.sessions.session(cookie_header(&headers))?;
    let asking = held(&apps.asking)?
        .remove(&answer.asking)
        .filter(|asking| asking.expires_at > now())
        .ok_or(ServerError::CodeUnknown)?;
    if asking.session_id != session.id {
        return Err(ServerError::CodeUnknown);
    }
    let state = asking.state.clone().unwrap_or_default();
    let with_state = |mut pairs: Vec<(&'static str, String)>| {
        if asking.state.is_some() {
            pairs.push(("state", state.clone()));
        }
        pairs
    };
    let pairs = match answer.decision.as_str() {
        "refuse" => with_state(vec![("error", "access_denied".to_owned())]),
        "approve" => {
            let name = held(&apps.store)?
                .app(&asking.client_id)
                .map(|app| app.name.clone())
                .ok_or(ServerError::RedirectUnregistered)?;
            let person = crate::routes::with_directory(&apps.state, |directory| {
                crate::read_api::own_person(directory.projection()?, &session.actor)
            })?;
            let picked = crate::mcp_oauth_grants::chosen(&apps.state, person, &answer.grants)?;
            let agent = apps.agent_for(&asking.client_id, &name, &session.actor)?;
            crate::mcp_oauth_grants::pass_on(&apps.state, person, &agent, &picked)?;
            let code = random()?;
            let at = now();
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
    back_to(&asking.redirect_uri, &pairs)
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
    Ok(apps.issue(&held_code.agent, &held_code.client_id, None)?)
}

fn refresh(apps: &Apps, asked: &Exchange) -> Result<Response, Refusal> {
    let Some(token) = &asked.refresh_token else {
        return Err(Refusal::Grant("a refresh carries its refresh token"));
    };
    let spent = digest(token);
    let agent = {
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
        issued.agent.clone()
    };
    Ok(apps.issue(&agent, &asked.client_id, Some(&spent))?)
}
