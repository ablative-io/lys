//! The hold of refused asks: the first identical refused ask is kept, a
//! repeat is answered under the same leaf, a made, moved-past or lapsed ask
//! is let go, the oldest is let go once the hold is full, and the body's
//! `operation` is no part of which ask it is.

use serde_json::json;

use super::{Ask, HELD_SECONDS, Logs, REFUSED_ASKS_HELD, RefusedAsks, ask_digest, digest};

const LOGS: Logs = Logs {
    directory: 10,
    grants: 3,
};
const NOW: u64 = 1_000;

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
    assert_eq!(hold.held(&same, LOGS, NOW)?, None);
    hold.refused(same.clone(), json!({"operation": "first"}), LOGS, NOW)?;
    hold.refused(same.clone(), json!({"operation": "second"}), LOGS, NOW + 1)?;
    assert_eq!(held_count(&hold)?, 1);
    assert_eq!(
        hold.held(&same, LOGS, NOW + 2)?,
        Some(json!({"operation": "first"}))
    );
    let other_body = ask(agent, "bb");
    assert_eq!(hold.held(&other_body, LOGS, NOW)?, None);
    let other_agent = ask(lys_identity::AgentId::generate()?, "aa");
    assert_eq!(hold.held(&other_agent, LOGS, NOW)?, None);
    hold.made(&same)?;
    assert_eq!(hold.held(&same, LOGS, NOW)?, None);
    assert_eq!(held_count(&hold)?, 0);
    Ok(())
}

#[test]
fn a_held_ask_is_let_go_once_either_log_has_moved() -> Result<(), Box<dyn std::error::Error>> {
    let hold = RefusedAsks::new();
    let same = ask(lys_identity::AgentId::generate()?, "aa");
    let leaf = json!({"operation": "first"});
    hold.refused(same.clone(), leaf.clone(), LOGS, NOW)?;
    let directory_moved = Logs {
        directory: LOGS.directory + 1,
        ..LOGS
    };
    assert_eq!(
        hold.held(&same, directory_moved, NOW)?,
        None,
        "a moved directory log is a miss"
    );
    assert_eq!(held_count(&hold)?, 0, "and the ask is let go");
    hold.refused(same.clone(), leaf, LOGS, NOW)?;
    let grants_moved = Logs {
        grants: LOGS.grants + 1,
        ..LOGS
    };
    assert_eq!(
        hold.held(&same, grants_moved, NOW)?,
        None,
        "a moved grant log is a miss"
    );
    assert_eq!(held_count(&hold)?, 0);
    Ok(())
}

#[test]
fn a_held_ask_lapses_after_held_seconds() -> Result<(), Box<dyn std::error::Error>> {
    let hold = RefusedAsks::new();
    let same = ask(lys_identity::AgentId::generate()?, "aa");
    hold.refused(same.clone(), json!({"operation": "first"}), LOGS, NOW)?;
    assert_eq!(
        hold.held(&same, LOGS, NOW + HELD_SECONDS - 1)?,
        Some(json!({"operation": "first"})),
        "held to the last second"
    );
    assert_eq!(
        hold.held(&same, LOGS, NOW + HELD_SECONDS)?,
        None,
        "then lapsed"
    );
    assert_eq!(held_count(&hold)?, 0);
    Ok(())
}

#[test]
fn the_ask_digest_leaves_the_operation_out() {
    let first = ask_digest(br#"{"display_name":"N","operation":"a"}"#);
    let second = ask_digest(br#"{"display_name":"N","operation":"b"}"#);
    let bare = ask_digest(br#"{"display_name":"N"}"#);
    assert_eq!(first, second, "a fresh operation id is the same ask");
    assert_eq!(first, bare, "and so is no operation at all");
    assert_ne!(
        first,
        ask_digest(br#"{"display_name":"M","operation":"a"}"#)
    );
    assert_eq!(
        ask_digest(b"[1,2]"),
        digest(b"[1,2]"),
        "a non-object body as it is"
    );
    assert_eq!(ask_digest(b"not json"), digest(b"not json"));
}

#[test]
fn a_full_hold_lets_the_oldest_ask_go() -> Result<(), Box<dyn std::error::Error>> {
    let hold = RefusedAsks::new();
    let agent = lys_identity::AgentId::generate()?;
    let first = ask(agent, "0");
    hold.refused(first.clone(), json!({"operation": "0"}), LOGS, NOW)?;
    for n in 1..=REFUSED_ASKS_HELD {
        hold.refused(
            ask(agent, &n.to_string()),
            json!({"operation": n.to_string()}),
            LOGS,
            NOW,
        )?;
    }
    assert_eq!(held_count(&hold)?, REFUSED_ASKS_HELD);
    assert_eq!(hold.held(&first, LOGS, NOW)?, None, "the oldest was let go");
    assert_eq!(
        hold.held(&ask(agent, "1"), LOGS, NOW)?,
        Some(json!({"operation": "1"})),
        "the next oldest is still held"
    );
    Ok(())
}
