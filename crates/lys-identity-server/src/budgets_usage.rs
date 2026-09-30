//! Spend sums charged events; context and plan percentages remain reported levels.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Number;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Held, Length, Measure, Period, Usage};

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
    /// The start of the current period or reported window.
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
    limit
        .period
        .map(|length| {
            Period {
                length,
                zone: limit.zone(zone).to_owned(),
            }
            .start_of(at_ms)
        })
        .transpose()
}

/// The next boundary is computed in the same zone, including daylight-saving changes.
pub fn reset(limit: &Limit, zone: &str, at_ms: i64) -> Result<Option<i64>, String> {
    use jiff::ToSpan;
    let Some(length) = limit.period else {
        return Ok(None);
    };
    let Some(start) = start(limit, zone, at_ms)? else {
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
    let incoming = incoming.filter(|usage| !held.charged.contains(&usage.event));
    let uses: Vec<_> = held
        .uses
        .iter()
        .chain(incoming)
        .filter(|usage| agents.contains(&usage.agent) && usage.at_ms <= at_ms)
        .collect();
    if limit.unit == Measure::PlanPercent {
        return plan(limit, agents, &uses, at_ms);
    }
    if limit.unit == Measure::ContextPercent {
        return context(limit, &uses);
    }
    let since = start(limit, zone, at_ms)?;
    if let Some(reason) = spend_gap(limit, agents, &uses, since) {
        return Ok(unavailable(limit, since, reason));
    }
    if limit.unit == Measure::Tokens
        && let Some(gap) = uses
            .iter()
            .filter(|usage| since.is_none_or(|start| usage.at_ms >= start))
            .flat_map(|usage| &usage.unavailable)
            .find(|gap| gap.figure == "tokens")
    {
        return Ok(unavailable(limit, since, &gap.reason));
    }
    let mut total = 0_u64;
    for usage in uses {
        if since.is_some_and(|start| usage.at_ms < start) {
            continue;
        }
        let amount = match limit.unit {
            Measure::Tokens => usage.tokens,
            Measure::RunningMs => usage.running_ms,
            Measure::Dollars => {
                let Some(amount) = usage.dollars_micros else {
                    continue;
                };
                amount
            }
            Measure::ContextPercent | Measure::PlanPercent => {
                return Err("a level reached spend summation".to_owned());
            }
        };
        total = total
            .checked_add(amount)
            .ok_or("budget spend overflows its reported unit")?;
    }
    present(limit, since, total)
}

fn plan(
    limit: &Limit,
    agents: &BTreeSet<String>,
    uses: &[&Usage],
    at_ms: i64,
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
    let reports = match plan_reports(agents, uses) {
        Ok(reports) => reports,
        Err(reason) => return Ok(unavailable(limit, None, reason)),
    };
    let now = u64::try_from(at_ms)
        .map_err(|error| format!("plan observation is before the Unix epoch: {error}"))?;
    let mut largest: Option<(Number, i64, String)> = None;
    let mut expired = false;
    for (account, report) in reports {
        let Some(window) = report.plan_windows.as_ref().and_then(|windows| {
            windows
                .iter()
                .find(|window| window.duration_minutes == duration)
        }) else {
            if report.unavailable.iter().any(|gap| {
                gap.figure == "plan_windows"
                    && gap.reason == format!("{duration}_minute_window_expired")
            }) {
                expired = true;
                continue;
            }
            return Ok(unavailable(
                limit,
                None,
                "the requested account plan window is unreported",
            ));
        };
        if window.resets_at_ms <= now {
            expired = true;
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
            None,
            if expired {
                "the reported account plan window has expired"
            } else {
                "no accounts report the requested plan window"
            },
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

fn spend_gap(
    limit: &Limit,
    agents: &BTreeSet<String>,
    uses: &[&Usage],
    since: Option<i64>,
) -> Option<String> {
    if matches!(limit.unit, Measure::Dollars | Measure::RunningMs) {
        let field = if limit.unit == Measure::Dollars {
            "dollars_micros"
        } else {
            "running_ms"
        };
        if let Some(gap) = uses
            .iter()
            .filter(|usage| since.is_none_or(|start| usage.at_ms >= start))
            .flat_map(|usage| &usage.unavailable)
            .find(|gap| gap.figure == field && gap.reason.contains("reset"))
        {
            return Some(gap.reason.clone());
        }
        let mut reports = BTreeMap::new();
        for usage in uses {
            let reported = if limit.unit == Measure::Dollars {
                usage.dollars_micros.is_some()
            } else {
                usage.reported_running_ms.is_some()
            };
            if usage.native_snapshot || reported {
                let key = (&usage.agent, &usage.session);
                if reports
                    .get(&key)
                    .is_none_or(|earlier: &&Usage| earlier.at_ms <= usage.at_ms)
                {
                    reports.insert(key, *usage);
                }
            }
        }
        if limit.unit == Measure::Dollars {
            for agent in agents {
                if !reports.keys().any(|(reported, _)| *reported == agent) {
                    return Some(format!("dollars have not been reported for agent {agent}"));
                }
            }
            if agents.is_empty() {
                return Some("no agents report dollar spend".to_owned());
            }
        }
        for report in reports.values() {
            let missing = if limit.unit == Measure::Dollars {
                report.dollars_micros.is_none()
            } else {
                report.reported_running_ms.is_none()
            };
            if missing {
                let reason = report
                    .unavailable
                    .iter()
                    .find(|gap| gap.figure == field)
                    .map_or("native source does not report this spend figure", |gap| {
                        gap.reason.as_str()
                    });
                return Some(reason.to_owned());
            }
        }
    }
    None
}

fn plan_reports<'a>(
    agents: &BTreeSet<String>,
    uses: &[&'a Usage],
) -> Result<BTreeMap<&'a str, &'a Usage>, String> {
    let mut sessions = BTreeMap::new();
    for (index, usage) in uses.iter().copied().enumerate() {
        if usage.plan_windows.is_some() {
            let key = (&usage.agent, &usage.session);
            if sessions
                .get(&key)
                .is_none_or(|(earlier_index, earlier): &(usize, &Usage)| {
                    (usage.at_ms, index) > (earlier.at_ms, *earlier_index)
                })
            {
                sessions.insert(key, (index, usage));
            }
        }
    }
    for agent in agents {
        if !sessions.keys().any(|(reported, _)| *reported == agent) {
            return Err(format!("plan window unreported for agent {agent}"));
        }
    }
    let mut accounts = BTreeMap::new();
    for (index, report) in sessions.values().copied() {
        let account = report
            .account
            .as_deref()
            .filter(|account| !account.is_empty())
            .ok_or("the reported plan window names no account")?;
        if accounts
            .get(account)
            .is_none_or(|(earlier_index, earlier): &(usize, &Usage)| {
                (report.at_ms, index) > (earlier.at_ms, *earlier_index)
            })
        {
            accounts.insert(account, (index, report));
        }
    }
    Ok(accounts
        .into_iter()
        .map(|(account, (_, report))| (account, report))
        .collect())
}
