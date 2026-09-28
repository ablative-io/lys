#![cfg(test)]
//! What resolving and cutting refuses: each refusal names the lantern and
//! writes nothing, and a `lit_in` that holds no copy or is no session id is
//! refused by name.

use std::path::Path;

use serde_json::json;

use crate::error::{ForkError, HomeError};
use crate::record::entries::EntryBody;
use crate::record::fork_cut::resolve_and_cut;

use super::{COMPACTED, Gate, NOTE, PARENT, SENTINELS, fixture_home, lantern_entry, session_files};

#[test]
fn each_refusal_names_the_lantern_and_writes_nothing() -> Gate {
    let (dir, home, lanterns) = fixture_home()?;
    let before = session_files(&home)?;
    let mut refusals: Vec<HomeError> = Vec::new();

    for id in ["no-such-lantern", "e5"] {
        let refused = resolve_and_cut(&home, id, None);
        assert!(
            matches!(&refused, Err(HomeError::Fork(ForkError::NoSuchLantern { lantern })) if lantern == id),
            "{refused:?}"
        );
        refusals.push(refused.err().ok_or("refused")?);
    }
    let elsewhere = resolve_and_cut(&home, &lanterns.o2, Some(COMPACTED));
    assert!(
        matches!(
            &elsewhere,
            Err(HomeError::UnknownLantern { session, id }) if session == COMPACTED && id == "O2"
        ),
        "{elsewhere:?}"
    );
    refusals.push(elsewhere.err().ok_or("refused")?);
    let nothing = resolve_and_cut(&home, &lanterns.l1, None);
    assert!(
        matches!(&nothing, Err(HomeError::Fork(ForkError::NothingToFork { lantern })) if *lantern == lanterns.l1),
        "{nothing:?}"
    );
    refusals.push(nothing.err().ok_or("refused")?);
    let absent = resolve_and_cut(&home, &lanterns.l5, Some("no-such-session"));
    assert!(
        matches!(&absent, Err(HomeError::UnknownSession { session }) if session == "no-such-session"),
        "{absent:?}"
    );
    refusals.push(absent.err().ok_or("refused")?);
    let other_kind = resolve_and_cut(&home, "e5", Some(PARENT));
    assert!(
        matches!(&other_kind, Err(HomeError::UnknownLantern { session, id }) if session == PARENT && id == "e5"),
        "{other_kind:?}"
    );
    refusals.push(other_kind.err().ok_or("refused")?);

    assert_eq!(refusals.len(), 6);
    for refused in &refusals {
        let text = refused.to_string();
        for sentinel in SENTINELS {
            assert!(!text.contains(sentinel));
        }
        assert!(!text.contains(NOTE));
    }
    let after = session_files(&home)?;
    assert_eq!(after.len(), before.len());
    assert_eq!(after, before);
    assert!(
        !Path::new(&home.root().join("sessions"))
            .join("A.jsonl")
            .exists()
    );
    drop(dir);
    Ok(())
}

#[test]
fn a_lit_in_session_that_holds_no_copy_refuses_naming_the_holder_read() -> Gate {
    let (dir, home, _) = fixture_home()?;
    {
        let reader = home.read_session(PARENT)?;
        let mut copy = home.create_session("A", "/fixture", None)?;
        copy.append_entry(&reader.entry("e1")?)?;
        copy.append_entry(&reader.entry("e2")?)?;
        // A copy whose `lit_in` names a session other than the one it
        // stands in: the light act cannot produce it.
        copy.append_entry(&lantern_entry("N2", "e2", "e2", Some("elsewhere")))?;
        // A session of the home that holds no copy of the lantern.
        home.create_session("elsewhere", "/fixture", None)?;
    }
    let mut refusals = 0;
    for (session, named) in [(None, "A"), (Some("A"), "A"), (Some(PARENT), PARENT)] {
        let refused = resolve_and_cut(&home, "N2", session);
        assert!(
            matches!(
                &refused,
                Err(HomeError::Fork(ForkError::LanternNotLitHere { lantern, session, lit_in }))
                    if lantern == "N2" && session == named && lit_in == "elsewhere"
            ),
            "{refused:?}"
        );
        refusals += 1;
    }
    assert_eq!(refusals, 3);
    drop(dir);
    Ok(())
}

#[test]
fn a_lit_in_that_is_not_a_session_id_is_refused_by_name() -> Gate {
    let (dir, home, _) = fixture_home()?;
    {
        let reader = home.read_session(PARENT)?;
        let mut copy = home.create_session("A", "/fixture", None)?;
        copy.append_entry(&reader.entry("e1")?)?;
        copy.append_entry(&reader.entry("e2")?)?;
        for (id, lit_in) in [
            ("N1", json!(null)),
            ("N2", json!(5)),
            ("N3", json!("../elsewhere")),
            ("N4", json!("no-such-session")),
        ] {
            // A `lit_in` that is not a session id: the light act cannot produce it.
            let mut entry = lantern_entry(id, "e2", "e2", Some("placeholder"));
            let EntryBody::Custom {
                data: Some(data), ..
            } = &mut entry.body
            else {
                return Err("a custom entry with data".into());
            };
            data["lit_in"] = lit_in;
            copy.append_entry(&entry)?;
        }
    }
    let before = session_files(&home)?;
    let mut refusals = 0;
    for (id, what) in [
        ("N1", "is null"),
        ("N2", "is not a string"),
        ("N3", "is not a safe session name"),
        ("N4", "names no session of the home"),
    ] {
        for session in [None, Some("A")] {
            let refused = resolve_and_cut(&home, id, session);
            assert!(
                matches!(
                    &refused,
                    Err(HomeError::Fork(ForkError::LitInNotASession { lantern, what: found }))
                        if lantern == id && *found == what
                ),
                "{refused:?}"
            );
            assert!(
                refused
                    .err()
                    .ok_or("refused")?
                    .to_string()
                    .starts_with("lit_in_not_a_session")
            );
            refusals += 1;
        }
    }
    assert_eq!(refusals, 8);
    assert_eq!(session_files(&home)?, before);
    drop(dir);
    Ok(())
}
