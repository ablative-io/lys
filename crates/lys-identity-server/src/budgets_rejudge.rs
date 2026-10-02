//! A budget's compaction is judged again at its boundary. The runner holds
//! an accepted compaction until the session's turn ends; if the budget it
//! was crossed under has since been changed, the crossing no longer stands
//! as it was judged, so the compaction is withdrawn before it is typed. One
//! the runner already began typing is kept as typed, and one the runner no
//! longer holds is kept as not done, so neither is asked again.

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
            Err(Undelivered::Refused(words)) => {
                let acted = refused(&crossing.operation, &words, at_ms);
                (state.say)(&format!(
                    "budget compaction {} was not withdrawn: {}",
                    crossing.operation, acted.words
                ));
                with_budgets_mut(state, |store| store.acted(acted))?;
            }
            Err(Undelivered::Unknown(words)) => (state.say)(&format!(
                "budget compaction {} awaits its withdrawal: {words}",
                crossing.operation
            )),
        }
    }
    Ok(())
}

/// What a refused withdrawal says of the crossing: one the runner had
/// already begun typing stands as typed; any other refusal means the runner
/// no longer holds it, so it will not be typed.
fn refused(operation: &str, words: &str, at_ms: i64) -> Acted {
    let (stands, words) = if words.starts_with("operation_past_its_boundary") {
        (
            Stands::Delivered,
            format!("it was typed before the change could withdraw it ({words})"),
        )
    } else {
        (
            Stands::Refused,
            format!("the runner no longer holds it, so it will not be typed ({words})"),
        )
    };
    Acted {
        operation: operation.to_owned(),
        stands,
        words,
        at_ms,
        ended: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{Stands, refused};

    #[test]
    fn a_refused_withdrawal_is_kept_as_typed_or_as_not_done() {
        let typed = refused("op", "operation_past_its_boundary: begun", 5);
        assert_eq!((typed.stands, typed.at_ms), (Stands::Delivered, 5));
        assert!(typed.words.contains("typed"));
        let gone = refused("op", "operation_unknown: none", 6);
        assert_eq!(gone.stands, Stands::Refused);
        assert!(gone.words.contains("operation_unknown"));
    }
}
