//! Gates on the lantern entry (HOME-004 R1) and on lighting (R3): the data
//! shapes, earlier bytes kept, the head on the lantern, the note byte for
//! byte, and each refusal by name with nothing written.

use std::error::Error;

use serde_json::{Value, json};

use crate::error::HomeError;
use crate::record::entries::{
    CUSTOM_LANTERN, CUSTOM_LANTERN_EPILOGUE, EntryBody, EpilogueData, LanternData,
};
use crate::record::lantern::light;
use crate::record::reader_tests::{FIXTURE, TRANSCRIPT_LINE, fixture_home};
use crate::record::{Home, now};

type Gate = Result<(), Box<dyn Error>>;

pub(crate) const NOTE: &str = "The Fold Held Under Replay";
pub(crate) const LIGHTER: &str = "fixture-lighter";

fn keys(value: &Value) -> Vec<String> {
    value
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

fn file_len(home: &Home) -> Result<u64, Box<dyn Error>> {
    Ok(std::fs::metadata(home.session_path(FIXTURE)?)?.len())
}

fn lantern_data(home: &Home, id: &str) -> Result<LanternData, Box<dyn Error>> {
    let entry = home.read_session(FIXTURE)?.entry(id)?;
    let EntryBody::Custom { custom_type, data } = entry.body else {
        return Err("not a custom entry".into());
    };
    assert_eq!(custom_type, CUSTOM_LANTERN);
    Ok(serde_json::from_value(data.ok_or("no data")?)?)
}

#[test]
fn the_custom_types_are_named_as_the_record_states() {
    assert_eq!(CUSTOM_LANTERN, "lys.lantern");
    assert_eq!(CUSTOM_LANTERN_EPILOGUE, "lys.lantern_epilogue");
}

#[test]
fn lantern_data_round_trips_with_exactly_its_four_keys() -> Gate {
    let data = LanternData {
        point: "e2".to_owned(),
        note: NOTE.to_owned(),
        lit_by: LIGHTER.to_owned(),
        lit_at: now(),
    };
    let value = serde_json::to_value(&data)?;
    let mut names = keys(&value);
    names.sort_unstable();
    assert_eq!(names, ["lit_at", "lit_by", "note", "point"]);
    let back: LanternData = serde_json::from_value(value)?;
    assert_eq!(back, data);
    Ok(())
}

#[test]
fn epilogue_data_serialises_with_exactly_its_four_keys() -> Gate {
    let data = EpilogueData {
        lantern: "l1".to_owned(),
        words: "further".to_owned(),
        added_by: "fixture-annotator".to_owned(),
        added_at: now(),
    };
    let mut names = keys(&serde_json::to_value(&data)?);
    names.sort_unstable();
    assert_eq!(names, ["added_at", "added_by", "lantern", "words"]);
    Ok(())
}

#[test]
fn lantern_data_with_an_extra_key_is_refused() {
    let value =
        json!({"point": "e2", "note": NOTE, "lit_by": LIGHTER, "lit_at": now(), "anchors": []});
    assert!(serde_json::from_value::<LanternData>(value).is_err());
}

#[test]
fn lighting_appends_one_line_under_the_head_and_moves_the_head_onto_it() -> Gate {
    let (_dir, home) = fixture_home()?;
    let file = home.session_path(FIXTURE)?;
    let before = std::fs::read(&file)?;
    let lit = light(&home, FIXTURE, "e2", NOTE, LIGHTER)?;
    let after = std::fs::read(&file)?;
    assert_eq!(&after[..before.len()], &before[..]);
    assert_eq!(std::str::from_utf8(&after)?.lines().count(), 7);
    assert_eq!(lit.session, FIXTURE);
    assert_eq!(lit.point, "e2");
    let owner = home.open_session(FIXTURE)?;
    assert_eq!(owner.head()?, Some(lit.id.as_str()));
    let entry = owner.entry(&lit.id)?;
    assert_eq!(entry.parent_id(), Some("e5"));
    assert!(entry.is_custom(CUSTOM_LANTERN));
    drop(owner);
    let data = lantern_data(&home, &lit.id)?;
    assert_eq!(data.point, "e2");
    assert_eq!(data.note, NOTE);
    assert_eq!(data.lit_by, LIGHTER);
    assert_eq!(data.lit_at, lit.lit_at);
    // A second lantern on the same point sits after the first, unchanged.
    let second = light(&home, FIXTURE, "e2", "second look", LIGHTER)?;
    assert_ne!(second.id, lit.id);
    assert_eq!(lantern_data(&home, &second.id)?.point, "e2");
    let again = std::fs::read(&file)?;
    assert_eq!(&again[..after.len()], &after[..]);
    Ok(())
}

#[test]
fn a_note_is_stored_byte_for_byte() -> Gate {
    let (_dir, home) = fixture_home()?;
    let note = "  kept as written  ";
    let lit = light(&home, FIXTURE, "e4", note, LIGHTER)?;
    assert_eq!(lantern_data(&home, &lit.id)?.note, note);
    Ok(())
}

#[test]
fn each_refusal_names_itself_and_writes_nothing() -> Gate {
    let (_dir, home) = fixture_home()?;
    let first = light(&home, FIXTURE, "e2", NOTE, LIGHTER)?;
    let epilogue = {
        let mut owner = home.open_session(FIXTURE)?;
        owner.append(EntryBody::Custom {
            custom_type: CUSTOM_LANTERN_EPILOGUE.to_owned(),
            data: Some(
                json!({"lantern": first.id, "words": "w", "added_by": "a", "added_at": now()}),
            ),
        })?
    };
    let len = file_len(&home)?;
    let head = home.open_session(FIXTURE)?.head()?.map(str::to_owned);
    let mut refusals = Vec::new();

    let unknown = light(&home, FIXTURE, "no-such-entry", NOTE, LIGHTER);
    assert!(matches!(
        &unknown,
        Err(HomeError::UnknownEntry { session, id }) if session == FIXTURE && id == "no-such-entry"
    ));
    refusals.push(unknown);

    for point in [first.id.as_str(), epilogue.as_str()] {
        let refused = light(&home, FIXTURE, point, NOTE, LIGHTER);
        assert!(matches!(
            &refused,
            Err(HomeError::PointIsLantern { session, id }) if session == FIXTURE && id == point
        ));
        refusals.push(refused);
    }

    for note in ["", " \n\t "] {
        let refused = light(&home, FIXTURE, "e2", note, LIGHTER);
        assert!(matches!(
            &refused,
            Err(HomeError::EmptyNote { what }) if *what == "note"
        ));
        refusals.push(refused);
    }

    {
        let other = home.open_session(FIXTURE)?;
        let while_held = light(&home, FIXTURE, "e2", NOTE, LIGHTER);
        assert!(matches!(&while_held, Err(HomeError::SessionHeld { .. })));
        refusals.push(while_held);
        drop(other);
    }

    let missing = light(&home, "no-such-session", "e2", NOTE, LIGHTER);
    assert!(matches!(
        &missing,
        Err(HomeError::UnknownSession { session }) if session == "no-such-session"
    ));
    refusals.push(missing);
    for name in std::fs::read_dir(home.root().join("sessions"))? {
        let name = name?.file_name().to_string_lossy().into_owned();
        assert!(!name.starts_with("no-such-session"), "{name}");
    }

    assert_eq!(refusals.len(), 7);
    for refused in refusals {
        let text = refused
            .err()
            .ok_or("a refusal was not refused")?
            .to_string();
        assert!(!text.contains(TRANSCRIPT_LINE));
        assert!(!text.contains(NOTE));
    }
    assert_eq!(file_len(&home)?, len);
    assert_eq!(home.open_session(FIXTURE)?.head()?.map(str::to_owned), head);
    Ok(())
}

#[test]
fn the_session_and_the_point_are_checked_before_the_note() -> Gate {
    let (_dir, home) = fixture_home()?;
    assert!(matches!(
        light(&home, "no-such-session", "e2", "", LIGHTER),
        Err(HomeError::UnknownSession { session }) if session == "no-such-session"
    ));
    assert!(matches!(
        light(&home, FIXTURE, "no-such-entry", "", LIGHTER),
        Err(HomeError::UnknownEntry { id, .. }) if id == "no-such-entry"
    ));
    let other = home.open_session(FIXTURE)?;
    assert!(matches!(
        light(&home, FIXTURE, "e2", "", LIGHTER),
        Err(HomeError::SessionHeld { .. })
    ));
    drop(other);
    assert!(matches!(
        light(&home, FIXTURE, "e2", "", LIGHTER),
        Err(HomeError::EmptyNote { what: "note" })
    ));
    Ok(())
}
