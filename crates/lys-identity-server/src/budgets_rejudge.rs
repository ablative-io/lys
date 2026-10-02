//! A budget's compaction is judged again at its boundary. The runner holds
//! an accepted compaction until the session's turn ends; if the budget it
//! was crossed under has since been changed, the crossing no longer stands
//! as it was judged, so the compaction is withdrawn before it is typed. One
//! the runner already began typing stands as typed, and is said.

use std::sync::Arc;

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_crossing::{Acted, Crossing, Stands};
use crate::budgets_state::Act;
use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_operate::{Undelivered, withdraw};

/// The accepted compactions whose budget has changed since they were
/// crossed.
fn stale(state: &AppState) -> Result<Vec<Crossing>, ServerError> {
    with_budgets(state, |store| {
        let held = store.held();
        Ok(held
            .crossings
            .crossed
            .iter()
            .filter(|crossing| crossing.act == Act::Compact)
            .filter(|crossing| {
                held.crossings
                    .acted
                    .get(&crossing.operation)
                    .is_some_and(|acted| acted.stands == Stands::Accepted)
            })
            .filter(|crossing| {
                held.limit_sets
                    .iter()
                    .find(|limits| limits.holder == crossing.holder)
                    .map(|limits| limits.version)
                    != Some(crossing.version)
            })
            .cloned()
            .collect())
    })
}

/// Withdraw every accepted compaction whose budget has changed, keeping
/// what came of each.
pub(crate) async fn rejudge(state: &Arc<AppState>) -> Result<(), ServerError> {
    if state.budgets.is_none() {
        return Ok(());
    }
    for crossing in stale(state)? {
        let Some(session) = crossing.session.as_deref() else {
            continue;
        };
        let why = format!(
            "the budget of {} changed after version {} was crossed",
            crossing.holder.id, crossing.version
        );
        let at_ms = jiff::Timestamp::now().as_millisecond();
        match withdraw(state, session, &crossing.operation, &why).await {
            Ok(outcome) => {
                with_budgets_mut(state, |store| {
                    store.acted(Acted::from_runner(&outcome, at_ms))
                })?;
            }
            Err(Undelivered::Refused(words)) => (state.say)(&format!(
                "budget compaction {} was not withdrawn and stands as the runner holds it: {words}",
                crossing.operation
            )),
            Err(Undelivered::Unknown(words)) => (state.say)(&format!(
                "budget compaction {} awaits its withdrawal: {words}",
                crossing.operation
            )),
        }
    }
    Ok(())
}
