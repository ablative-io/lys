//! The words routes (AGENTS-001 R1) and the one rendering every delivery
//! asks (R3): the administrator sets the workspace layer and the templates;
//! an agent's responsible person or the administrator sets its layer and
//! its sessions' layers; anyone who may read the agent reads its slots
//! resolved. A preview renders a slot for a session with the numbers the
//! caller supplies and sends nothing.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::{AgentId, IdentityId};
use lys_runner::render::{GoalLine, Inputs, Rendered, Variable, render};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::goals_state::Standing;
use crate::read_api::own_person;
use crate::routes::{AppState, is_administrator, signed_in, with_directory};
use crate::session::now;
use crate::variables_api::session_agent;
use crate::variables_state::Scope;
use crate::words_state::{Contributed, Layer, Resolved, Setting, Slot, Words, WordsError};
use crate::words_store::{Save, WordsKept};

/// A save of a slot at a layer.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsSaveBody)]
#[serde(deny_unknown_fields)]
pub struct SaveBody {
    /// The setting.
    pub setting: Setting,
    /// The revision read; 0 for a slot never saved.
    pub revision: u64,
}

/// A save of a template.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsTemplateBody)]
#[serde(deny_unknown_fields)]
pub struct TemplateBody {
    /// The text.
    pub text: String,
    /// The revision read; 0 for a template never saved.
    pub revision: u64,
}

/// A preview: a slot rendered for a session or an agent, sending nothing.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsPreviewBody)]
#[serde(deny_unknown_fields)]
pub struct PreviewBody {
    /// The slot.
    pub slot: Slot,
    /// The agent, when the preview is for one.
    #[serde(default)]
    pub agent: Option<String>,
    /// The session, when the preview is for one.
    #[serde(default)]
    pub session: Option<String>,
    /// The numbers and names the slot carries, by placeholder name.
    #[serde(default)]
    pub numbers: BTreeMap<String, String>,
}

/// The revision a save made.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = WordsSaved)]
pub struct Saved {
    /// The revision now held.
    pub revision: u64,
}

/// An agent's slots resolved, and its own layer as held.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[schema(as = WordsForAgent)]
pub struct ForAgent {
    /// The agent.
    pub agent: String,
    /// Each slot resolved for the agent, in slot order.
    pub resolved: Vec<Resolved>,
    /// The agent layer's slots as held, by slot name.
    pub held: BTreeMap<String, crate::words_state::Held>,
}

/// Words rendered for a delivery: the text, where the words came from, and
/// every revision that contributed, words and template and variables and
/// goals, to be written in the operation's receipt before the delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsDelivered)]
#[serde(deny_unknown_fields)]
pub struct Delivered {
    /// The slot.
    pub slot: Slot,
    /// The rendered text.
    pub text: String,
    /// The layer the words came from.
    pub source: String,
    /// Every revision that contributed, outermost first.
    pub contributed: Vec<Contributed>,
    /// Each placeholder that rendered empty for want of a key.
    pub missing: Vec<String>,
}

/// The words routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/words", get(read_all))
        .route("/words/preview", post(preview))
        .route("/words/templates/{name}", post(save_template))
        .route("/words/{slot}", post(save_workspace))
        .route("/agents/{id}/words", get(read_agent))
        .route("/agents/{id}/words/{slot}", post(save_agent))
        .route("/runtime/sessions/{id}/words/{slot}", post(save_session))
}

pub(crate) fn words(state: &AppState) -> Result<&WordsKept, ServerError> {
    state.words.as_ref().ok_or_else(|| {
        WordsError::Unavailable {
            reason: "the configuration names no words_dir".to_owned(),
        }
        .into()
    })
}

fn body<T>(given: Result<Json<T>, JsonRejection>) -> Result<T, ServerError> {
    given
        .map(|Json(body)| body)
        .map_err(|refused| ServerError::RequestMalformed {
            reason: refused.body_text(),
        })
}

/// The signed-in actor's name for a receipt: their person, or `operator`
/// for the install's operator token, which is bound to no person.
fn named(state: &AppState, actor: &lys_identity::Actor) -> Result<String, ServerError> {
    match with_directory(state, |directory| {
        own_person(directory.projection()?, actor)
    }) {
        Ok(person) => Ok(person.to_string()),
        Err(ServerError::NoPerson) => Ok("operator".to_owned()),
        Err(error) => Err(error),
    }
}

/// The administrator, by name.
fn administrator(state: &AppState, headers: &HeaderMap) -> Result<String, ServerError> {
    let actor = signed_in(state, headers)?;
    if !is_administrator(state, &actor)? {
        return Err(ServerError::NotAdmitted {
            reason: "only the administrator sets the workspace's words and templates",
        });
    }
    named(state, &actor)
}

/// The caller admitted to an agent's words: its responsible person or the
/// administrator, by name.
fn over_agent(state: &AppState, headers: &HeaderMap, agent: &str) -> Result<String, ServerError> {
    let actor = signed_in(state, headers)?;
    let agent_id = AgentId::from_str(agent).map_err(|_unread| ServerError::AgentNotVisible)?;
    if is_administrator(state, &actor)? {
        return named(state, &actor);
    }
    let person = with_directory(state, |directory| {
        own_person(directory.projection()?, &actor)
    })?;
    let responsible = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let record = projection
            .record(IdentityId::Agent(agent_id))
            .ok_or(ServerError::AgentNotVisible)?;
        Ok(record.responsible())
    })?;
    if responsible == Some(person) {
        return Ok(person.to_string());
    }
    Err(ServerError::AgentNotVisible)
}

async fn read_all(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Words>, ServerError> {
    signed_in(&state, &headers)?;
    Ok(Json(crate::words_store::held(words(&state)?)?))
}

async fn save_workspace(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(slot): Path<String>,
    given: Result<Json<SaveBody>, JsonRejection>,
) -> Result<Json<Saved>, ServerError> {
    let body = body(given)?;
    let by = administrator(&state, &headers)?;
    let slot = Slot::named(&slot)?;
    save(&state, Layer::Workspace, slot, body, by)
}

async fn save_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    given: Result<Json<TemplateBody>, JsonRejection>,
) -> Result<Json<Saved>, ServerError> {
    let body = body(given)?;
    let by = administrator(&state, &headers)?;
    let revision =
        crate::words_store::template(words(&state)?, &name, &body.text, body.revision, &by)?;
    Ok(Json(Saved { revision }))
}

async fn read_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ForAgent>, ServerError> {
    over_agent(&state, &headers, &id)?;
    let held = crate::words_store::held(words(&state)?)?;
    let layer = Layer::Agent { id: id.clone() };
    Ok(Json(ForAgent {
        resolved: Slot::ALL
            .into_iter()
            .map(|slot| held.resolve(slot, Some(&id), None))
            .collect(),
        held: held
            .layers
            .get(&layer.key())
            .map(|slots| {
                slots
                    .iter()
                    .map(|(slot, held)| (slot.name().to_owned(), held.clone()))
                    .collect()
            })
            .unwrap_or_default(),
        agent: id,
    }))
}

async fn save_agent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, slot)): Path<(String, String)>,
    given: Result<Json<SaveBody>, JsonRejection>,
) -> Result<Json<Saved>, ServerError> {
    let body = body(given)?;
    let by = over_agent(&state, &headers, &id)?;
    let slot = Slot::named(&slot)?;
    save(&state, Layer::Agent { id }, slot, body, by)
}

async fn save_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((id, slot)): Path<(String, String)>,
    given: Result<Json<SaveBody>, JsonRejection>,
) -> Result<Json<Saved>, ServerError> {
    let body = body(given)?;
    let agent = session_agent(&state, &id)?;
    let by = over_agent(&state, &headers, &agent)?;
    let slot = Slot::named(&slot)?;
    save(&state, Layer::Session { id }, slot, body, by)
}

fn save(
    state: &AppState,
    layer: Layer,
    slot: Slot,
    body: SaveBody,
    by: String,
) -> Result<Json<Saved>, ServerError> {
    let revision = crate::words_store::set(
        words(state)?,
        Save {
            layer,
            slot,
            setting: body.setting,
            revision: body.revision,
            by,
        },
    )?;
    Ok(Json(Saved { revision }))
}

async fn preview(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    given: Result<Json<PreviewBody>, JsonRejection>,
) -> Result<Json<Delivered>, ServerError> {
    let body = body(given)?;
    let agent = match (&body.agent, &body.session) {
        (Some(agent), _) => Some(agent.clone()),
        (None, Some(session)) => Some(session_agent(&state, session)?),
        (None, None) => None,
    };
    match &agent {
        Some(agent) => {
            over_agent(&state, &headers, agent)?;
        }
        None => {
            administrator(&state, &headers)?;
        }
    }
    let delivered = deliverable(
        &state,
        body.slot,
        agent.as_deref(),
        body.session.as_deref(),
        body.numbers,
        None,
    )?;
    Ok(Json(delivered))
}

/// The words of `slot` for `session` of `agent`, resolved and rendered now
/// with `numbers` and, for a reminder, the goal's `deadline`; none when no
/// layer and no built-in sets the slot (the compaction command the profile
/// names). Nothing is sent: the caller writes the result in its receipt and
/// then delivers.
pub(crate) fn deliverable(
    state: &AppState,
    slot: Slot,
    agent: Option<&str>,
    session: Option<&str>,
    numbers: BTreeMap<String, String>,
    deadline: Option<u64>,
) -> Result<Delivered, ServerError> {
    let resolved = match state.words.as_ref() {
        Some(words) => crate::words_store::resolve(words, slot, agent, session)?,
        None => Words::default().resolve(slot, agent, session),
    };
    let Some(template) = resolved.text else {
        return Err(WordsError::Malformed {
            reason: format!(
                "no layer sets the {} slot and it has no built-in wording",
                slot.name()
            ),
        }
        .into());
    };
    let (rendered, read) = rendered(state, &template, agent, session, numbers, deadline)?;
    let mut contributed = resolved.contributed;
    contributed.extend(read);
    Ok(Delivered {
        slot,
        text: rendered.text,
        source: resolved.source,
        contributed,
        missing: rendered.missing,
    })
}

/// Render `template` for `session` of `agent` from the variables and goals
/// as they read now.
pub(crate) fn rendered(
    state: &AppState,
    template: &str,
    agent: Option<&str>,
    session: Option<&str>,
    numbers: BTreeMap<String, String>,
    deadline: Option<u64>,
) -> Result<(Rendered, Vec<Contributed>), ServerError> {
    let at = now();
    let mut inputs = Inputs {
        numbers,
        deadline,
        now: at,
        ..Inputs::default()
    };
    if let Some(variables) = state.variables.as_ref() {
        if let Some(session) = session {
            inputs.session = Some(scope(
                variables,
                Scope::Session {
                    id: session.to_owned(),
                },
            )?);
        }
        if let Some(agent) = agent {
            inputs.agent = Some(scope(
                variables,
                Scope::Agent {
                    id: agent.to_owned(),
                },
            )?);
        }
    }
    if let (Some(goals), Some(agent)) = (state.goals.as_ref(), agent) {
        let holder = crate::goals_state::Holder {
            kind: crate::goals_state::HolderKind::Agent,
            id: agent.to_owned(),
        };
        let (items, revision) =
            goals.with(|store| Ok((store.of_holder(&holder), store.revision())))?;
        inputs.goals = items
            .into_iter()
            .filter(|item| item.standing == Standing::Open && item.active())
            .map(|item| GoalLine {
                id: item.goal.id.clone(),
                kind: kind_name(item.goal.kind).to_owned(),
                words: item.words().to_owned(),
                deadline: item.goal.deadline,
            })
            .collect();
        inputs.goals_revision = Some(revision);
    }
    let rendered = render(template, &inputs);
    let read = rendered
        .contributed
        .iter()
        .map(|read| Contributed {
            kind: read.kind.clone(),
            key: read.key.clone(),
            revision: read.revision,
        })
        .collect();
    Ok((rendered, read))
}

fn scope(
    variables: &crate::variables_store::VariablesKept,
    scope: Scope,
) -> Result<lys_runner::render::Scope, ServerError> {
    let read = crate::variables_store::read(variables, &scope)?;
    Ok(lys_runner::render::Scope {
        name: scope.name(),
        revision: read.revision,
        values: read
            .values
            .into_iter()
            .map(|(name, variable)| {
                (
                    name,
                    Variable {
                        value: variable.value,
                    },
                )
            })
            .collect(),
    })
}

fn kind_name(kind: crate::goals_state::Kind) -> &'static str {
    match kind {
        crate::goals_state::Kind::Goal => "goal",
        crate::goals_state::Kind::Expectation => "expectation",
        crate::goals_state::Kind::Deliverable => "deliverable",
    }
}
