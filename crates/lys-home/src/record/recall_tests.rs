//! Gates on recall (HOME-004 R5): a phrase inside one text only, both
//! lanterns on a point, epilogues in order, a held session still read, a
//! broken session named and skipped, nothing written, and never a line of
//! the transcript.

use std::error::Error;

use serde_json::Value;

use crate::error::HomeError;
use crate::record::Home;
use crate::record::epilogue_tests::{WORDS_ONE, WORDS_TWO, lit_fixture};
use crate::record::lantern::light;
use crate::record::lantern_tests::LIGHTER;
use crate::record::reader_tests::{FIXTURE, TRANSCRIPT_LINE, message, snapshot};
use crate::record::recall::{RecallReport, recall_by_note, recall_by_point};

type Gate = Result<(), Box<dyn Error>>;

const OTHER: &str = "fixture-other";

/// The recall fixture: the R4 fixture plus `fixture-other` with one message
/// o1 and lantern L3 at it; returns (L1, L2, E1, E2, L3).
fn recall_fixture() -> Result<(tempfile::TempDir, Home, [String; 5]), Box<dyn Error>> {
    let (dir, home, [l1, l2, e1, e2]) = lit_fixture()?;
    {
        let mut other = home.create_session(OTHER, "/fixture", None)?;
        other.append_entry(&message("o1", None, "fixture"))?;
    }
    let l3 = light(
        &home,
        OTHER,
        "o1",
        "nothing shared but the fold held",
        LIGHTER,
    )?
    .id;
    Ok((dir, home, [l1, l2, e1, e2, l3]))
}

fn ids(report: &RecallReport) -> Vec<&str> {
    report.lanterns.iter().map(|row| row.id.as_str()).collect()
}

fn keys(value: &Value) -> Vec<String> {
    let mut names: Vec<String> = value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    names.sort_unstable();
    names
}

/// Every recall the gates run, in one place, so each is checked for writing
/// nothing and printing no transcript.
fn every_recall(home: &Home) -> Vec<Result<RecallReport, HomeError>> {
    vec![
        recall_by_note(home, "fold"),
        recall_by_note(home, "FOLD HELD"),
        recall_by_note(home, "cobalt"),
        recall_by_note(home, "under replay and"),
        recall_by_note(home, "rings true a later"),
        recall_by_note(home, ""),
        recall_by_note(home, " \t "),
        recall_by_point(home, FIXTURE, "e2"),
        recall_by_point(home, FIXTURE, "e4"),
        recall_by_point(home, "no-such-session", "e2"),
        recall_by_point(home, FIXTURE, "no-such-entry"),
    ]
}

#[test]
fn recall_by_note_finds_a_phrase_inside_one_text_only() -> Gate {
    let (_dir, home, [l1, _, _, _, l3]) = recall_fixture()?;
    let fold = recall_by_note(&home, "fold")?;
    assert_eq!(ids(&fold), [l1.as_str(), l3.as_str()]);
    assert!(fold.skipped.is_empty());
    assert_eq!(
        ids(&recall_by_note(&home, "FOLD HELD")?),
        [l1.as_str(), l3.as_str()]
    );
    let cobalt = recall_by_note(&home, "cobalt")?;
    assert_eq!(ids(&cobalt), [l1.as_str()]);
    let words: Vec<&str> = cobalt
        .lanterns
        .first()
        .ok_or("no row")?
        .epilogues
        .iter()
        .map(|e| e.words.as_str())
        .collect();
    assert_eq!(words, [WORDS_ONE, WORDS_TWO]);
    assert!(ids(&recall_by_note(&home, "under replay and")?).is_empty());
    assert!(ids(&recall_by_note(&home, "rings true a later")?).is_empty());
    for blank in ["", " \t "] {
        assert!(matches!(
            recall_by_note(&home, blank),
            Err(HomeError::EmptyNote { what }) if what == "words"
        ));
    }
    Ok(())
}

#[test]
fn recall_by_point_lists_every_lantern_on_the_entry_and_refuses_unknown_names() -> Gate {
    let (_dir, home, [l1, l2, _, _, _]) = recall_fixture()?;
    assert_eq!(
        ids(&recall_by_point(&home, FIXTURE, "e2")?),
        [l1.as_str(), l2.as_str()]
    );
    let none = recall_by_point(&home, FIXTURE, "e4")?;
    assert!(none.lanterns.is_empty());
    assert!(none.skipped.is_empty());
    assert!(matches!(
        recall_by_point(&home, "no-such-session", "e2"),
        Err(HomeError::UnknownSession { session }) if session == "no-such-session"
    ));
    assert!(matches!(
        recall_by_point(&home, FIXTURE, "no-such-entry"),
        Err(HomeError::UnknownEntry { session, id }) if session == FIXTURE && id == "no-such-entry"
    ));
    Ok(())
}

#[test]
fn a_row_carries_exactly_its_fields() -> Gate {
    let (_dir, home, _) = recall_fixture()?;
    let report = serde_json::to_value(recall_by_note(&home, "fold")?)?;
    assert_eq!(keys(&report), ["lanterns", "skipped"]);
    let rows = report["lanterns"].as_array().ok_or("no rows")?;
    assert_eq!(rows.len(), 2);
    let mut epilogues = 0;
    for row in rows {
        assert_eq!(
            keys(row),
            [
                "epilogues",
                "id",
                "lit_at",
                "lit_by",
                "note",
                "point",
                "session"
            ]
        );
        for epilogue in row["epilogues"].as_array().ok_or("no epilogues")? {
            assert_eq!(keys(epilogue), ["added_at", "added_by", "words"]);
            epilogues += 1;
        }
    }
    assert_eq!(epilogues, 2);
    Ok(())
}

#[test]
fn a_held_session_is_still_recalled() -> Gate {
    let (_dir, home, [l1, _, _, _, l3]) = recall_fixture()?;
    let owner = home.open_session(FIXTURE)?;
    let fold = recall_by_note(&home, "fold")?;
    assert_eq!(ids(&fold), [l1.as_str(), l3.as_str()]);
    assert!(fold.skipped.is_empty());
    drop(owner);
    Ok(())
}

#[test]
fn a_broken_session_is_skipped_by_name_and_the_rest_listed() -> Gate {
    let (_dir, home, [l1, _, _, _, l3]) = recall_fixture()?;
    std::fs::write(home.session_path("fixture-broken")?, "not json\n")?;
    let fold = recall_by_note(&home, "fold")?;
    assert_eq!(ids(&fold), [l1.as_str(), l3.as_str()]);
    assert_eq!(fold.skipped.len(), 1);
    let skipped = fold.skipped.first().ok_or("nothing skipped")?;
    assert_eq!(skipped.session, "fixture-broken");
    assert!(!skipped.reason.is_empty());
    Ok(())
}

#[test]
fn every_recall_writes_nothing_and_prints_no_transcript() -> Gate {
    let (_dir, home, _) = recall_fixture()?;
    std::fs::write(home.session_path("fixture-broken")?, "not json\n")?;
    let before = snapshot(&home)?;
    let results = every_recall(&home);
    assert_eq!(results.len(), 11);
    for result in results {
        let text = match result {
            Ok(report) => serde_json::to_string(&report)?,
            Err(e) => e.to_string(),
        };
        assert!(!text.contains(TRANSCRIPT_LINE));
    }
    assert_eq!(snapshot(&home)?, before);
    Ok(())
}
