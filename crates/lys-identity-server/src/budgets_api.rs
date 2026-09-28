//! Budgets on agents, teams and people, read and changed through the API.
//!
//! A budget is set by its holder's responsible person or an administrator:
//! an agent's by the person responsible for it, a team's by its owner, a
//! person's by that person. Reads are checked as changes are, so another
//! person's budget is refused `not_permitted` and its values are never
//! shown. A change names the version it was read at, and a change that
//! crossed another is refused `BudgetVersionConflict`.

use std::str::FromStr;
use std::sync::{Arc, PoisonError};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::{Actor, AgentId, IdentityId};
use serde::{Deserialize, Serialize};

use crate::budgets_state::{Act, Budget, Holder, HolderKind, Measure, Period};
use crate::budgets_store::BudgetStore;
use crate::error::ServerError;
use crate::read_api::own_person;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;

/// A budget to set on a holder.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct BudgetBody {
    measure: Measure,
    limit: u64,
    #[serde(default)]
    period: Option<Period>,
    act: Act,
    /// The version read before this change; 0 for a budget not yet set.
    version: u64,
}

/// The budgets held on one holder.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct BudgetsView {
    /// The holder.
    pub holder: Holder,
    /// Its budgets, one for each measure set.
    pub budgets: Vec<Budget>,
}

/// The budget routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/budgets/{kind}/{id}", get(read).put(set))
}

fn kind_of(kind: &str) -> Result<HolderKind, ServerError> {
    match kind {
        "agent" => Ok(HolderKind::Agent),
        "team" => Ok(HolderKind::Team),
        "person" => Ok(HolderKind::Person),
        other => Err(ServerError::BudgetRefused {
            refusal: "holder_unknown",
            words: format!("{other} is not a holder: agent, team or person"),
        }),
    }
}

fn refused(holder: &Holder) -> ServerError {
    ServerError::NotPermitted {
        reason: format!(
            "only the responsible person or an administrator reads or sets budgets on {} {}",
            kind_name(holder.kind),
            holder.id
        ),
    }
}

fn kind_name(kind: HolderKind) -> &'static str {
    match kind {
        HolderKind::Agent => "agent",
        HolderKind::Team => "team",
        HolderKind::Person => "person",
    }
}

/// The caller's person, once the caller is proved to hold authority over
/// `holder`: an administrator, or the person responsible for it.
fn authorised(state: &AppState, actor: &Actor, holder: &Holder) -> Result<String, ServerError> {
    let administrator = state.admission.administrator(actor).is_ok();
    let (person, responsible) = with_directory(state, |directory| {
        let projection = directory.projection()?;
        let person = own_person(projection, actor)?.to_string();
        let responsible = match holder.kind {
            HolderKind::Agent => AgentId::from_str(&holder.id)
                .ok()
                .and_then(|agent| projection.record(IdentityId::Agent(agent)))
                .and_then(lys_identity::projection::Record::responsible)
                .map(|owner| owner.to_string()),
            HolderKind::Person => Some(holder.id.clone()),
            HolderKind::Team => None,
        };
        Ok((person, responsible))
    })?;
    let responsible = match holder.kind {
        HolderKind::Team => crate::teams_api::with_teams(state, |store| {
            Ok(store
                .team(&holder.id)
                .map(|team| team.created.owner.clone()))
        })?,
        HolderKind::Agent | HolderKind::Person => responsible,
    };
    if administrator || responsible.as_deref() == Some(person.as_str()) {
        Ok(person)
    } else {
        Err(refused(holder))
    }
}

fn with_budgets<T>(
    state: &AppState,
    act: impl FnOnce(&mut BudgetStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state
        .budgets
        .as_ref()
        .ok_or_else(|| ServerError::BudgetsUnavailable {
            reason: "the configuration names no budgets_dir".to_owned(),
        })?;
    let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
    store.settle()?;
    act(&mut store)
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Json<BudgetsView>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let holder = Holder {
        kind: kind_of(&kind)?,
        id,
    };
    authorised(&state, &actor, &holder)?;
    let budgets = with_budgets(&state, |store| {
        Ok(store
            .held()
            .budgets
            .iter()
            .filter(|budget| budget.holder == holder)
            .cloned()
            .collect())
    })?;
    Ok(Json(BudgetsView { holder, budgets }))
}

async fn set(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
    body: Result<Json<BudgetBody>, JsonRejection>,
) -> Result<Json<Budget>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|rejected| ServerError::BudgetRefused {
        refusal: "budget_malformed",
        words: rejected.body_text(),
    })?;
    let holder = Holder {
        kind: kind_of(&kind)?,
        id,
    };
    let by = authorised(&state, &actor, &holder)?;
    let budget = Budget {
        holder,
        measure: body.measure,
        limit: body.limit,
        period: body.period,
        act: body.act,
        version: body.version + 1,
        by,
        at: now(),
    }
    .checked()
    .map_err(|refused| ServerError::BudgetRefused {
        refusal: refused.refusal,
        words: refused.words,
    })?;
    with_budgets(&state, |store| store.set(budget, body.version)).map(Json)
}
