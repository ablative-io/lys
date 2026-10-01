//! Giving reads only the giver's current limits under the append's store lock.

use crate::budgets_limits::Limit;
use crate::budgets_state::{Held, Holder};
use crate::error::ServerError;
use crate::error_budget::BudgetError;
use crate::error_holding::HoldingError;

/// Refuse a gift unless each unit, period, zone and amount is held by its giver.
/// Multiple matching restrictions use their tightest amount, regardless of act.
/// An empty collection removes every restriction and cannot be given by an agent.
pub fn holds(
    held: &Held,
    giver: &Holder,
    giving: &[Limit],
    organisation_zone: &str,
) -> Result<(), ServerError> {
    let not_held = |reason: String| HoldingError::NotHeld {
        giver: giver.id.clone(),
        reason,
    };
    let holding = held
        .limit_set(giver)
        .ok_or_else(|| not_held("no current budget belongs to the giver".to_owned()))?;
    if giving.is_empty() {
        return Err(not_held("an unrestricted budget is not a held amount".to_owned()).into());
    }
    for requested in giving {
        let amount = scaled(requested)?;
        let mut ceiling = None;
        for own in holding.limits.iter().filter(|own| {
            own.unit == requested.unit
                && own.period == requested.period
                && own.zone(organisation_zone) == requested.zone(organisation_zone)
        }) {
            let amount = scaled(own)?;
            ceiling = Some(ceiling.map_or(amount, |earlier: u64| earlier.min(amount)));
        }
        if ceiling.is_none_or(|ceiling| amount > ceiling) {
            return Err(not_held(format!(
                "{} {:?} in {} at {} is not covered by the giver's current limits",
                requested.unit.name(),
                requested.period,
                requested.zone(organisation_zone),
                requested.amount,
            ))
            .into());
        }
    }
    Ok(())
}

fn scaled(limit: &Limit) -> Result<u64, ServerError> {
    limit.scaled().map_err(|refused| {
        BudgetError::BudgetRefused {
            refusal: refused.refusal,
            words: refused.words,
        }
        .into()
    })
}
