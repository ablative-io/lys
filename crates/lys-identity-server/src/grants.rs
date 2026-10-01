//! The grant routes: list, read, issue, pass on, revoke, the two questions,
//! why this caller may act and who can, and the delegation form's question,
//! what the caller cannot give a recipient.
//!
//! Every route calls the grants' one authority owner through `with_grants`,
//! which takes the directory lock first and the grants' second, so every
//! decision reads one directory projection and one grant revision. Browser,
//! API and tool requests reach the same owner and the same checks; the route a
//! body names is recorded and never changes a decision.
//!
//! The root authority is the person the configured administrator's login is
//! bound to. The grants are opened on first use, once that person exists, so
//! no second setting can name a different person. A caller sees a grant when
//! it holds, issued or answers for it, when it holds a grant the grant derives
//! from, or when it is the root authority. A refusal that would name a grant
//! or identity the caller may not see is answered with its name and without
//! the record.
//!
//! Every route that names a resource kind, issuing, passing on and every
//! check, first asks the apps whether an approved app declares that kind:
//! a kind no approved app declares is refused `kind_not_registered`, one of
//! an app not yet approved `app_not_approved`, and one of a retired app
//! `app_retired`, so a grant on a retired app's kind stays readable and is
//! never exercised. A check on a resource of an app kind also reaches the
//! resources it is placed in whose kinds its schema lists as parents,
//! nearest first, so a relation held on a parent flows to its children.
//!
//! The cannot-give question, `GET /grants/cannot-give`, is asked from a
//! source grant the caller holds, for a recipient the caller may name: any
//! person, or an agent it may see. A source it does not hold is refused
//! exactly as one that does not exist, and a recipient it may not name exactly
//! as one that does not exist, so no refusal says whether either exists. It
//! reads and records nothing.
//!
//! Every grant answered carries whether it stands and its effective end,
//! judged here over its whole chain by the same admission a check runs, so a
//! screen renders the service's judgement and walks no chain of its own. A
//! refusal that stops a grant standing is read as the caller may read it.

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::Router;
use axum::http::HeaderMap;
use axum::routing::{get, post};
use lys_identity::grants::{ExerciseRequest, GrantError, Grants, MemoryRelationships, Model};
use lys_identity::projection::Projection;
use lys_identity::signer::load_service_key;
use lys_identity::{IdentityId, PersonId};
use lys_log_store::{FileLeafStore, LeafStore};

use crate::apps_store::AppStore;
use crate::error::ServerError;
pub(crate) use crate::grant_sight::grant_view;
use crate::routes::{AppState, signed_in, with_directory};
use crate::spicedb::{Relationships, SpiceDb, SpiceDbEngine};

mod handlers;
use handlers::{cannot_give, check, list, model, read, revoke, who, why};
pub(crate) use handlers::{delegate, issue_root};

/// The grants as the service holds them.
pub type GrantState = Grants<FileLeafStore, Relationships>;

/// What the grants are opened from.
pub struct GrantSetup {
    /// The directory the grant log is kept in.
    pub log_dir: PathBuf,
    /// The grant log's origin, used when the log is created.
    pub log_origin: String,
    /// The file holding the service's event signing key seed.
    pub key_file: PathBuf,
    /// Lys's own model, the app `lys`'s current schema, as the apps log
    /// last gave it. The log is its only source once the app `lys` exists.
    pub model: RwLock<Model>,
    /// The permission engine the grants are mirrored into, if one is named.
    pub spicedb: Option<SpiceDbEngine>,
    pub(crate) model_revision: std::sync::atomic::AtomicU64,
    pub(crate) refresh: std::sync::Mutex<()>,
}

impl GrantSetup {
    pub(crate) fn require_model(&self, revision: u64) -> Result<(), ServerError> {
        if self
            .model_revision
            .load(std::sync::atomic::Ordering::Acquire)
            != revision
        {
            return Err(crate::apps_error::AppError::AppsUnavailable {
                reason: "the app model awaits publication; retry the app approval or schema change"
                    .to_owned(),
            }
            .into());
        }
        Ok(())
    }

    /// Lys's own model as the apps log last gave it.
    pub fn model(&self) -> Result<Model, ServerError> {
        Ok(self
            .model
            .read()
            .map_err(|error| crate::apps_error::AppError::AppsUnavailable {
                reason: format!("the app model lock is poisoned: {error}"),
            })?
            .clone())
    }

    /// Hold `model` as Lys's own model from now on.
    pub fn hold_model(&self, model: Model) -> Result<(), ServerError> {
        *self
            .model
            .write()
            .map_err(|error| crate::apps_error::AppError::AppsUnavailable {
                reason: format!("the app model lock is poisoned: {error}"),
            })? = model;
        Ok(())
    }

    fn open(&self, root_authority: PersonId, model: Model) -> Result<GrantState, ServerError> {
        if !self.log_dir.exists() {
            FileLeafStore::create(&self.log_dir, &self.log_origin).map_err(|error| {
                ServerError::ConfigInvalid {
                    reason: format!("the grant log could not be created: {error}"),
                }
            })?;
        }
        let log_dir = self.log_dir.clone();
        let relationships = match &self.spicedb {
            Some(engine) => {
                Relationships::SpiceDb(SpiceDb::open_connected(&engine.connection()?, &model)?)
            }
            None => Relationships::Memory(MemoryRelationships::default()),
        };
        let mut grants = Grants::open(
            Box::new(move || FileLeafStore::open(&log_dir)),
            load_service_key(&self.key_file)?,
            relationships,
            model,
            root_authority,
        )?;
        if self.spicedb.is_some() {
            grants.project()?;
        }
        Ok(grants)
    }
}

/// The grant routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/grants", get(list).post(delegate))
        .route("/grants/model", get(model))
        .route("/grants/roots", post(issue_root))
        .route("/grants/check", post(check))
        .route("/grants/check/batch", post(crate::grants_batch::batch))
        .route("/grants/which", post(crate::grants_batch::which))
        .route("/grants/why", post(why))
        .route("/grants/who", post(who))
        .route("/grants/reach", post(crate::grants_reach::reach))
        .route("/grants/cannot-give", get(cannot_give))
        .route("/grants/{id}", get(read))
        .route("/grants/{id}/revoke", post(revoke))
}

/// What one grant request is judged with.
pub struct Judged<'a, S: LeafStore = FileLeafStore> {
    /// The directory's projection.
    pub directory: &'a Projection,
    /// The grants.
    pub grants: &'a mut Grants<S, Relationships>,
    /// The root authority.
    pub root: PersonId,
    /// The apps, whose approved schemas say which kinds are judged at all.
    pub apps: &'a mut AppStore<S>,
}

/// Run `act` with the directory's projection, the apps and the grants, in
/// that lock order, opening the grants on first use under the model the
/// apps log gives.
pub(crate) fn with_grants<T>(
    state: &AppState,
    act: impl FnOnce(Judged<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory_grants(state, act, |_, answer| Ok(answer))
}

/// Hold the directory and grants together for a directory mutation whose
/// caller must first exercise an ordinary grant. No lock is reacquired.
pub(crate) fn with_directory_grants<A, T>(
    state: &AppState,
    judge: impl FnOnce(Judged<'_>) -> Result<A, ServerError>,
    apply: impl FnOnce(&mut lys_identity::Directory<FileLeafStore>, A) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory_grants_model(state, true, judge, apply)
}

pub(crate) fn with_schema_grants<T>(
    state: &AppState,
    act: impl FnOnce(Judged<'_>) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory_grants_model(state, false, act, |_, answer| Ok(answer))
}

fn with_directory_grants_model<A, T>(
    state: &AppState,
    require_model: bool,
    judge: impl FnOnce(Judged<'_>) -> Result<A, ServerError>,
    apply: impl FnOnce(&mut lys_identity::Directory<FileLeafStore>, A) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    with_directory(state, |directory| {
        let projection = crate::service_account_grants::projection(state, directory.projection()?)?;
        let administrator =
            state
                .admission
                .administrator_login()?
                .ok_or(ServerError::NotAdmitted {
                    reason: "no administrator is set up yet, so there is no root authority to judge a grant under",
                })?;
        let root = projection
            .person_for(&administrator)
            .ok_or(ServerError::NotAdmitted {
                reason: "the configured administrator's login is bound to no person, so there is no root authority to judge a grant under",
            })?;
        let mut apps =
            state
                .apps
                .lock()
                .map_err(|error| crate::apps_error::AppError::AppsUnavailable {
                    reason: format!("the apps lock is poisoned: {error}"),
                })?;
        apps.settle()?;
        if require_model {
            state.grant_setup.require_model(apps.model_revision())?;
        }
        let mut slot = state
            .grants
            .lock()
            .map_err(|error| GrantError::LogUnavailable {
                reason: format!("the grants lock is poisoned: {error}"),
            })?;
        let grants = if let Some(grants) = &mut *slot {
            grants
        } else {
            let opened = state.grant_setup.open(root, apps.model()?)?;
            (state.say)(&format!("grant log {}", opened.ledger().start()));
            slot.insert(opened)
        };
        let authorized = judge(Judged {
            directory: &projection,
            grants,
            root,
            apps: &mut apps,
        })?;
        drop(projection);
        apply(directory, authorized)
    })
}

/// Refuse unless the permission engine, where one is named, gives the caller
/// the action on the resource. The grants' own decision is made first and
/// nothing is recorded, so a refusal the grants name is answered by its name.
pub(crate) fn engine_permits<S: LeafStore>(
    grants: &mut Grants<S, Relationships>,
    directory: &Projection,
    request: &ExerciseRequest,
    at: u64,
) -> Result<(), GrantError> {
    if matches!(grants.relationships(), Relationships::Memory(_)) {
        return Ok(());
    }
    let permit = grants.explain(directory, request, at, None)?;
    let Relationships::SpiceDb(engine) = grants.relationships() else {
        return Ok(());
    };
    if engine.check(&request.resource, &request.action, request.caller, at)? {
        return Ok(());
    }
    Err(GrantError::PermissionAbsent {
        grant: permit.grant.to_string(),
    })
}

/// Whether a decision is an exercise, recorded as a use, or a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The caller is about to act: a permit is recorded as a use.
    Exercise,
    /// A question: nothing is recorded.
    Explain,
}

/// The grants' decision on `request`, reaching from its resource to each
/// parent it is placed in that its schema lists, nearest first, answered
/// with the permit and the resource whose grant permits it. The first
/// resource's refusal is the one answered when none permits.
pub(crate) fn decide<S: LeafStore>(
    judged: &mut Judged<'_, S>,
    request: &ExerciseRequest,
    at: u64,
    at_least: Option<u64>,
    decision: Decision,
) -> Result<(lys_identity::grants::Permit, lys_identity::grants::Resource), GrantError> {
    let mut first = None;
    for resource in judged.apps.reach(&request.resource) {
        let asked = ExerciseRequest {
            resource: resource.clone(),
            ..request.clone()
        };
        let decided =
            engine_permits(judged.grants, judged.directory, &asked, at).and_then(
                |()| match decision {
                    Decision::Exercise => {
                        judged.grants.check(judged.directory, &asked, at, at_least)
                    }
                    Decision::Explain => {
                        judged
                            .grants
                            .explain(judged.directory, &asked, at, at_least)
                    }
                },
            );
        match decided {
            Ok(permit) => return Ok((permit, resource)),
            Err(error) => {
                first.get_or_insert(error);
            }
        }
    }
    Err(first.unwrap_or_else(|| GrantError::NotHeld {
        identity: request.caller.to_string(),
        resource: request.resource.to_string(),
        action: request.action.to_string(),
    }))
}

/// The identity the signed-in caller's login is bound to, person or agent.
pub(crate) fn caller(
    state: &AppState,
    headers: &HeaderMap,
    directory: &Projection,
) -> Result<IdentityId, ServerError> {
    let actor = signed_in(state, headers)?;
    crate::caller_admission::active_caller(directory, &actor)
}
