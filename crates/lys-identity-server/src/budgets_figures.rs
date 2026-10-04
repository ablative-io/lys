//! What an agent used whether or not a limit is set: its context now, its
//! tokens, dollars and running time today and this week, and each
//! account's windows as last reported. Each figure is the one a limit of
//! that unit and period would be held against, computed by the same
//! function, so a screen and a budget never disagree; a figure nobody
//! reported is unavailable with its reason, never zero.

use std::collections::BTreeSet;

use lys_runner::tracking_budget::PlanWindow;
use serde::Serialize;

use crate::budgets_api::{current_usage, with_budgets};
use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, Length, Measure};
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
const SHOWN: [(Measure, Option<Length>); 7] = [
    (Measure::ContextPercent, None),
    (Measure::Tokens, Some(Length::Day)),
    (Measure::Tokens, Some(Length::Week)),
    (Measure::Dollars, Some(Length::Day)),
    (Measure::Dollars, Some(Length::Week)),
    (Measure::RunningMs, Some(Length::Day)),
    (Measure::RunningMs, Some(Length::Week)),
];

/// `agent`'s figures and its accounts' windows.
pub(crate) fn figures(
    state: &AppState,
    agent: &str,
) -> Result<(Vec<Used>, Vec<AccountWindows>), ServerError> {
    let zone = crate::configuration_api::organisation(state)?.zone;
    let agents = BTreeSet::from([agent.to_owned()]);
    let sessions = crate::runtime_api::session_agents(state, &agents)?;
    let at_ms = jiff::Timestamp::now().as_millisecond();
    with_budgets(state, |store| {
        let held = store.held();
        let figures = SHOWN
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
                current_usage(held, &limit, &agents, &zone, at_ms, sessions.as_ref())
            })
            .collect::<Result<Vec<_>, ServerError>>()?;
        let accounts = held
            .index
            .account_reports(agent)
            .into_iter()
            .filter_map(|(account, position)| {
                let usage = held.uses.get(position)?;
                Some(AccountWindows {
                    account: account.to_owned(),
                    at_ms: usage.at_ms,
                    windows: usage.plan_windows.clone()?,
                })
            })
            .collect();
        Ok((figures, accounts))
    })
}
