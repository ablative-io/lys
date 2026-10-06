//! A current boundary uses the existing policy and point measurement.

use super::{Act, Held, HolderKind, Measure, Standing};

impl Held {
    /// Judge a current owned boundary using the existing folded measurement.
    ///
    /// # Errors
    /// Names invalid limits or an inconsistent folded policy.
    pub fn control_context(
        &self,
        standing: &Standing,
        status: &lys_runner::harness_control::ControlStatus,
        session: &str,
        at_ms: i64,
    ) -> Result<lys_runner::harness_control::ContextDecision, String> {
        use lys_runner::harness_control::ContextDecision;
        let held = |reason: &str| ContextDecision::Held {
            crossing: status.crossing.clone(),
            reason: reason.to_owned(),
        };
        let mut threshold = None;
        for limits in &self.limit_sets {
            let covered = match limits.holder.kind {
                HolderKind::Agent => limits.holder.id == standing.agent,
                HolderKind::Person => standing.person.as_ref() == Some(&limits.holder.id),
                HolderKind::Team => standing.teams.contains(&limits.holder.id),
            };
            if !covered {
                continue;
            }
            for limit in self.effective_limits(limits).into_iter().filter(|limit| {
                limit.unit == Measure::ContextPercent
                    && matches!(limit.act, Act::Compact | Act::Stop)
            }) {
                let amount = limit.scaled().map_err(|error| error.words)?;
                if threshold.is_none_or(|(earlier, _)| {
                    amount < earlier || (amount == earlier && limit.act == Act::Stop)
                }) {
                    threshold = Some((amount, limit.act));
                }
            }
        }
        let Some((threshold, action)) = threshold else {
            return Ok(ContextDecision::Released);
        };
        let Some((observed, Some(figure))) =
            self.context_availability.reading(&standing.agent, session)
        else {
            return Ok(held(
                "context_measurement_unavailable: the next turn has no current valid measurement",
            ));
        };
        if observed > at_ms {
            return Ok(held(
                "context_measurement_future: the report is ahead of this observation",
            ));
        }
        let scaled = figure
            .checked_mul(1_000_000)
            .ok_or("context percentage overflows micros")?;
        let receipt = status
            .crossing
            .as_ref()
            .and_then(|crossing| self.crossings.acted.get(crossing));
        if scaled < threshold {
            if status.crossing.is_some()
                && receipt.is_none_or(|acted| {
                    acted.stands != crate::budgets_crossing::Stands::Confirmed
                        || observed <= acted.at_ms
                })
            {
                return Ok(held(
                    "context_post_measurement_missing: a fresh report is required after the control act",
                ));
            }
            return Ok(ContextDecision::Released);
        }
        if action == Act::Stop {
            return Ok(held(
                "context_stop_pending: input waits for the existing process stop owner",
            ));
        }
        if let (Some(crossing), Some(receipt)) = (&status.crossing, receipt)
            && receipt.stands == crate::budgets_crossing::Stands::Accepted
        {
            return Ok(ContextDecision::Compact {
                crossing: crossing.clone(),
            });
        }
        Ok(held(
            if receipt
                .is_some_and(|acted| acted.stands == crate::budgets_crossing::Stands::Uncertain)
            {
                "context_delivery_uncertain: compaction is not repeated"
            } else {
                "context_above_limit: the next turn remains held without recursive compaction"
            },
        ))
    }
}
