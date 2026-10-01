//! The session routes: a person lists their live sessions and ends one, and
//! the configured administrator does the same for any person.
//!
//! A session is a person's when its login resolves, through the directory's
//! projection, to that person, so a person's sessions span every login bound
//! to them. A session is named by its public id and never by its cookie
//! secret. A session that is not live and a session that is another person's
//! are refused `SessionUnknown` alike, so an answer tells nothing of another
//! person's sessions. Ending the caller's own current session also clears the
//! cookie. Every end is logged once, naming the caller's login and the ended
//! session's public id; that log line is the only trace an end leaves, since
//! sessions live in memory and no end is written to the directory's log.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::projection::Projection;
use lys_identity::{Actor, PersonId};
use serde::{Deserialize, Serialize};

use crate::caller_admission::own_account_person;
use crate::error::ServerError;
use crate::read_api::person_record;
use crate::routes::{AppState, cookie_header, signed_in, with_directory};
use crate::session::SessionEntry;

/// A session's login: the issuer that authenticated it and the subject it names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionLogin {
    /// The issuer URL, exactly as recorded.
    pub issuer: String,
    /// The subject at that issuer, exactly as recorded.
    pub subject: String,
}

/// One live session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionView {
    /// The session's public id.
    pub id: String,
    /// The login the session was signed in through.
    pub login: SessionLogin,
    /// When the session began, in seconds since the Unix epoch.
    pub started_at: u64,
    /// When the session ends, in seconds since the Unix epoch.
    pub ends_at: u64,
    /// Whether this is the session the request was made in.
    pub current: bool,
}

/// One person's live sessions, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SessionsView {
    /// The person's enduring id.
    pub person: String,
    /// Their live sessions, by when they began, newest first.
    pub sessions: Vec<SessionView>,
}

/// The answer to ending a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct EndedView {
    /// The ended session's public id.
    pub ended: String,
}

/// The session routes: the signed-in person's own, and the administrator's for any person.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sessions", get(own_sessions))
        .route("/sessions/{id}/end", post(end_own))
        .route("/directory/people/{id}/sessions", get(person_sessions))
        .route(
            "/directory/people/{id}/sessions/{session}/end",
            post(end_persons),
        )
}

/// Whether `actor`'s login resolves to `person` in `projection`.
fn belongs(projection: &Projection, person: PersonId, actor: &Actor) -> bool {
    projection.person_for(actor.binding()) == Some(person)
}

/// The person a directory route names, refused as an unknown identity when
/// the directory holds no such person.
fn named_person(projection: &Projection, person: PersonId) -> Result<PersonId, ServerError> {
    person_record(projection, person)?;
    Ok(person)
}

fn session_view(entry: SessionEntry, current: &str) -> SessionView {
    SessionView {
        current: entry.id == current,
        login: SessionLogin {
            issuer: entry.actor.binding().issuer().to_owned(),
            subject: entry.actor.binding().subject().to_owned(),
        },
        id: entry.id,
        started_at: entry.started_at,
        ends_at: entry.ends_at,
    }
}

/// The live sessions of the person `person_of` reads from the projection,
/// marking the caller's `current` one.
fn sessions_of(
    state: &AppState,
    current: &str,
    person_of: impl FnOnce(&Projection) -> Result<PersonId, ServerError>,
) -> Result<Json<SessionsView>, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = person_of(projection)?;
        let mut entries = state
            .sessions
            .live(|actor| belongs(projection, person, actor));
        entries.sort_by(|a, b| {
            b.started_at
                .cmp(&a.started_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(Json(SessionsView {
            person: person.to_string(),
            sessions: entries
                .into_iter()
                .map(|entry| session_view(entry, current))
                .collect(),
        }))
    })
}

/// End the session `id` of the person `person_of` reads from the projection,
/// on behalf of `actor`, whose own session is `current`. The end is logged
/// once; ending `current` clears the cookie.
fn end_session(
    state: &AppState,
    actor: &Actor,
    current: &str,
    id: &str,
    person_of: impl FnOnce(&Projection) -> Result<PersonId, ServerError>,
) -> Result<Response, ServerError> {
    let ended = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = person_of(projection)?;
        state
            .sessions
            .end(id, |owner| belongs(projection, person, owner))
    })?;
    tracing::info!(
        issuer = actor.binding().issuer(),
        subject = actor.binding().subject(),
        session = ended.id.as_str(),
        "session ended"
    );
    let cleared = ended.id == current;
    let body = Json(EndedView { ended: ended.id });
    if cleared {
        Ok(([(header::SET_COOKIE, state.sessions.clear_cookie())], body).into_response())
    } else {
        Ok(body.into_response())
    }
}

/// The signed-in caller and the public id of the session they called in.
fn caller(state: &AppState, headers: &HeaderMap) -> Result<(Actor, String), ServerError> {
    let actor = signed_in(state, headers)?;
    let current = state.sessions.current(cookie_header(headers))?;
    Ok((actor, current))
}

/// The signed-in person's live sessions, across every login bound to them.
async fn own_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SessionsView>, ServerError> {
    let (actor, current) = caller(&state, &headers)?;
    sessions_of(&state, &current, |projection| {
        own_account_person(projection, &actor)
    })
}

/// End one of the signed-in person's live sessions.
async fn end_own(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ServerError> {
    let (actor, current) = caller(&state, &headers)?;
    end_session(&state, &actor, &current, &id, |projection| {
        own_account_person(projection, &actor)
    })
}

/// Any person's live sessions, for the configured administrator.
async fn person_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SessionsView>, ServerError> {
    let (actor, current) = caller(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let person = PersonId::from_str(&id)?;
    sessions_of(&state, &current, |projection| {
        named_person(projection, person)
    })
}

/// End one of any person's live sessions, for the configured administrator.
///
/// Ending another person's session leaves no durable record: the directory's
/// log is not written, and the one trace of it is the server's own log line.
/// It is not an audited act.
async fn end_persons(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, session)): Path<(String, String)>,
) -> Result<Response, ServerError> {
    let (actor, current) = caller(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let person = PersonId::from_str(&id)?;
    end_session(&state, &actor, &current, &session, |projection| {
        named_person(projection, person)
    })
}
