//! The current profile supplies session declarations to the import plan.

use super::{DestinationEntry, Manifest, Plan, SeatFacts, assemble};
use crate::error::ServerError;
use crate::routes::AppState;
use std::collections::BTreeSet;

/// Read every source the manifest declares for its seat at one instant and
/// assemble the plan, refusals and all; nothing is sent, started, stopped
/// or changed.
pub async fn gather(manifest: &Manifest, state: &AppState) -> Result<Plan, ServerError> {
    let seat = crate::seats_api::seat(state, &manifest.seat)?;
    let people = crate::seats_api::responsible(state, &BTreeSet::from([seat.added.agent.clone()]))?;
    let facts = SeatFacts {
        seat: seat.added.name.clone(),
        agent: seat.added.agent.clone(),
        responsible: people.get(&seat.added.agent).cloned().flatten(),
    };
    let number = seat.added.profile_version;
    let settings = crate::provisioning_api::with_provisioning(state, |store| {
        let version = store.version(&seat.added.agent, number);
        Ok(version.map(|version| {
            (
                version.settings.harness.clone(),
                version.settings.session.clone(),
            )
        }))
    })?;
    let (declared, session) = settings.unwrap_or((None, None));
    let captured_at = crate::session::now();
    let mut fragments = vec![
        crate::seat_import_sources::read_files_with_session(
            manifest,
            declared.as_ref(),
            session.as_ref(),
        ),
        crate::seat_import_references::read_references(manifest, state),
    ];
    if let Some(monitor) = &manifest.monitor {
        fragments.push(
            crate::seat_import_monitor::read_monitor_mapped(
                monitor,
                captured_at,
                &facts.seat,
                &facts.agent,
                &manifest.recipient_map,
            )
            .await,
        );
    }
    let owners = crate::seat_import_apply::Owners::of(state);
    let agent = facts.agent.clone();
    let bind = |destination: &DestinationEntry| owners.revision(destination, &agent);
    Ok(assemble(
        facts,
        manifest.harness,
        captured_at,
        fragments,
        &bind,
    ))
}

/// [`gather`], refused `import_plan_refused` naming every refusal when the
/// plan holds one or reads a source incompletely.
pub async fn build(manifest: &Manifest, state: &AppState) -> Result<Plan, ServerError> {
    let plan = gather(manifest, state).await?;
    confirmable(&plan)?;
    Ok(plan)
}

/// Refuse `plan` when it holds a refusal.
pub fn confirmable(plan: &Plan) -> Result<(), ServerError> {
    if plan.refusals.is_empty() {
        Ok(())
    } else {
        Err(crate::error_seat_import::SeatImportError::PlanRefused {
            seat: plan.seat.clone(),
            refusals: plan.refusals.clone(),
        }
        .into())
    }
}
