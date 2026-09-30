//! Every holder's limits are evaluated independently, preserving each action and zone.

use serde::{Deserialize, Serialize};
use serde_json::Number;

use crate::budgets_state::{Act, Budget, Held, Holder, HolderKind, Length, Measure, Refused};

/// A limit's period and action belong to that limit, including migrated values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(as = BudgetLimit)]
pub struct Limit {
    /// The reported unit.
    pub unit: Measure,
    /// The amount in the named unit.
    #[schema(value_type = f64)]
    pub amount: Number,
    /// No period for a context level.
    pub period: Option<Length>,
    /// The act taken when this limit is reached.
    pub act: Act,
    /// An explicit migrated zone; absent limits use the organisation zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
}

/// One atomically versioned collection of limits for a holder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// The holder.
    pub holder: Holder,
    /// Every threshold and its action, in the order set.
    pub limits: Vec<Limit>,
    /// Warn once per limit's period at this percentage of its amount.
    #[schema(value_type = Option<f64>)]
    pub warn_at: Option<Number>,
    /// The whole holder's optimistic version.
    pub version: u64,
    /// Who set it.
    pub by: String,
    /// When it was set, in seconds since the Unix epoch.
    pub at: u64,
}

pub(crate) fn refused(name: &'static str, words: impl Into<String>) -> Refused {
    Refused {
        refusal: name,
        words: words.into(),
    }
}

impl Limit {
    /// A checked integer for comparisons: monetary and percentage units use micros.
    pub fn scaled(&self) -> Result<u64, Refused> {
        let amount = match self.unit {
            Measure::Tokens | Measure::RunningMs => {
                lys_runner::tracking_budget::scaled_exact(&self.amount, 0)
            }
            Measure::Dollars | Measure::PlanPercent | Measure::ContextPercent => {
                lys_runner::tracking_budget::scaled_exact(&self.amount, 6)
            }
        };
        amount.ok_or_else(|| {
            refused(
                "BudgetAmountRefused",
                format!(
                    "{} is not a nonnegative representable amount of {}",
                    self.amount,
                    self.unit.name()
                ),
            )
        })
    }

    /// Validate units, levels and zones before any version is stored.
    pub fn checked(&self, holder: &Holder) -> Result<(), Refused> {
        self.scaled()?;
        if matches!(self.unit, Measure::ContextPercent | Measure::PlanPercent)
            && self
                .amount
                .as_f64()
                .is_none_or(|amount| !(0.0..=100.0).contains(&amount))
        {
            return Err(refused(
                "BudgetAmountRefused",
                "a percentage is from 0 to 100",
            ));
        }
        if holder.kind == HolderKind::Team && self.unit == Measure::ContextPercent {
            return Err(refused(
                "TeamUnitRefused",
                "context_percent is a session level, not team spend; set it on each agent",
            ));
        }
        match (self.unit, self.period) {
            (Measure::ContextPercent, None)
            | (Measure::PlanPercent, Some(Length::FiveHour | Length::Week))
            | (Measure::Tokens | Measure::RunningMs | Measure::Dollars, Some(_)) => {}
            (Measure::ContextPercent, Some(_)) => {
                return Err(refused(
                    "BudgetPeriodRefused",
                    "context_percent is a level and has no period",
                ));
            }
            (Measure::PlanPercent, _) => {
                return Err(refused(
                    "BudgetPeriodRefused",
                    "plan_percent needs a reported five_hour or week window",
                ));
            }
            (_, None) => {
                return Err(refused(
                    "BudgetPeriodRefused",
                    "spend names a five_hour, day, week or month period",
                ));
            }
        }
        if let Some(zone) = &self.zone {
            let known = jiff::tz::TimeZone::get(zone).map_err(|error| {
                refused(
                    "BudgetZoneRefused",
                    format!("{zone} is not an IANA zone: {error}"),
                )
            })?;
            if known.iana_name().is_none() {
                return Err(refused(
                    "BudgetZoneRefused",
                    format!("{zone} has no named IANA zone"),
                ));
            }
        }
        Ok(())
    }

    /// Convert an old restriction without changing its action, amount or zone.
    pub fn from_budget(budget: &Budget) -> Self {
        Self {
            unit: budget.measure,
            amount: budget.limit.into(),
            period: budget.period.as_ref().map(|period| period.length),
            act: budget.act,
            zone: budget.period.as_ref().map(|period| period.zone.clone()),
        }
    }

    /// The explicit migrated zone wins; otherwise the organisation setting applies.
    pub fn zone<'a>(&'a self, organisation: &'a str) -> &'a str {
        self.zone.as_deref().unwrap_or(organisation)
    }
}

impl Limits {
    /// Refuse malformed collections as a whole, before writing any leaf.
    pub fn checked(self) -> Result<Self, Refused> {
        if self.holder.id.is_empty() || self.limits.len() > 64 {
            return Err(refused(
                "BudgetLimitsRefused",
                "a holder is named and has at most 64 limits",
            ));
        }
        for (index, limit) in self.limits.iter().enumerate() {
            limit.checked(&self.holder)?;
            if self.limits[..index].contains(limit) {
                return Err(refused(
                    "BudgetLimitsRefused",
                    "an identical limit occurs twice",
                ));
            }
        }
        if self.warn_at.as_ref().is_some_and(|percent| {
            lys_runner::tracking_budget::scaled_exact(percent, 6).is_none()
                || percent
                    .as_f64()
                    .is_none_or(|value| !(0.0..=100.0).contains(&value))
        }) {
            return Err(refused(
                "BudgetWarningRefused",
                "warn_at is null or a percentage from 0 to 100",
            ));
        }
        Ok(self)
    }
}

impl Held {
    /// The current, whole-holder collection, including migrated restrictions.
    pub fn limit_set(&self, holder: &Holder) -> Option<&Limits> {
        self.limit_sets
            .iter()
            .find(|limits| limits.holder == *holder)
    }

    /// Fold an old version into the current collection without losing provenance.
    pub(crate) fn migrate_budget(&mut self, budget: &Budget) -> Result<(), String> {
        let previous = self
            .budget(&budget.holder, budget.measure)
            .map_or(0, |held| held.version);
        let increase = budget
            .version
            .checked_sub(previous)
            .ok_or("old budget version moved backwards")?;
        self.merge_budget(budget, increase)
    }

    /// A snapshot's old versions contribute their whole version to the holder.
    pub(crate) fn merge_budget(&mut self, budget: &Budget, increase: u64) -> Result<(), String> {
        let limit = Limit::from_budget(budget);
        if let Some(held) = self
            .limit_sets
            .iter_mut()
            .find(|held| held.holder == budget.holder)
        {
            held.version = held
                .version
                .checked_add(increase)
                .ok_or("migrated holder version exhausted")?;
            if let Some(earlier) = held.limits.iter_mut().find(|held| held.unit == limit.unit) {
                *earlier = limit;
            } else {
                held.limits.push(limit);
            }
            if budget.at >= held.at {
                held.by.clone_from(&budget.by);
                held.at = budget.at;
            }
        } else {
            self.limit_sets.push(Limits {
                holder: budget.holder.clone(),
                limits: vec![limit],
                warn_at: None,
                version: increase,
                by: budget.by.clone(),
                at: budget.at,
            });
        }
        Ok(())
    }

    /// Preserve an earlier personal restriction until an administrator confirms it.
    pub fn effective_limits(&self, limits: &Limits) -> Vec<Limit> {
        limits
            .limits
            .iter()
            .map(|limit| {
                self.unconfirmed
                    .iter()
                    .find(|pending| {
                        pending.requested.holder == limits.holder
                            && pending.requested.measure == limit.unit
                    })
                    .map_or_else(
                        || limit.clone(),
                        |pending| Limit::from_budget(&pending.effective),
                    )
            })
            .collect()
    }
}
