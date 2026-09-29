//! Preserve the effective earlier budget when an old self-edit could loosen it.
//! Requested values remain visible beside the effective full budget until an
//! administrator writes a confirmed version. Snapshot v2 retains this evidence.

use serde::{Deserialize, Serialize};

use crate::budgets_state::{Budget, Held, HolderKind};

/// A legacy personal budget awaiting an administrator's confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Unconfirmed {
    /// What the old self-edit requested; never rewritten or concealed.
    pub requested: Budget,
    /// What remains enforced, including its period and action.
    pub effective: Budget,
    /// Why confirmation is required.
    pub reason: String,
}

impl Held {
    /// Fold attribution alongside each legacy set, before replacing the latest value.
    pub(crate) fn legacy_set(&mut self, budget: &Budget) {
        let earlier = self
            .unconfirmed
            .iter()
            .find(|entry| {
                entry.requested.holder == budget.holder && entry.requested.measure == budget.measure
            })
            .map(|entry| entry.effective.clone())
            .or_else(|| self.budget(&budget.holder, budget.measure).cloned());
        self.confirmed(budget);
        if budget.holder.kind != HolderKind::Person || budget.by != budget.holder.id {
            return;
        }
        let effective = match earlier {
            Some(earlier)
                if budget.limit > earlier.limit
                    || budget.period != earlier.period
                    || budget.act != earlier.act =>
            {
                earlier
            }
            _ => budget.clone(),
        };
        self.unconfirmed.push(Unconfirmed {
            requested: budget.clone(), effective,
            reason: "a legacy self-set personal budget requires administrator confirmation; an earlier stricter budget remains effective".to_owned(),
        });
    }

    /// Remove the pending judgement only when an authorized version replaces it.
    pub(crate) fn confirmed(&mut self, budget: &Budget) {
        self.unconfirmed.retain(|entry| {
            entry.requested.holder != budget.holder || entry.requested.measure != budget.measure
        });
    }

    /// The full budget actually enforced for a latest recorded value.
    pub(crate) fn effective<'a>(&'a self, budget: &'a Budget) -> &'a Budget {
        self.unconfirmed
            .iter()
            .find(|entry| {
                entry.requested.holder == budget.holder && entry.requested.measure == budget.measure
            })
            .map_or(budget, |entry| &entry.effective)
    }
}
