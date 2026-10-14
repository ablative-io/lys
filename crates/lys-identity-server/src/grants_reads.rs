//! Settle under the authority hold, read outside it, then judge only at that revision.

use std::collections::{BTreeMap, BTreeSet};

use lys_identity::IdentityId;
use lys_identity::grants::{Action, ExerciseRequest, GrantError, RelationshipStore, Resource};

use crate::error::ServerError;
use crate::error_grant_stream::GrantStreamError;
use crate::grants::{Judged, with_grants, with_prepared_directory_grants};
use crate::routes::AppState;
#[path = "grants_preparation.rs"]
mod preparation;
use preparation::Style;

pub(crate) use crate::spicedb::context::Data as Prepared;
use crate::spicedb::context::Hold;

/// Counts and elapsed time belong to the request, including refused readings.
pub(crate) struct ReachRead {
    started: std::time::Instant,
    resources: usize,
    holders: std::cell::Cell<usize>,
    questions: std::cell::Cell<usize>,
    batches: std::cell::Cell<usize>,
    reason: std::cell::RefCell<Option<String>>,
}

impl ReachRead {
    pub(crate) fn new(resources: usize) -> Self {
        Self {
            started: std::time::Instant::now(),
            resources,
            holders: std::cell::Cell::new(0),
            questions: std::cell::Cell::new(0),
            batches: std::cell::Cell::new(0),
            reason: std::cell::RefCell::new(Some("grant questions were not prepared".to_owned())),
        }
    }
}

impl Drop for ReachRead {
    fn drop(&mut self) {
        tracing::info!(target: "lys_identity_server::passfast",
            resources = self.resources, holders = self.holders.get(),
            questions = self.questions.get(), batches = self.batches.get(),
            elapsed_ms = self.started.elapsed().as_millis(),
            reason = ?self.reason.borrow().as_deref(), "reach grant reads");
    }
}

#[derive(Clone, Copy)]
enum ReadingStyle<'a> {
    Ordinary(Style),
    Reach(&'a ReachRead),
}

impl ReadingStyle<'_> {
    fn prepare(
        self,
        engine: &crate::spicedb::Relationships,
        stamp: &lys_identity::grants::ReadStamp,
        asked: preparation::Asked,
        at: u64,
    ) -> Result<Prepared, GrantError> {
        match self {
            Self::Ordinary(style) => preparation::prepare(engine, stamp, asked, at, style),
            Self::Reach(measured) => prepare_reach(engine, stamp, asked, at, measured),
        }
    }
}

/// Reach prepares the carried questions together without a rights-claim cap.
pub(crate) fn with_reach_grants<T>(
    state: &AppState,
    at: u64,
    measured: &ReachRead,
    questions: impl Fn(&mut Judged<'_>) -> Result<Vec<ExerciseRequest>, ServerError>,
    act: impl FnOnce(Judged<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(
        state,
        None,
        at,
        questions,
        act,
        |_, answer| Ok(answer),
        ReadingStyle::Reach(measured),
    )
}

fn prepare_reach(
    engine: &crate::spicedb::Relationships,
    stamp: &lys_identity::grants::ReadStamp,
    asked: preparation::Asked,
    at: u64,
    measured: &ReachRead,
) -> Result<Prepared, GrantError> {
    let started = std::time::Instant::now();
    measured.holders.set(
        asked
            .keys()
            .map(|(holder, _, _)| *holder)
            .collect::<BTreeSet<_>>()
            .len(),
    );
    measured.questions.set(asked.len());
    measured.batches.set(0);
    let batch = if asked.is_empty() {
        *measured.reason.borrow_mut() = Some("no carried actions on visible grants".to_owned());
        None
    } else {
        let covered = asked
            .keys()
            .map(|(holder, resource, _)| (*holder, resource.clone()))
            .collect();
        let questions = asked.keys().cloned().collect();
        match engine
            .read_batch(stamp.revision(), covered, &questions, at)
            .and_then(|batch| {
                // A partial bulk answer is discarded before an independently certified read.
                if let Some(error) = batch
                    .checks
                    .values()
                    .find_map(|answer| answer.as_ref().err())
                {
                    return Err(error.clone());
                }
                Ok(batch)
            }) {
            Ok(batch) => {
                measured.batches.set(1);
                *measured.reason.borrow_mut() = None;
                Some(batch)
            }
            Err(error) => {
                *measured.reason.borrow_mut() = Some(error.to_string());
                None
            }
        }
    };
    let mut prepared = match batch {
        Some(batch) => {
            let (subject, resource, _) =
                asked
                    .keys()
                    .next()
                    .ok_or_else(|| GrantError::PermissionEngineUnavailable {
                        reason: "the reach batch has no question for its frame".to_owned(),
                    })?;
            whole_reach(stamp, batch, *subject, resource)?
        }
        None => preparation::from_batch(engine, stamp, asked, at, None)?,
    };
    prepared.reads_elapsed_ms = started.elapsed().as_millis();
    // Reach explains from relationships, as before. Name an unavailable ancillary
    // permission check rather than silently treating it as an answered question.
    if let Some(error) = prepared
        .checks
        .values()
        .find_map(|answer| answer.as_ref().err())
    {
        let mut reason = measured.reason.borrow_mut();
        *reason = Some(match reason.as_ref() {
            Some(batch) => format!("{batch}; unbatched permission check: {error}"),
            None => format!("unbatched permission check: {error}"),
        });
    }
    Ok(prepared)
}

fn whole_reach(
    stamp: &lys_identity::grants::ReadStamp,
    batch: crate::spicedb::batch::Batch,
    subject: IdentityId,
    resource: &Resource,
) -> Result<Prepared, GrantError> {
    // Reach uses a whole frame. Keep the exported graph once rather than copying
    // each holder's source graph again for every resource it is asked about.
    let reading = stamp.read_about(&ReachSnapshot(batch.snapshot), subject, resource)?;
    Ok(Prepared {
        revision: stamp.revision(),
        resources_read: 1,
        reads_elapsed_ms: 0,
        batch: true,
        truncated: false,
        stamp: Some(stamp.clone()),
        validation: Ok(()),
        readings: BTreeMap::from([((subject, resource.clone()), Ok(reading))]),
        checks: batch.checks,
    })
}

struct ReachSnapshot(crate::spicedb::snapshot::Snapshot);

impl RelationshipStore for ReachSnapshot {
    fn admit_resource(&self, resource: &Resource) -> Result<(), GrantError> {
        self.0.admit_resource(resource)
    }

    fn revision(&self) -> Result<u64, GrantError> {
        self.0.revision()
    }

    fn write(
        &mut self,
        revision: u64,
        touch: &[lys_identity::grants::Relationship],
        delete: &[lys_identity::grants::Relationship],
    ) -> Result<(), GrantError> {
        self.0.write(revision, touch, delete)
    }

    fn read_about(
        &self,
        subject: IdentityId,
        resource: &Resource,
    ) -> Result<BTreeSet<lys_identity::grants::Relationship>, GrantError> {
        if !self.0.covers(subject, resource) {
            return Err(GrantError::PermissionEngineUnavailable {
                reason: "the resource is outside the reach batch".to_owned(),
            });
        }
        self.read()
    }

    fn read_about_at(
        &self,
        subject: IdentityId,
        resource: &Resource,
        revision: u64,
    ) -> Result<BTreeSet<lys_identity::grants::Relationship>, GrantError> {
        let projected = self.revision()?;
        if revision != projected {
            return Err(GrantError::StaleDecision {
                required: revision,
                projected,
            });
        }
        self.read_about(subject, resource)
    }

    fn read(&self) -> Result<BTreeSet<lys_identity::grants::Relationship>, GrantError> {
        self.0.read()
    }
}

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
    with_directory(
        state,
        at_least,
        at,
        questions,
        act,
        apply,
        ReadingStyle::Ordinary(Style::Resource),
    )
}

pub(crate) fn with_pass_grants<T>(
    state: &AppState,
    at: u64,
    cap: Option<usize>,
    bind: bool,
    questions: impl Fn(&mut Judged<'_>) -> Result<Vec<ExerciseRequest>, ServerError>,
    act: impl FnOnce(Judged<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(
        state,
        None,
        at,
        questions,
        act,
        |_, answer| Ok(answer),
        ReadingStyle::Ordinary(Style::Pass { cap, bind }),
    )
}

fn with_directory<A, T>(
    state: &AppState,
    at_least: Option<u64>,
    at: u64,
    questions: impl Fn(&mut Judged<'_>) -> Result<Vec<ExerciseRequest>, ServerError>,
    act: impl FnOnce(Judged<'_>) -> Result<A, ServerError>,
    apply: impl FnOnce(
        &mut lys_identity::Directory<lys_log_store::FileLeafStore>,
        A,
    ) -> Result<T, ServerError>,
    style: ReadingStyle<'_>,
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
                    let reach = match style {
                        ReadingStyle::Ordinary(_) => judged.apps.reach(&request.resource),
                        ReadingStyle::Reach(_) => vec![request.resource.clone()],
                    };
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
        let prepared = style.prepare(&engine, &stamp, asked, at)?;
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

#[cfg(test)]
#[path = "grants_reads_tests.rs"]
mod tests;
