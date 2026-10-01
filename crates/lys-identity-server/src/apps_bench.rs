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
//! A bench-only lock serializes scratch cleanup and questions. Schema writes
//! share the engine's schema writer lock with live grants, while network
//! calls hold no live grants lock. An answer is refused if its captured
//! grant or model revision changed during the call.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::grants::{AppSchema, Model};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::apps_api::with_apps;
use crate::apps_bench_scratch::{Draft, Examples, answer_in};
use crate::apps_binding::{Acting, acting, new_secret};
use crate::apps_error::AppError;
use crate::apps_state::By;
use crate::error::ServerError;
use crate::grants::{GrantSetup, GrantState};
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
    asking: Mutex<()>,
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
            asking: Mutex::new(()),
        })
    }

    fn open(&self) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, Bench>>, ServerError> {
        self.open.lock().map_err(|error| {
            AppError::AppsUnavailable {
                reason: format!("the benches lock is poisoned: {error}"),
            }
            .into()
        })
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
    with_apps(state, |apps, projection| {
        let who = acting(state, apps.held(), headers, projection)?;
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
    let mut benches = state.benches.open()?;
    std::fs::create_dir_all(&dir).map_err(|error| AppError::AppsUnavailable {
        reason: format!("the bench's namespace could not be made: {error}"),
    })?;
    benches.insert(
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
        .open()?
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
    let engine = state.grant_setup.spicedb.as_ref();
    let examples = Examples {
        holdings: &body.holdings,
        placements: &body.placements,
        question: &body.question,
    };
    let answer = |model: &Model| {
        let draft = Draft {
            app: &bench.app,
            schema: &bench.schema,
            by: &bench.opened_by,
            lys: model,
            engine,
        };
        answer_in(&draft, &examples)
    };
    let Some(settings) = engine else {
        return answer(&state.grant_setup.model()?).map(Json);
    };
    with_engine(
        &state.benches.asking,
        &state.grants,
        &state.grant_setup,
        |model| {
            let left =
                SpiceDb::clear_scratch(settings).map_err(|error| AppError::AppsUnavailable {
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
            answer(model)
        },
    )
    .map(Json)
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    AppError::AppsUnavailable {
        reason: reason.into(),
    }
    .into()
}

fn with_engine<T>(
    asking: &Mutex<()>,
    grants: &Mutex<Option<GrantState>>,
    setup: &GrantSetup,
    call: impl FnOnce(&Model) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let question = asking.try_lock().map_err(|error| {
        unavailable(format!(
            "the bench engine is unavailable: {error}; retry this question"
        ))
    })?;
    let (revision, model_revision, model) = {
        let held = grants
            .lock()
            .map_err(|error| unavailable(format!("the bench's grants are unavailable: {error}")))?;
        let model = setup
            .model
            .read()
            .map_err(|error| unavailable(format!("the bench's model is unavailable: {error}")))?
            .clone();
        (
            held.as_ref().map(GrantState::revision),
            setup
                .model_revision
                .load(std::sync::atomic::Ordering::Acquire),
            model,
        )
    };
    let answered = call(&model)?;
    let held = grants
        .lock()
        .map_err(|error| unavailable(format!("the bench's grants are unavailable: {error}")))?;
    if held.as_ref().map(GrantState::revision) != revision
        || setup
            .model_revision
            .load(std::sync::atomic::Ordering::Acquire)
            != model_revision
    {
        return Err(unavailable(
            "the grants or app model changed during the bench question; retry this question",
        ));
    }
    drop(held);
    drop(question);
    Ok(answered)
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
        .open()?
        .remove(&id)
        .ok_or(AppError::BenchUnknown)?;
    std::fs::remove_dir_all(&closed.dir).map_err(|error| AppError::AppsUnavailable {
        reason: format!("the bench's namespace could not be removed: {error}"),
    })?;
    Ok(Json(serde_json::json!({"closed": id})))
}

#[cfg(test)]
#[path = "apps_bench_lock_tests.rs"]
mod lock_tests;

#[cfg(test)]
#[path = "apps_bench_poison_tests.rs"]
mod poison_tests;
