//! The seat list's reading of a supervised seat's session.

use std::collections::BTreeMap;

use super::{Live, Reading, reading_of};

fn live(session: &str) -> Live {
    Live {
        session: session.to_owned(),
        pid: Some(7),
        alive: true,
        harness: lys_runner::liveness::HarnessLiveness {
            managed: true,
            started_fresh: false,
        },
        harness_session: None,
        turn_active: false,
        last_signal_at: None,
        started_at: 1,
        ended: false,
    }
}

/// A supervised seat's session, which the machine's runner does not list,
/// is read from its owner and listed as that session (e6d5bc36).
#[test]
fn a_seat_whose_owner_holds_its_session_is_listed_with_that_session() {
    let held = vec![live("other")];
    let owned = BTreeMap::from([("seat".to_owned(), Ok(live("seat")))]);
    assert!(matches!(
        reading_of(&held, &owned, "seat"),
        Reading::Read(Some(found)) if found.session == "seat" && found.alive
    ));
    assert!(matches!(
        reading_of(&held, &owned, "other"),
        Reading::Read(Some(found)) if found.session == "other"
    ));
}

/// An owner that cannot be asked leaves its seat unread with the owner's
/// reason, never read as stopped (5c791270); an owner that answered no such
/// session reads as not held.
#[test]
fn an_owner_that_cannot_be_asked_reads_unread_with_its_reason() {
    let held: Vec<Live> = Vec::new();
    let owned = BTreeMap::from([(
        "seat".to_owned(),
        Err("the owner's socket refused the connection".to_owned()),
    )]);
    assert!(matches!(
        reading_of(&held, &owned, "seat"),
        Reading::Unread(reason) if reason == "the owner's socket refused the connection"
    ));
    assert!(matches!(
        reading_of(&held, &owned, "gone"),
        Reading::Read(None)
    ));
}
