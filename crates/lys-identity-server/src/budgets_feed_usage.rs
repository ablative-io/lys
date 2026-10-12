//! Per-entry validation and attribution without discarding a refused page.

use std::sync::Arc;

use lys_runner::refusals::{REFUSAL_VERSION, RefusalRecord};
use lys_runner::tracking::UsageRecord;

use super::{Windows, convert, refused};
use crate::budgets_api::with_budgets;
use crate::budgets_state::Usage;
use crate::error::ServerError;
use crate::routes::AppState;

pub(super) async fn keep(
    state: &Arc<AppState>,
    machine: &str,
    (seq, at, session): (u64, u64, &str),
    record: &UsageRecord,
    windows: &Windows,
) -> Result<Option<RefusalRecord>, ServerError> {
    let usage = match prepare(state, machine, session, record, windows)? {
        Ok(usage) => usage,
        Err(error) => {
            let agent = crate::runtime_api::with_runtime(state, |store| {
                Ok(store
                    .session(session)
                    .and_then(|tracked| tracked.agent.clone()))
            })?;
            return Ok(Some(RefusalRecord {
                version: REFUSAL_VERSION,
                source: format!("identity-feed:{machine}"),
                attempt: format!("feed-{seq}"),
                session: session.to_owned(),
                agent: agent.map_or_else(String::new, std::convert::identity),
                at,
                tool: "usage".to_owned(),
                target: format!("seq={seq}; session={session}; followed_machine={machine}"),
                policy_version: 0,
                rule: None,
                check: "feed_entry_skipped".to_owned(),
                grantable: false,
                permission: None,
                grantor: None,
                words: format!("seq={seq}; session={session}; followed_machine={machine}: {error}"),
            }));
        }
    };
    let attribution = usage.event.clone();
    crate::budgets_enforce::keep(state, usage).await?;
    crate::budgets_act::review(state, &record.session, &format!("feed-{seq}")).await?;
    if attribution != format!("native:{machine}:{}", record.id) {
        (state.say)(&format!(
            "budgets: feed entry seq={seq} session={session} followed_machine={machine} attributed as {attribution}"
        ));
    }
    Ok(None)
}

// Store failures stop the page; entry validation is returned separately so its
// refusal can be kept with the cursor without discarding the remaining entries.
fn prepare(
    state: &AppState,
    machine: &str,
    session: &str,
    record: &UsageRecord,
    windows: &Windows,
) -> Result<Result<Usage, ServerError>, ServerError> {
    if session != record.session {
        return Ok(Err(refused(
            "runner feed entry and usage name different sessions",
        )));
    }
    let tracked = crate::runtime_api::with_runtime(state, |store| {
        Ok(store
            .session(session)
            .map(|tracked| (tracked.machine.clone(), tracked.agent.clone())))
    })?;
    let Some((tracked_machine, agent)) = tracked else {
        return Ok(Err(ServerError::RuntimeSessionUnknown));
    };
    let Some(agent) = agent else {
        return Ok(Err(refused("runner feed usage has no tracked agent")));
    };
    if tracked_machine != machine {
        let live = crate::network_api::with_network(state, |store| {
            Ok(store
                .machine(&tracked_machine)
                .map(|record| record.retired.is_none()))
        })?;
        match live {
            Some(true) => {}
            Some(false) => {
                return Ok(Err(refused(format!(
                    "runner feed usage names retired machine {tracked_machine}"
                ))));
            }
            None => {
                return Ok(Err(refused(format!(
                    "runner feed usage names missing machine {tracked_machine}"
                ))));
            }
        }
    }
    with_budgets(state, |store| {
        Ok(convert(
            &tracked_machine,
            &agent,
            record,
            store.held(),
            windows,
        ))
    })
}
