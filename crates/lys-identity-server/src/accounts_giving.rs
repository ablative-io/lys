//! Account enablement under a live grant and current shared membership.

use lys_identity::grants::{Action, ExerciseRequest, Resource, Route};
use lys_identity::{Actor, IdentityId, PersonId};

use crate::error::ServerError;
use crate::error_team::TeamError;
use crate::grants::{Decision, decide_as, with_grants};
use crate::routes::AppState;
use crate::session::now;

fn admitted(
    state: &AppState,
    actor: &Actor,
    person: PersonId,
    enabled: bool,
    expected: Option<&str>,
    decision: Decision,
) -> Result<String, ServerError> {
    with_grants(state, |mut judged| {
        crate::caller_admission::active_caller(judged.directory, actor)?;
        let agent = actor.provenance().agent().ok_or(ServerError::NoPerson)?;
        judged.apps.admit_kind(None, "account")?;
        judged.apps.admit_action("account", "write")?;
        let request = ExerciseRequest {
            caller: IdentityId::Agent(agent),
            route: Route::Api,
            resource: Resource::new("account", &person.to_string())?,
            action: Action::new("write")?,
        };
        let at = now();
        decide_as(actor, &mut judged, &request, at, None, Decision::Explain)?;
        let target = judged
            .directory
            .record(IdentityId::Person(person))
            .ok_or_else(|| lys_identity::IdentityError::IdentityUnknown {
                identity: person.to_string(),
            })?;
        if !enabled {
            if judged
                .directory
                .record(IdentityId::Agent(agent))
                .and_then(lys_identity::projection::Record::responsible)
                == Some(person)
            {
                return Err(ServerError::NotAdmitted {
                    reason: "an agent cannot disable its responsible person's sign-in",
                });
            }
            if person == judged.root {
                return Err(ServerError::NotAdmitted {
                    reason: "an agent cannot disable an administrator's sign-in",
                });
            }
        }
        let account = target
            .bindings()
            .iter()
            .find(|binding| binding.issuer() == state.oidc.issuer())
            .map(|binding| binding.subject().to_owned())
            .ok_or_else(|| super::refused("this person has no Lys account to change"))?;
        if expected.is_some_and(|expected| expected != account) {
            return Err(super::refused(
                "the person's sign-in account changed before the update",
            ));
        }
        let teams = state.teams.as_ref().ok_or_else(|| TeamError::Unavailable {
            reason: "the configuration names no teams_dir".to_owned(),
        })?;
        let mut teams = teams.lock().map_err(|error| TeamError::Unavailable {
            reason: format!("the teams lock is poisoned: {error}"),
        })?;
        teams.settle()?;
        crate::routes::people_giving::holds(teams.teams_iter(), agent, person)?;
        if decision == Decision::Exercise {
            decide_as(actor, &mut judged, &request, at, None, decision)?;
            if !enabled {
                state.sessions.revoke_matching(|actor| {
                    judged.directory.person_for(actor.binding()) == Some(person)
                })?;
                if let Some(provider) = &state.provider {
                    provider.revoke_person(&person.to_string())?;
                }
            }
        }
        Ok(account)
    })
}

pub(super) async fn enabled(
    state: &AppState,
    actor: &Actor,
    person: PersonId,
    enabled: bool,
) -> Result<axum::Json<serde_json::Value>, ServerError> {
    let id = admitted(state, actor, person, enabled, None, Decision::Explain)?;
    let api = super::api(state)?;
    let guard = api.lock_account(&id).await?;
    let mut update = super::update_of(&super::read(api, &id).await?)?;
    let fields = update
        .as_object_mut()
        .ok_or_else(|| ServerError::SignInProvidersUnavailable {
            reason: "the sign-in account update is not an object".to_owned(),
        })?;
    fields.remove("password");
    fields.insert("enabled".to_owned(), serde_json::Value::Bool(enabled));
    admitted(state, actor, person, enabled, Some(&id), Decision::Exercise)?;
    api.call(reqwest::Method::PUT, &format!("/users/{id}"), Some(&update))
        .await
        .map_err(super::account_refusal)?;
    let answer = super::shown(api, &id).await;
    drop(guard);
    answer
}
