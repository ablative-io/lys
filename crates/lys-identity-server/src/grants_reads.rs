//! Settle under the authority hold, read outside it, then judge only at that revision.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, GrantError, RelationshipStore, Resource};

use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;
use crate::grants::{Judged, with_grants, with_prepared_directory_grants};
use crate::routes::AppState;
use crate::spicedb::Relationships;

pub(crate) use crate::spicedb::context::Data as Prepared;
use crate::spicedb::context::Hold;

impl Prepared {
    pub(crate) fn frame<'d, S: lys_log_store::LeafStore>(
        &self,
        judged: &Judged<'d, S>,
        asked: &ExerciseRequest,
    ) -> Result<lys_identity::grants::Frame<'d>, GrantError> {
        self.validation.clone()?;
        let reading = self
            .readings
            .get(&(asked.caller, asked.resource.clone()))
            .ok_or_else(|| GrantError::PermissionEngineUnavailable {
                reason: "the decision has no prepared relationship reading".to_owned(),
            })?
            .clone()?;
        judged.grants.frame_from(judged.directory, reading)
    }
}

/// Every question is prepared once per subject/resource, including the resource reach.
pub(crate) fn with_read_grants<T>(
    state: &AppState,
    at_least: Option<u64>,
    at: u64,
    questions: impl Fn(&mut Judged<'_>) -> Result<Vec<ExerciseRequest>, ServerError>,
    act: impl FnOnce(Judged<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_read_directory_grants(state, at_least, at, questions, act, |_, answer| Ok(answer))
}

pub(crate) fn with_read_directory_grants<A, T>(
    state: &AppState,
    at_least: Option<u64>,
    at: u64,
    questions: impl Fn(&mut Judged<'_>) -> Result<Vec<ExerciseRequest>, ServerError>,
    act: impl FnOnce(Judged<'_>) -> Result<A, ServerError>,
    apply: impl FnOnce(
        &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
        A,
    ) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let mut apply = Some(apply);
    let mut act = Some(act);
    for attempt in 0..2 {
        let observed = with_grants(state, |judged| {
            Ok(judged.grants.relationships().reader()?)
        })?;
        let known = observed.revision()?;
        let (revision, app_revision, plan, mut engine, asked) =
            with_grants(state, |mut judged| {
                let plan = judged.grants.read_plan(known, at_least)?;
                let mut asked = BTreeMap::new();
                for request in questions(&mut judged)? {
                    let reach = judged.apps.reach(&request.resource);
                    asked.insert(
                        (
                            request.caller,
                            request.resource.clone(),
                            request.action.clone(),
                        ),
                        (request, reach),
                    );
                }
                Ok((
                    judged.grants.revision(),
                    judged.apps.len(),
                    plan,
                    judged.grants.relationships().reader()?,
                    asked,
                ))
            })?;
        let stamp = plan.execute(&mut engine)?;
        let mut prepared = Prepared {
            revision: stamp.revision(),
            stamp: Some(stamp.clone()),
            validation: engine.certify(stamp.revision()),
            readings: BTreeMap::new(),
            checks: BTreeMap::new(),
        };
        for ((subject, resource, action), (request, reach)) in asked {
            let reading = prepared
                .readings
                .entry((subject, resource.clone()))
                .or_insert_with(|| stamp.read_about(&engine, subject, &resource))
                .clone();
            for resource in reach {
                prepared
                    .readings
                    .entry((subject, resource.clone()))
                    .or_insert_with(|| reading.clone());
                let check = match &engine {
                    Relationships::SpiceDb(engine) => {
                        engine.check_at(&resource, &action, subject, at, stamp.revision())
                    }
                    #[cfg(not(test))]
                    Relationships::Memory(_) => Ok(true),
                    #[cfg(test)]
                    Relationships::Memory(_) | Relationships::Fixture(_) => Ok(true),
                };
                prepared
                    .checks
                    .entry((subject, resource, action.clone()))
                    .or_insert(check);
            }
            drop(request);
        }
        let guard = Hold::enter(&prepared);
        let result = with_prepared_directory_grants(
            state,
            &prepared,
            |judged| {
                judged.grants.settle_log()?;
                if judged.grants.revision() != revision || judged.apps.len() != app_revision {
                    return Ok(Err(judged.grants.revision()));
                }
                let act = act.take().ok_or_else(|| ServerError::RequestMalformed {
                    reason: "the decision was already judged".to_owned(),
                })?;
                act(judged).map(Ok)
            },
            |directory, answer| match answer {
                Err(now) => Ok(Err(now)),
                Ok(answer) => {
                    let apply = apply.take().ok_or_else(|| ServerError::RequestMalformed {
                        reason: "the decision was already applied".to_owned(),
                    })?;
                    apply(directory, answer).map(Ok)
                }
            },
        )?;
        drop(guard);
        match result {
            Ok(answer) => return Ok(answer),
            Err(now) if attempt == 1 => {
                return Err(GrantStreamError::BindingRevisionMoved {
                    decided: revision,
                    now,
                }
                .into());
            }
            Err(_) => {}
        }
    }
    Err(ServerError::Grant(
        GrantError::PermissionEngineUnavailable {
            reason: "the decision revision could not be retained".to_owned(),
        },
    ))
}

/// All resource/action questions about one subject for an audience's rights listing.
pub(crate) fn subject_questions(
    judged: &Judged<'_>,
    subject: IdentityId,
    app: Option<&str>,
) -> Result<Vec<ExerciseRequest>, ServerError> {
    let mut resources: BTreeSet<Resource> = judged
        .grants
        .book()
        .held_by(subject)
        .map(|record| record.grant().resource().clone())
        .collect();
    for placed in &judged.apps.held().placements {
        resources.insert(Resource::new(&placed.child_kind, &placed.child_id)?);
    }
    let mut asked = Vec::new();
    for resource in resources {
        if app.is_some_and(|app| lys_identity::grants::owner_of(resource.kind()) != app) {
            continue;
        }
        let actions: BTreeSet<Action> = judged
            .apps
            .schema(lys_identity::grants::owner_of(resource.kind()))
            .and_then(|schema| schema.kinds().get(resource.kind()))
            .map_or_else(
                || {
                    judged
                        .grants
                        .model()
                        .relations()
                        .flat_map(|(_, actions)| actions)
                        .cloned()
                        .collect()
                },
                |kind| kind.actions.clone(),
            );
        for action in actions {
            asked.push(ExerciseRequest {
                caller: subject,
                route: lys_identity::grants::Route::Api,
                resource: resource.clone(),
                action,
            });
        }
    }
    Ok(asked)
}

pub(crate) fn resource_questions(
    judged: &Judged<'_>,
    resource: &Resource,
    action: &Action,
) -> Vec<ExerciseRequest> {
    let book = judged.grants.book();
    let subjects: BTreeSet<IdentityId> = judged
        .apps
        .reach(resource)
        .iter()
        .flat_map(|reached| book.on_resource(reached))
        .map(|record| record.grant().holder())
        .collect();
    subjects
        .into_iter()
        .map(|caller| ExerciseRequest {
            caller,
            route: lys_identity::grants::Route::Api,
            resource: resource.clone(),
            action: action.clone(),
        })
        .collect()
}
