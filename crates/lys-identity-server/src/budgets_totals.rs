//! Period totals advance with confirmed records; a failed read never replaces a cached figure.

use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included};
use std::sync::Mutex;

use crate::budgets_limits::Limit;
use crate::budgets_state::{Length, Measure, Usage};

type Timeline = BTreeMap<(i64, usize), usize>;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Spend {
    pub(crate) total: Option<u128>,
    pub(crate) gap: Option<usize>,
    /// How many records in the period carry no figure for the unit.
    pub(crate) gaps: usize,
}

impl Spend {
    pub(crate) const fn empty() -> Self {
        Self {
            total: Some(0),
            gap: None,
            gaps: 0,
        }
    }

    pub(crate) fn add(
        &mut self,
        unit: Measure,
        usage: &Usage,
        position: usize,
    ) -> Result<(), String> {
        let gap = usage.unavailable.iter().any(|gap| match unit {
            Measure::Tokens => gap.figure == "tokens",
            Measure::RunningMs => gap.figure == "running_ms" && gap.reason.contains("reset"),
            Measure::Dollars => {
                usage.dollars_micros.is_none()
                    && gap.figure == "dollars_micros"
                    && gap.reason.contains("reset")
            }
            Measure::PlanPercent | Measure::ContextPercent => false,
        });
        if gap {
            self.gaps += 1;
            if self.gap.is_none_or(|earlier| position < earlier) {
                self.gap = Some(position);
            }
        }
        let amount = match unit {
            Measure::Tokens => usage.tokens,
            Measure::RunningMs => usage.running_ms,
            Measure::Dollars => {
                let Some(amount) = usage.dollars_micros else {
                    return Ok(());
                };
                amount
            }
            Measure::PlanPercent | Measure::ContextPercent => {
                return Err("a level reached spend summation".to_owned());
            }
        };
        self.total = self
            .total
            .and_then(|total| total.checked_add(u128::from(amount)));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct State {
    since: i64,
    at_ms: i64,
    spend: [Spend; 3],
}

impl State {
    const fn new(since: i64, at_ms: i64) -> Self {
        Self {
            since,
            at_ms,
            spend: [Spend::empty(); 3],
        }
    }

    fn read(
        &mut self,
        uses: &[Usage],
        timeline: impl Iterator<Item = usize>,
    ) -> Result<(), String> {
        for position in timeline {
            #[cfg(test)]
            crate::budgets_work::visit(crate::budgets_work::Work::Usage);
            let usage = uses
                .get(position)
                .ok_or_else(|| format!("period index names missing record {position}"))?;
            for (unit, spend) in [Measure::Tokens, Measure::RunningMs, Measure::Dollars]
                .into_iter()
                .zip(&mut self.spend)
            {
                spend.add(unit, usage, position)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Period {
    length: Option<Length>,
    zone: String,
    state: State,
}

#[derive(Debug, Default)]
pub(crate) struct Totals {
    periods: Mutex<Vec<Period>>,
}

impl Clone for Totals {
    fn clone(&self) -> Self {
        // Derived totals are rebuilt from copied record positions rather than copied locks.
        Self::default()
    }
}

impl Totals {
    pub(crate) fn record(&mut self, usage: &Usage, position: usize) -> Result<(), String> {
        let periods = self
            .periods
            .get_mut()
            .map_err(|error| format!("period totals lock poisoned: {error}"))?;
        for period in periods {
            if usage.at_ms >= period.state.since && usage.at_ms <= period.state.at_ms {
                for (unit, spend) in [Measure::Tokens, Measure::RunningMs, Measure::Dollars]
                    .into_iter()
                    .zip(&mut period.state.spend)
                {
                    spend.add(unit, usage, position)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn query(
        &self,
        uses: &[Usage],
        timeline: &Timeline,
        limit: &Limit,
        zone: &str,
        since: Option<i64>,
        at_ms: i64,
    ) -> Result<Spend, String> {
        let unit = match limit.unit {
            Measure::Tokens => 0,
            Measure::RunningMs => 1,
            Measure::Dollars => 2,
            Measure::PlanPercent | Measure::ContextPercent => {
                return Err("a level reached spend summation".to_owned());
            }
        };
        let mut periods = self
            .periods
            .lock()
            .map_err(|error| format!("period totals lock poisoned: {error}"))?;
        let since = since.unwrap_or(i64::MIN);
        let zone = limit.zone(zone);
        let position = periods
            .iter()
            .position(|period| period.length == limit.period && period.zone == zone);
        if let Some(position) = position {
            let period = &mut periods[position];
            let mut next = period.state;
            if next.since != since || at_ms < next.at_ms {
                next = State::new(since, at_ms);
                if since <= at_ms {
                    next.read(
                        uses,
                        timeline
                            .range((since, 0)..=(at_ms, usize::MAX))
                            .map(|(_, position)| *position),
                    )?;
                }
            } else {
                let after = next.at_ms;
                next.read(
                    uses,
                    timeline
                        .range((Excluded((after, usize::MAX)), Included((at_ms, usize::MAX))))
                        .map(|(_, position)| *position),
                )?;
                next.at_ms = at_ms;
            }
            period.state = next;
            return Ok(next.spend[unit]);
        }
        let mut state = State::new(since, at_ms);
        if since <= at_ms {
            state.read(
                uses,
                timeline
                    .range((since, 0)..=(at_ms, usize::MAX))
                    .map(|(_, position)| *position),
            )?;
        }
        periods.push(Period {
            length: limit.period,
            zone: zone.to_owned(),
            state,
        });
        Ok(state.spend[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_period_sums_what_was_reported_and_counts_what_was_not() -> Result<(), String> {
        let reported = |tokens| Usage {
            tokens,
            ..Usage::default()
        };
        let unreported = Usage {
            unavailable: vec![lys_runner::tracking::Unavailable {
                figure: "tokens".to_owned(),
                reason: "the response reported no token figures".to_owned(),
            }],
            ..Usage::default()
        };
        let mut spend = Spend::empty();
        spend.add(Measure::Tokens, &reported(300), 0)?;
        spend.add(Measure::Tokens, &unreported, 1)?;
        spend.add(Measure::Tokens, &reported(200), 2)?;
        spend.add(Measure::Tokens, &unreported, 3)?;
        // The sum is of what was reported; the first gap is named, and both are counted.
        assert_eq!(
            (spend.total, spend.gap, spend.gaps),
            (Some(500), Some(1), 2)
        );
        // A gap in tokens is not a gap in running time.
        let mut running = Spend::empty();
        running.add(Measure::RunningMs, &unreported, 0)?;
        assert_eq!((running.gap, running.gaps), (None, 0));
        Ok(())
    }
}
