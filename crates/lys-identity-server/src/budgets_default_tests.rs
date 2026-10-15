//! The organisation default: in force until saved, set aside by an agent's
//! or its person's own context limit, switched off by an empty saved row.

use std::collections::BTreeSet;

use super::{COMPACT, WARNINGS, built_in, holder};
use crate::budgets_limits::{Limit, Limits};
use crate::budgets_state::{Act, Held, Holder, HolderKind, Measure, Standing};

fn standing(agent: &str, person: &str) -> Standing {
    Standing {
        agent: agent.to_owned(),
        teams: BTreeSet::new(),
        person: Some(person.to_owned()),
    }
}

fn context(amount: u64, act: Act) -> Limit {
    Limit {
        unit: Measure::ContextPercent,
        amount: amount.into(),
        period: None,
        act,
        zone: None,
    }
}

fn kept(holder: Holder, limits: Vec<Limit>) -> Limits {
    Limits {
        holder,
        limits,
        warn_at: None,
        version: 1,
        by: "person-a".to_owned(),
        at: 1,
    }
}

fn levels(held: &Held, standing: &Standing) -> Vec<(u64, Act)> {
    held.in_force()
        .filter(|limits| crate::budgets_members::covers(&limits.holder, standing))
        .flat_map(|limits| held.applying_limits(&limits, standing))
        .filter(|(_, limit)| limit.unit == Measure::ContextPercent)
        .map(|(_, limit)| (limit.amount.as_u64().unwrap_or(u64::MAX), limit.act))
        .collect()
}

#[test]
fn an_agent_with_no_limit_inherits_warnings_at_20_and_38_and_compaction_at_40() {
    let held = Held::default();
    assert_eq!(WARNINGS, [20, 38]);
    assert_eq!(COMPACT, 40);
    assert_eq!(
        levels(&held, &standing("agent-a", "person-a")),
        vec![(20, Act::Notice), (38, Act::Notice), (40, Act::Compact)]
    );
    let default = held.context_default(&standing("agent-a", "person-a"));
    assert!(default.built_in);
    assert!(!default.set_aside);
    assert_eq!(default.version, 0);
    assert_eq!(default.limits, built_in().limits);
    assert!(
        built_in().checked().is_ok(),
        "the built-in default passes the checks a saved row passes"
    );
}

#[test]
fn an_agents_own_context_limit_sets_the_default_aside_for_it_alone() {
    let mut held = Held::default();
    held.limit_sets.push(kept(
        Holder {
            kind: HolderKind::Agent,
            id: "agent-a".to_owned(),
        },
        vec![context(60, Act::Compact)],
    ));
    assert_eq!(
        levels(&held, &standing("agent-a", "person-a")),
        vec![(60, Act::Compact)]
    );
    assert!(
        held.context_default(&standing("agent-a", "person-a"))
            .set_aside
    );
    assert_eq!(
        levels(&held, &standing("agent-b", "person-a")),
        vec![(20, Act::Notice), (38, Act::Notice), (40, Act::Compact)],
        "another agent still inherits the default"
    );
}

#[test]
fn a_persons_context_limit_sets_the_default_aside_for_each_of_its_agents() {
    let mut held = Held::default();
    held.limit_sets.push(kept(
        Holder {
            kind: HolderKind::Person,
            id: "person-a".to_owned(),
        },
        vec![context(50, Act::Compact)],
    ));
    assert_eq!(
        levels(&held, &standing("agent-a", "person-a")),
        vec![(50, Act::Compact)]
    );
}

#[test]
fn an_agents_spend_limit_leaves_the_default_in_force() {
    let mut held = Held::default();
    held.limit_sets.push(kept(
        Holder {
            kind: HolderKind::Agent,
            id: "agent-a".to_owned(),
        },
        vec![Limit {
            unit: Measure::Tokens,
            amount: 1_000.into(),
            period: Some(crate::budgets_state::Length::Day),
            act: Act::Stop,
            zone: None,
        }],
    ));
    assert_eq!(
        levels(&held, &standing("agent-a", "person-a")),
        vec![(20, Act::Notice), (38, Act::Notice), (40, Act::Compact)]
    );
}

#[test]
fn a_saved_organisation_row_replaces_the_default_and_an_empty_one_switches_it_off() {
    let mut held = Held::default();
    held.limit_sets.push(kept(
        holder(),
        vec![context(30, Act::Notice), context(50, Act::Compact)],
    ));
    assert_eq!(
        levels(&held, &standing("agent-a", "person-a")),
        vec![(30, Act::Notice), (50, Act::Compact)]
    );
    assert_eq!(
        held.limit_set_in_force(&holder())
            .map(|limits| limits.version),
        Some(1)
    );
    held.limit_sets[0].limits.clear();
    assert_eq!(levels(&held, &standing("agent-a", "person-a")), Vec::new());
    let default = held.context_default(&standing("agent-a", "person-a"));
    assert!(!default.built_in);
    assert!(default.limits.is_empty());
}

#[test]
fn the_built_in_default_is_found_by_its_holder_at_version_0_and_by_no_other() {
    let held = Held::default();
    assert_eq!(
        held.limit_set_in_force(&holder())
            .map(|limits| limits.version),
        Some(0)
    );
    let other = Holder {
        kind: HolderKind::Organisation,
        id: "elsewhere".to_owned(),
    };
    assert!(held.limit_set_in_force(&other).is_none());
    let refused = kept(other, vec![context(40, Act::Compact)]).checked();
    assert_eq!(
        refused.map(|_| ()).map_err(|error| error.refusal),
        Err("BudgetLimitsRefused")
    );
}
