#![cfg(test)]
//! Gates on the runtime reports' folded state: its indexes answer a lookup
//! by visiting one session, they are rebuilt when a sealed state is read
//! back, they are never sealed, and the sealed bytes are the ones an owned
//! copy of the state sealed.

use serde::Serialize;

use super::{FORMAT, Found, Held, Report, Reported, Tracked};

/// A snapshot sealed as it was before it borrowed: an owned copy of the
/// state beside its format.
#[derive(Serialize)]
struct Copied {
    format: String,
    held: Held,
}

/// The state as the base declared it, before it carried indexes: its
/// sessions and nothing else. Written out here, not taken from `Held`, so
/// the bytes are checked against a shape the indexed state cannot change.
#[derive(Serialize)]
struct BaseHeld {
    sessions: Vec<Tracked>,
}

/// A snapshot sealed as the base sealed it.
#[derive(Serialize)]
struct BaseSealed {
    format: String,
    held: BaseHeld,
}

fn report(operation: &str, session: &str, state: Reported) -> Report {
    Report {
        operation: operation.to_owned(),
        session: session.to_owned(),
        agent: Some("agent-a".to_owned()),
        machine: "op-00000000000000000000000000000001".to_owned(),
        state,
        what: "seen".to_owned(),
        confirmation: String::new(),
        reported_by: "agent-a".to_owned(),
        at: 5,
        launch: None,
    }
}

fn kept(sessions: usize) -> Result<Held, String> {
    let mut held = Held::default();
    for n in 0..sessions {
        let session = format!("s-{n}");
        held.hold(report(&format!("op-{n}-a"), &session, Reported::Starting))?;
        held.hold(report(&format!("op-{n}-b"), &session, Reported::Running))?;
    }
    Ok(held)
}

#[test]
fn a_lookup_visits_the_one_session_it_names() -> Result<(), String> {
    let held = kept(1_000)?;
    let before = held.visited();
    let session = held.session("s-999").ok_or("s-999 is kept")?;
    assert_eq!(session.reports.len(), 2);
    let found = held.operation("op-998-b").ok_or("op-998-b is kept")?;
    assert_eq!(found.session, "s-998");
    assert_eq!(held.find("op-5-a", "s-5"), Found::Kept(5, 0));
    assert_eq!(held.find("op-new", "s-7"), Found::Session(7));
    assert_eq!(held.find("op-new", "s-new"), Found::Neither);
    assert_eq!(
        held.visited() - before,
        4,
        "each lookup that finds visits one"
    );
    Ok(())
}

#[test]
fn a_state_read_back_is_indexed_and_seals_the_same_bytes() -> Result<(), String> {
    let held = kept(50)?;
    let copied = serde_json::to_vec(&Copied {
        format: FORMAT.to_owned(),
        held: held.clone(),
    })
    .map_err(|error| error.to_string())?;
    let sealed = held.encode()?;
    assert_eq!(sealed, copied);
    let base = serde_json::to_vec(&BaseSealed {
        format: FORMAT.to_owned(),
        held: BaseHeld {
            sessions: held.sessions().to_vec(),
        },
    })
    .map_err(|error| error.to_string())?;
    assert_eq!(sealed, base, "the bytes the base sealed");
    let text = String::from_utf8(sealed).map_err(|error| error.to_string())?;
    assert!(text.starts_with(r#"{"format":"lys-runtime-state/v1","held":{"sessions":[{"#));
    assert!(!text.contains("by_session") && !text.contains("by_operation"));
    assert!(!text.contains("visited"));

    let read = Held::decode(&copied)?;
    assert_eq!(read, held);
    assert_eq!(read.find("op-49-b", "s-49"), Found::Kept(49, 1));
    assert_eq!(read.find("op-new", "s-3"), Found::Session(3));
    assert_eq!(read.visited(), 2);
    Ok(())
}
