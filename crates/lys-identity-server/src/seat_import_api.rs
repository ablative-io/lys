//! The seat import routes (AGENTS-003 R4, R5):
//!
//! - `POST /seats/{name}/import/dry-run` `{manifest}`: read the seat's
//!   declared sources at one instant and answer the whole plan, refusals
//!   and all. Nothing is sent, started, stopped or changed; the plan is
//!   kept in memory as the seat's latest dry run.
//! - `POST /seats/{name}/import/confirm` `{operation, plan_id,
//!   plan_revision}`: one signed person confirms that exact plan of that
//!   one seat. Every source is read again and every destination revision
//!   checked before anything is reserved; a completed import of the same
//!   source revision vector answers its receipt and writes nothing.
//! - `GET /seats/{name}/import`: every import of the seat, its selected
//!   one, and its latest dry run's plan id.
//!
//! An import never starts, stops or restarts a seat, moves a secret or
//! deletes a source; a seat of the agent that is running is refused.

use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::error_seat_import::SeatImportError;
use crate::openapi_table::{GET, POST};
use crate::openapi_types::Entry;
use crate::routes::{AppState, signed_in};
use crate::seat_import_apply::{Owners, Unsignalled, apply};
use crate::seat_import_plan::{
    CredentialReference, DestinationEntry, Excluded, Manifest, Plan, ScheduleCounts, SourceEntry,
    confirmable, gather, record, vector_key,
};
use crate::seat_import_state::{Confirmation, Halted, Operation, Reserved, Step};
use crate::seat_import_store::{Preview, Reservation};
use crate::seats_api::{body, checked_name, seat, with_seats};
use crate::seats_state::Seat;
use crate::session::now;

/// What a receipt says of the source revisions it names.
pub const SNAPSHOT: &str = "each source's revision was read once, one source after another, at the plan's captured instant; no atomic snapshot across independent sources is claimed";

/// A dry run's request.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatImportDryRunBody {
    /// The seat's declared sources.
    pub manifest: Manifest,
}

/// A confirmation's request.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeatImportConfirmBody {
    /// The operation id the import is reserved under.
    pub operation: String,
    /// The dry run's plan id.
    pub plan_id: String,
    /// The plan revision the dry run showed.
    pub plan_revision: String,
}

/// One import's receipt.
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct SeatImportReceipt {
    /// The operation.
    pub operation: String,
    /// The seat.
    pub seat: String,
    /// Its agent.
    pub agent: String,
    /// `completed`, `stopped` or `in_progress`.
    pub state: String,
    /// The plan confirmed.
    pub plan_id: String,
    /// Its revision.
    pub plan_revision: String,
    /// The instant its sources were classified at.
    pub captured_at: u64,
    /// The captured source revision vector.
    pub sources: Vec<SourceEntry>,
    /// What the source revisions are, and are not, claimed to be.
    pub snapshot: String,
    /// Every step kept, with the record and revision it left.
    pub steps: Vec<Step>,
    /// Every credential, as a reference.
    pub references: Vec<CredentialReference>,
    /// The differences confirmed.
    pub replacements: Vec<String>,
    /// Every item not imported, with why.
    pub excluded: Vec<Excluded>,
    /// The schedules counted.
    pub schedule_counts: ScheduleCounts,
    /// Every rule, as an inactive transfer entry owned by liminal services.
    pub transfers: Vec<DestinationEntry>,
    /// Who confirmed it and what admitted them.
    pub confirmation: Confirmation,
    /// Why it stopped, when it did.
    pub halted: Option<Halted>,
    /// When its manifest was kept, once complete.
    pub completed_at: Option<u64>,
    /// The destination writes this request made: zero for a rerun.
    pub writes: u64,
}

/// A seat's imports.
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
pub struct SeatImportStatus {
    /// The seat.
    pub seat: String,
    /// The operation of its selected import, when one completed.
    pub selected: Option<String>,
    /// The plan id of its latest dry run, while the service holds it.
    pub previewed: Option<String>,
    /// Every import of it, oldest first.
    pub imports: Vec<SeatImportReceipt>,
}

/// The seat import routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/seats/{name}/import", get(status))
        .route("/seats/{name}/import/dry-run", post(dry_run))
        .route("/seats/{name}/import/confirm", post(confirm))
}

/// The seat import routes' schemas, for the document.
pub(crate) fn types(api: &mut lys_openapi::Api) -> Vec<Entry> {
    vec![
        (
            GET,
            "/seats/{name}/import",
            None,
            Some(api.schema::<SeatImportStatus>()),
        ),
        (
            POST,
            "/seats/{name}/import/dry-run",
            Some(api.schema::<SeatImportDryRunBody>()),
            Some(api.schema::<Plan>()),
        ),
        (
            POST,
            "/seats/{name}/import/confirm",
            Some(api.schema::<SeatImportConfirmBody>()),
            Some(api.schema::<SeatImportReceipt>()),
        ),
    ]
}

/// `operation`'s receipt, naming `writes` made by this request.
pub fn receipt(operation: &Operation, writes: u64) -> SeatImportReceipt {
    let plan = &operation.reserved.plan;
    SeatImportReceipt {
        operation: operation.reserved.operation.clone(),
        seat: operation.reserved.seat.clone(),
        agent: plan.agent.clone(),
        state: operation.state().to_owned(),
        plan_id: plan.plan_id.clone(),
        plan_revision: plan.plan_revision.clone(),
        captured_at: plan.captured_at,
        sources: plan.sources.clone(),
        snapshot: SNAPSHOT.to_owned(),
        steps: operation.steps.clone(),
        references: plan.references.clone(),
        replacements: plan.replacements.clone(),
        excluded: plan.excluded.clone(),
        schedule_counts: plan.schedule_counts.clone(),
        transfers: plan
            .destinations
            .iter()
            .filter(|destination| destination.record_kind == record::RULE_TRANSFER)
            .cloned()
            .collect(),
        confirmation: operation.reserved.confirmation.clone(),
        halted: operation.halted.clone(),
        completed_at: operation.completed.as_ref().map(|completed| completed.at),
        writes,
    }
}

/// Refuse a name naming more than one seat, then one that is not a seat name.
fn one_seat(name: &str) -> Result<(), ServerError> {
    if name.contains(['*', '?', ',', ' ']) || name == "all" {
        return Err(SeatImportError::BulkRefused {
            name: name.to_owned(),
        }
        .into());
    }
    checked_name(name)
}

/// Refuse while any seat of `seat`'s agent is running.
fn not_running(state: &AppState, seat: &Seat) -> Result<(), ServerError> {
    let running = with_seats(state, |store| {
        Ok(store
            .seats()
            .iter()
            .find(|held| held.added.agent == seat.added.agent && held.running)
            .map(|held| held.added.name.clone()))
    })?;
    match running {
        Some(running) => Err(SeatImportError::SeatRunning {
            seat: running,
            agent: seat.added.agent.clone(),
        }
        .into()),
        None => Ok(()),
    }
}

/// Refuse when any source the dry run showed reads another revision now.
fn sources_unchanged(shown: &Plan, now: &Plan) -> Result<(), ServerError> {
    for source in &shown.sources {
        let read = now
            .sources
            .iter()
            .find(|read| read.id == source.id)
            .map_or("absent", |read| read.source_revision.as_str());
        if read != source.source_revision {
            return Err(SeatImportError::SourceChanged {
                locator: source.locator.clone(),
                shown: source.source_revision.clone(),
                now: read.to_owned(),
            }
            .into());
        }
    }
    Ok(())
}

/// Refuse when any destination the dry run bound is at another revision
/// now, or the plan otherwise reads differently.
fn destinations_unchanged(shown: &Plan, now: &Plan) -> Result<(), ServerError> {
    let stale = || -> ServerError {
        SeatImportError::PlanStale {
            seat: shown.seat.clone(),
            plan_id: shown.plan_id.clone(),
            held: now.plan_revision.clone(),
            given: shown.plan_revision.clone(),
        }
        .into()
    };
    for destination in &shown.destinations {
        let held = now
            .destinations
            .iter()
            .find(|held| {
                held.record_kind == destination.record_kind
                    && held.record_id == destination.record_id
            })
            .ok_or_else(stale)?;
        match (destination.expected_revision, held.expected_revision) {
            (Some(expected), Some(held)) if expected != held => {
                return Err(SeatImportError::DestinationMoved {
                    record: format!("{} {}", destination.record_kind, destination.record_id),
                    expected,
                    held,
                }
                .into());
            }
            (Some(_), Some(_)) => {}
            (None, _) | (_, None) => return Err(stale()),
        }
    }
    if shown.plan_revision == now.plan_revision {
        Ok(())
    } else {
        Err(stale())
    }
}

async fn dry_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    given: Result<Json<SeatImportDryRunBody>, JsonRejection>,
) -> Result<Json<Plan>, ServerError> {
    let given = body(given)?;
    one_seat(&name)?;
    if given.manifest.seat != name {
        return Err(SeatImportError::SeatMismatch {
            seat: name,
            named: given.manifest.seat,
        }
        .into());
    }
    let seat = seat(&state, &name)?;
    let url = format!("/seats/{name}/import/dry-run");
    crate::seat_import_rights::may_preview(&state, &headers, &seat, &url)?;
    let plan = gather(&given.manifest, &state).await?;
    state.seat_imports.remember(
        &name,
        Preview {
            manifest: given.manifest,
            plan: plan.clone(),
        },
    )?;
    Ok(Json(plan))
}

async fn confirm(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
    given: Result<Json<SeatImportConfirmBody>, JsonRejection>,
) -> Result<Json<SeatImportReceipt>, ServerError> {
    let given = body(given)?;
    one_seat(&name)?;
    OperationId::from_str(&given.operation)?;
    let seat = seat(&state, &name)?;
    let url = format!("/seats/{name}/import/confirm");
    let confirmation = crate::seat_import_rights::confirmed_by(&state, &headers, &seat, &url)?;
    let preview = state
        .seat_imports
        .preview(&name)?
        .filter(|preview| preview.plan.plan_id == given.plan_id)
        .ok_or_else(|| SeatImportError::PlanUnknown {
            seat: name.clone(),
            plan_id: given.plan_id.clone(),
        })?;
    if preview.plan.plan_revision != given.plan_revision {
        return Err(SeatImportError::PlanStale {
            seat: name,
            plan_id: given.plan_id,
            held: preview.plan.plan_revision,
            given: given.plan_revision,
        }
        .into());
    }
    let read = gather(&preview.manifest, &state).await?;
    sources_unchanged(&preview.plan, &read)?;
    let key = vector_key(&preview.plan);
    if let Some(completed) = state.seat_imports.completed(&key)? {
        return Ok(Json(receipt(&completed, 0)));
    }
    destinations_unchanged(&preview.plan, &read)?;
    confirmable(&read)?;
    not_running(&state, &seat)?;
    let reserved = Reserved {
        operation: given.operation,
        seat: name,
        key,
        plan: preview.plan,
        confirmation,
        at: now(),
    };
    let operation = match state.seat_imports.reserve(reserved)? {
        Reservation::Completed(completed) => return Ok(Json(receipt(&completed, 0))),
        Reservation::Fresh(operation) | Reservation::Resumed(operation) => operation,
    };
    let fence = || crate::operator::upgrade_pending(&state);
    let applied = apply(
        &state.seat_imports,
        &Owners::of(&state),
        &operation,
        &fence,
        &Unsignalled,
    )?;
    Ok(Json(receipt(&applied.operation, applied.writes)))
}

async fn status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> Result<Json<SeatImportStatus>, ServerError> {
    signed_in(&state, &headers)?;
    one_seat(&name)?;
    seat(&state, &name)?;
    let imports = state
        .seat_imports
        .of_seat(&name)?
        .iter()
        .map(|operation| receipt(operation, 0))
        .collect();
    let selected = state
        .seat_imports
        .selected(&name)?
        .map(|operation| operation.reserved.operation);
    let previewed = state
        .seat_imports
        .preview(&name)?
        .map(|preview| preview.plan.plan_id);
    Ok(Json(SeatImportStatus {
        seat: name,
        selected,
        previewed,
        imports,
    }))
}
