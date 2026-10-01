//! Nesting changes visibility and leaves grant authority unchanged.

use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::{Actor, AgentId, OperationId};
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::read_api::{login, own_person};
use crate::read_views::Login;
use crate::routes::{AppState, signed_in, with_directory};
use crate::session::now;
use crate::teams_api::{
    DESCRIPTION_MAX, NAME_MAX, TeamChanged, change, kept, malformed, team_id, with_teams, words,
};
use crate::teams_state::{Created, Held, Line, Refused};

/// A creation whose event version carries the tree position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreatedV1 {
    /// The unchanged creation record.
    pub created: Created,
    /// The parent, when this team is nested.
    pub parent: Option<String>,
    /// The lead, when an admitted member is named.
    pub lead: Option<String>,
}

/// One explicit replacement of a team's tree position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NestedV1 {
    /// The operation naming this change.
    pub operation: String,
    /// The team changed.
    pub team: String,
    /// Its new parent, or none for a top-level team.
    pub parent: Option<String>,
    /// Its new lead, or none for no lead.
    pub lead: Option<String>,
    /// The caller who made the change.
    pub by: Login,
    /// When the change was kept.
    pub at: u64,
}

/// A team creation, with an optional tree position.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = TeamCreateBody)]
pub(crate) struct CreateBody {
    operation: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    lead: Option<String>,
}

/// Replace the parent and lead under one idempotent operation.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = TeamNestingBody)]
pub(crate) struct NestingBody {
    operation: String,
    parent: Option<String>,
    lead: Option<String>,
}

/// Check against the complete held tree, before writing and when replaying.
pub(crate) fn allows(
    held: &Held,
    id: &str,
    parent: Option<&str>,
    lead: Option<&str>,
) -> Result<(), Refused> {
    if let Some(parent) = parent {
        let mut visited = BTreeSet::new();
        let mut ancestor = Some(parent);
        while let Some(next) = ancestor {
            if next == id || !visited.insert(next) {
                return Err(Refused::ParentCycle {
                    team: id.to_owned(),
                    parent: parent.to_owned(),
                });
            }
            let team = held.team(next).ok_or(Refused::Unknown)?;
            if team.retired.is_some() {
                return Err(Refused::Retired);
            }
            ancestor = team.parent.as_deref();
        }
    }
    if let Some(lead) = lead {
        let admitted = AgentId::from_str(lead).is_ok()
            && held.team(id).is_some_and(|team| {
                team.members.iter().any(|member| member == lead)
                    && !team.held.iter().any(|member| member.member == lead)
            });
        if !admitted {
            return Err(Refused::LeadNotMember {
                team: id.to_owned(),
                lead: lead.to_owned(),
            });
        }
    }
    Ok(())
}

fn position(
    parent: Option<&str>,
    lead: Option<String>,
) -> Result<(Option<String>, Option<String>), ServerError> {
    let parent = parent.map(team_id).transpose()?;
    let lead = lead
        .map(|lead| {
            AgentId::from_str(&lead)
                .map(|id| id.to_string())
                .map_err(|error| malformed(format!("lead is not an agent id: {error}")))
        })
        .transpose()?;
    Ok((parent, lead))
}

pub(crate) async fn create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<CreateBody>, JsonRejection>,
) -> Result<Json<TeamChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = OperationId::from_str(&body.operation)?.to_string();
    let name = words("name", &body.name, NAME_MAX)?;
    if name.is_empty() {
        return Err(malformed("a team has a name"));
    }
    let description = words("description", &body.description, DESCRIPTION_MAX)?;
    let (parent, lead) = position(body.parent.as_deref(), body.lead)?;
    if parent.is_some() || lead.is_some() {
        crate::teams_migration::require_committed(&state)?;
    }
    parent_owned(&state, &actor, parent.as_deref())?;
    crate::teams_migration::advance(&state)?;
    with_directory(&state, |directory| {
        let owner = own_person(directory.projection()?, &actor)?;
        let created = Created {
            id,
            owner: owner.to_string(),
            name,
            description,
            by: login(actor.binding()),
            at: now(),
        };
        let line = if parent.is_some() || lead.is_some() {
            Line::CreatedV1(CreatedV1 {
                created,
                parent,
                lead,
            })
        } else {
            Line::Created(created)
        };
        with_teams(&state, |store| kept(store, line))
    })
    .map(Json)
}

pub(crate) async fn nest(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<NestingBody>, JsonRejection>,
) -> Result<Json<TeamChanged>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let Json(body) = body.map_err(|refused| malformed(refused.body_text()))?;
    let id = team_id(&id)?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let (parent, lead) = position(body.parent.as_deref(), body.lead)?;
    crate::teams_migration::require_committed(&state)?;
    parent_owned(&state, &actor, parent.as_deref())?;
    change(&state, &actor, &id, |team, by| {
        Ok(Line::NestedV1(NestedV1 {
            operation,
            team,
            parent,
            lead,
            by,
            at: now(),
        }))
    })
    .map(Json)
}

fn parent_owned(state: &AppState, actor: &Actor, parent: Option<&str>) -> Result<(), ServerError> {
    let Some(parent) = parent else {
        return Ok(());
    };
    if crate::routes::is_administrator(state, actor)? {
        return Ok(());
    }
    with_directory(state, |directory| {
        let own = own_person(directory.projection()?, actor)?.to_string();
        with_teams(state, |store| {
            if store
                .team(parent)
                .is_none_or(|team| team.created.owner != own)
            {
                return Err(ServerError::NotAdmitted {
                    reason: "only a parent team's owner or the administrator nests a team under it",
                });
            }
            Ok(())
        })
    })
}
