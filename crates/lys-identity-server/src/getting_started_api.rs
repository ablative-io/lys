//! Getting started's answers, kept for the person who gave them: today one,
//! whether they said the computer Lys runs on is not for agents ("Not this
//! one"). `PUT /me/getting-started` keeps the caller's answer and `GET /me`
//! answers it to that person alone, so it is not asked again from another
//! browser.
//!
//! Each person's answers are one small file in the folder `getting-started`
//! beside the directory log, replaced whole as the canvas is: written beside
//! it, flushed, moved over it, and the folder flushed. Nothing is appended or
//! replayed, nothing is read at start, and a person who never answered has no
//! file. The answers give nothing and take nothing: no grant, check or record
//! reads them.

use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use axum::routing::put;
use axum::{Json, Router};
use lys_identity::PersonId;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::routes::{AppState, signed_in, with_directory};

/// The getting-started route.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/me/getting-started", put(keep))
}

pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<crate::openapi_types::Entry> {
    vec![(
        lys_openapi::Method::Put,
        "/me/getting-started",
        Some(api.schema::<GettingStarted>()),
        Some(api.schema::<GettingStarted>()),
    )]
}

/// A person's getting-started answers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GettingStarted {
    /// Whether they said the computer Lys runs on is not for agents.
    pub host_declined: bool,
}

/// A person's answers as `GET /me` shows them: kept, or the refusal that
/// stopped them being read, in their place, so `/me` itself still answers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum GettingStartedPart {
    /// The answers kept, or none given.
    Kept(GettingStarted),
    /// The answers could not be read.
    Refused {
        /// The refusal, by name.
        refusal: String,
        /// Why, in the refusal's words.
        reason: String,
    },
}

/// Each person's getting-started answers, one file for each beside the directory log.
pub struct GettingStartedStore {
    dir: PathBuf,
    changing: Mutex<()>,
}

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    ServerError::DirectoryUnavailable {
        reason: format!("getting started's answers: {what}"),
    }
}

impl GettingStartedStore {
    /// The answers kept in the folder `getting-started` beside the directory
    /// log `log_dir`. Nothing is read or made here.
    #[must_use]
    pub fn beside(log_dir: &Path) -> Self {
        Self {
            dir: log_dir.with_file_name("getting-started"),
            changing: Mutex::new(()),
        }
    }

    fn file(&self, person: PersonId) -> PathBuf {
        self.dir.join(format!("{person}.json"))
    }

    /// `person`'s answers; none given when they never answered.
    ///
    /// # Errors
    /// Refuses unreadable or malformed kept answers.
    pub fn read(&self, person: PersonId) -> Result<GettingStarted, ServerError> {
        match fs::read(self.file(person)) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                unavailable(format!(
                    "the file kept for {person} cannot be read: {error}"
                ))
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(GettingStarted::default())
            }
            Err(error) => Err(unavailable(error)),
        }
    }

    /// `person`'s answers as `GET /me` shows them.
    #[must_use]
    pub fn part(&self, person: PersonId) -> GettingStartedPart {
        match self.read(person) {
            Ok(answers) => GettingStartedPart::Kept(answers),
            Err(error) => GettingStartedPart::Refused {
                refusal: error.name(),
                reason: error.to_string(),
            },
        }
    }

    /// Keep `answers` as the whole of `person`'s answers before answering.
    ///
    /// # Errors
    /// Refuses a poisoned lock. Propagates encoding, folder creation and
    /// durable replacement failures.
    pub fn keep(&self, person: PersonId, answers: GettingStarted) -> Result<(), ServerError> {
        let _one = self
            .changing
            .lock()
            .map_err(|error| unavailable(format!("the lock is poisoned: {error}")))?;
        let bytes = serde_json::to_vec(&answers).map_err(unavailable)?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)
            .map_err(unavailable)?;
        crate::canvas_store::replace(&self.file(person), &bytes).map_err(unavailable)
    }
}

/// The signed-in person, refused by name for anyone who is not one.
fn person(state: &AppState, headers: &HeaderMap) -> Result<PersonId, ServerError> {
    let actor = signed_in(state, headers)?;
    with_directory(state, |directory| {
        crate::read_api::own_person(directory.projection()?, &actor)
    })
}

async fn keep(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<GettingStarted>, JsonRejection>,
) -> Result<Json<GettingStarted>, ServerError> {
    let person = person(&state, &headers)?;
    let answers = crate::requests_api::taken(body)?;
    state.getting_started.keep(person, answers)?;
    Ok(Json(answers))
}
