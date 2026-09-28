//! The schema builder's test bench: a draft schema held apart from every
//! standing schema, asked "may X do Y to Z" over example holdings and
//! placements.
//!
//! A bench is opened with a draft, which is checked exactly as a registration
//! or a change is, and held in this process under an id of its own, in no
//! log the service keeps, so no standing check ever sees a draft. Each bench
//! has a namespace directory of its own under the benches' directory. Each
//! question is answered by the real check against a scratch copy of the
//! draft loaded into a throwaway namespace within it (`apps_bench_scratch`),
//! and the answer names the path that allowed it, or the refusal the check
//! named. Closing the bench removes its namespace, and nothing of it remains.
//! A start removes every namespace an earlier run left, since no bench
//! outlives the process that opened it.
//!
//! When the service runs its grants on `SpiceDB`, a question is asked under
//! the service's grants' lock, since the scratch scope it writes shares the
//! engine's one schema with the service's. Every scratch scope the engine
//! holds when a question begins is one an earlier question could not remove,
//! and is removed first.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::grants::AppSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::apps_api::with_apps;
use crate::apps_bench_scratch::{Draft, Examples, answer_in};
use crate::apps_binding::{Acting, acting, new_secret};
use crate::apps_error::AppError;
use crate::apps_state::By;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::spicedb::SpiceDb;

/// Every refusal the bench routes answer with: an example the draft cannot
/// hold is refused as the service refuses it, and a namespace that cannot be
/// made or removed `apps_unavailable`.
pub(crate) const BENCH: &[&str] = &[
    "NotAdmitted",
    "schema_invalid",
    "bench_unknown",
    "kind_not_registered",
    "not_your_app",
    "placement_invalid",
    "RelationUnknown",
    "apps_unavailable",
];

/// The open benches, by id, and the directory their namespaces are made in.
#[derive(Debug)]
pub struct Benches {
    dir: PathBuf,
    open: Mutex<BTreeMap<String, Bench>>,
}

impl Benches {
    /// The benches, their namespaces made under `dir`. Whatever an earlier
    /// run left there is removed, and said through `say`.
    pub fn new(dir: PathBuf, say: &dyn Fn(&str)) -> Result<Self, ServerError> {
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|error| AppError::AppsUnavailable {
                reason: format!("the benches an earlier run left could not be removed: {error}"),
            })?;
            say("bench namespaces left by an earlier run removed");
        }
        Ok(Self {
            dir,
            open: Mutex::new(BTreeMap::new()),
        })
    }

    fn open(&self) -> std::sync::MutexGuard<'_, BTreeMap<String, Bench>> {
        self.open.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// One open bench.
#[derive(Debug, Clone)]
pub struct Bench {
    app: String,
    schema: Value,
    opened_by: By,
    dir: PathBuf,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct OpenBody {
    app: String,
    #[schema(value_type = Object)]
    schema: Value,
}

/// A resource, by kind and id.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Example {
    /// The kind.
    pub kind: String,
    /// The id.
    pub id: String,
}

/// An example person or agent holding a relation on an example resource.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Holding {
    /// The example person or agent, by any name.
    pub subject: String,
    /// The relation held.
    pub relation: String,
    /// The resource it is held on.
    pub resource: Example,
}

/// An example resource placed in an example parent.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// The child.
    pub child: Example,
    /// The parent it is placed in.
    pub parent: Example,
}

/// The question "may X do Y to Z".
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// X: the example person or agent.
    pub subject: String,
    /// Y: the action.
    pub action: String,
    /// Z: the resource.
    pub resource: Example,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct AskBody {
    holdings: Vec<Holding>,
    #[serde(default)]
    placements: Vec<Placement>,
    question: Question,
}

/// The bench's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct BenchAnswer {
    /// Whether the draft allows it.
    pub allowed: bool,
    /// Each step of the path that allowed it, or that was tried.
    pub path: Vec<String>,
    /// The refusal the real check would name, when refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,
}

/// The bench routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/apps/bench", post(open))
        .route("/apps/bench/{id}/ask", post(ask))
        .route("/apps/bench/{id}/close", post(close))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

/// Who acts on a bench: the administrator for any app, an app for its own.
fn bencher(state: &AppState, headers: &HeaderMap, app: Option<&str>) -> Result<By, ServerError> {
    with_apps(state, |apps| {
        let who = acting(state, apps.held(), headers)?;
        match &who {
            Acting::Administrator(_) => Ok(who.by()),
            Acting::App { app: own, .. } if app.is_none_or(|app| app == own) => Ok(who.by()),
            _ => Err(ServerError::NotAdmitted {
                reason: "only the administrator, or the app itself, opens a bench on an app's schema",
            }),
        }
    })
}

async fn open(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<OpenBody>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let opened_by = bencher(&state, &headers, Some(&body.app))?;
    AppSchema::parse(&body.app, &body.schema).map_err(AppError::from)?;
    let (id, _) = new_secret()?;
    let dir = state.benches.dir.join(&id);
    std::fs::create_dir_all(&dir).map_err(|error| AppError::AppsUnavailable {
        reason: format!("the bench's namespace could not be made: {error}"),
    })?;
    state.benches.open().insert(
        id.clone(),
        Bench {
            app: body.app.clone(),
            schema: body.schema,
            opened_by,
            dir,
        },
    );
    Ok(Json(serde_json::json!({"bench": id, "app": body.app})))
}

/// The bench `id`, when `by` opened it.
fn bench(state: &AppState, id: &str, by: &By) -> Result<Bench, ServerError> {
    state
        .benches
        .open()
        .get(id)
        .filter(|bench| bench.opened_by == *by)
        .cloned()
        .ok_or_else(|| AppError::BenchUnknown.into())
}

async fn ask(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<AskBody>, JsonRejection>,
) -> Result<Json<BenchAnswer>, ServerError> {
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let by = bencher(&state, &headers, None)?;
    let bench = bench(&state, &id, &by)?;
    let (asking, _) = new_secret()?;
    let engine = state.grant_setup.spicedb.as_ref();
    let draft = Draft {
        app: &bench.app,
        schema: &bench.schema,
        by: &bench.opened_by,
        lys: &state.grant_setup.model(),
        engine,
    };
    let examples = Examples {
        holdings: &body.holdings,
        placements: &body.placements,
        question: &body.question,
    };
    let Some(settings) = engine else {
        return answer_in(&bench.dir.join(asking), &draft, &examples).map(Json);
    };
    let held = state.grants.lock().unwrap_or_else(PoisonError::into_inner);
    let left = SpiceDb::clear_scratch(settings).map_err(|error| AppError::AppsUnavailable {
        reason: format!(
            "the scratch scopes an earlier question left could not be removed: {error}"
        ),
    })?;
    if left > 0 {
        tracing::warn!(
            scopes = left,
            "scratch scopes an earlier question left were removed"
        );
    }
    let answered = answer_in(&bench.dir.join(asking), &draft, &examples);
    drop(held);
    answered.map(Json)
}

async fn close(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ServerError> {
    let by = bencher(&state, &headers, None)?;
    bench(&state, &id, &by)?;
    let closed = state
        .benches
        .open()
        .remove(&id)
        .ok_or(AppError::BenchUnknown)?;
    std::fs::remove_dir_all(&closed.dir).map_err(|error| AppError::AppsUnavailable {
        reason: format!("the bench's namespace could not be removed: {error}"),
    })?;
    Ok(Json(serde_json::json!({"closed": id})))
}
