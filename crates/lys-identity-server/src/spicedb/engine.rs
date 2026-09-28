//! Road step 2 as the service runs it: the client, the schema written at
//! start-up, the one evaluator, and the projector opened with the grants.
//!
//! The engine is started with the service when the configuration names
//! `SpiceDB`'s gRPC address, and a stored schema other than `schema.zed`
//! stops the start by name. The projector is opened when the grants are,
//! once the root authority is known, and keeps its position in a file beside
//! the grant log.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use lys_identity::PersonId;
use lys_identity::grants::{GrantError, Grants, Recorded, RelationshipStore};
use lys_log_store::LeafStore;

use super::check::{Clock, Evaluator, ServiceClock};
use super::client::{ClientConfig, SpiceDbApi, SpiceDbClient};
use super::error::SpiceDbError;
use super::gateway::SpiceDbSettings;
use super::projector::{FilePosition, Projector, Reached};
use super::schema::{Started, ensure};

/// The client, the evaluator and the projector of one service.
pub struct Engine {
    evaluator: Evaluator,
    projector: Mutex<Option<Projector>>,
    position: PathBuf,
    max_updates_per_write: u32,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("evaluator", &self.evaluator)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

/// Where the projector keeps its position for the grant log at `log_dir`:
/// beside it, never inside the log's own directory.
fn position_beside(log_dir: &Path) -> PathBuf {
    let mut name = log_dir
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".spicedb-position");
    log_dir.with_file_name(name)
}

impl Engine {
    /// Reach the `SpiceDB` `settings` names and make sure it holds the
    /// schema, for the grant log at `log_dir`. Answers what the schema
    /// start found beside the engine.
    pub fn start(
        settings: &SpiceDbSettings,
        log_dir: &Path,
    ) -> Result<(Self, Started), SpiceDbError> {
        let client = SpiceDbClient::connect(&ClientConfig::from_settings(settings)?)?;
        Self::engage(
            Arc::new(client),
            Arc::new(ServiceClock),
            settings.max_updates_per_write,
            log_dir,
        )
    }

    /// The engine over `client`, reading caveat times from `clock`, writing
    /// at most `max_updates_per_write` updates at once, for the grant log at
    /// `log_dir`; the schema is made sure of first.
    pub fn engage(
        client: Arc<dyn SpiceDbApi>,
        clock: Arc<dyn Clock>,
        max_updates_per_write: u32,
        log_dir: &Path,
    ) -> Result<(Self, Started), SpiceDbError> {
        let started = ensure(client.as_ref())?;
        Ok((
            Self {
                evaluator: Evaluator::new(client, clock),
                projector: Mutex::new(None),
                position: position_beside(log_dir),
                max_updates_per_write,
            },
            started,
        ))
    }

    /// The one evaluator.
    pub fn evaluator(&self) -> &Evaluator {
        &self.evaluator
    }

    /// Bring the projection up to every committed event of `grants`, whose
    /// root authority is `root`, answering where the projector stands. When
    /// it cannot move, it stands where it stood, and the refusal is logged:
    /// a check then asks `SpiceDB` against the projection as it stands,
    /// under the freshness rule.
    pub fn catch_up<S: LeafStore, R: RelationshipStore>(
        &self,
        grants: &Grants<S, R>,
        root: PersonId,
    ) -> Result<Reached, GrantError> {
        let mut slot = self
            .projector
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let projector = if let Some(projector) = &mut *slot {
            projector
        } else {
            let opened = Projector::open(
                Arc::clone(self.evaluator.client()),
                Box::new(FilePosition::new(&self.position)),
                self.max_updates_per_write,
                root,
            )?;
            slot.insert(opened)
        };
        if let Err(refusal) = projector.catch_up(grants.book(), grants.revision()) {
            tracing::warn!(%refusal, "the permission projection did not catch up");
        }
        Ok(projector.reached().clone())
    }

    /// Answer `recorded` only once the projection has applied it, as the
    /// grants answer a change only once it is projected.
    pub fn settle<S: LeafStore, R: RelationshipStore>(
        &self,
        grants: &Grants<S, R>,
        root: PersonId,
        recorded: Recorded,
    ) -> Result<Recorded, GrantError> {
        let reached = self.catch_up(grants, root)?;
        if reached.position > recorded.index {
            return Ok(recorded);
        }
        Err(GrantError::ProjectionPending {
            operation: recorded.event.operation().to_string(),
            grant: recorded.event.grant().to_string(),
            index: recorded.index,
        })
    }
}
