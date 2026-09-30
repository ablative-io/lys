//! The caller's tree is computed on the request from the service's held state.

use std::collections::BTreeMap;
use std::sync::{Arc, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::routing::get;
use axum::{Json, Router};
use lys_identity::IdentityId;

use crate::agent_signature::signed_agent;
use crate::budgets_api::with_budgets;
use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::goals_state::{GoalError, Holder, HolderKind, Standing as GoalStanding};
use crate::grants::caller;
use crate::provisioning_api::with_provisioning;
use crate::routes::{AppState, with_directory};
use crate::runtime_api::with_runtime;
use crate::teams_api::with_teams;
use crate::tree_views::{Root, TreeState, TreeTeam, TreeView, reach};

/// One read, with no task or runner request started by it.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/tree", get(read))
}

async fn read(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    uri: Uri,
) -> Result<Json<TreeView>, ServerError> {
    let (identity, directory) = with_directory(&state, |store| {
        let directory = store.projection()?;
        let path = uri
            .path_and_query()
            .map_or(uri.path(), axum::http::uri::PathAndQuery::as_str);
        let identity =
            if let Some(agent) = signed_agent(&state, directory, &headers, ("GET", path, &[]))? {
                IdentityId::Agent(agent)
            } else {
                caller(&state, &headers, directory)?
            };
        if matches!(identity, IdentityId::ServiceAccount(_)) {
            return Err(ServerError::NoPerson);
        }
        Ok((identity, directory.clone()))
    })?;
    let root = Root {
        id: identity.to_string(),
        name: directory
            .record(identity)
            .ok_or(ServerError::NoPerson)?
            .profile()
            .display_name()
            .to_owned(),
    };
    let (at, at_ms) = instant()?;
    let teams = with_teams(&state, |store| Ok(store.teams().to_vec()))?;
    let reachable = reach(&teams, identity);
    if reachable.is_empty() {
        return Ok(Json(TreeView {
            root,
            teams: Vec::new(),
        }));
    }
    let mut held = TreeState {
        directory,
        teams,
        roles: roles(&state)?,
        profiles: with_provisioning(&state, |store| Ok(store.profiles().to_vec()))?,
        sessions: with_runtime(&state, |store| Ok(store.sessions().to_vec()))?,
        budgets: with_budgets(&state, |store| Ok(store.held().clone()))?,
        goals: BTreeMap::new(),
        at,
        at_ms,
    };
    let standings = held.standings();
    held.goals = goals(&state, &standings)?;
    let mut built = BTreeMap::new();
    let mut pending: Vec<_> = held
        .teams
        .iter()
        .filter(|team| reachable.contains(&team.created.id))
        .collect();
    // Build children first so the walk itself does not recurse on request depth.
    while !pending.is_empty() {
        let before = pending.len();
        let mut remaining = Vec::new();
        for team in pending {
            let children: Vec<_> = held
                .teams
                .iter()
                .filter(|child| {
                    child.parent.as_deref() == Some(team.created.id.as_str())
                        && reachable.contains(&child.created.id)
                })
                .collect();
            if children
                .iter()
                .any(|child| !built.contains_key(&child.created.id))
            {
                remaining.push(team);
                continue;
            }
            let members = team
                .members
                .iter()
                .filter(|member| !team.held.iter().any(|hold| hold.member == **member))
                .map(|member| held.agent(member, &standings))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();
            let lead = team
                .lead
                .as_deref()
                .map(|lead| held.agent(lead, &standings))
                .transpose()?
                .flatten();
            let teams = children
                .iter()
                .map(|child| {
                    built.remove(&child.created.id).ok_or_else(|| {
                        unavailable("a child vanished while rendering the held tree")
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            built.insert(
                team.created.id.clone(),
                TreeTeam {
                    id: team.created.id.clone(),
                    name: team.created.name.clone(),
                    lead,
                    members,
                    teams,
                },
            );
        }
        if remaining.len() == before {
            return Err(unavailable("the stored parent links contain a cycle"));
        }
        pending = remaining;
    }
    let teams = held
        .teams
        .iter()
        .filter(|team| {
            reachable.contains(&team.created.id)
                && !team
                    .parent
                    .as_ref()
                    .is_some_and(|parent| reachable.contains(parent))
        })
        .map(|team| {
            built
                .remove(&team.created.id)
                .ok_or_else(|| unavailable("a reachable root was not rendered"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(TreeView { root, teams }))
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::Team(TeamError::Unavailable {
        reason: reason.into(),
    })
}

fn instant() -> Result<(u64, i64), ServerError> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| unavailable(format!("tree read time: {error}")))?;
    let millis = i64::try_from(elapsed.as_millis())
        .map_err(|error| unavailable(format!("tree read milliseconds: {error}")))?;
    Ok((elapsed.as_secs(), millis))
}

fn roles(state: &AppState) -> Result<Vec<crate::roles_records::Role>, ServerError> {
    let roles = state
        .roles
        .as_ref()
        .ok_or_else(|| ServerError::RolesUnavailable {
            reason: "the configuration names no roles_file".to_owned(),
        })?;
    let mut roles = roles.lock().unwrap_or_else(PoisonError::into_inner);
    roles.settle()?;
    Ok(roles.roles().to_vec())
}

fn goals(
    state: &AppState,
    standings: &[crate::budgets_state::Standing],
) -> Result<BTreeMap<String, Vec<String>>, ServerError> {
    let goals = state.goals.as_ref().ok_or_else(|| GoalError::Unavailable {
        reason: "the configuration names no goals_dir".to_owned(),
    })?;
    goals.with(|store| {
        let mut answers = BTreeMap::new();
        for standing in standings {
            let mut items = store.of_holder(&Holder {
                kind: HolderKind::Agent,
                id: standing.agent.clone(),
            });
            for team in &standing.teams {
                items.extend(store.of_holder(&Holder {
                    kind: HolderKind::Team,
                    id: team.clone(),
                }));
            }
            answers.insert(
                standing.agent.clone(),
                items
                    .into_iter()
                    .filter(|item| item.standing == GoalStanding::Open)
                    .map(|item| item.goal.words)
                    .collect(),
            );
        }
        Ok(answers)
    })
}
