//! Account windows remain levels reported by the provider, never token-derived estimates.

use std::collections::{BTreeMap, BTreeSet};

use lys_runner::tracking_budget::PlanWindow;
use serde_json::Number;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Length, Usage};

use super::{Purpose, ReportedPart, Used, present, unavailable};

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;

struct Reports<'a> {
    windows: BTreeMap<&'a str, Option<&'a PlanWindow>>,
    missing: Vec<String>,
}

pub(super) fn plan(
    limit: &Limit,
    agents: &BTreeSet<String>,
    uses: &[&Usage],
    at_ms: i64,
    purpose: Purpose,
    never_run: &impl Fn(&str) -> bool,
) -> Result<Used, String> {
    let duration = match limit.period {
        Some(Length::FiveHour) => 300,
        Some(Length::Week) => 10_080,
        _ => {
            return Ok(unavailable(
                limit,
                None,
                "the plan has no reported window for this period",
            ));
        }
    };
    let now = u64::try_from(at_ms)
        .map_err(|error| format!("plan observation is before the Unix epoch: {error}"))?;
    let reports = match plan_reports(agents, uses, duration, purpose, never_run) {
        Ok(reports) => reports,
        Err(reason) => return Ok(unavailable(limit, None, reason)),
    };
    if reports.windows.is_empty() && reports.missing.is_empty() && purpose == Purpose::Spend {
        return present(limit, None, 0);
    }
    let mut largest: Option<(Number, i64, String)> = None;
    let mut expired: Option<i64> = None;
    for (account, window) in &reports.windows {
        let Some(window) = window else {
            continue;
        };
        if window.resets_at_ms <= now {
            let boundary = i64::try_from(window.resets_at_ms)
                .map_err(|error| format!("reported plan reset is not representable: {error}"))?;
            expired = Some(expired.map_or(boundary, |earlier| earlier.max(boundary)));
            continue;
        }
        lys_runner::tracking_budget::percent(&serde_json::Value::Number(
            window.used_percent.clone(),
        ))
        .ok_or("reported plan percentage is invalid")?;
        let length = duration
            .checked_mul(60_000)
            .ok_or("reported plan duration overflows")?;
        let since = window
            .resets_at_ms
            .checked_sub(length)
            .and_then(|value| i64::try_from(value).ok())
            .ok_or("reported plan window start is not representable")?;
        let larger = largest
            .as_ref()
            .map(|(earlier, _, _)| {
                lys_runner::tracking_budget::compare(&window.used_percent, earlier)
                    .map(|order| order == std::cmp::Ordering::Greater)
            })
            .transpose()?
            .unwrap_or(true);
        if larger {
            largest = Some((window.used_percent.clone(), since, (*account).to_owned()));
        }
    }
    if !reports.missing.is_empty() {
        let mut answer = unavailable(limit, None, reports.missing.join("; "));
        answer.reported = largest.map(|(figure, _, _)| ReportedPart {
            figure,
            missing: reports.missing.len(),
        });
        return Ok(answer);
    }
    match largest {
        Some((amount, since, account)) => Ok(Used {
            unit: limit.unit,
            period: limit.period,
            figure: Some(amount),
            since_ms: Some(since),
            unavailable: None,
            reported: None,
            account: Some(account),
            source: None,
            context: None,
            tokens: None,
        }),
        None => Ok(unavailable(
            limit,
            expired,
            expired.map_or_else(
                || "no accounts report the requested plan window".to_owned(),
                |boundary| format!("the plan window reset at {boundary}; no report since"),
            ),
        )),
    }
}

fn plan_reports<'a>(
    agents: &BTreeSet<String>,
    uses: &[&'a Usage],
    duration: u64,
    purpose: Purpose,
    never_run: &impl Fn(&str) -> bool,
) -> Result<Reports<'a>, String> {
    let mut sessions = BTreeMap::new();
    let mut resets = BTreeMap::new();
    for (index, usage) in uses.iter().copied().enumerate() {
        let window = usage.plan_windows.as_ref().and_then(|windows| {
            windows
                .iter()
                .find(|window| window.duration_minutes == duration)
        });
        if let Some(window) = window {
            let key = (&usage.agent, &usage.session, usage.account.as_deref());
            let latest = resets.entry(key).or_insert((index, usage, window));
            if (usage.at_ms, index) > (latest.1.at_ms, latest.0) {
                *latest = (index, usage, window);
            }
        }
        if usage.plan_windows.is_some() {
            let key = (&usage.agent, &usage.session);
            let latest = sessions.entry(key).or_insert((index, usage, window));
            if (usage.at_ms, index) > (latest.1.at_ms, latest.0) {
                *latest = (index, usage, window);
            }
        }
    }
    let mut missing = Vec::new();
    for agent in agents {
        if (purpose != Purpose::Spend || !never_run(agent))
            && !sessions.keys().any(|(reported, _)| *reported == agent)
        {
            missing.push(format!("plan window unreported for agent {agent}"));
        }
    }
    let mut accounts = BTreeMap::new();
    for (mut index, mut report, mut window) in sessions.values().copied() {
        let account = report
            .account
            .as_deref()
            .filter(|account| !account.is_empty())
            .ok_or("the reported plan window names no account")?;
        if window.is_none()
            && let Some((earlier_index, earlier_report, earlier_window)) =
                resets.get(&(&report.agent, &report.session, Some(account)))
        {
            // A snapshot that omitted this window did not replace the provider's last reading.
            index = *earlier_index;
            report = *earlier_report;
            window = Some(*earlier_window);
        }
        let latest = accounts.entry(account).or_insert((index, report, window));
        if (window.is_some() && latest.2.is_none())
            || (window.is_some() == latest.2.is_some()
                && (report.at_ms, index) > (latest.1.at_ms, latest.0))
        {
            *latest = (index, report, window);
        }
    }
    let windows = accounts
        .into_iter()
        .map(|(account, (_, report, window))| {
            if window.is_none() {
                let reason = report
                    .unavailable
                    .iter()
                    .find(|gap| gap.figure == "plan_windows");
                missing.push(match reason {
                    Some(gap) => format!(
                        "{duration}-minute plan window unreported for account {account}: {}",
                        gap.reason
                    ),
                    None => {
                        format!("{duration}-minute plan window unreported for account {account}")
                    }
                });
            }
            (account, window)
        })
        .collect();
    Ok(Reports { windows, missing })
}
