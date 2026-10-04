//! Derived record positions select a period or the latest session report without history scans.

use std::collections::{BTreeMap, BTreeSet};

use crate::budgets_state::{Measure, Usage};

type Timeline = BTreeMap<(i64, usize), usize>;

#[derive(Debug, Default, Clone)]
struct Session {
    dollars: Timeline,
    running: Timeline,
    baseline: Timeline,
    plan: Timeline,
    context: Timeline,
    windows: BTreeMap<(Option<String>, u64), Timeline>,
}

#[derive(Debug, Default, Clone)]
struct Agent {
    periods: Timeline,
    sessions: BTreeMap<String, Session>,
    unbound: Session,
    totals: crate::budgets_totals::Totals,
    /// Each account's reports of its windows, whichever session made them.
    accounts: BTreeMap<String, Timeline>,
    /// The uses that each count one model call, in the order made.
    calls: Timeline,
    /// Each call's use, by the call's id.
    call_ids: BTreeMap<String, usize>,
}

#[cfg(test)]
#[path = "budgets_index_tests.rs"]
mod tests;

/// Rebuilt from retained records on open; never part of signed state.
#[derive(Debug, Default)]
pub struct Index {
    agents: BTreeMap<String, Agent>,
    /// Every model a kept call named.
    models: BTreeSet<String>,
}

fn insert(timeline: &mut Timeline, usage: &Usage, position: usize) {
    timeline.insert((usage.at_ms, position), position);
}

fn latest(timeline: &Timeline, since: Option<i64>, at_ms: i64) -> Option<usize> {
    if since.is_some_and(|since| since > at_ms) {
        return None;
    }
    timeline
        .range((since.unwrap_or(i64::MIN), 0)..=(at_ms, usize::MAX))
        .next_back()
        .map(|(_, position)| *position)
}

impl Clone for Index {
    fn clone(&self) -> Self {
        #[cfg(test)]
        crate::budgets_work::visit(crate::budgets_work::Work::IndexCopy);
        Self {
            agents: self.agents.clone(),
            models: self.models.clone(),
        }
    }
}

impl Index {
    pub(crate) fn last_reported(&self, agent: &str) -> Option<i64> {
        self.agents
            .get(agent)
            .and_then(|agent| agent.periods.last_key_value())
            .map(|((at, _), _)| *at)
    }

    /// The position of each account's latest report of its windows for
    /// `agent`, by the account's name: one lookup for each account, never a
    /// walk of the uses.
    pub(crate) fn account_reports(&self, agent: &str) -> Vec<(&str, usize)> {
        self.agents.get(agent).map_or_else(Vec::new, |agent| {
            agent
                .accounts
                .iter()
                .filter_map(|(account, timeline)| {
                    timeline
                        .last_key_value()
                        .map(|(_, position)| (account.as_str(), *position))
                })
                .collect()
        })
    }

    /// The positions of up to `count` of `agent`'s calls made before
    /// `before`, newest first, and whether older ones remain: a walk of that
    /// many index rows, never of the uses.
    pub(crate) fn calls(
        &self,
        agent: &str,
        before: Option<(i64, usize)>,
        count: usize,
    ) -> (Vec<usize>, bool) {
        let Some(agent) = self.agents.get(agent) else {
            return (Vec::new(), false);
        };
        let mut older = agent
            .calls
            .range(..before.unwrap_or((i64::MAX, usize::MAX)))
            .rev()
            .map(|(_, position)| *position);
        let page = older.by_ref().take(count).collect();
        (page, older.next().is_some())
    }

    /// The position of the use that counts `agent`'s call `id`.
    pub(crate) fn call(&self, agent: &str, id: &str) -> Option<usize> {
        self.agents.get(agent)?.call_ids.get(id).copied()
    }

    pub(crate) fn has_usage(&self, agent: &str) -> bool {
        self.agents.contains_key(agent)
    }

    pub(crate) fn from_uses(uses: &[Usage]) -> Result<Self, String> {
        let mut index = Self::default();
        for (position, usage) in uses.iter().enumerate() {
            #[cfg(test)]
            crate::budgets_work::visit(crate::budgets_work::Work::Usage);
            index.insert(usage, position)?;
        }
        Ok(index)
    }

    /// Every model a kept call named, in name order.
    pub(crate) const fn models(&self) -> &BTreeSet<String> {
        &self.models
    }

    pub(crate) fn insert(&mut self, usage: &Usage, position: usize) -> Result<(), String> {
        if let Some(model) = usage.call.as_ref().and_then(|call| call.model.as_ref())
            && !self.models.contains(model)
        {
            self.models.insert(model.clone());
        }
        if let Some(agent) = self.agents.get_mut(&usage.agent) {
            agent.insert(usage, position)?;
        } else {
            let mut agent = Agent::default();
            agent.insert(usage, position)?;
            self.agents.insert(usage.agent.clone(), agent);
        }
        Ok(())
    }

    pub(crate) fn positions(
        &self,
        agents: &BTreeSet<String>,
        unit: Measure,
        at_ms: i64,
        since: Option<i64>,
    ) -> BTreeSet<usize> {
        let mut positions = BTreeSet::new();
        if unit == Measure::Tokens {
            return positions;
        }
        for agent in agents.iter().filter_map(|agent| self.agents.get(agent)) {
            if unit == Measure::Dollars {
                positions.extend(latest(&agent.periods, since, at_ms));
            }
            for session in agent
                .sessions
                .values()
                .chain(std::iter::once(&agent.unbound))
            {
                let timeline = match unit {
                    Measure::Dollars => &session.dollars,
                    Measure::RunningMs => &session.running,
                    Measure::PlanPercent => &session.plan,
                    Measure::ContextPercent => &session.context,
                    Measure::Tokens => continue,
                };
                positions.extend(latest(timeline, since, at_ms));
                if unit == Measure::PlanPercent {
                    for timeline in session.windows.values() {
                        positions.extend(latest(timeline, None, at_ms));
                    }
                }
            }
        }
        positions
    }

    pub(crate) fn spend(
        &self,
        uses: &[Usage],
        agents: &BTreeSet<String>,
        limit: &crate::budgets_limits::Limit,
        zone: &str,
        since: Option<i64>,
        at_ms: i64,
    ) -> Result<crate::budgets_totals::Spend, String> {
        let mut spend = crate::budgets_totals::Spend {
            total: Some(0),
            gap: None,
        };
        for agent in agents.iter().filter_map(|agent| self.agents.get(agent)) {
            let added = agent
                .totals
                .query(uses, &agent.periods, limit, zone, since, at_ms)?;
            spend.total = spend
                .total
                .zip(added.total)
                .and_then(|(total, added)| total.checked_add(added));
            if let Some(gap) = added.gap
                && spend.gap.is_none_or(|earlier| gap < earlier)
            {
                spend.gap = Some(gap);
            }
        }
        Ok(spend)
    }

    pub(crate) fn baseline(&self, agent: &str, session: &str, at_ms: i64) -> Option<usize> {
        self.agents
            .get(agent)
            .and_then(|agent| agent.sessions.get(session))
            .and_then(|session| latest(&session.baseline, None, at_ms))
    }
}

impl Agent {
    fn insert(&mut self, usage: &Usage, position: usize) -> Result<(), String> {
        if let (Some(account), Some(_)) = (&usage.account, &usage.plan_windows) {
            insert(
                self.accounts.entry(account.clone()).or_default(),
                usage,
                position,
            );
        }
        if let Some(call) = &usage.call {
            insert(&mut self.calls, usage, position);
            self.call_ids.insert(call.id.clone(), position);
        }
        self.totals.record(usage, position)?;
        insert(&mut self.periods, usage, position);
        if let Some(named) = usage.session.as_deref() {
            if let Some(session) = self.sessions.get_mut(named) {
                session.insert(usage, position);
            } else {
                let mut session = Session::default();
                session.insert(usage, position);
                self.sessions.insert(named.to_owned(), session);
            }
        } else {
            self.unbound.insert(usage, position);
        }
        Ok(())
    }
}

impl Session {
    fn insert(&mut self, usage: &Usage, position: usize) {
        if usage.native_snapshot || usage.dollars_micros.is_some() {
            insert(&mut self.dollars, usage, position);
        }
        if usage.native_snapshot || usage.reported_running_ms.is_some() {
            insert(&mut self.running, usage, position);
        }
        if usage.reported_running_ms.is_some() {
            insert(&mut self.baseline, usage, position);
        }
        if let Some(windows) = &usage.plan_windows {
            insert(&mut self.plan, usage, position);
            for window in windows {
                insert(
                    self.windows
                        .entry((usage.account.clone(), window.duration_minutes))
                        .or_default(),
                    usage,
                    position,
                );
            }
        }
        if usage.context_percent.is_some() {
            insert(&mut self.context, usage, position);
        }
    }
}
