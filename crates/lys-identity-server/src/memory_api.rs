//! The memory route: what an agent's home keeps, as the home wrote it. Each
//! memory is a lantern lit on a point of a session, and the context given
//! last is the newest `lys.given` entry of the home.
//!
//! The administrator, the person responsible for the agent and the agent
//! itself read it; to anyone else the agent's memory is not visible. A
//! memory's note, the words of its epilogues and every line of a
//! transcript stay in the home: the answer carries where a memory stands,
//! who lit it and when, and says so in `notes_shown`.
//!
//! One home is kept for each agent, in the directory of `homes_dir` named
//! by the agent's id. The route reads and never writes, so an agent with no
//! home has none made for it. The home is read off the async workers, on a
//! blocking thread, and each session file is opened once for both the
//! memories and the context given last.

use std::path::{Path as FilePath, PathBuf};
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Extension, Json, Router};
use lys_home::{GivenSeen, Home, HomeError, LanternRow, Skipped, recall_all_and_last_given};
use serde::Serialize;

use crate::agent_sight::seen_agent;
use crate::config::Config;
use crate::error::ServerError;
use crate::routes::AppState;

/// Where the homes are kept, as the configuration names it.
#[derive(Clone)]
struct Homes {
    dir: Option<PathBuf>,
}

/// One memory, without its words.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryLine {
    /// The lantern's entry id.
    pub id: String,
    /// The session it stands in.
    pub session: String,
    /// The entry it was lit on.
    pub point: String,
    /// Who lit it.
    pub lit_by: String,
    /// When it was lit, RFC 3339 as the home wrote it.
    pub lit_at: String,
    /// The session it was lit from, null when the record names none.
    pub lit_in: Option<String>,
    /// How many epilogues were added to it.
    pub epilogues: usize,
}

/// A session that could not be read.
#[derive(Debug, Clone, Serialize)]
pub struct SkippedLine {
    /// The session.
    pub session: String,
    /// Why it was skipped.
    pub reason: String,
}

/// The context given last, by its sizes.
#[derive(Debug, Clone, Serialize)]
pub struct GivenView {
    /// The session the entry stands in.
    pub session: String,
    /// The entry's id.
    pub entry: String,
    /// When it was appended, RFC 3339 as the home wrote it.
    pub given_at: String,
    /// The harness that gave it.
    pub harness: String,
    /// The version of that harness.
    pub harness_version: String,
    /// How many documents were given.
    pub documents: usize,
    /// How many environment names were given.
    pub environment: usize,
}

/// Who the memory of this agent is shown to.
#[derive(Debug, Clone, Serialize)]
pub struct VisibleTo {
    /// The agent itself.
    pub agent: String,
    /// The person responsible for it.
    pub responsible: Option<String>,
    /// The administrator, always.
    pub administrator: bool,
}

/// The answer of the memory route.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryView {
    /// The agent.
    pub agent: String,
    /// Whether a home is kept for the agent. Without one both lists are
    /// empty.
    pub home: bool,
    /// Every memory, by session then position.
    pub memories: Vec<MemoryLine>,
    /// Every session that could not be read.
    pub skipped: Vec<SkippedLine>,
    /// The context given last, null while nothing was given.
    pub last_given: Option<GivenView>,
    /// Who this memory is shown to.
    pub visible_to: VisibleTo,
    /// Whether the words of a memory are in this answer. They never are.
    pub notes_shown: bool,
}

/// The memory route, reading the homes `config` names.
pub fn routes(config: &Config) -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/memory", get(read))
        .layer(Extension(Homes {
            dir: config.homes_dir.clone(),
        }))
}

/// `reason` with the home's own path read as `home`, so where the service
/// keeps its homes is not served.
fn placed(reason: &str, root: &FilePath) -> String {
    reason.replace(&root.display().to_string(), "home")
}

fn unavailable(error: &HomeError, root: &FilePath) -> ServerError {
    ServerError::MemoryUnavailable {
        reason: placed(&error.to_string(), root),
    }
}

fn line(row: LanternRow) -> MemoryLine {
    MemoryLine {
        id: row.id,
        session: row.session,
        point: row.point,
        lit_by: row.lit_by,
        lit_at: row.lit_at,
        lit_in: row.lit_in,
        epilogues: row.epilogues.len(),
    }
}

fn given(seen: GivenSeen) -> GivenView {
    GivenView {
        session: seen.session,
        entry: seen.entry,
        given_at: seen.given_at,
        harness: seen.record.harness,
        harness_version: seen.record.harness_version,
        documents: seen.record.documents.len(),
        environment: seen.record.environment.len(),
    }
}

/// Each session skipped by either reading, once, in the order first named.
fn skipped(recalled: Vec<Skipped>, given: Vec<Skipped>, root: &FilePath) -> Vec<SkippedLine> {
    let mut lines: Vec<SkippedLine> = Vec::new();
    for skip in recalled.into_iter().chain(given) {
        if lines.iter().all(|line| line.session != skip.session) {
            lines.push(SkippedLine {
                reason: placed(&skip.reason, root),
                session: skip.session,
            });
        }
    }
    lines
}

async fn read(
    State(state): State<Arc<AppState>>,
    Extension(homes): Extension<Homes>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<MemoryView>, ServerError> {
    let seen = seen_agent(&state, &headers, &id)?;
    let dir = homes.dir.ok_or_else(|| ServerError::MemoryUnavailable {
        reason: "the configuration names no homes_dir".to_owned(),
    })?;
    let agent = seen.agent.to_string();
    let mut view = MemoryView {
        agent: agent.clone(),
        home: false,
        memories: Vec::new(),
        skipped: Vec::new(),
        last_given: None,
        visible_to: VisibleTo {
            agent: agent.clone(),
            responsible: seen.responsible.map(|person| person.to_string()),
            administrator: true,
        },
        notes_shown: false,
    };
    let root = dir.join(&agent);
    let place = root.clone();
    let read = tokio::task::spawn_blocking(move || match Home::read(&place) {
        Ok(home) => recall_all_and_last_given(&home).map(Some),
        Err(HomeError::NoHome { .. }) => Ok(None),
        Err(error) => Err(error),
    })
    .await
    .map_err(|error| ServerError::MemoryUnavailable {
        reason: format!("the home could not be read: {error}"),
    })?
    .map_err(|error| unavailable(&error, &root))?;
    let Some((recalled, last)) = read else {
        return Ok(Json(view));
    };
    view.home = true;
    view.memories = recalled.lanterns.into_iter().map(line).collect();
    view.skipped = skipped(recalled.skipped, last.skipped, &root);
    view.last_given = last.last.map(given);
    Ok(Json(view))
}
