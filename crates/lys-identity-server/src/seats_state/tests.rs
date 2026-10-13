#![cfg(test)]
use super::{Added, Held, Line, Moved, Retired, Started, Stopped};

fn added(operation: &str, name: &str) -> Line {
    Line::Added(Added {
        operation: operation.to_owned(),
        name: name.to_owned(),
        agent: "agent".to_owned(),
        harness: "claude".to_owned(),
        profile_version: 1,
        machine: "op-1".to_owned(),
        working_folder: None,
        account: None,
        by: "person".to_owned(),
        at: 1,
    })
}

#[test]
fn a_seat_folds_its_sessions_and_raises_its_revision() -> Result<(), String> {
    let mut held = Held::default();
    held.hold(added("op-a", "waffles"))?;
    held.hold(Line::Started(Started {
        qualification: None,
        transport: "managed".to_owned(),
        operation: "op-b".to_owned(),
        name: "waffles".to_owned(),
        session: "op-b".to_owned(),
        harness_session: Some("uuid".to_owned()),
        by: "person".to_owned(),
        at: 2,
    }))?;
    let seat = held.seat("waffles").ok_or("no seat")?;
    assert!(seat.running);
    assert_eq!(seat.revision, 2);
    assert_eq!(seat.session.as_deref(), Some("op-b"));
    held.hold(Line::Stopped(Stopped {
        operation: "op-c".to_owned(),
        name: "waffles".to_owned(),
        session: "op-b".to_owned(),
        forced: false,
        by: "person".to_owned(),
        at: 3,
    }))?;
    let seat = held.seat("waffles").ok_or("no seat")?;
    assert!(!seat.running);
    assert_eq!(seat.revision, 3);
    let sealed = held.encode()?;
    assert_eq!(Held::decode(&sealed)?, held);
    assert_eq!(Held::decode(&sealed)?.seat("waffles"), held.seat("waffles"));
    Ok(())
}

#[test]
fn a_name_or_an_operation_is_held_once() -> Result<(), String> {
    let mut held = Held::default();
    held.hold(added("op-a", "waffles"))?;
    assert!(held.hold(added("op-b", "waffles")).is_err());
    assert!(held.hold(added("op-a", "other")).is_err());
    assert!(held.seat("other").is_none());
    Ok(())
}

#[test]
fn qualification_marks_survive_a_snapshot_and_an_old_install_migrates_without_one()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(added("op-old", "versioned"))?;
    let legacy = held.encode()?;
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&legacy).map_err(|error| error.to_string())?;
    legacy["held"]["seats"][0]
        .as_object_mut()
        .ok_or("no old seat")?
        .remove("qualification");
    let mut migrated =
        Held::decode(&serde_json::to_vec(&legacy).map_err(|error| error.to_string())?)?;
    assert!(
        migrated
            .seat("versioned")
            .ok_or("no migrated seat")?
            .qualification
            .is_none()
    );
    migrated.hold(Line::Started(Started {
        qualification: Some("unqualified version 2.1.300; qualified 2.1.296".to_owned()),
        transport: "managed".to_owned(),
        operation: "op-new".to_owned(),
        name: "versioned".to_owned(),
        session: "session".to_owned(),
        harness_session: None,
        by: "person".to_owned(),
        at: 2,
    }))?;
    let kept = Held::decode(&migrated.encode()?)?;
    assert_eq!(
        kept.seat("versioned")
            .ok_or("seat missing")?
            .qualification
            .as_deref(),
        Some("unqualified version 2.1.300; qualified 2.1.296")
    );
    Ok(())
}

#[test]
fn moves_and_retirement_survive_snapshots_without_losing_old_operations() -> Result<(), String> {
    let mut held = Held::default();
    held.hold(added("op-a", "waffles"))?;
    let mut legacy = serde_json::to_value(&held).map_err(|error| error.to_string())?;
    legacy["seats"][0]
        .as_object_mut()
        .ok_or("no seat object")?
        .remove("retired");
    let legacy: Held = serde_json::from_value(legacy).map_err(|error| error.to_string())?;
    assert!(!legacy.seat("waffles").ok_or("no legacy seat")?.retired);
    held.hold(Line::Moved(Moved {
        operation: "op-b".to_owned(),
        name: "waffles".to_owned(),
        profile_version: 2,
        harness: "claude".to_owned(),
        by: "person".to_owned(),
        at: 2,
    }))?;
    assert_eq!(
        held.seat("waffles")
            .ok_or("no moved seat")?
            .added
            .profile_version,
        2
    );
    held.hold(Line::Retired(Retired {
        operation: "op-c".to_owned(),
        name: "waffles".to_owned(),
        by: "person".to_owned(),
        at: 3,
    }))?;
    let restored = Held::decode(&held.encode()?)?;
    assert!(restored.seat("waffles").ok_or("no retired seat")?.retired);
    assert_eq!(
        restored.seat("waffles").ok_or("no retired seat")?.revision,
        3
    );
    held.hold(added("op-d", "waffles"))?;
    assert!(!held.seat("waffles").ok_or("no new seat")?.retired);
    assert_eq!(held.operations.len(), 4);
    assert_eq!(held.seats.len(), 2);
    assert!(held.operation("op-c").is_some());
    Ok(())
}

#[test]
fn moving_or_retiring_a_running_seat_does_not_change_its_state() -> Result<(), String> {
    let mut held = Held::default();
    held.hold(added("op-a", "waffles"))?;
    held.hold(Line::Started(Started {
        qualification: None,
        transport: "managed".to_owned(),
        operation: "op-b".to_owned(),
        name: "waffles".to_owned(),
        session: "op-b".to_owned(),
        harness_session: None,
        by: "person".to_owned(),
        at: 2,
    }))?;
    let before = held.clone();
    assert!(
        held.hold(Line::Moved(Moved {
            operation: "op-c".to_owned(),
            name: "waffles".to_owned(),
            profile_version: 2,
            harness: "claude".to_owned(),
            by: "person".to_owned(),
            at: 3,
        }))
        .is_err()
    );
    assert_eq!(held, before);
    assert!(
        held.hold(Line::Retired(Retired {
            operation: "op-d".to_owned(),
            name: "waffles".to_owned(),
            by: "person".to_owned(),
            at: 3,
        }))
        .is_err()
    );
    assert_eq!(held, before);
    Ok(())
}
