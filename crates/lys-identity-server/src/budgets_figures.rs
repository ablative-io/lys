//! What an agent used whether or not a limit is set: its context now, its
//! tokens, dollars and running time today and this week, and each
//! account's windows as last reported. Context belongs to one live session's
//! last reported call; period spend uses the budget's agent aggregate.
//! A figure nobody reported is unavailable with its reason, never zero.

use std::collections::BTreeSet;

use lys_runner::tracking_budget::PlanWindow;
use serde::Serialize;

use crate::budgets_api::{WidgetSessions, current_widget_usage, widget_sessions, with_budgets};
use crate::budgets_feed::Windows;
use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Held, Length, Measure};
use crate::budgets_usage::Used;
use crate::error::ServerError;
use crate::routes::AppState;

/// One account's windows as an agent's usage last reported them.
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AccountWindows {
    /// The account, as the report named it.
    pub account: String,
    /// When it was reported, in milliseconds since the Unix epoch.
    pub at_ms: i64,
    /// Each window: its length, how much of it is used, when it resets.
    pub windows: Vec<PlanWindow>,
}

/// The figures every agent is shown, in the order shown.
const SHOWN: [(Measure, Option<Length>); 9] = [
    (Measure::ContextPercent, None),
    (Measure::Tokens, Some(Length::Day)),
    (Measure::Tokens, Some(Length::Week)),
    (Measure::Dollars, Some(Length::Day)),
    (Measure::Dollars, Some(Length::Week)),
    (Measure::RunningMs, Some(Length::Day)),
    (Measure::RunningMs, Some(Length::Week)),
    (Measure::PlanPercent, Some(Length::FiveHour)),
    (Measure::Tokens, Some(Length::FiveHour)),
];

/// `agent`'s figures and its accounts' windows.
pub(crate) fn figures(
    state: &AppState,
    agent: &str,
) -> Result<(Vec<Used>, Vec<AccountWindows>), ServerError> {
    let organisation = crate::configuration_api::organisation(state)?;
    let agents = BTreeSet::from([agent.to_owned()]);
    let sessions = widget_sessions(state, &agents)?;
    let at_ms = jiff::Timestamp::now().as_millisecond();
    with_budgets(state, |store| {
        let held = store.held();
        let figures = figures_held(
            held,
            &agents,
            &organisation.zone,
            at_ms,
            sessions.as_ref(),
            &organisation.model_windows,
        )?;
        let accounts = held.account_windows(agent, at_ms).map_err(|reason| {
            ServerError::Agents(crate::error_agents::AgentsError::Budget(
                crate::error_budget::BudgetError::BudgetsUnavailable { reason },
            ))
        })?;
        Ok((figures, accounts))
    })
}

fn figures_held(
    held: &Held,
    agents: &BTreeSet<String>,
    zone: &str,
    at_ms: i64,
    sessions: Option<&WidgetSessions>,
    windows: &Windows,
) -> Result<Vec<Used>, ServerError> {
    SHOWN
        .iter()
        .map(|(unit, period)| {
            // The amount is never read for a figure: only a limit's unit and period are.
            let limit = Limit {
                unit: *unit,
                amount: 0_u64.into(),
                period: *period,
                act: Act::Tell,
                zone: None,
            };
            current_widget_usage(held, &limit, agents, zone, at_ms, sessions, windows)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::budgets_state::{CallSeen, Leaf, Usage};
    use crate::runtime_store::SessionActivity;

    use super::*;

    const NOW: i64 = 1_791_962_700_000;
    const MODEL: &str = "claude-opus-5-5";

    fn held() -> Result<Held, String> {
        let mut held = Held::default();
        for (session, at_ms, input, output, written, read) in [
            ("older", NOW - 1_000, 1, 30, 0, 979_999),
            ("current", NOW - 2_000, 4, 59, 4_232, 54_559),
        ] {
            held.hold(Leaf::Used(Usage {
                event: session.to_owned(),
                agent: "agent".to_owned(),
                session: Some(session.to_owned()),
                at_ms,
                tokens: input + output + written + read,
                call: Some(Box::new(CallSeen {
                    id: session.to_owned(),
                    model: Some(MODEL.to_owned()),
                    input_tokens: Some(input),
                    output_tokens: Some(output),
                    cache_creation_tokens: Some(written),
                    cache_read_tokens: Some(read),
                    run: session.to_owned(),
                    record: None,
                })),
                ..Usage::default()
            }))?;
        }
        held.hold(Leaf::Used(Usage {
            event: "reported-provider-window".to_owned(),
            agent: "agent".to_owned(),
            session: Some("current".to_owned()),
            account: Some("shared".to_owned()),
            at_ms: NOW - 500,
            native_snapshot: true,
            plan_windows: Some(vec![PlanWindow {
                duration_minutes: 300,
                used_percent: 1.into(),
                resets_at_ms: u64::try_from(NOW + 18_000_000).map_err(|error| error.to_string())?,
            }]),
            ..Usage::default()
        }))?;
        held.hold(Leaf::Used(Usage {
            event: "reported-cost-status".to_owned(),
            agent: "agent".to_owned(),
            session: Some("current".to_owned()),
            account: Some("shared".to_owned()),
            at_ms: NOW - 250,
            native_snapshot: true,
            dollars_micros: Some(991_276),
            ..Usage::default()
        }))?;
        Ok(held)
    }

    fn reading(figures: &[Used], unit: Measure, period: Option<Length>) -> Result<&Used, String> {
        figures
            .iter()
            .find(|figure| figure.unit == unit && figure.period == period)
            .ok_or_else(|| format!("missing {unit:?} {period:?} figure"))
    }

    #[test]
    fn usage_api_figures_choose_the_current_session_because_context_is_not_an_agent_maximum()
    -> Result<(), Box<dyn std::error::Error>> {
        let held = held()?;
        let agents = BTreeSet::from(["agent".to_owned()]);
        let started = u64::try_from(NOW / 1_000)?;
        let sessions = WidgetSessions {
            activity: BTreeMap::from([("agent".to_owned(), SessionActivity::default())]),
            started: BTreeMap::from([(
                "agent".to_owned(),
                BTreeMap::from([
                    ("older".to_owned(), started - 100),
                    ("current".to_owned(), started - 50),
                ]),
            )]),
        };
        let figures = figures_held(
            &held,
            &agents,
            "Australia/Melbourne",
            NOW,
            Some(&sessions),
            &Windows::from([(MODEL.to_owned(), Some(1_000_000))]),
        )?;
        let context = reading(&figures, Measure::ContextPercent, None)?;
        assert_eq!(
            serde_json::to_value(&context.figure)?,
            serde_json::json!(5.8795)
        );
        let context = context.context.as_ref().ok_or("context metadata absent")?;
        assert_eq!(context.session.as_deref(), Some("current"));
        assert_eq!(context.read_at_ms, Some(NOW - 2_000));
        assert_eq!(context.live_sessions.len(), 2);
        let tokens = reading(&figures, Measure::Tokens, Some(Length::Day))?
            .tokens
            .as_ref()
            .ok_or("token components absent")?;
        assert_eq!(tokens.input_tokens.reported, Some(5));
        assert_eq!(tokens.output_tokens.reported, Some(89));
        assert_eq!(tokens.cache_creation_tokens.reported, Some(4_232));
        assert_eq!(tokens.cache_read_tokens.reported, Some(1_034_558));
        assert_eq!((tokens.calls, tokens.sessions), (2, 2));
        assert_eq!(
            reading(&figures, Measure::Tokens, Some(Length::FiveHour))?
                .tokens
                .as_ref()
                .ok_or("five-hour components absent")?
                .calls,
            2
        );
        assert_eq!(
            reading(&figures, Measure::PlanPercent, Some(Length::FiveHour))?.figure,
            Some(1.into())
        );
        assert_eq!(
            held.account_windows("agent", NOW)?[0].windows[0].used_percent,
            1.into()
        );
        assert_eq!(
            serde_json::to_value(&reading(&figures, Measure::Dollars, Some(Length::Day))?.figure)?,
            serde_json::json!(0.991276)
        );
        Ok(())
    }

    #[test]
    fn unavailable_runtime_names_the_context_gap_without_hiding_reported_token_components()
    -> Result<(), Box<dyn std::error::Error>> {
        let figures = figures_held(
            &held()?,
            &BTreeSet::from(["agent".to_owned()]),
            "Australia/Melbourne",
            NOW,
            None,
            &Windows::new(),
        )?;
        let context = reading(&figures, Measure::ContextPercent, None)?;
        assert_eq!(context.figure, None);
        assert_eq!(context.context, None);
        assert_eq!(
            context.unavailable.as_deref(),
            Some("session liveness was not reported: the configuration names no runtime_dir")
        );
        assert_eq!(
            reading(&figures, Measure::Tokens, Some(Length::Day))?
                .tokens
                .as_ref()
                .ok_or("token components absent")?
                .calls,
            2
        );
        Ok(())
    }
}
