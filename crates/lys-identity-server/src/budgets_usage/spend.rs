//! Spend stays unavailable until required live sessions have native coverage.

use std::collections::{BTreeMap, BTreeSet};

use crate::budgets_limits::Limit;
use crate::budgets_state::{Measure, Usage};

use super::Purpose;

type SessionReports<'a> = BTreeMap<Option<&'a str>, &'a Usage>;

pub(super) fn spend_gap(
    limit: &Limit,
    agents: &BTreeSet<String>,
    uses: &[&Usage],
    since: Option<i64>,
    purpose: Purpose,
    sessions: Option<&BTreeMap<String, crate::runtime_store::SessionActivity>>,
) -> Option<String> {
    let field = match limit.unit {
        Measure::Dollars => "dollars_micros",
        Measure::RunningMs => "running_ms",
        _ => return None,
    };
    if let Some(gap) = uses
        .iter()
        .filter(|usage| since.is_none_or(|start| usage.at_ms >= start))
        .filter(|usage| limit.unit != Measure::Dollars || usage.dollars_micros.is_none())
        .flat_map(|usage| &usage.unavailable)
        .find(|gap| gap.figure == field && gap.reason.contains("reset"))
    {
        return Some(gap.reason.clone());
    }
    let mut reports: BTreeMap<&str, SessionReports<'_>> = BTreeMap::new();
    let mut observed = BTreeSet::new();
    for usage in uses {
        if limit.unit == Measure::Dollars && purpose == Purpose::Spend {
            observed.insert(usage.agent.as_str());
        }
        let reported = if limit.unit == Measure::Dollars {
            usage.dollars_micros.is_some()
        } else {
            usage.reported_running_ms.is_some()
        };
        if usage.native_snapshot || reported {
            let reported = reports.entry(usage.agent.as_str()).or_default();
            let key = usage.session.as_deref();
            let latest = reported.entry(key).or_insert(*usage);
            if latest.at_ms <= usage.at_ms {
                *latest = *usage;
            }
        }
    }
    if limit.unit == Measure::Dollars {
        for agent in agents {
            let activity = sessions.and_then(|known| known.get(agent));
            let reported = reports.get(agent.as_str());
            if purpose == Purpose::Spend
                && let Some(activity) = activity
                && let Some(session) = activity.unreported_session(|session| {
                    reported.is_some_and(|reports| reports.contains_key(&Some(session)))
                })
            {
                return Some(format!(
                    "dollars have not been reported for live session {session} of agent {agent}"
                ));
            }
            if (purpose != Purpose::Spend
                || activity.map_or(sessions.is_none(), |known| known.active_since(since))
                || observed.contains(agent.as_str()))
                && reported.is_none()
            {
                return Some(format!("dollars have not been reported for agent {agent}"));
            }
        }
        if agents.is_empty() && purpose == Purpose::Source {
            return Some("no agents report dollar spend".to_owned());
        }
    }
    for report in reports.values().flat_map(BTreeMap::values) {
        if (limit.unit == Measure::Dollars && report.dollars_micros.is_none())
            || (limit.unit == Measure::RunningMs && report.reported_running_ms.is_none())
        {
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
    None
}
