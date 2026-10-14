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
                agent: agent.unwrap_or_default(),
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
    let agent = usage.agent.clone();
    crate::budgets_enforce::keep(state, usage).await?;
    if let Err(error) =
        crate::budgets_act::review(state, &record.session, &format!("feed-{seq}")).await
    {
        match error {
            error @ (ServerError::Runner { .. }
            | ServerError::RuntimeSessionUnknown
            | ServerError::AgentNotVisible
            | ServerError::RequestMalformed { .. }) => {
                let coordinate =
                    format!("seq={seq}; session={session}; followed_machine={machine}");
                let words =
                    format!("{coordinate}: usage retained; control review refused: {error}");
                (state.say)(&format!("budgets: feed_control_refused {words}"));
                return Ok(Some(RefusalRecord {
                    version: REFUSAL_VERSION,
                    source: format!("identity-feed:{machine}"),
                    attempt: format!("feed-{seq}"),
                    session: session.to_owned(),
                    agent,
                    at,
                    tool: "usage".to_owned(),
                    target: coordinate,
                    policy_version: 0,
                    rule: None,
                    check: "feed_control_refused".to_owned(),
                    grantable: false,
                    permission: None,
                    grantor: None,
                    words,
                }));
            }
            error => return Err(error),
        }
    }
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
        Ok(convert_observed(
            &tracked_machine,
            &agent,
            record,
            store.held(),
            windows,
        ))
    })
}

fn convert_observed(
    machine: &str,
    agent: &str,
    record: &UsageRecord,
    prior: &crate::budgets_state::Held,
    windows: &Windows,
) -> Result<Usage, ServerError> {
    let mut usage = convert(machine, agent, record, prior, windows)?;
    usage.clear_unobserved_windows();
    Ok(usage)
}

#[cfg(test)]
mod tests {
    use lys_runner::tracking::{Figures, Measure, RECORD_VERSION, Unavailable};
    use lys_runner::tracking_budget::PlanWindow;

    use super::{UsageRecord, Windows, convert_observed};
    use crate::budgets_state::{Held, Leaf};
    use crate::error::ServerError;

    fn status_record() -> UsageRecord {
        UsageRecord {
            version: RECORD_VERSION,
            id: "proxy-status".to_owned(),
            runner: "runner".to_owned(),
            session: "session".to_owned(),
            generation: 1,
            offset: Some(100),
            turn: None,
            observed_at: 10,
            measure: Measure::Snapshot,
            figures: Figures {
                running_ms: Some(100),
                dollars_micros: Some(990_000),
                ..Figures::default()
            },
            unavailable: vec![Unavailable {
                figure: "context_tokens".to_owned(),
                reason: "counted_from_the_calls_through_the_proxy".to_owned(),
            }],
            adapter: "lys-proxy-usage/1".to_owned(),
            model: Some("claude-opus-5-5".to_owned()),
            account: Some("account".to_owned()),
            account_unknown: None,
            context_window: 0,
            profile_version: 1,
            run: Some("0123456789abcdef0123456789abcdef".to_owned()),
            record: None,
        }
    }

    #[test]
    fn a_proxy_status_without_window_observation_keeps_the_previous_account_reading()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut held = Held::default();
        let mut prior = status_record();
        prior.id = "provider-window".to_owned();
        prior.observed_at = 9;
        prior.figures.plan_windows.push(PlanWindow {
            duration_minutes: 300,
            used_percent: 37.into(),
            resets_at_ms: 18_000_000,
        });
        held.hold(Leaf::Used(convert_observed(
            "machine",
            "agent",
            &prior,
            &held,
            &Windows::new(),
        )?))?;
        let status =
            convert_observed("machine", "agent", &status_record(), &held, &Windows::new())?;
        assert_eq!(status.plan_windows, None);
        assert_eq!(status.dollars_micros, Some(990_000));
        assert_eq!(status.reported_running_ms, Some(100));
        held.hold(Leaf::Used(status))?;
        let accounts = held.account_windows("agent", 10)?;
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].at_ms, 9);
        assert_eq!(accounts[0].windows[0].used_percent, 37.into());
        Ok(())
    }

    #[test]
    fn an_explicit_window_gap_is_retained_with_the_sources_reason() -> Result<(), ServerError> {
        let mut record = status_record();
        record.unavailable.push(Unavailable {
            figure: "plan_windows".to_owned(),
            reason: "300_minute_window_expired".to_owned(),
        });
        let usage = convert_observed(
            "machine",
            "agent",
            &record,
            &Held::default(),
            &Windows::new(),
        )?;
        assert_eq!(usage.plan_windows, Some(Vec::new()));
        assert!(
            usage.unavailable.iter().any(
                |gap| gap.figure == "plan_windows" && gap.reason == "300_minute_window_expired"
            )
        );
        Ok(())
    }
}
