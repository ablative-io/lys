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
}

#[derive(Debug, Default, Clone)]
struct Agent {
    periods: Timeline,
    sessions: BTreeMap<String, Session>,
    unbound: Session,
    totals: crate::budgets_totals::Totals,
}

/// Rebuilt from retained records on open; never part of signed state.
#[derive(Debug, Default, Clone)]
pub struct Index {
    agents: BTreeMap<String, Agent>,
}

fn insert(timeline: &mut Timeline, usage: &Usage, position: usize) {
    timeline.insert((usage.at_ms, position), position);
}

fn latest(timeline: &Timeline, at_ms: i64) -> Option<usize> {
    timeline
        .range(..=(at_ms, usize::MAX))
        .next_back()
        .map(|(_, position)| *position)
}

impl Index {
    pub(crate) fn from_uses(uses: &[Usage]) -> Result<Self, String> {
        let mut index = Self::default();
        for (position, usage) in uses.iter().enumerate() {
            #[cfg(test)]
            crate::budgets_work::visit(crate::budgets_work::Work::Usage);
            index.insert(usage, position)?;
        }
        Ok(index)
    }

    pub(crate) fn insert(&mut self, usage: &Usage, position: usize) -> Result<(), String> {
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
    ) -> BTreeSet<usize> {
        let mut positions = BTreeSet::new();
        for agent in agents.iter().filter_map(|agent| self.agents.get(agent)) {
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
                positions.extend(latest(timeline, at_ms));
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
            .and_then(|session| latest(&session.baseline, at_ms))
    }
}

impl Agent {
    fn insert(&mut self, usage: &Usage, position: usize) -> Result<(), String> {
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
        if usage.plan_windows.is_some() {
            insert(&mut self.plan, usage, position);
        }
        if usage.context_percent.is_some() {
            insert(&mut self.context, usage, position);
        }
    }
}
