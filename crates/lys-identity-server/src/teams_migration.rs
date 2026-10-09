//! Recheck legacy memberships before any team action. During a reversible
//! upgrade the holds exist only in memory: no new leaf or snapshot can prevent
//! the previous binary reading its store. The next mutation after commit, or
//! the next start, durably completes the same deterministic migration.

use std::str::FromStr;

use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{AgentId, IdentityId, LoginBinding};
use sha2::{Digest, Sha256};

use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::grants::with_grants;
use crate::routes::{AppState, hex, with_directory};
use crate::session::now;
use crate::teams_api::with_teams;
use crate::teams_state::{Changed, Checked, Hold, Line};

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::Team(TeamError::Unavailable {
        reason: reason.into(),
    })
}

fn pending(state: &AppState) -> Result<bool, ServerError> {
    crate::operator::upgrade_pending(state).map_err(|error| {
        unavailable(format!(
            "membership migration cannot read upgrade intent {}: {error}",
            state.operator_upgrade_file.as_deref().map_or_else(
                || "(unmanaged)".to_owned(),
                |path| path.display().to_string()
            )
        ))
    })
}

/// New-format team events wait until rollback is closed.
pub fn require_committed(state: &AppState) -> Result<(), ServerError> {
    if pending(state)? {
        return Err(unavailable(
            "upgrade_pending: new-format team events are refused while the upgrade is reversible",
        ));
    }
    Ok(())
}

/// Finish pending migration on a family admission, never on a clock or a read.
pub fn advance(state: &AppState) -> Result<(), ServerError> {
    if state.teams.is_some() && !pending(state)? {
        with_teams(state, crate::teams_store::TeamStore::finish_migration)?;
    }
    Ok(())
}

/// Derive and enforce legacy holds before reminders or budget settlement start.
pub fn at_start(state: &AppState) -> Result<(), ServerError> {
    if state.teams.is_none() {
        return Ok(());
    }
    let teams = with_teams(state, |store| {
        Ok(if store.migration_checked() {
            None
        } else {
            Some(store.teams().to_vec())
        })
    })?;
    if let Some(teams) = teams {
        let mut lines = Vec::new();
        for team in teams.iter().filter(|team| team.retired.is_none()) {
            for member in &team.members {
                if team.held.iter().any(|hold| hold.member == *member) {
                    continue;
                }
                let added = team
                    .changes
                    .iter()
                    .rev()
                    .find_map(|line| match line {
                        Line::Added(added) if added.member == *member => Some(added),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        unavailable(format!(
                            "team `{}` member `{member}` has no adding record",
                            team.created.id
                        ))
                    })?;
                if !permitted(state, added)? {
                    let digest = Sha256::digest(format!(
                        "lys/teams/legacy-membership/v1\n{}",
                        added.operation
                    ));
                    lines.push(Line::Held(Hold {
                        operation: format!("op-{}", &hex(&digest)[..32]),
                        team: team.created.id.clone(), member: member.clone(),
                        reason: format!("membership added by {}/{} under operation `{}` has no current authority for this member", added.by.provider, added.by.subject, added.operation),
                        at: added.at,
                    }));
                }
            }
        }
        lines.push(Line::Checked(Checked {
            operation: "lys/teams/legacy-membership/v1/checked".to_owned(),
            at: now(),
        }));
        with_teams(state, |store| store.stage_migration(lines))?;
    }
    advance(state)?;
    Ok(())
}

/// Read historical attribution as evidence, never forge a new authenticated actor.
fn permitted(state: &AppState, added: &Changed) -> Result<bool, ServerError> {
    let binding = LoginBinding::new(&added.by.provider, &added.by.subject)?;
    if state.admission.administrator_login()?.as_ref() == Some(&binding) {
        return Ok(true);
    }
    let caller = with_directory(state, |directory| {
        Ok(directory.projection()?.person_for(&binding))
    })?;
    let Some(caller) = caller else {
        return Ok(false);
    };
    let Ok(agent) = AgentId::from_str(&added.member) else {
        return Ok(caller.to_string() == added.member);
    };
    let responsible = with_directory(state, |directory| {
        Ok(directory
            .projection()?
            .record(IdentityId::Agent(agent))
            .is_some_and(|record| record.responsible() == Some(caller)))
    })?;
    if responsible {
        return Ok(true);
    }
    with_grants(state, |judged| {
        let request = ExerciseRequest {
            caller: IdentityId::Person(caller),
            route: Route::Api,
            resource: Resource::new("agent", &added.member)?,
            action: Action::new(crate::runner_sessions::OPERATE)?,
        };
        Ok(crate::grants::allowed(judged.grants.explain(
            judged.directory,
            &request,
            now(),
            None,
        ))?)
    })
}

#[cfg(test)]
pub(crate) fn test_permitted(state: &AppState, added: &Changed) -> Result<bool, ServerError> {
    permitted(state, added)
}
