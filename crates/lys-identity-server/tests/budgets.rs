//! DIRECTORY-051 R2: budgets on agents, teams and people. A team's budget
//! applies to each agent it covers, an agent's own applies before it, a
//! period is counted in the zone it names and no other, a usage event seen
//! again charges nothing, and the folded budgets survive a restart.

use std::collections::BTreeSet;
use std::error::Error;

use lys_identity_server::budgets_state::{
    Act, Budget, Held, Holder, HolderKind, Leaf, Length, Measure, Period, Standing, Usage, covered,
};

fn budget(kind: HolderKind, id: &str, limit: u64, zone: &str) -> Budget {
    Budget {
        holder: Holder {
            kind,
            id: id.to_owned(),
        },
        measure: Measure::Tokens,
        limit,
        period: Some(Period {
            length: Length::Day,
            zone: zone.to_owned(),
        }),
        act: Act::Tell,
        version: 1,
        by: "person-ada".to_owned(),
        at: 1,
    }
}

fn scribe() -> Standing {
    Standing {
        agent: "agent-scribe".to_owned(),
        teams: BTreeSet::from(["team-docs".to_owned()]),
        person: Some("person-ada".to_owned()),
    }
}

fn used(event: &str, at_ms: i64, tokens: u64) -> Leaf {
    Leaf::Used(Usage {
        event: event.to_owned(),
        agent: "agent-scribe".to_owned(),
        at_ms,
        tokens,
        running_ms: 0,
    })
}

#[test]
fn a_team_budget_applies_to_each_agent_it_covers() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Leaf::Set(budget(HolderKind::Team, "team-docs", 900, "UTC")))?;
    let applying = held
        .applying(&scribe(), Measure::Tokens)
        .ok_or("no budget applies to a team member")?;
    assert_eq!(applying.holder.id, "team-docs");
    let team = held.budgets[0].clone();
    assert_eq!(
        covered(&team, [&scribe(), &Standing::default()]),
        BTreeSet::from(["agent-scribe".to_owned()])
    );
    Ok(())
}

#[test]
fn an_agents_own_budget_applies_before_its_teams() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Leaf::Set(budget(
        HolderKind::Person,
        "person-ada",
        100,
        "UTC",
    )))?;
    held.hold(Leaf::Set(budget(HolderKind::Team, "team-docs", 900, "UTC")))?;
    held.hold(Leaf::Set(budget(
        HolderKind::Agent,
        "agent-scribe",
        5000,
        "UTC",
    )))?;
    let applying = held
        .applying(&scribe(), Measure::Tokens)
        .ok_or("no budget applies")?;
    assert_eq!(applying.holder.kind, HolderKind::Agent);
    assert_eq!(applying.limit, 5000);
    Ok(())
}

#[test]
fn a_period_is_counted_in_the_zone_it_names() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    let sydney = budget(HolderKind::Agent, "agent-scribe", 1000, "Australia/Sydney");
    held.hold(Leaf::Set(sydney.clone()))?;
    // 2026-09-28T13:30Z is 23:30 in Sydney on the 28th; 14:30Z is 00:30 on the 29th.
    let before_midnight = 1_790_602_200_000;
    let after_midnight = before_midnight + 3_600_000;
    held.hold(used("e1", before_midnight, 40))?;
    held.hold(used("e2", after_midnight, 7))?;
    let agents = BTreeSet::from(["agent-scribe".to_owned()]);
    assert_eq!(held.spent(&sydney, &agents, after_midnight)?, 7);
    assert_eq!(held.spent(&sydney, &agents, before_midnight)?, 40);
    let utc = budget(HolderKind::Agent, "agent-scribe", 1000, "UTC");
    assert_eq!(held.spent(&utc, &agents, after_midnight)?, 47);
    Ok(())
}

#[test]
fn a_usage_event_seen_again_charges_nothing() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    let own = budget(HolderKind::Agent, "agent-scribe", 1000, "UTC");
    held.hold(Leaf::Set(own.clone()))?;
    held.hold(used("e1", 1_790_602_200_000, 40))?;
    held.hold(used("e1", 1_790_602_200_000, 40))?;
    let agents = BTreeSet::from(["agent-scribe".to_owned()]);
    assert_eq!(held.spent(&own, &agents, 1_790_602_200_000)?, 40);
    Ok(())
}

#[test]
fn a_period_without_its_zone_is_refused_zone_missing() {
    let unzoned = budget(HolderKind::Agent, "agent-scribe", 1000, " ");
    assert_eq!(
        unzoned.checked().err().map(|refused| refused.refusal),
        Some("zone_missing")
    );
    let unknown = budget(HolderKind::Agent, "agent-scribe", 1000, "Mars/Olympus");
    assert_eq!(
        unknown.checked().err().map(|refused| refused.refusal),
        Some("zone_unknown")
    );
}

#[test]
fn the_budgets_and_their_period_survive_a_restart() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    let sydney = budget(HolderKind::Agent, "agent-scribe", 1000, "Australia/Sydney");
    held.hold(Leaf::Set(sydney.clone()))?;
    held.hold(used("e1", 1_790_602_200_000, 40))?;
    let again = Held::decode(&held.encode()?)?;
    assert_eq!(again, held);
    let agents = BTreeSet::from(["agent-scribe".to_owned()]);
    assert_eq!(again.spent(&sydney, &agents, 1_790_602_200_000)?, 40);
    Ok(())
}

#[test]
fn a_version_that_does_not_follow_is_refused() -> Result<(), Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Leaf::Set(budget(
        HolderKind::Agent,
        "agent-scribe",
        1000,
        "UTC",
    )))?;
    let skipped = Budget {
        version: 3,
        ..budget(HolderKind::Agent, "agent-scribe", 10, "UTC")
    };
    assert!(held.hold(Leaf::Set(skipped)).is_err());
    Ok(())
}
