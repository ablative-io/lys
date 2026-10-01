//! An unknown delivery stays pending under its durable operation identity.

use std::error::Error;

use lys_identity_server::budgets_crossing::{Acted, Crossing, Crossings, Stands};
use lys_identity_server::budgets_state::{Act, Holder, HolderKind, Measure};

#[test]
fn an_uncertain_crossing_is_reasked_under_the_same_id_after_reopen() -> Result<(), Box<dyn Error>> {
    let crossing = Crossing {
        operation: "stable-operation".to_owned(),
        holder: Holder {
            kind: HolderKind::Agent,
            id: "agent".to_owned(),
        },
        measure: Measure::Tokens,
        version: 1,
        limit: 10.into(),
        figure: Some(10.into()),
        unavailable: None,
        limit_index: 0,
        warning: false,
        account: None,
        act: Act::Stop,
        agent: "agent".to_owned(),
        session: Some("session".to_owned()),
        text: None,
        at_ms: 1,
    };
    let mut held = Crossings::default();
    held.hold(crossing.clone());
    held.acted.insert(
        crossing.operation.clone(),
        Acted {
            operation: crossing.operation.clone(),
            stands: Stands::Uncertain,
            words: "the runner answer was lost".to_owned(),
            at_ms: 2,
            ended: None,
        },
    );
    let restored: Crossings = serde_json::from_slice(&serde_json::to_vec(&held)?)?;
    assert_eq!(restored.unsettled(), vec![crossing]);
    assert_eq!(restored.acted["stable-operation"].stands, Stands::Uncertain);
    Ok(())
}
