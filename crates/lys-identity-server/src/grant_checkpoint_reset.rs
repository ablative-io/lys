//! The operator's reset of a refused grant checkpoint (ACCESS-006 R5),
//! served at `POST /grants/checkpoint/reset` to the administrator alone.
//!
//! While the grant log's checkpoint is refused the grants are not opened:
//! every route that needs them answers `CheckpointRefused`, naming the
//! refusal, and nothing is rebuilt from the log's history by itself. This
//! route is the one way back. The administrator names the refusal it
//! discards, by its `Snapshot…` name as the refusal gave it; the reset
//! confirms that refusal is the one standing, folds every leaf of the
//! verified log into a fresh book, writes a new checkpoint and opens the
//! grants, answering the refusal discarded and the revision reached. Grants
//! already open have no refused checkpoint, and a reset of them, or one
//! naming another refusal, is refused `ResetRefused` and changes nothing.

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::HeaderMap;
use lys_identity::grants::{GrantError, Grants, MemoryRelationships};
use lys_identity::signer::load_service_key;
use lys_log_store::FileLeafStore;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::grants::GrantState;
use crate::grants_batch::asker;
use crate::routes::AppState;
use crate::spicedb::{Relationships, SpiceDb};

/// Which refused checkpoint the operator discards.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResetBody {
    /// The refusal's `Snapshot…` name, exactly as `CheckpointRefused` gave it.
    pub discard: String,
}

/// What the reset discarded and where the rebuilt grants stand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ResetAnswer {
    /// The refusal discarded, its name and words.
    pub discarded: String,
    /// The revision the grants stand at, rebuilt from every leaf.
    pub revision: u64,
}

fn admitted(reason: &'static str) -> ServerError {
    ServerError::NotAdmitted { reason }
}

/// The grants opened by the operator's reset, discarding `discard`.
fn reopened(
    state: &AppState,
    root: lys_identity::PersonId,
    model: lys_identity::grants::Model,
    discard: &str,
) -> Result<(GrantState, String), ServerError> {
    let setup = &state.grant_setup;
    let relationships = match &setup.spicedb {
        Some(engine) => {
            Relationships::SpiceDb(SpiceDb::open_connected(&engine.connection()?, &model)?)
        }
        None => Relationships::Memory(MemoryRelationships::default()),
    };
    let log_dir = setup.log_dir.clone();
    let tail = lys_log_store::witness::FileTailProvider::at(&setup.log_dir);
    Ok(Grants::open_reset(
        Box::new(move || FileLeafStore::open(&log_dir)),
        (
            load_service_key(&setup.key_file)?,
            relationships,
            model,
            root,
        ),
        lys_identity::restart::SNAPSHOT_EVERY,
        Some(Arc::new(tail)),
        discard,
    )?)
}

/// Discard the refused grant checkpoint the administrator names and open
/// the grants from every leaf of their log.
pub async fn reset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Result<Json<ResetBody>, JsonRejection>,
) -> Result<Json<ResetAnswer>, ServerError> {
    let Json(body) = body.map_err(|refused| ServerError::RequestMalformed {
        reason: refused.body_text(),
    })?;
    if asker(&state, &headers)?.is_some() {
        return Err(admitted(
            "only the administrator resets the grant checkpoint",
        ));
    }
    let root = crate::agent_roots::administrator(&state)?.ok_or_else(|| {
        admitted("no administrator is set up yet, so there is no root authority to reset under")
    })?;
    let model = state
        .apps
        .lock()
        .map_err(|error| crate::apps_error::AppError::AppsUnavailable {
            reason: format!("the apps lock is poisoned: {error}"),
        })?
        .model()?;
    let mut slot = state
        .grants
        .lock()
        .map_err(|error| GrantError::LogUnavailable {
            reason: format!("the grants lock is poisoned: {error}"),
        })?;
    if slot.is_some() {
        return Err(GrantError::ResetRefused {
            reason: "the grants are open, so their checkpoint is not refused; nothing is discarded"
                .to_owned(),
        }
        .into());
    }
    let (grants, discarded) = reopened(&state, root, model, &body.discard)?;
    (state.say)(&format!(
        "grant log reset by the administrator, discarding {discarded}: {}",
        grants.ledger().start()
    ));
    let revision = grants.revision();
    slot.insert(grants);
    state.grant_setup.changes.published(revision);
    Ok(Json(ResetAnswer {
        discarded,
        revision,
    }))
}
