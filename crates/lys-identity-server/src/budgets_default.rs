//! The organisation's limits, which every agent inherits by default.
//!
//! Until an administrator saves limits on the organisation holder, the
//! built-in default is in force: a progress warning at 20 percent of a
//! session's context window, a final warning at 38, and a compaction at 40.
//! Nothing is written to the budgets' log for it, so an install that never
//! changes it keeps no organisation row and an older build still reads the
//! log. A saved row replaces the default whole, at version 1 onwards; a
//! saved row with no limits switches it off.
//!
//! An agent covered by a context limit of its own, or of its responsible
//! person's, does not inherit the organisation's context limits: such a
//! limit sets the default aside rather than adding a second compaction to
//! it. The organisation's other limits reach every agent.

use std::borrow::Cow;

use serde::Serialize;

use crate::budgets_limits::{Limit, Limits};
use crate::budgets_members::covers;
use crate::budgets_state::{Act, Held, Holder, HolderKind, Measure, Standing};

/// The one organisation holder's id.
pub const ORGANISATION: &str = "organisation";

/// The built-in progress and final warnings, in percent of the context window.
pub const WARNINGS: [u64; 2] = [20, 38];

/// The built-in compaction level, in percent of the context window.
pub const COMPACT: u64 = 40;

/// Who keeps the built-in default, in the place a saved row names a person.
pub const BUILT_IN_BY: &str = "lys";

/// The organisation holder.
#[must_use]
pub fn holder() -> Holder {
    Holder {
        kind: HolderKind::Organisation,
        id: ORGANISATION.to_owned(),
    }
}

/// The default in force while no administrator has saved one, at version 0.
#[must_use]
pub fn built_in() -> Limits {
    let context = |amount: u64, act: Act| Limit {
        unit: Measure::ContextPercent,
        amount: amount.into(),
        period: None,
        act,
        zone: None,
    };
    let mut limits: Vec<Limit> = WARNINGS
        .iter()
        .map(|amount| context(*amount, Act::Notice))
        .collect();
    limits.push(context(COMPACT, Act::Compact));
    Limits {
        holder: holder(),
        limits,
        warn_at: None,
        version: 0,
        by: BUILT_IN_BY.to_owned(),
        at: 0,
    }
}

/// The organisation's context limits as one agent's budget view shows them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
pub struct ContextDefault {
    /// The organisation's context limits, in the order kept; empty when an
    /// administrator switched the default off.
    pub limits: Vec<Limit>,
    /// The organisation row's version: 0 while the built-in default is in force.
    pub version: u64,
    /// True while the built-in default is in force and no row is saved.
    pub built_in: bool,
    /// True when a context limit of the agent's own, or its person's, sets the
    /// default aside for this agent.
    pub set_aside: bool,
}

impl Held {
    /// Every collection in force: each one stored, and the built-in default
    /// while no organisation row is stored.
    pub fn in_force(&self) -> impl Iterator<Item = Cow<'_, Limits>> {
        let default = self
            .limit_set(&holder())
            .is_none()
            .then(|| Cow::Owned(built_in()));
        self.limit_sets.iter().map(Cow::Borrowed).chain(default)
    }

    /// The collection in force for `holder`: the stored one, or the built-in
    /// default for the organisation while none is stored.
    #[must_use]
    pub fn limit_set_in_force(&self, holder: &Holder) -> Option<Cow<'_, Limits>> {
        self.limit_set(holder).map(Cow::Borrowed).or_else(|| {
            (holder.kind == HolderKind::Organisation && holder.id == ORGANISATION)
                .then(|| Cow::Owned(built_in()))
        })
    }

    /// Whether a context limit other than the organisation's covers the
    /// agent `standing` describes.
    #[must_use]
    pub fn own_context(&self, standing: &Standing) -> bool {
        self.limit_sets
            .iter()
            .filter(|limits| {
                limits.holder.kind != HolderKind::Organisation && covers(&limits.holder, standing)
            })
            .any(|limits| {
                self.effective_limits(limits)
                    .iter()
                    .any(|limit| limit.unit == Measure::ContextPercent)
            })
    }

    /// The effective limits of `collection` that apply to the agent
    /// `standing` describes, each with its stable position in the
    /// collection: the organisation's context limits are left out for an
    /// agent with a context limit of its own.
    #[must_use]
    pub fn applying_limits(&self, collection: &Limits, standing: &Standing) -> Vec<(usize, Limit)> {
        let set_aside =
            collection.holder.kind == HolderKind::Organisation && self.own_context(standing);
        self.effective_limits(collection)
            .into_iter()
            .enumerate()
            .filter(|(_, limit)| !(set_aside && limit.unit == Measure::ContextPercent))
            .collect()
    }

    /// The organisation's context limits as they reach the agent `standing`
    /// describes.
    #[must_use]
    pub fn context_default(&self, standing: &Standing) -> ContextDefault {
        let organisation = holder();
        let stored = self.limit_set(&organisation);
        let in_force = stored.map_or_else(|| Cow::Owned(built_in()), Cow::Borrowed);
        ContextDefault {
            limits: self
                .effective_limits(&in_force)
                .into_iter()
                .filter(|limit| limit.unit == Measure::ContextPercent)
                .collect(),
            version: in_force.version,
            built_in: stored.is_none(),
            set_aside: self.own_context(standing),
        }
    }
}

/// The context default an agent's budget view shows; none for any other holder.
#[must_use]
pub fn view(held: &Held, holder: &Holder, standings: &[Standing]) -> Option<ContextDefault> {
    if holder.kind != HolderKind::Agent {
        return None;
    }
    let standing = standings
        .iter()
        .find(|standing| standing.agent == holder.id)
        .cloned()
        .unwrap_or_else(|| Standing {
            agent: holder.id.clone(),
            ..Standing::default()
        });
    Some(held.context_default(&standing))
}

#[cfg(test)]
#[path = "budgets_default_tests.rs"]
mod tests;
