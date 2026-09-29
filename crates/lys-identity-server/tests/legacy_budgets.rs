#![cfg(test)]
//! Legacy self-edits retain the complete earlier restriction until an
//! administrator confirms, including edits of period alone and restart.

use lys_identity_server::budgets_state::{
    Act, Budget, Held, Holder, HolderKind, Leaf, Length, Measure, Period, Standing,
};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn budget() -> Budget {
    Budget {
        holder: Holder {
            kind: HolderKind::Person,
            id: "person-bea".to_owned(),
        },
        measure: Measure::Tokens,
        limit: 100,
        period: Some(Period {
            length: Length::Week,
            zone: "UTC".to_owned(),
        }),
        act: Act::Stop,
        version: 1,
        by: "person-ada".to_owned(),
        at: 1,
    }
}

fn standing() -> Standing {
    Standing {
        person: Some("person-bea".to_owned()),
        ..Standing::default()
    }
}

#[test]
fn a_self_raise_keeps_the_earlier_budget_through_snapshot_and_confirmation() -> TestResult {
    let original = budget();
    let raised = Budget {
        limit: 900,
        version: 2,
        by: "person-bea".to_owned(),
        ..original.clone()
    };
    let mut held = Held::default();
    held.hold(Leaf::Set(original.clone()))?;
    held.hold(Leaf::Set(raised.clone()))?;
    assert_eq!(held.budgets, vec![raised.clone()]);
    assert_eq!(held.unconfirmed.len(), 1);
    assert_eq!(held.applying(&standing(), Measure::Tokens), Some(&original));
    let mut restored = Held::decode(&held.encode()?)?;
    assert_eq!(restored, held);
    let confirmed = Budget {
        version: 3,
        by: "person-ada".to_owned(),
        ..raised
    };
    restored.hold(Leaf::Confirmed(confirmed.clone()))?;
    assert!(restored.unconfirmed.is_empty());
    assert_eq!(
        restored.applying(&standing(), Measure::Tokens),
        Some(&confirmed)
    );
    Ok(())
}

#[test]
fn a_period_only_self_edit_keeps_the_entire_earlier_budget() -> TestResult {
    let original = budget();
    let changed = Budget {
        period: Some(Period {
            length: Length::Day,
            zone: "UTC".to_owned(),
        }),
        version: 2,
        by: "person-bea".to_owned(),
        ..original.clone()
    };
    let mut held = Held::default();
    held.hold(Leaf::Set(original.clone()))?;
    held.hold(Leaf::Set(changed.clone()))?;
    assert_eq!(held.unconfirmed[0].requested, changed);
    assert_eq!(held.unconfirmed[0].effective, original);
    assert_eq!(held.applying(&standing(), Measure::Tokens), Some(&original));
    Ok(())
}

#[test]
fn a_self_set_with_no_predecessor_remains_effective_but_unconfirmed() -> TestResult {
    let own = Budget {
        by: "person-bea".to_owned(),
        ..budget()
    };
    let mut held = Held::default();
    held.hold(Leaf::Set(own.clone()))?;
    assert_eq!(held.unconfirmed.len(), 1);
    assert_eq!(held.applying(&standing(), Measure::Tokens), Some(&own));
    Ok(())
}

#[test]
fn a_later_self_edit_cannot_launder_an_earlier_self_raise() -> TestResult {
    let original = budget();
    let mut held = Held::default();
    held.hold(Leaf::Set(original.clone()))?;
    for (version, limit) in [(2, 900), (3, 800)] {
        held.hold(Leaf::Set(Budget {
            version,
            limit,
            by: "person-bea".to_owned(),
            ..original.clone()
        }))?;
    }
    assert_eq!(held.applying(&standing(), Measure::Tokens), Some(&original));
    assert_eq!(held.unconfirmed[0].requested.limit, 800);
    Ok(())
}
