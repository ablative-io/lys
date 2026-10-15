//! A budget's compaction is judged again at its boundary. The runner holds
//! an accepted compaction until the session's turn ends; if the budget it
//! was crossed under has since been changed, the crossing no longer stands
//! as it was judged, so the compaction is withdrawn before it is typed. One
//! the runner already began typing is kept as typed, and one the runner no
//! longer holds is kept as not done, so neither is asked again.

use std::sync::Arc;

use crate::budgets_api::{with_budgets, with_budgets_mut};
use crate::budgets_crossing::{Acted, Crossing, Stands};
use crate::budgets_state::{Act, HolderKind, Measure};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_operate::{Undelivered, withdraw};

/// The accepted compactions whose budget has changed since they were
/// crossed, including an organisation default the agent's own context limit
/// has since set aside.
fn stale(state: &AppState) -> Result<Vec<Crossing>, ServerError> {
    let standings = crate::budgets_members::standings(state)?;
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
            .filter(|crossing| changed(held, &standings, crossing))
            .cloned()
            .collect())
    })
}

/// Whether `crossing` no longer stands as judged: its holder's limits moved
/// to another version, or it came from the organisation's context default
/// and the agent now has a context limit of its own, or no standing at all.
fn changed(
    held: &crate::budgets_state::Held,
    standings: &[crate::budgets_state::Standing],
    crossing: &Crossing,
) -> bool {
    held.limit_set_in_force(&crossing.holder)
        .map(|limits| limits.version)
        != Some(crossing.version)
        || (crossing.holder.kind == HolderKind::Organisation
            && crossing.measure == Measure::ContextPercent
            && standings
                .iter()
                .find(|standing| standing.agent == crossing.agent)
                .is_none_or(|standing| held.own_context(standing)))
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
    use super::{Stands, changed, refused};
    use crate::budgets_crossing::Crossing;
    use crate::budgets_limits::{Limit, Limits};
    use crate::budgets_state::{Act, Held, Holder, HolderKind, Measure, Standing};

    fn compaction(holder: Holder, version: u64) -> Crossing {
        Crossing {
            operation: "op".to_owned(),
            holder,
            measure: Measure::ContextPercent,
            version,
            limit: 40.into(),
            figure: Some(41.into()),
            unavailable: None,
            limit_index: 2,
            warning: false,
            context_tokens: None,
            context_window: None,
            account: None,
            act: Act::Compact,
            agent: "agent-a".to_owned(),
            session: Some("session-a".to_owned()),
            text: None,
            at_ms: 1,
        }
    }

    fn agents_own(limits: Vec<Limit>) -> Limits {
        Limits {
            holder: Holder {
                kind: HolderKind::Agent,
                id: "agent-a".to_owned(),
            },
            limits,
            warn_at: None,
            version: 1,
            by: "person-a".to_owned(),
            at: 1,
        }
    }

    #[test]
    fn a_default_compaction_stands_until_the_default_changes_or_is_set_aside() {
        let standings = [Standing {
            agent: "agent-a".to_owned(),
            ..Standing::default()
        }];
        let crossing = compaction(crate::budgets_default::holder(), 0);
        let mut held = Held::default();
        assert!(
            !changed(&held, &standings, &crossing),
            "the built-in default stands"
        );
        assert!(
            changed(&held, &[], &crossing),
            "an agent with no standing is not compacted"
        );

        held.limit_sets.push(agents_own(vec![Limit {
            unit: Measure::Tokens,
            amount: 10.into(),
            period: Some(crate::budgets_state::Length::Day),
            act: Act::Stop,
            zone: None,
        }]));
        assert!(
            !changed(&held, &standings, &crossing),
            "a spend limit leaves it standing"
        );

        held.limit_sets[0].limits.push(Limit {
            unit: Measure::ContextPercent,
            amount: 70.into(),
            period: None,
            act: Act::Compact,
            zone: None,
        });
        assert!(
            changed(&held, &standings, &crossing),
            "its own context limit sets it aside"
        );

        let mut saved = Held::default();
        let mut default = crate::budgets_default::built_in();
        default.version = 1;
        saved.limit_sets.push(default);
        assert!(
            changed(&saved, &standings, &crossing),
            "a saved default is a new version"
        );
    }

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
