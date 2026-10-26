//! One administrator act moves a login; grants stay on their people.
use std::str::FromStr;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use lys_identity::{
    Actor, Change, IdentityEvent, IdentityId, LifecycleState, LoginBinding, OperationId,
    OrganisationId, PersonId, Profile, Transition,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::routes::{AppState, Shared, signed_in, with_directory};

/// Exactly one existing person or one new display name.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
#[schema(as = PeopleLoginMoveTarget)]
pub(crate) enum Target {
    Existing(Existing),
    New(New),
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = PeopleLoginMoveExisting)]
pub(crate) struct Existing {
    person: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = PeopleLoginMoveNew)]
pub(crate) struct New {
    display_name: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = PeopleLoginMoveBody)]
pub(crate) struct Move {
    operation: String,
    issuer: String,
    subject: String,
    target: Target,
}

struct Moving {
    actor: Actor,
    operation: OperationId,
    binding: LoginBinding,
    source: PersonId,
    target: PersonId,
    profile: Option<Profile>,
}

impl Moving {
    fn events(&self, organisation: OrganisationId) -> Result<Vec<IdentityEvent>, ServerError> {
        let now = crate::session::now();
        let event = |step, person, change| -> Result<IdentityEvent, ServerError> {
            Ok(IdentityEvent::new(
                OperationId::from_str(&crate::people_admit::derived(&self.operation, step))?,
                self.actor.clone(),
                IdentityId::Person(person),
                now,
                change,
            )?)
        };
        let mut events = vec![event(
            "unbind",
            self.source,
            Change::UnbindLogin {
                binding: self.binding.clone(),
            },
        )?];
        if let Some(profile) = &self.profile {
            events.push(event(
                "register",
                self.target,
                Change::RegisterPerson {
                    profile: profile.clone(),
                    organisation: Some(organisation),
                },
            )?);
        }
        events.push(event(
            "bind",
            self.target,
            Change::BindLogin {
                binding: self.binding.clone(),
            },
        )?);
        if self.profile.is_some() {
            events.push(event(
                "activate",
                self.target,
                Change::Transition {
                    transition: Transition::Activate,
                    from: LifecycleState::Registered,
                    to: LifecycleState::Active,
                    reason: "sign-in moved to its rightful person".to_owned(),
                },
            )?);
        }
        Ok(events)
    }

    fn commit(&self, state: &AppState) -> Result<Vec<Value>, ServerError> {
        with_directory(state, |directory| {
            let projection = directory.projection()?;
            if projection.person_for(&self.binding) != Some(self.source) {
                return Err(ServerError::LoginNotBound);
            }
            if self.target == self.source {
                return Err(ServerError::SameTarget);
            }
            let organisation = projection
                .organisation_of(IdentityId::Person(self.source))
                .ok_or(ServerError::NotSameOrganisation)?;
            if self.profile.is_none() {
                let record = projection
                    .record(IdentityId::Person(self.target))
                    .ok_or(ServerError::NotSameOrganisation)?;
                if record.organisation() != Some(organisation) {
                    return Err(ServerError::NotSameOrganisation);
                }
            }
            let events = self.events(organisation)?;
            // Refuse before the batch if sessions cannot be durably ended.
            // Every named refusal above precedes both writes.
            state
                .sessions
                .revoke_matching(|held| held.binding() == &self.binding)?;
            directory
                .commit_batch(&events)?
                .iter()
                .map(crate::directory_views::receipt_json)
                .collect()
        })
    }
}

pub(crate) async fn move_login(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Result<Json<Move>, JsonRejection>,
) -> Result<Json<Value>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    let source = PersonId::from_str(&id)?;
    crate::people_admit_handed::administrator(&state, &actor, source)?;
    let Json(body) = body.map_err(|error| ServerError::RequestMalformed {
        reason: error.body_text(),
    })?;
    let (target, profile) = match body.target {
        Target::Existing(Existing { person }) => (PersonId::from_str(&person)?, None),
        Target::New(New { display_name }) => {
            (PersonId::generate()?, Some(Profile::new(&display_name)?))
        }
    };
    let moving = Moving {
        actor,
        operation: OperationId::from_str(&body.operation)?,
        binding: LoginBinding::new(&body.issuer, &body.subject)?,
        source,
        target,
        profile,
    };
    // Admission and account changes take this same account hold before the
    // directory, so a session cannot slip across a completed move.
    let guard = match state.sign_in_providers.as_ref() {
        Some(api) if moving.binding.issuer() == state.oidc.issuer() => {
            Some(api.lock_account(moving.binding.subject()).await?)
        }
        _ => None,
    };
    let actor = moving.actor.clone();
    let receipts = moving.commit(&state)?;
    drop(guard);
    // The moved login may leave a dead one behind, or an address free for
    // a person waiting on it (DIRECTORY-124 R2).
    crate::sign_in_reconcile::after_change(&state, &actor).await;
    let (source_grants, target_grants) = crate::grants::with_grants(&state, |judged| {
        let ids = |person| {
            judged
                .grants
                .book()
                .held_by(IdentityId::Person(person))
                .filter(|held| held.revoked().is_none())
                .map(|held| held.grant().id().to_string())
                .collect::<Vec<_>>()
        };
        Ok((ids(source), ids(target)))
    })?;
    Ok(Json(json!({
        "source": source.to_string(), "target": target.to_string(), "receipts": receipts,
        "source_grants": source_grants, "target_grants": target_grants,
    })))
}
