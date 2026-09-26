//! Gates on epilogues (HOME-004 R4): appended after the lantern at the head,
//! ordinal from 1, earlier lines unchanged, and each refusal by name with
//! nothing written.

use std::error::Error;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::entries::{CUSTOM_LANTERN_EPILOGUE, EpilogueData};
use crate::record::epilogue::{Added, add_epilogue, epilogue_of};
use crate::record::lantern::light;
use crate::record::lantern_tests::{LIGHTER, NOTE};
use crate::record::reader_tests::{FIXTURE, TRANSCRIPT_LINE, fixture_home};

type Gate = Result<(), Box<dyn Error>>;

pub(crate) const ANNOTATOR: &str = "fixture-annotator";
pub(crate) const WORDS_ONE: &str = "and the replay fold rings true";
pub(crate) const WORDS_TWO: &str = "a later word: cobalt";

/// The R4 fixture: L1 and L2 at e2, then E1 and E2 on L1; returns the home
/// with (L1, L2, E1, E2).
pub(crate) fn lit_fixture() -> Result<(tempfile::TempDir, Home, [String; 4]), Box<dyn Error>> {
    let (dir, home) = fixture_home()?;
    let l1 = light(&home, FIXTURE, "e2", NOTE, LIGHTER)?.id;
    let l2 = light(&home, FIXTURE, "e2", "second look", LIGHTER)?.id;
    let e1 = add_epilogue(&home, FIXTURE, &l1, WORDS_ONE, ANNOTATOR)?.id;
    let e2 = add_epilogue(&home, FIXTURE, &l1, WORDS_TWO, ANNOTATOR)?.id;
    Ok((dir, home, [l1, l2, e1, e2]))
}

fn epilogue_data(home: &Home, id: &str) -> Result<EpilogueData, Box<dyn Error>> {
    let entry = home.read_session(FIXTURE)?.entry(id)?;
    assert!(entry.is_custom(CUSTOM_LANTERN_EPILOGUE));
    Ok(epilogue_of(&entry.body).ok_or("not an epilogue")?)
}

fn file_len(home: &Home) -> Result<u64, Box<dyn Error>> {
    Ok(std::fs::metadata(home.session_path(FIXTURE)?)?.len())
}

#[test]
fn an_epilogue_hangs_under_the_head_and_names_its_lantern() -> Gate {
    let (_dir, home) = fixture_home()?;
    let l1 = light(&home, FIXTURE, "e2", NOTE, LIGHTER)?.id;
    let l2 = light(&home, FIXTURE, "e2", "second look", LIGHTER)?.id;
    let file = home.session_path(FIXTURE)?;
    let before = std::fs::read(&file)?;
    let added = add_epilogue(&home, FIXTURE, &l1, WORDS_ONE, ANNOTATOR)?;
    let after = std::fs::read(&file)?;
    assert_eq!(&after[..before.len()], &before[..]);
    assert_eq!(
        std::str::from_utf8(&after)?.lines().count(),
        std::str::from_utf8(&before)?.lines().count() + 1
    );
    assert_eq!(
        added,
        Added {
            id: added.id.clone(),
            lantern: l1.clone(),
            session: FIXTURE.to_owned(),
            added_by: ANNOTATOR.to_owned(),
            added_at: added.added_at.clone(),
            ordinal: 1,
        }
    );
    let owner = home.open_session(FIXTURE)?;
    assert_eq!(owner.head()?, Some(added.id.as_str()));
    assert_eq!(owner.entry(&added.id)?.parent_id(), Some(l2.as_str()));
    drop(owner);
    let data = epilogue_data(&home, &added.id)?;
    assert_eq!(data.lantern, l1);
    assert_eq!(data.words, WORDS_ONE);
    assert_eq!(data.added_by, ANNOTATOR);
    assert_eq!(data.added_at, added.added_at);
    // A second epilogue follows the first and leaves its line unchanged.
    let second = add_epilogue(&home, FIXTURE, &l1, WORDS_TWO, ANNOTATOR)?;
    assert_eq!(second.ordinal, 2);
    assert_eq!(
        home.open_session(FIXTURE)?.head()?,
        Some(second.id.as_str())
    );
    let again = std::fs::read(&file)?;
    assert_eq!(&again[..after.len()], &after[..]);
    for report in [&added, &second] {
        let text = serde_json::to_string(report)?;
        assert!(!text.contains(WORDS_ONE));
        assert!(!text.contains("cobalt"));
    }
    Ok(())
}

#[test]
fn each_refusal_names_itself_and_writes_nothing() -> Gate {
    let (_dir, home, [l1, _, e1, _]) = lit_fixture()?;
    let len = file_len(&home)?;
    let mut refusals = Vec::new();
    for id in ["no-such-lantern", "e2", e1.as_str()] {
        let refused = add_epilogue(&home, FIXTURE, id, WORDS_ONE, ANNOTATOR);
        assert!(matches!(
            &refused,
            Err(HomeError::UnknownLantern { session, id: named }) if session == FIXTURE && named == id
        ));
        refusals.push(refused);
    }
    for words in ["", "   "] {
        let refused = add_epilogue(&home, FIXTURE, &l1, words, ANNOTATOR);
        assert!(matches!(
            &refused,
            Err(HomeError::EmptyNote { what }) if *what == "epilogue"
        ));
        refusals.push(refused);
    }
    {
        let other = home.open_session(FIXTURE)?;
        let while_held = add_epilogue(&home, FIXTURE, &l1, WORDS_ONE, ANNOTATOR);
        assert!(matches!(&while_held, Err(HomeError::SessionHeld { .. })));
        refusals.push(while_held);
        drop(other);
    }
    let missing = add_epilogue(&home, "no-such-session", &l1, WORDS_ONE, ANNOTATOR);
    assert!(matches!(
        &missing,
        Err(HomeError::UnknownSession { session }) if session == "no-such-session"
    ));
    refusals.push(missing);
    assert_eq!(refusals.len(), 7);
    for refused in refusals {
        let text = refused
            .err()
            .ok_or("a refusal was not refused")?
            .to_string();
        assert!(!text.contains(TRANSCRIPT_LINE));
        assert!(!text.contains(WORDS_ONE));
    }
    assert_eq!(file_len(&home)?, len);
    Ok(())
}

#[test]
fn the_session_and_the_lantern_are_checked_before_the_words() -> Gate {
    let (_dir, home, [l1, _, _, _]) = lit_fixture()?;
    assert!(matches!(
        add_epilogue(&home, "no-such-session", &l1, "", ANNOTATOR),
        Err(HomeError::UnknownSession { session }) if session == "no-such-session"
    ));
    assert!(matches!(
        add_epilogue(&home, FIXTURE, "e2", "", ANNOTATOR),
        Err(HomeError::UnknownLantern { id, .. }) if id == "e2"
    ));
    let other = home.open_session(FIXTURE)?;
    assert!(matches!(
        add_epilogue(&home, FIXTURE, &l1, "", ANNOTATOR),
        Err(HomeError::SessionHeld { .. })
    ));
    drop(other);
    assert!(matches!(
        add_epilogue(&home, FIXTURE, &l1, "", ANNOTATOR),
        Err(HomeError::EmptyNote { what: "epilogue" })
    ));
    Ok(())
}
