//! The hold of refused asks: the first identical refused ask is kept, a
//! repeat is answered under the same leaf, a made ask is let go, and the
//! oldest is let go once the hold is full.

use serde_json::json;

use super::{Ask, REFUSED_ASKS_HELD, RefusedAsks};

/// How many refused asks `hold` holds.
fn held_count(hold: &RefusedAsks) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(hold
        .held
        .lock()
        .map_err(|error| format!("the hold is poisoned: {error}"))?
        .len())
}

fn ask(agent: lys_identity::AgentId, body_sha256: &str) -> Ask {
    Ask {
        agent,
        method: "POST".to_owned(),
        path: "/identities/x/profile".to_owned(),
        body_sha256: body_sha256.to_owned(),
    }
}

#[test]
fn a_refused_ask_is_held_once_and_let_go_when_made() -> Result<(), Box<dyn std::error::Error>> {
    let hold = RefusedAsks::new();
    let agent = lys_identity::AgentId::generate()?;
    let same = ask(agent, "aa");
    assert_eq!(hold.held(&same)?, None);
    hold.refused(same.clone(), json!({"operation": "first"}))?;
    hold.refused(same.clone(), json!({"operation": "second"}))?;
    assert_eq!(held_count(&hold)?, 1);
    assert_eq!(hold.held(&same)?, Some(json!({"operation": "first"})));
    let other_body = ask(agent, "bb");
    assert_eq!(hold.held(&other_body)?, None);
    let other_agent = ask(lys_identity::AgentId::generate()?, "aa");
    assert_eq!(hold.held(&other_agent)?, None);
    hold.made(&same)?;
    assert_eq!(hold.held(&same)?, None);
    assert_eq!(held_count(&hold)?, 0);
    Ok(())
}

#[test]
fn a_full_hold_lets_the_oldest_ask_go() -> Result<(), Box<dyn std::error::Error>> {
    let hold = RefusedAsks::new();
    let agent = lys_identity::AgentId::generate()?;
    let first = ask(agent, "0");
    hold.refused(first.clone(), json!({"operation": "0"}))?;
    for n in 1..=REFUSED_ASKS_HELD {
        hold.refused(
            ask(agent, &n.to_string()),
            json!({"operation": n.to_string()}),
        )?;
    }
    assert_eq!(held_count(&hold)?, REFUSED_ASKS_HELD);
    assert_eq!(hold.held(&first)?, None, "the oldest was let go");
    assert_eq!(
        hold.held(&ask(agent, "1"))?,
        Some(json!({"operation": "1"})),
        "the next oldest is still held"
    );
    Ok(())
}
