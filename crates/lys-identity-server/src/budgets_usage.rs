//! Spend sums charged events; context and plan percentages remain reported levels.

use std::collections::{BTreeMap, BTreeSet};

use lys_runner::tracking_budget::PlanWindow;
use serde::Serialize;
use serde_json::Number;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Held, Length, Measure, Period, Usage};

mod spend;
use spend::spend_gap;

/// One limit's measured figure or its named gap, never an invented zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct Used {
    /// The limit's unit.
    pub unit: Measure,
    /// The limit's period.
    pub period: Option<Length>,
    /// The current figure in the named unit.
    #[schema(value_type = Option<f64>)]
    pub figure: Option<Number>,
    /// The current period's start, reported window's start, or recorded expired reset.
    pub since_ms: Option<i64>,
    /// Why the figure is unavailable; absent exactly when a figure is present.
    pub unavailable: Option<String>,
    /// The account that reported the highest window, retained only for acts.
    #[serde(skip)]
    pub(crate) account: Option<String>,
}

/// Convert checked comparison micros back to the declared unit without a float cast.
pub fn number(unit: Measure, amount: u64) -> Result<Number, String> {
    match unit {
        Measure::Tokens | Measure::RunningMs => Ok(amount.into()),
        Measure::Dollars | Measure::PlanPercent | Measure::ContextPercent => {
            if amount % 1_000_000 == 0 {
                return Ok((amount / 1_000_000).into());
            }
            let text = format!("{}.{:06}", amount / 1_000_000, amount % 1_000_000);
            text.trim_end_matches('0')
                .parse()
                .map_err(|error| format!("reported amount does not encode: {error}"))
        }
    }
}

/// The calendar or fixed interval starts in the limit's effective zone.
pub fn start(limit: &Limit, zone: &str, at_ms: i64) -> Result<Option<i64>, String> {
    let Some(length) = limit.period else {
        return Ok(None);
    };
    Period {
        length,
        zone: limit.zone(zone).to_owned(),
    }
    .start_of(at_ms)
    .map(Some)
}

/// The next boundary is computed in the same zone, including daylight-saving changes.
pub fn reset(limit: &Limit, zone: &str, at_ms: i64) -> Result<Option<i64>, String> {
    use jiff::ToSpan;
    let Some((length, start)) = limit.period.zip(start(limit, zone, at_ms)?) else {
        return Ok(None);
    };
    if length == Length::FiveHour {
        return start
            .checked_add(18_000_000)
            .map(Some)
            .ok_or_else(|| "period reset overflows milliseconds".to_owned());
    }
    let zone = jiff::tz::TimeZone::get(limit.zone(zone)).map_err(|error| error.to_string())?;
    let local = jiff::Timestamp::from_millisecond(start)
        .map_err(|error| error.to_string())?
        .to_zoned(zone);
    let next = match length {
        Length::Day => local.date().checked_add(1.days()),
        Length::Week => local.date().checked_add(7.days()),
        Length::Month => local.date().checked_add(1.months()),
        Length::FiveHour => return Err("fixed period reached calendar reset".to_owned()),
    }
    .map_err(|error| error.to_string())?;
    Ok(Some(
        next.to_zoned(local.time_zone().clone())
            .map_err(|error| error.to_string())?
            .timestamp()
            .as_millisecond(),
    ))
}

fn unavailable(limit: &Limit, since_ms: Option<i64>, reason: impl Into<String>) -> Used {
    Used {
        unit: limit.unit,
        period: limit.period,
        figure: None,
        since_ms,
        unavailable: Some(reason.into()),
        account: None,
    }
}

fn present(limit: &Limit, since_ms: Option<i64>, amount: u64) -> Result<Used, String> {
    Ok(Used {
        unit: limit.unit,
        period: limit.period,
        figure: Some(number(limit.unit, amount)?),
        since_ms,
        unavailable: None,
        account: None,
    })
}

/// Include an uncharged event only when computing the threshold it is about to cross.
pub fn figure(
    held: &Held,
    limit: &Limit,
    agents: &BTreeSet<String>,
    zone: &str,
    at_ms: i64,
    incoming: Option<&Usage>,
) -> Result<Used, String> {
    figure_with_sessions(held, limit, agents, zone, at_ms, incoming, None)
}

/// Distinguish a never-run agent from one awaiting its first usage report.
pub(crate) fn figure_with_sessions(
    held: &Held,
    limit: &Limit,
    agents: &BTreeSet<String>,
    zone: &str,
    at_ms: i64,
    incoming: Option<&Usage>,
    sessions: Option<&BTreeMap<String, crate::runtime_store::SessionActivity>>,
) -> Result<Used, String> {
    if let Some(known) = sessions
        && let Some(agent) = agents.iter().find(|agent| !known.contains_key(*agent))
    {
        return Err(format!(
            "session activity was not selected for agent {agent}"
        ));
    }
    Reading {
        held,
        limit,
        agents,
        zone,
        at_ms,
        incoming,
        sessions,
    }
    .figure(Purpose::Spend)
}

/// Probe native reporting separately from a fresh period's known zero spend.
pub(crate) fn source_figure(
    held: &Held,
    limit: &Limit,
    agents: &BTreeSet<String>,
    zone: &str,
    at_ms: i64,
) -> Result<Used, String> {
    Reading {
        held,
        limit,
        agents,
        zone,
        at_ms,
        incoming: None,
        sessions: None,
    }
    .figure(Purpose::Source)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Purpose {
    Spend,
    Source,
}

struct Reading<'a> {
    held: &'a Held,
    limit: &'a Limit,
    agents: &'a BTreeSet<String>,
    zone: &'a str,
    at_ms: i64,
    incoming: Option<&'a Usage>,
    sessions: Option<&'a BTreeMap<String, crate::runtime_store::SessionActivity>>,
}

impl Reading<'_> {
    fn figure(self, purpose: Purpose) -> Result<Used, String> {
        let Self {
            held,
            limit,
            agents,
            zone,
            at_ms,
            incoming,
            sessions,
        } = self;
        let incoming = incoming.filter(|usage| !held.charged.contains(&usage.event));
        let since = if matches!(limit.unit, Measure::PlanPercent | Measure::ContextPercent) {
            None
        } else {
            start(limit, zone, at_ms)?
        };
        let selected = held.index.positions(
            agents,
            limit.unit,
            at_ms,
            if purpose == Purpose::Spend {
                since
            } else {
                None
            },
        );
        let selected = selected
            .into_iter()
            .map(|position| {
                held.uses
                    .get(position)
                    .ok_or_else(|| format!("usage index names missing record {position}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let uses: Vec<_> = selected
            .into_iter()
            .chain(incoming)
            .filter(|usage| {
                #[cfg(test)]
                crate::budgets_work::visit(crate::budgets_work::Work::Usage);
                agents.contains(&usage.agent)
                    && usage.at_ms <= at_ms
                    && (purpose != Purpose::Spend || since.is_none_or(|since| usage.at_ms >= since))
            })
            .collect();
        let never_run = |agent: &str| {
            sessions.is_some_and(|known| {
                known
                    .get(agent)
                    .is_some_and(crate::runtime_store::SessionActivity::never_ran)
            }) && !held.index.has_usage(agent)
                && incoming.is_none_or(|usage| usage.agent != agent)
        };
        if limit.unit == Measure::PlanPercent {
            return plan(limit, agents, &uses, at_ms, purpose, &never_run);
        }
        if limit.unit == Measure::ContextPercent {
            return context(limit, &uses);
        }
        let mut spend = held
            .index
            .spend(&held.uses, agents, limit, zone, since, at_ms)?;
        if let Some(incoming) = incoming.filter(|usage| {
            agents.contains(&usage.agent)
                && usage.at_ms <= at_ms
                && since.is_none_or(|since| usage.at_ms >= since)
        }) {
            spend.add(limit.unit, incoming, held.uses.len())?;
        }
        if let Some(position) = spend.gap {
            let usage = if position == held.uses.len() {
                incoming.ok_or("period gap names no incoming record")?
            } else {
                held.uses
                    .get(position)
                    .ok_or("period gap names a missing record")?
            };
            return Ok(unavailable(limit, since, period_gap(limit.unit, usage)?));
        }
        if let Some(reason) = spend_gap(limit, agents, &uses, since, purpose, sessions) {
            return Ok(unavailable(limit, since, reason));
        }
        let total = spend
            .total
            .and_then(|total| u64::try_from(total).ok())
            .ok_or("budget spend overflows its reported unit")?;
        present(limit, since, total)
    }
}

fn period_gap(unit: Measure, usage: &Usage) -> Result<&str, String> {
    let field = match unit {
        Measure::Tokens => "tokens",
        Measure::RunningMs => "running_ms",
        Measure::Dollars => "dollars_micros",
        Measure::PlanPercent | Measure::ContextPercent => {
            return Err("a level reached spend summation".to_owned());
        }
    };
    usage
        .unavailable
        .iter()
        .find(|gap| {
            gap.figure == field && (unit == Measure::Tokens || gap.reason.contains("reset"))
        })
        .map(|gap| gap.reason.as_str())
        .ok_or_else(|| "period gap names a record without its source refusal".to_owned())
}

fn plan(
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
    let reports = match plan_reports(agents, uses, duration, now, purpose, never_run) {
        Ok(reports) => reports,
        Err(reason) => return Ok(unavailable(limit, None, reason)),
    };
    if reports.is_empty() && purpose == Purpose::Spend {
        return present(limit, None, 0);
    }
    let mut largest: Option<(Number, i64, String)> = None;
    let mut expired: Option<i64> = None;
    for (account, window) in reports {
        let Some(window) = window else {
            return Ok(unavailable(
                limit,
                None,
                "the requested account plan window is unreported",
            ));
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
            largest = Some((window.used_percent.clone(), since, account.to_owned()));
        }
    }
    match largest {
        Some((amount, since, account)) => Ok(Used {
            unit: limit.unit,
            period: limit.period,
            figure: Some(amount),
            since_ms: Some(since),
            unavailable: None,
            account: Some(account),
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

fn context(limit: &Limit, uses: &[&Usage]) -> Result<Used, String> {
    let mut latest = BTreeMap::new();
    for usage in uses {
        if let Some(context) = usage.context_percent {
            let slot = latest
                .entry((&usage.agent, &usage.session))
                .or_insert((i64::MIN, context));
            if usage.at_ms >= slot.0 {
                *slot = (usage.at_ms, context);
            }
        }
    }
    match latest.values().map(|(_, context)| *context).max() {
        Some(context) => present(
            limit,
            None,
            context
                .checked_mul(1_000_000)
                .ok_or("context percentage overflows")?,
        ),
        None => Ok(unavailable(
            limit,
            None,
            "context_percent has not been reported",
        )),
    }
}

fn plan_reports<'a>(
    agents: &BTreeSet<String>,
    uses: &[&'a Usage],
    duration: u64,
    now: u64,
    purpose: Purpose,
    never_run: &impl Fn(&str) -> bool,
) -> Result<BTreeMap<&'a str, Option<&'a PlanWindow>>, String> {
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
    for agent in agents {
        if (purpose != Purpose::Spend || !never_run(agent))
            && !sessions.keys().any(|(reported, _)| *reported == agent)
        {
            return Err(format!("plan window unreported for agent {agent}"));
        }
    }
    let mut accounts = BTreeMap::new();
    for (index, report, window) in sessions.values().copied() {
        let account = report
            .account
            .as_deref()
            .filter(|account| !account.is_empty())
            .ok_or("the reported plan window names no account")?;
        let latest = accounts.entry(account).or_insert((index, report, window));
        if (report.at_ms, index) > (latest.1.at_ms, latest.0) {
            *latest = (index, report, window);
        }
    }
    Ok(accounts
        .into_iter()
        .map(|(account, (_, report, window))| {
            let window = window.or_else(|| {
                if !report.unavailable.iter().any(|gap| {
                    gap.figure == "plan_windows"
                        && gap.reason == format!("{duration}_minute_window_expired")
                }) {
                    return None;
                }
                resets
                    .get(&(&report.agent, &report.session, Some(account)))
                    .map(|(_, _, window)| *window)
                    .filter(|window| window.resets_at_ms <= now)
            });
            (account, window)
        })
        .collect())
}
