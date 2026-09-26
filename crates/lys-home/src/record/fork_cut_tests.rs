//! Gates on the fork's names and refusals (HOME-006 R1): the two custom
//! types are named as the record states, and each of the four refusals
//! names itself and the ids it was given, carrying no note and no data.

use crate::error::HomeError;
use crate::record::entries::{CUSTOM_FORK, CUSTOM_FORKED_FROM};

#[test]
fn the_fork_custom_types_are_named_as_the_record_states() {
    assert_eq!(CUSTOM_FORK, "lys.fork");
    assert_eq!(CUSTOM_FORKED_FROM, "lys.forked_from");
}

#[test]
fn each_fork_refusal_names_itself_and_the_ids_it_was_given() {
    let mut refusals = Vec::new();

    let unknown = HomeError::NoSuchLantern {
        lantern: "no-such-lantern".into(),
    }
    .to_string();
    assert!(unknown.contains("no-such-lantern"), "{unknown}");
    refusals.push(unknown);

    let ambiguous = HomeError::LanternAmbiguous {
        lantern: "x".into(),
        sessions: vec!["a".into(), "b".into()],
    }
    .to_string();
    assert!(ambiguous.starts_with("lantern_ambiguous"), "{ambiguous}");
    for id in ["x", "a", "b"] {
        assert!(ambiguous.contains(id), "{ambiguous}");
    }
    refusals.push(ambiguous);

    let not_here = HomeError::LanternNotLitHere {
        lantern: "x".into(),
        session: "a".into(),
        lit_in: "b".into(),
    }
    .to_string();
    assert!(not_here.starts_with("lantern_not_lit_here"), "{not_here}");
    for id in ["x", "a", "b"] {
        assert!(not_here.contains(id), "{not_here}");
    }
    refusals.push(not_here);

    let nothing = HomeError::NothingToFork {
        lantern: "x".into(),
    }
    .to_string();
    assert!(nothing.starts_with("nothing_to_fork"), "{nothing}");
    assert!(nothing.contains('x'), "{nothing}");
    assert!(
        nothing.contains("before any assistant message"),
        "{nothing}"
    );
    refusals.push(nothing);

    assert_eq!(refusals.len(), 4);
}
