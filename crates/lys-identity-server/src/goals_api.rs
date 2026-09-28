//! The goal routes: the person responsible for an agent, a team's owner or
//! the administrator sets goals, expectations and deliverables on it, each
//! with its words, its deadline and its reminders, and reads them with
//! their standing and every reminder fired.
//!
//! An item is marked met, missed or dropped only by its responsible person
//! or by the holder of a grant of the action the item names on its holder.
//! The agent an item judges, or a member agent of the team it is held on,
//! is refused `not_your_judgement` for its own, whatever it holds. A met
//! deliverable carries the evidence as a claim in the judge's words, kept
//! naming them; Lys calls no app to check it.
//!
//! Reminders are the goals log's timers. One task waits until the next one
//! falls due, or until something a timer depends on is kept, and runs a
//! pass: each due reminder is delivered into each live session of its
//! holder through the session's runner, under the runner's operation ids,
//! and kept with its delivery or its refusal by name. A set runs a pass
//! before it answers, so a reminder due at once is answered fired.

use std::str::FromStr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Uri};
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{AgentId, IdentityId, OperationId};
use lys_runner::operations::Operation;
use serde::{Deserialize, Serialize};

use crate::agent_signature::signed_agent;
use crate::error::ServerError;
use crate::goals_state::{
    EvidenceKind, Goal, GoalError, Holder, HolderKind, Item, Kind, Marked, Remind, Standing,
};
use crate::goals_store::{Deliver, Delivering, Goals};
use crate::grants::{caller, with_grants};
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runtime_api::with_runtime;
use crate::session::now;

/// The most characters an item's words carry.
const WORDS_MAX: usize = 500;

/// The most reminders one item carries.
const REMINDERS_MAX: usize = 16;

/// The fewest seconds between interval reminders.
const EVERY_MIN: u64 = 60;

/// An item to set.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalSetBody)]
#[serde(deny_unknown_fields)]
pub struct SetBody {
    operation: String,
    kind: Kind,
    words: String,
    deadline: u64,
    #[serde(default)]
    evidence: Option<EvidenceKind>,
    #[serde(default)]
    judged_by: Option<String>,
    #[serde(default)]
    reminders: Vec<Remind>,
}

/// A judgement to make.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = GoalMarkBody)]
#[serde(deny_unknown_fields)]
pub struct MarkBody {
    operation: String,
    standing: Standing,
    words: String,
    #[serde(default)]
    evidence: Option<String>,
}

/// The items held on an agent or a team.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct GoalsView {
    /// Every item, in the order set.
    pub goals: Vec<Item>,
}

/// The goal routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agents/{id}/goals", get(agent_goals).post(set_agent_goal))
        .route("/teams/{id}/goals", get(team_goals).post(set_team_goal))
        .route("/goals/{goal}/mark", post(mark))
}

fn malformed(reason: impl Into<String>) -> ServerError {
    ServerError::RequestMalformed {
        reason: reason.into(),
    }
}

fn goals(state: &AppState) -> Result<&Goals, ServerError> {
    state.goals.as_ref().ok_or_else(|| {
        GoalError::Unavailable {
            reason: "the configuration names no goals_dir".to_owned(),
        }
        .into()
    })
}

/// The holder the caller may set and read goals on, and the person
/// responsible for them: an agent's responsible person or a team's owner,
/// or the administrator.
fn holder(
    state: &AppState,
    headers: &HeaderMap,
    kind: HolderKind,
    id: &str,
) -> Result<(Holder, String), ServerError> {
    let actor = signed_in(state, headers)?;
    let administrator = state.admission.administrator(&actor).is_ok();
    let person = with_directory(state, |directory| {
        own_person(directory.projection()?, &actor)
    })?;
    let responsible = match kind {
        HolderKind::Agent => {
            let agent = AgentId::from_str(id).map_err(|_unread| ServerError::AgentNotVisible)?;
            let responsible = with_directory(state, |directory| {
                let projection = directory.projection()?;
                let record = projection
                    .record(IdentityId::Agent(agent))
                    .ok_or(ServerError::AgentNotVisible)?;
                Ok(record.responsible())
            })?;
            if !administrator && responsible != Some(person) {
                return Err(ServerError::AgentNotVisible);
            }
            responsible.unwrap_or(person).to_string()
        }
        HolderKind::Team => {
            let owner = team(state, id)?.created.owner;
            if !administrator && owner != person.to_string() {
                return Err(ServerError::NotAdmitted {
                    reason: "only the team's owner or the administrator sets and reads its goals",
                });
            }
            owner
        }
    };
    let holder = Holder {
        kind,
        id: id.to_owned(),
    };
    Ok((holder, responsible))
}

fn team(state: &AppState, id: &str) -> Result<crate::teams_state::Team, ServerError> {
    let store = state
        .teams
        .as_ref()
        .ok_or_else(|| ServerError::TeamsUnavailable {
            reason: "the configuration names no teams_dir".to_owned(),
        })?;
    let mut store = store
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    store.settle()?;
    store.team(id).cloned().ok_or(ServerError::TeamUnknown)
}

/// The agents `holder` judges: the agent, or every agent in the team.
fn judged(state: &AppState, holder: &Holder) -> Result<Vec<String>, ServerError> {
    match holder.kind {
        HolderKind::Agent => Ok(vec![holder.id.clone()]),
        HolderKind::Team => Ok(team(state, &holder.id)?
            .members
            .into_iter()
            .filter(|member| AgentId::from_str(member).is_ok())
            .collect()),
    }
}

async fn agent_goals(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<GoalsView>, ServerError> {
    read(&state, &headers, HolderKind::Agent, &id)
}

async fn team_goals(
    state: State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<GoalsView>, ServerError> {
    read(&state, &headers, HolderKind::Team, &id)
}

fn read(
    state: &AppState,
    headers: &HeaderMap,
    kind: HolderKind,
    id: &str,
) -> Result<Json<GoalsView>, ServerError> {
    let (holder, _responsible) = holder(state, headers, kind, id)?;
    let goals = goals(state)?.with(|store| Ok(store.of_holder(&holder)))?;
    Ok(Json(GoalsView { goals }))
}

async fn set_agent_goal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<SetBody>, JsonRejection>,
) -> Result<Json<Item>, ServerError> {
    set(&state, &headers, (HolderKind::Agent, &id), body).await
}

async fn set_team_goal(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<SetBody>, JsonRejection>,
) -> Result<Json<Item>, ServerError> {
    set(&state, &headers, (HolderKind::Team, &id), body).await
}

fn checked(body: &SetBody) -> Result<(), ServerError> {
    let words = body.words.trim();
    if words.is_empty() || words.chars().count() > WORDS_MAX {
        return Err(malformed(format!(
            "words carry between 1 and {WORDS_MAX} characters"
        )));
    }
    if body.reminders.len() > REMINDERS_MAX {
        return Err(malformed(format!(
            "an item carries at most {REMINDERS_MAX} reminders"
        )));
    }
    for remind in &body.reminders {
        match remind {
            Remind::Before { seconds: 0 } => {
                return Err(malformed(
                    "a reminder before the deadline is at least 1 second before it",
                ));
            }
            Remind::Every { seconds } if *seconds < EVERY_MIN => {
                return Err(malformed(format!(
                    "interval reminders are at least {EVERY_MIN} seconds apart"
                )));
            }
            _ => {}
        }
    }
    if let Some(action) = &body.judged_by {
        Action::new(action).map_err(|refused| malformed(refused.to_string()))?;
    }
    Ok(())
}

async fn set(
    state: &Arc<AppState>,
    headers: &HeaderMap,
    (kind, id): (HolderKind, &str),
    body: Result<Json<SetBody>, JsonRejection>,
) -> Result<Json<Item>, ServerError> {
    let actor = signed_in(state, headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let operation = OperationId::from_str(&body.operation)?;
    checked(&body)?;
    let (holder, responsible) = holder(state, headers, kind, id)?;
    let set_by = with_directory(state, |directory| {
        own_person(directory.projection()?, &actor)
    })?;
    let at = now();
    if body.deadline <= at {
        return Err(malformed("the deadline has passed"));
    }
    let goal = Goal {
        id: operation.to_string(),
        holder,
        kind: body.kind,
        words: body.words.trim().to_owned(),
        deadline: body.deadline,
        evidence: body.evidence,
        judged_by: body.judged_by,
        reminders: body.reminders,
        responsible,
        set_by: set_by.to_string(),
        at,
    };
    let goals = goals(state)?;
    let id = goals.with(|store| store.set(goal))?.goal.id;
    goals.changed.notify_one();
    crate::goals_store::remind(goals, &Live(state), now()).await?;
    goals
        .with(|store| {
            store
                .item(&id)
                .cloned()
                .ok_or_else(|| GoalError::Unknown.into())
        })
        .map(Json)
}

/// The identity asking: an agent by its signed request, or the signed-in caller.
fn asker(
    state: &AppState,
    headers: &HeaderMap,
    uri: &Uri,
    bytes: &[u8],
) -> Result<IdentityId, ServerError> {
    with_directory(state, |directory| {
        let projection = directory.projection()?;
        match signed_agent(state, projection, headers, ("POST", uri.path(), bytes))? {
            Some(agent) => Ok(IdentityId::Agent(agent)),
            None => caller(state, headers, projection),
        }
    })
}

/// Admit `asker` to judge `item`: never an agent it judges; its responsible
/// person; or the holder of a grant of the action it names on its holder.
fn judge(state: &AppState, asker: IdentityId, item: &Item) -> Result<(), ServerError> {
    let judged = judged(state, &item.goal.holder)?;
    if let IdentityId::Agent(agent) = asker {
        if judged.contains(&agent.to_string()) {
            return Err(GoalError::NotYourJudgement {
                agent: agent.to_string(),
            }
            .into());
        }
    }
    if asker.to_string() == item.goal.responsible {
        return Ok(());
    }
    if let Some(action) = &item.goal.judged_by {
        let kind = match item.goal.holder.kind {
            HolderKind::Agent => "agent",
            HolderKind::Team => "team",
        };
        let granted = with_grants(state, |held| {
            let request = ExerciseRequest {
                caller: asker,
                route: Route::Api,
                resource: Resource::new(kind, &item.goal.holder.id)?,
                action: Action::new(action)?,
            };
            Ok(held
                .grants
                .explain(held.directory, &request, now(), None)
                .is_ok())
        })?;
        if granted {
            return Ok(());
        }
    }
    Err(ServerError::NotPermitted {
        reason: format!(
            "{asker} is not responsible for goal {} and holds no grant it names, so it may not judge it",
            item.goal.id
        ),
    })
}

async fn mark(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(goal): Path<String>,
    uri: Uri,
    bytes: Bytes,
) -> Result<Json<Item>, ServerError> {
    let body: MarkBody =
        serde_json::from_slice(&bytes).map_err(|refused| malformed(refused.to_string()))?;
    let operation = OperationId::from_str(&body.operation)?;
    if body.standing == Standing::Open {
        return Err(malformed("a mark is met, missed or dropped"));
    }
    let words = body.words.trim();
    if words.is_empty() || words.chars().count() > WORDS_MAX {
        return Err(malformed(format!(
            "words carry between 1 and {WORDS_MAX} characters"
        )));
    }
    let asker = asker(&state, &headers, &uri, &bytes)?;
    let goals = goals(&state)?;
    let item = goals.with(|store| {
        store
            .item(&goal)
            .cloned()
            .ok_or_else(|| GoalError::Unknown.into())
    })?;
    judge(&state, asker, &item)?;
    let marked = Marked {
        operation: operation.to_string(),
        goal,
        standing: body.standing,
        by: asker.to_string(),
        words: words.to_owned(),
        evidence: body.evidence.map(|claim| claim.trim().to_owned()),
        at: now(),
    };
    let item = goals.with(|store| store.mark(marked))?;
    goals.changed.notify_one();
    Ok(Json(item))
}

/// Reminders delivered through the runners the service reaches.
struct Live<'a>(&'a Arc<AppState>);

impl Deliver for Live<'_> {
    fn sessions(&self, holder: &Holder) -> Result<Vec<String>, String> {
        let agents = judged(self.0, holder).map_err(|error| error.to_string())?;
        if self.0.runtime.is_none() {
            return Ok(Vec::new());
        }
        with_runtime(self.0, |store| {
            Ok(store
                .sessions()
                .iter()
                .filter(|tracked| !tracked.stopped())
                .filter(|tracked| {
                    tracked
                        .agent
                        .as_ref()
                        .is_some_and(|agent| agents.contains(agent))
                })
                .map(|tracked| tracked.session.clone())
                .collect())
        })
        .map_err(|error| error.to_string())
    }

    fn operate(&self, operation: Operation) -> Delivering<'_> {
        Box::pin(crate::runner_operate::operate(self.0, operation))
    }
}

/// Start the task that fires each reminder when it falls due, for as long
/// as `state` is served.
pub fn remind_from(state: &Arc<AppState>) {
    let Some(goals) = state.goals.as_ref() else {
        return;
    };
    let changed = Arc::clone(&goals.changed);
    let served = Arc::downgrade(state);
    tokio::spawn(async move {
        loop {
            let Some(state) = served.upgrade() else {
                return;
            };
            let next = match pass(&state).await {
                Ok(next) => next,
                Err(error) => {
                    (state.say)(&format!("goals: a reminder pass failed: {error}"));
                    None
                }
            };
            drop(state);
            match next {
                Some(due) => {
                    let wait = std::time::Duration::from_secs(due.saturating_sub(now()));
                    tokio::select! {
                        () = tokio::time::sleep(wait) => {}
                        () = changed.notified() => {}
                    }
                }
                None => changed.notified().await,
            }
        }
    });
}

/// One pass, answering when the next reminder falls due.
async fn pass(state: &Arc<AppState>) -> Result<Option<u64>, ServerError> {
    let goals = goals(state)?;
    crate::goals_store::remind(goals, &Live(state), now()).await?;
    goals.with(|store| Ok(store.next_due()))
}
