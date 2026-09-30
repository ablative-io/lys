//! What the budgets' log folds to: each budget at its latest version, and
//! the usage each has been charged, so a start reads the sealed state and
//! only the leaves after it.
//!
//! A budget names its holder (an agent, a team or a person), its measure,
//! its limit and its act. Of an agent's own budget, its teams' and its
//! responsible person's, the tightest limit applies, so a team's owner cannot
//! loosen the budget a person or administrator set. A budget over a
//! period names the zone the period is counted in; there is no machine
//! default. Usage is charged once per event: an event seen again adds
//! nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use jiff::civil::Weekday;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan, Zoned};
use serde::{Deserialize, Serialize};

use crate::budgets_crossing::{Acted, Crossing, Crossings};

/// The snapshot domain the budgets' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/budgets-state/v2";

const FORMAT: &str = "lys-budgets-state/v3";

/// Who a budget is held on.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum HolderKind {
    /// One agent.
    Agent,
    /// Each agent of a team.
    Team,
    /// Each agent a person is responsible for.
    Person,
}

/// A budget's holder.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(deny_unknown_fields)]
pub struct Holder {
    /// Its kind.
    pub kind: HolderKind,
    /// Its id.
    pub id: String,
}

/// What a budget measures.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    /// The session's context, in percent of its window; no period.
    ContextPercent,
    /// Tokens used in a period.
    Tokens,
    /// Running time in a period, in milliseconds.
    RunningMs,
    /// Reported dollar spend, never inferred from token prices.
    Dollars,
    /// A reported account window's level.
    PlanPercent,
}

impl Measure {
    /// Its wire name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::ContextPercent => "context_percent",
            Self::Tokens => "tokens",
            Self::RunningMs => "running_ms",
            Self::Dollars => "dollars",
            Self::PlanPercent => "plan_percent",
        }
    }
}

/// What a reached budget does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Act {
    /// Ask the harness to compact at its next turn boundary.
    Compact,
    /// Type a notice into the session at its next turn boundary.
    Notice,
    /// End the session.
    Stop,
    /// Tell the responsible person.
    Tell,
}

/// How long a period is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Length {
    /// From midnight to midnight.
    Day,
    /// From Monday's midnight to the next.
    Week,
    /// An epoch-aligned five-hour interval.
    FiveHour,
    /// From the first midnight of a month to the next.
    Month,
}

impl Length {
    /// Its wire name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::FiveHour => "five_hour",
            Self::Month => "month",
        }
    }
}

/// The period a budget counts over, in a named zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Period {
    /// Its length.
    pub length: Length,
    /// The IANA zone it is counted in.
    pub zone: String,
}

/// One version of a budget.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    /// Its holder.
    pub holder: Holder,
    /// What it measures.
    pub measure: Measure,
    /// Its limit, in the measure's unit.
    pub limit: u64,
    /// The period, for a measure counted over one.
    pub period: Option<Period>,
    /// What it does when reached.
    pub act: Act,
    /// Its version, from 1.
    pub version: u64,
    /// Who set it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// A measured use one agent made, as the runner's feed reported it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Usage {
    /// The feed event's stable id: an event seen again charges nothing.
    pub event: String,
    /// The agent.
    pub agent: String,
    /// When, in milliseconds since the Unix epoch.
    pub at_ms: i64,
    /// Tokens used.
    pub tokens: u64,
    /// Running time, in milliseconds.
    pub running_ms: u64,
    /// Reported dollar spend; absent is not zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dollars_micros: Option<u64>,
    /// The account that the native record attributes the figures to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// Account windows observed by this snapshot; absent spend records do not replace them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_windows: Option<Vec<lys_runner::tracking_budget::PlanWindow>>,
    /// The native cumulative running-time baseline, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reported_running_ms: Option<u64>,
    /// A native snapshot updates availability and account levels, even without spend.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub native_snapshot: bool,
    /// Each unavailable figure and its reported reason.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unavailable: Vec<lys_runner::tracking::Unavailable>,
    /// The session it was measured in, when one is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    /// The session's context, in percent of its window, when measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_percent: Option<u64>,
    /// The budgets this use crossed, kept with it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub crossed: Vec<Crossing>,
}

/// One leaf of the budgets' log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Leaf {
    /// A budget set or changed.
    Set(Budget),
    /// An entire holder's limits replaced under one optimistic version.
    LimitsSet(crate::budgets_limits::Limits),
    /// A version admitted under the administrator-only personal-budget rule.
    Confirmed(Budget),
    /// A use charged.
    Used(Usage),
    /// What came of a crossing's act.
    Acted(Acted),
    /// A tool call a runner's judge denied.
    Refused(Box<lys_runner::refusals::RefusalRecord>),
    /// How far a runner's feed has been read.
    FeedRead(crate::refusals_store::FeedRead),
}

/// Why a budget was refused, by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The refusal's name.
    pub refusal: &'static str,
    /// Why, in words.
    pub words: String,
}

fn refused(refusal: &'static str, words: impl Into<String>) -> Refused {
    Refused {
        refusal,
        words: words.into(),
    }
}

/// Where an agent stands: its own id, the teams it is in, its responsible
/// person.
#[derive(Debug, Clone, Default)]
pub struct Standing {
    /// The agent.
    pub agent: String,
    /// The teams it is in.
    pub teams: BTreeSet<String>,
    /// Its responsible person.
    pub person: Option<String>,
}

/// The budgets as their log folds them.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The old versions retained as migration and confirmation evidence.
    pub budgets: Vec<Budget>,
    /// The current limits, migrated before any read or enforcement.
    #[serde(default)]
    pub limit_sets: Vec<crate::budgets_limits::Limits>,
    /// Legacy self-set personal budgets and what remains effective.
    #[serde(default)]
    pub unconfirmed: Vec<crate::budgets_legacy::Unconfirmed>,
    /// The events already charged.
    pub charged: BTreeSet<String>,
    /// The uses charged, in the order kept.
    pub uses: Vec<Usage>,
    /// The crossings and what came of their acts.
    #[serde(default)]
    pub crossings: Crossings,
    /// The refusals read from the runners' feeds.
    #[serde(default)]
    pub refusals: crate::refusals_store::Refusals,
    /// Derived record positions, rebuilt on open and excluded from signed state.
    #[serde(skip)]
    pub index: crate::budgets_index::Index,
}

impl Clone for Held {
    fn clone(&self) -> Self {
        #[cfg(test)]
        crate::budgets_work::visit(crate::budgets_work::Work::StateCopy);
        Self {
            budgets: self.budgets.clone(),
            limit_sets: self.limit_sets.clone(),
            unconfirmed: self.unconfirmed.clone(),
            charged: self.charged.clone(),
            uses: self.uses.clone(),
            crossings: self.crossings.clone(),
            refusals: self.refusals.clone(),
            index: self.index.clone(),
        }
    }
}

impl PartialEq for Held {
    fn eq(&self, other: &Self) -> bool {
        self.budgets == other.budgets
            && self.limit_sets == other.limit_sets
            && self.unconfirmed == other.unconfirmed
            && self.charged == other.charged
            && self.uses == other.uses
            && self.crossings == other.crossings
            && self.refusals == other.refusals
    }
}

impl Eq for Held {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

#[derive(Serialize)]
struct Sealing<'a> {
    format: &'static str,
    held: &'a Held,
}

impl Budget {
    /// The budget, refused by name when its shape is wrong: a measure over
    /// a period without one, a period without its zone, a zone that is not
    /// known, or a context budget over 100 percent.
    pub fn checked(self) -> Result<Self, Refused> {
        if self.holder.id.is_empty() {
            return Err(refused("budget_invalid", "a budget names its holder"));
        }
        match (self.measure, &self.period) {
            (Measure::Dollars | Measure::PlanPercent, _) => {
                return Err(refused(
                    "budget_invalid",
                    "reported dollars and plan levels are set through limit collections",
                ));
            }
            (Measure::ContextPercent, Some(_)) => {
                return Err(refused(
                    "budget_invalid",
                    "a context budget holds at every moment and has no period",
                ));
            }
            (Measure::ContextPercent, None) if self.limit > 100 => {
                return Err(refused(
                    "budget_invalid",
                    "a context budget is a percent, from 0 to 100",
                ));
            }
            (Measure::Tokens | Measure::RunningMs, None) => {
                return Err(refused(
                    "period_missing",
                    "a budget of tokens or running time names its period",
                ));
            }
            (_, Some(period)) if period.zone.trim().is_empty() => {
                return Err(refused(
                    "zone_missing",
                    "a period names the zone it is counted in; there is no default zone",
                ));
            }
            (_, Some(period)) => {
                TimeZone::get(&period.zone).map_err(|error| {
                    refused(
                        "zone_unknown",
                        format!("{} is not a known zone: {error}", period.zone),
                    )
                })?;
            }
            (Measure::ContextPercent, None) => {}
        }
        Ok(self)
    }
}

impl Period {
    /// The start of the period `at_ms` falls in, in milliseconds since the
    /// Unix epoch, counted in the period's zone.
    pub fn start_of(&self, at_ms: i64) -> Result<i64, String> {
        let zone = TimeZone::get(&self.zone).map_err(|error| error.to_string())?;
        let instant = Timestamp::from_millisecond(at_ms).map_err(|error| error.to_string())?;
        let local: Zoned = instant.to_zoned(zone);
        let day = local.date();
        let first = match self.length {
            Length::FiveHour => {
                return at_ms
                    .div_euclid(18_000_000)
                    .checked_mul(18_000_000)
                    .ok_or_else(|| "five-hour start overflows milliseconds".to_owned());
            }
            Length::Month => jiff::civil::Date::new(day.year(), day.month(), 1)
                .map_err(|error| error.to_string())?,
            Length::Day => day,
            Length::Week => {
                let back = i64::from(day.weekday().since(Weekday::Monday));
                day.checked_sub(back.days())
                    .map_err(|error| error.to_string())?
            }
        };
        let start = first
            .to_zoned(local.time_zone().clone())
            .map_err(|error| error.to_string())?;
        Ok(start.timestamp().as_millisecond())
    }
}

impl Held {
    /// The budget held on `holder` for `measure`.
    pub fn budget(&self, holder: &Holder, measure: Measure) -> Option<&Budget> {
        self.budgets
            .iter()
            .find(|budget| budget.holder == *holder && budget.measure == measure)
    }

    /// Fold one leaf. A budget whose version does not follow the one held
    /// is refused, since every kept version was checked before it was kept.
    pub fn hold(&mut self, leaf: Leaf) -> Result<(), String> {
        let confirmed = matches!(&leaf, Leaf::Confirmed(_));
        match leaf {
            Leaf::LimitsSet(limits) => {
                let expected = self
                    .limit_set(&limits.holder)
                    .map_or(Some(1), |held| held.version.checked_add(1))
                    .ok_or("budget version exhausted")?;
                if limits.version != expected {
                    return Err("limit collection version does not follow the one held".to_owned());
                }
                limits.clone().checked().map_err(|error| error.words)?;
                self.unconfirmed
                    .retain(|pending| pending.requested.holder != limits.holder);
                if let Some(held) = self
                    .limit_sets
                    .iter_mut()
                    .find(|held| held.holder == limits.holder)
                {
                    *held = limits;
                } else {
                    self.limit_sets.push(limits);
                }
            }
            Leaf::Set(budget) | Leaf::Confirmed(budget) => {
                let expected = self
                    .budget(&budget.holder, budget.measure)
                    .map_or(Some(1), |held| held.version.checked_add(1))
                    .ok_or("budget version exhausted")?;
                if budget.version != expected {
                    return Err(format!(
                        "budget version {} does not follow the one held",
                        budget.version
                    ));
                }
                if confirmed {
                    self.confirmed(&budget);
                } else {
                    self.legacy_set(&budget);
                }
                self.migrate_budget(&budget)?;
                let held = self
                    .budgets
                    .iter_mut()
                    .find(|held| held.holder == budget.holder && held.measure == budget.measure);
                match held {
                    None if budget.version == 1 => self.budgets.push(budget),
                    Some(held) => *held = budget,
                    _ => {
                        return Err(format!(
                            "budget version {} does not follow the one held",
                            budget.version
                        ));
                    }
                }
            }
            Leaf::Used(mut usage) => {
                if self.charged.insert(usage.event.clone()) {
                    for crossing in std::mem::take(&mut usage.crossed) {
                        crossing.checked()?;
                        self.crossings.hold(crossing);
                    }
                    if let (Some(session), Some(figure)) = (&usage.session, usage.context_percent) {
                        self.crossings.context.insert(session.clone(), figure);
                    }
                    self.index.insert(&usage, self.uses.len())?;
                    self.uses.push(usage);
                }
            }
            Leaf::Acted(acted) => {
                self.crossings.acted(acted);
            }
            Leaf::Refused(record) => self.refusals.hold(*record),
            Leaf::FeedRead(read) => self.refusals.read_to(read),
        }
        Ok(())
    }

    /// The budget that applies to `standing` for `measure`: the strictest of
    /// its agent's, its teams' and its person's, so no holder's budget can
    /// loosen a tighter one set on another.
    pub fn applying(&self, standing: &Standing, measure: Measure) -> Option<&Budget> {
        self.budgets
            .iter()
            .map(|budget| self.effective(budget))
            .filter(|budget| {
                budget.measure == measure
                    && match budget.holder.kind {
                        HolderKind::Agent => budget.holder.id == standing.agent,
                        HolderKind::Team => standing.teams.contains(&budget.holder.id),
                        HolderKind::Person => standing.person.as_ref() == Some(&budget.holder.id),
                    }
            })
            .min_by_key(|budget| budget.limit)
    }

    /// What `agents` have used of `budget`'s measure in the period `at_ms`
    /// falls in.
    pub fn spent(
        &self,
        budget: &Budget,
        agents: &BTreeSet<String>,
        at_ms: i64,
    ) -> Result<u64, String> {
        let Some(period) = &budget.period else {
            return Ok(0);
        };
        let start = period.start_of(at_ms)?;
        let mut total = 0_u64;
        for usage in &self.uses {
            if !agents.contains(&usage.agent) || usage.at_ms < start || usage.at_ms > at_ms {
                continue;
            }
            let figure =
                match budget.measure {
                    Measure::Tokens => usage.tokens,
                    Measure::RunningMs => usage.running_ms,
                    Measure::ContextPercent => 0,
                    Measure::Dollars | Measure::PlanPercent => return Err(
                        "old single-measure accounting cannot read reported dollar or plan figures"
                            .to_owned(),
                    ),
                };
            total = total
                .checked_add(figure)
                .ok_or("budget spend overflows its unit")?;
        }
        Ok(total)
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let leaf = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a budget leaf: {error}"))?;
            self.hold(leaf)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealing {
            format: FORMAT,
            held: self,
        })
        .map_err(|error| format!("budgets state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("budgets state: {error}"))?;
        if !matches!(
            sealed.format.as_str(),
            "lys-budgets-state/v1" | "lys-budgets-state/v2" | "lys-budgets-state/v3"
        ) {
            return Err(format!(
                "budgets state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        let mut held = sealed.held;
        held.index = Arc::new(crate::budgets_index::Index::from_uses(&held.uses)?);
        for crossing in &held.crossings.crossed {
            crossing.checked()?;
        }
        for usage in &held.uses {
            held.context_availability.keep(usage)?;
        }
        if held.limit_sets.is_empty() {
            for budget in held.budgets.clone() {
                held.merge_budget(&budget, budget.version)?;
            }
        }
        held.index = crate::budgets_index::Index::from_uses(&held.uses)?;
        Ok(held)
    }
}

/// Every agent `budget` covers, among the `standings` given.
pub fn covered<'a>(
    budget: &Budget,
    standings: impl IntoIterator<Item = &'a Standing>,
) -> BTreeSet<String> {
    standings
        .into_iter()
        .filter(|standing| match budget.holder.kind {
            HolderKind::Agent => standing.agent == budget.holder.id,
            HolderKind::Team => standing.teams.contains(&budget.holder.id),
            HolderKind::Person => standing.person.as_deref() == Some(budget.holder.id.as_str()),
        })
        .map(|standing| standing.agent.clone())
        .collect()
}

/// The budgets by holder, for reads that group them.
pub fn by_holder(budgets: &[Budget]) -> BTreeMap<Holder, Vec<Budget>> {
    let mut out: BTreeMap<Holder, Vec<Budget>> = BTreeMap::new();
    for budget in budgets {
        out.entry(budget.holder.clone())
            .or_default()
            .push(budget.clone());
    }
    out
}
