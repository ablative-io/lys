//! The apps as they are kept: a leaf store of their own beside the grant
//! log, one leaf for each registration, decision, schema version, placement
//! and registrar, written before the act is answered. A leaf is stored whole
//! or not at all.
//!
//! The store is born with its checkpoint beside its log: what the leaves fold
//! to is sealed in the log's signed snapshot every [`SNAPSHOT_EVERY`] leaves
//! and at once after a rebuild, so a start reads the snapshot and only the
//! leaves after it. A snapshot refused, or a state that does not read back,
//! sends the start to every leaf, by name, never silently.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written. The same act sent again under its operation id in the
//! same words answers what it did the first time and writes nothing.
//!
//! The store holds each approved app's current schema read and checked, and
//! answers from it which kinds an approved app declares. It holds no app's
//! name or kind of its own: every app it knows is one registered through the
//! API, and with none registered it holds only `lys`.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_identity::grants::{AppSchema, KindModel, LYS_APP, Model, Resource, owner_of};
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};

use crate::apps_error::AppError;
use crate::apps_state::{App, DOMAIN, Held, Line, Refused, Standing};
use crate::error::ServerError;

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the apps' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/apps";

/// The apps, read from their leaf store and appended to it.
pub struct AppStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    schemas: BTreeMap<String, AppSchema>,
    start: Start,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

/// A log opened and folded: the log, what it folds to, and how it started.
type Opened<S> = (FrontierLog<S>, Held, Start);

fn unavailable(what: impl std::fmt::Display) -> AppError {
    AppError::AppsUnavailable {
        reason: what.to_string(),
    }
}

impl AppStore<FileLeafStore> {
    /// The apps kept in the directory `dir`, which is created when it does
    /// not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> AppStore<S> {
    /// The apps kept in the leaf store `reopen` opens, their snapshots
    /// signed by `key`.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        let (log, held, start) = opened(&reopen, &key)?;
        let schemas = schemas_of(&held)?;
        let mut store = Self {
            reopen,
            key,
            log,
            held,
            schemas,
            start: start.clone(),
            since_snapshot: 0,
            snapshot_failure: None,
            uncertain: false,
        };
        store.after_start(&start);
        Ok(store)
    }

    /// How the log was started: from its snapshot, or from every leaf and
    /// the refusal that sent it there.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// The number of leaves the log holds.
    pub fn len(&self) -> u64 {
        self.log.len()
    }

    /// Whether the log holds no leaf.
    pub fn is_empty(&self) -> bool {
        self.log.len() == 0
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    fn after_start(&mut self, start: &Start) {
        match start {
            Start::Resumed { replayed, .. } => self.since_snapshot = *replayed,
            Start::Rebuilt { .. } => self.write_snapshot(),
        }
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot();
        }
    }

    fn write_snapshot(&mut self) {
        let written = self.held.encode().and_then(|state| {
            self.log
                .write_snapshot(DOMAIN, &state, &self.key)
                .map_err(|error| error.to_string())
        });
        match written {
            Ok(_) => {
                self.since_snapshot = 0;
                self.snapshot_failure = None;
            }
            Err(reason) => self.snapshot_failure = Some(reason),
        }
    }

    /// Resolve an append whose outcome is not known, by opening the leaf
    /// store again and reading what it holds. Until that succeeds nothing is
    /// answered from memory and nothing is appended.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, held, start) = opened(&self.reopen, &self.key)?;
            self.schemas = schemas_of(&held)?;
            self.log = log;
            self.held = held;
            self.start = start.clone();
            self.uncertain = false;
            self.after_start(&start);
        }
        Ok(())
    }

    /// Append one line as one leaf. A failed append is settled by reading
    /// back: the line is kept only if the leaf store holds exactly it.
    fn append(&mut self, line: Line) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&line).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            if let Err(reason) = self.held.hold(line) {
                self.uncertain = true;
                return Err(unavailable(reason).into());
            }
            self.schemas = schemas_of(&self.held)?;
            self.since_snapshot += 1;
            if self.since_snapshot >= SNAPSHOT_EVERY.get() {
                self.write_snapshot();
            }
            return Ok(());
        };
        self.uncertain = true;
        self.settle()?;
        match self.log.leaf_bytes(index).map_err(unavailable)? {
            Some(held) if held == bytes => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))
            .into()),
            None => Err(unavailable(failure).into()),
        }
    }

    /// What the log folds to.
    pub fn held(&self) -> &Held {
        &self.held
    }

    /// The app named `id`.
    pub fn app(&self, id: &str) -> Option<&App> {
        self.held.app(id)
    }

    /// Keep `line`, answering the line its operation was first kept as. Sent
    /// again in the same words it is kept once; the same operation in other
    /// words is refused, and so is a line the apps as they stand do not take.
    pub fn keep(&mut self, line: Line) -> Result<Line, ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.operation(line.operation()) {
            if !same_act(&kept, &line) {
                return Err(AppError::AppOperationReused {
                    operation: line.operation().to_owned(),
                }
                .into());
            }
            return Ok(kept);
        }
        self.held
            .allows(&line)
            .map_err(|refused| refusal(&line, refused))?;
        self.append(line.clone())?;
        Ok(line)
    }

    /// The current schema of the approved, unretired app `app`.
    pub fn schema(&self, app: &str) -> Option<&AppSchema> {
        self.schemas.get(app)
    }

    /// The model the grants judge by: Lys's own relations at the app `lys`'s
    /// current version, and each approved, unretired app's kinds at its own.
    pub fn model(&self) -> Result<Model, ServerError> {
        let lys = self
            .held
            .app(LYS_APP)
            .and_then(App::current)
            .ok_or_else(|| unavailable("the app lys is not recorded, so no model can be read"))?;
        let schema = self
            .schemas
            .get(LYS_APP)
            .ok_or_else(|| unavailable("the app lys has no schema"))?;
        let relations = schema
            .kinds()
            .values()
            .flat_map(|kind| kind.relations.clone())
            .collect::<Vec<_>>();
        let kinds = self.kind_models();
        Ok(Model::new(lys.version, relations)?.with_kinds(kinds))
    }

    /// Each approved, unretired app's kinds, at its current version.
    pub fn kind_models(&self) -> BTreeMap<String, KindModel> {
        let mut kinds = BTreeMap::new();
        for (app, schema) in &self.schemas {
            let version = self
                .held
                .app(app)
                .and_then(App::current)
                .map_or(0, |current| current.version);
            kinds.extend(schema.kind_models(version));
        }
        kinds
    }

    /// Refuse `kind` unless an approved, unretired app's current schema
    /// declares it, and, for a caller acting for the app `acting_for`,
    /// unless that app owns it.
    pub fn admit_kind(&self, acting_for: Option<&str>, kind: &str) -> Result<(), AppError> {
        let owner = owner_of(kind);
        if let Some(acting_for) = acting_for
            && owner != acting_for
        {
            return Err(AppError::NotYourApp {
                kind: kind.to_owned(),
                owner: owner.to_owned(),
                acting_for: acting_for.to_owned(),
            });
        }
        let not_registered = || AppError::KindNotRegistered {
            kind: kind.to_owned(),
        };
        let app = self.held.app(owner).ok_or_else(not_registered)?;
        match app.standing() {
            Standing::Pending | Standing::Declined => Err(AppError::AppNotApproved {
                app: owner.to_owned(),
            }),
            Standing::Retired => Err(AppError::AppRetired {
                app: owner.to_owned(),
            }),
            Standing::Approved => match self.schemas.get(owner) {
                Some(schema) if schema.kind(kind).is_some() => Ok(()),
                _ => Err(not_registered()),
            },
        }
    }

    /// Refuse `action` on an app kind that does not declare it. Lys's own
    /// kinds declare no list of actions, so any action is asked of them.
    pub fn admit_action(&self, kind: &str, action: &str) -> Result<(), AppError> {
        let owner = owner_of(kind);
        if owner == LYS_APP {
            return Ok(());
        }
        let declared = self
            .schemas
            .get(owner)
            .and_then(|schema| schema.kind(kind))
            .is_some_and(|declared| declared.actions.iter().any(|held| held.as_str() == action));
        if declared {
            return Ok(());
        }
        Err(AppError::ActionNotDeclared {
            kind: kind.to_owned(),
            action: action.to_owned(),
        })
    }

    /// The resources whose grants reach `resource`: itself, then each parent
    /// it is placed in that its kind's schema lists, nearest first.
    pub fn reach(&self, resource: &Resource) -> Vec<Resource> {
        let Some(schema) = self.schemas.get(owner_of(resource.kind())) else {
            return vec![resource.clone()];
        };
        schema.reach(resource, |child| {
            self.held
                .parent(child.kind(), child.id())
                .and_then(|placed| Resource::new(&placed.parent_kind, &placed.parent_id).ok())
        })
    }
}

/// Whether `kept` and `sent` are the same act, whenever each was received.
fn same_act(kept: &Line, sent: &Line) -> bool {
    let (Ok(mut kept), Ok(mut sent)) = (serde_json::to_value(kept), serde_json::to_value(sent))
    else {
        return false;
    };
    for value in [&mut kept, &mut sent] {
        if let Some(object) = value.as_object_mut() {
            object.remove("at");
        }
    }
    kept == sent
}

/// The refusal a line the apps do not take is answered with.
fn refusal(line: &Line, refused: Refused) -> ServerError {
    let app = line.app().unwrap_or_default().to_owned();
    let error = match refused {
        Refused::Exists => AppError::AppExists { app },
        Refused::Unknown => AppError::AppUnknown { app },
        Refused::Lys => AppError::AppIsLys {
            reason: "the app lys is Lys itself and is never retired",
        },
        Refused::Pending => AppError::SchemaChangePending { app },
        Refused::NothingPending => AppError::AppDecided { app },
        Refused::Moved(current) => AppError::SchemaVersionMoved {
            replaces: match line {
                Line::Proposed(proposed) => proposed.replaces,
                Line::Applied(applied) => applied.version.saturating_sub(1),
                _ => current,
            },
            current,
        },
        Refused::Placed => AppError::PlacementInvalid {
            reason: "the resource is already placed in a parent".to_owned(),
        },
        Refused::Standing(standing) => match (line, standing) {
            (Line::Approved(_) | Line::Declined(_), _) => AppError::AppDecided { app },
            (_, Standing::Retired) => AppError::AppRetired { app },
            _ => AppError::AppNotApproved { app },
        },
    };
    error.into()
}

/// Each approved, unretired app's current schema, read and checked.
fn schemas_of(held: &Held) -> Result<BTreeMap<String, AppSchema>, ServerError> {
    let mut schemas = BTreeMap::new();
    for app in &held.apps {
        if app.standing() != Standing::Approved {
            continue;
        }
        let Some(current) = app.current() else {
            continue;
        };
        let id = &app.registered.app;
        let schema = AppSchema::parse(id, &current.schema).map_err(|error| {
            unavailable(format!(
                "the app {id}'s schema version {} does not read back: {error}",
                current.version
            ))
        })?;
        schemas.insert(id.clone(), schema);
    }
    Ok(schemas)
}

/// Open the log from its snapshot, or from every leaf when the snapshot or
/// its state is refused, and fold what the start hands back.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, ServerError> {
    let store = reopen().map_err(unavailable)?;
    let started =
        open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let mut held = match started.state.as_deref().map(Held::decode) {
        None => Held::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => return rebuilt(reopen, reason),
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start))
}

/// Open the log from every leaf, because the snapshot's state was refused
/// for `reason`.
fn rebuilt<S: LeafStore>(reopen: &Reopen<S>, reason: String) -> Result<Opened<S>, ServerError> {
    let (log, tail) = FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
    let mut held = Held::default();
    held.fold(&tail).map_err(unavailable)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start))
}
