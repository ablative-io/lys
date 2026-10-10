#![cfg(test)]
//! An import counts and classifies every Argus schedule once at its
//! `captured_at` (AGENTS-003 R6): finished first, then expired when `until`
//! is at or before `captured_at`, otherwise live; only live schedules for
//! the seat are imported, stopped ones stay stopped, a shared schedule is
//! one destination, and an unmapped recipient or an event trigger refuses.
//! The bodies are recorded Argus shapes; no Argus is asked.

use std::collections::BTreeMap;

use lys_identity_server::seat_import_plan::{Fragment, ScheduleCounts};
use lys_identity_server::seat_import_schedules::{
    RECIPIENT_UNMAPPED, SCHEDULE_UNREPRESENTABLE, Standing, classify, classify_for, standing,
};
use serde_json::{Value, json};

/// 2030-03-17T17:46:40Z.
const CAPTURED_AT: u64 = 1_900_000_000;
const AT_CAPTURE: &str = "2030-03-17T17:46:40Z";
const BEFORE: &str = "2030-03-17T17:46:39Z";
const AFTER: &str = "2030-03-17T17:46:41Z";
const LATER: &str = "2030-04-01T00:00:00Z";
const EARLIER: &str = "2030-01-01T00:00:00Z";

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(session, seat)| ((*session).to_owned(), (*seat).to_owned()))
        .collect()
}

/// A recorded Argus schedule row for `sessions`, in `state`, ending `until`.
fn schedule(id: &str, state: &str, until: Option<&str>, sessions: &[&str]) -> Value {
    let recipients: Vec<Value> = sessions
        .iter()
        .map(|session| {
            json!({
                "session_id": session,
                "target": {"transport": "herdr", "address": "w1:p1"},
                "last_sent": "2030-03-17T17:00:00Z",
            })
        })
        .collect();
    json!({
        "id": id,
        "revision": 7,
        "state": state,
        "text": "Please report your current progress.",
        "template": false,
        "use_prompt_profile": false,
        "delta": false,
        "events": [],
        "rows": [],
        "event_queue": [],
        "at": EARLIER,
        "until": until,
        "interval_seconds": 1800,
        "window": null,
        "starts_at": null,
        "max_occurrences": 40,
        "occurrences_completed": 12,
        "delivery_counts": {"submitted": 12, "skipped": 0, "failed": 0, "uncertain": 0},
        "recipients": recipients,
        "recipient_index": 0,
        "outcomes": [],
        "author": "Tom",
        "next_at": "2030-03-17T18:00:00Z",
        "last_outcome": {"status": "submitted"},
        "last_request_id": "sm-req-12",
        "last_finished_at": "2030-03-17T17:30:00Z",
    })
}

fn names(fragment: &Fragment) -> Vec<&str> {
    fragment
        .refusals
        .iter()
        .map(|refused| refused.name.as_str())
        .collect()
}

#[test]
fn seat_import_schedule_counts() {
    let schedules = [
        schedule("finished-open", "finished", Some(LATER), &["s-a"]),
        schedule("finished-past", "finished", Some(BEFORE), &["s-a"]),
        schedule("expired-before", "active", Some(BEFORE), &["s-a"]),
        schedule("expired-at-capture", "paused", Some(AT_CAPTURE), &["s-a"]),
        schedule("live-paused", "paused", Some(AFTER), &["s-a"]),
        schedule("live-failed", "failed", None, &["s-a"]),
        schedule("live-uncertain", "uncertain", Some(LATER), &["s-a"]),
    ];
    let fragment = classify(&schedules, CAPTURED_AT, &map(&[("s-a", "a")]), "a");
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    assert_eq!(
        fragment.schedule_counts,
        Some(ScheduleCounts {
            total: 7,
            live: 3,
            expired: 2,
            finished: 2,
            not_imported: 4,
        })
    );
    let excluded: Vec<&str> = fragment
        .excluded
        .iter()
        .map(|excluded| excluded.source_id.as_str())
        .collect();
    assert_eq!(
        excluded,
        [
            "monitor:schedule:finished-open",
            "monitor:schedule:finished-past",
            "monitor:schedule:expired-before",
            "monitor:schedule:expired-at-capture",
        ]
    );
    assert!(fragment.excluded.iter().all(|excluded| excluded.revision == "7"));
    assert_eq!(fragment.destinations.len(), 3);
    for entry in &fragment.destinations {
        assert_eq!(entry.record_kind, "schedule");
        assert_eq!(entry.change["state"], "stopped", "{entry:?}");
        assert_eq!(entry.change["occurrences_completed"], 12);
        assert_eq!(entry.change["spent"]["last_request_id"], "sm-req-12");
        assert_eq!(entry.change["bindings"][0]["enabled"], false);
    }
    let reasons: Vec<&Value> = fragment
        .destinations
        .iter()
        .map(|entry| &entry.change["stopped_reason"])
        .collect();
    assert_eq!(reasons, [&json!("paused"), &json!("failed"), &json!("uncertain")]);
}

#[test]
fn until_is_exclusive_at_exactly_captured_at() -> Result<(), String> {
    let at = |until: &str| standing(&schedule("s", "active", Some(until), &["s-a"]), CAPTURED_AT);
    assert_eq!(at(AT_CAPTURE)?, Standing::Expired);
    assert_eq!(at("2030-03-18T03:46:40+10:00")?, Standing::Expired);
    assert_eq!(at(BEFORE)?, Standing::Expired);
    assert_eq!(at("2030-03-17T17:46:40.000000001Z")?, Standing::Live);
    assert_eq!(at(AFTER)?, Standing::Live);
    let open = schedule("s", "active", None, &["s-a"]);
    assert_eq!(standing(&open, CAPTURED_AT)?, Standing::Live);
    assert!(standing(&json!({"state": "active", "until": 5}), CAPTURED_AT).is_err());
    assert!(standing(&json!({"state": "active", "until": "tomorrow"}), CAPTURED_AT).is_err());
    Ok(())
}

#[test]
fn finished_takes_precedence_over_until() -> Result<(), String> {
    for until in [Some(LATER), Some(AT_CAPTURE), Some(BEFORE), None] {
        let finished = schedule("s", "finished", until, &["s-a"]);
        assert_eq!(standing(&finished, CAPTURED_AT)?, Standing::Finished, "{until:?}");
    }
    let fragment = classify(
        &[schedule("done", "finished", Some(BEFORE), &["s-a"])],
        CAPTURED_AT,
        &map(&[("s-a", "a")]),
        "a",
    );
    let counts = fragment.schedule_counts.ok_or("no counts")?;
    assert_eq!((counts.finished, counts.expired), (1, 0));
    assert!(fragment.excluded[0].reason.contains("finished"));
    Ok(())
}

#[test]
fn an_unmapped_recipient_refuses_the_schedule() {
    let shared = schedule("shared", "active", Some(LATER), &["s-a", "s-stranger"]);
    let fragment = classify(&[shared], CAPTURED_AT, &map(&[("s-a", "a")]), "a");
    assert_eq!(names(&fragment), [RECIPIENT_UNMAPPED]);
    assert_eq!(fragment.refusals[0].member, "schedule shared.recipients[1]");
    assert!(fragment.refusals[0].detail.contains("s-stranger"));
    assert!(fragment.destinations.is_empty());
}

#[test]
fn seat_import_shared_schedule() {
    let shared = schedule("shared", "active", Some(LATER), &["s-a", "s-b"]);
    let recipients = map(&[("s-a", "a"), ("s-b", "b")]);
    let first = classify(std::slice::from_ref(&shared), CAPTURED_AT, &recipients, "a");
    let second = classify(std::slice::from_ref(&shared), CAPTURED_AT, &recipients, "b");
    for (fragment, seat) in [(&first, "a"), (&second, "b")] {
        assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
        let [entry] = fragment.destinations.as_slice() else {
            panic!("one destination for seat {seat}: {:?}", fragment.destinations);
        };
        assert_eq!(entry.record_id, "schedule:shared");
        assert_eq!(
            entry.change["schedule"]["recipients"],
            json!([{"kind": "agent", "id": seat}]),
            "the schedule delivers to this seat alone until the other binding is confirmed"
        );
        assert_eq!(entry.change["bindings"].as_array().map(Vec::len), Some(2));
        assert_eq!(entry.change["next_due"], 1_900_000_800);
        assert_eq!(entry.change["occurrences_completed"], 12);
        let bindings = entry.change["bindings"].as_array().cloned().unwrap_or_default();
        let this: Vec<&Value> = bindings
            .iter()
            .filter(|binding| binding["this_import"] == true)
            .map(|binding| &binding["seat"])
            .collect();
        assert_eq!(this, [&json!(seat)]);
        assert!(bindings.iter().all(|binding| binding["enabled"] == false));
        assert_eq!(fragment.prerequisites.len(), 2);
    }
    let definition = |fragment: &Fragment| {
        let mut schedule = fragment.destinations[0].change["schedule"].clone();
        schedule["recipients"] = Value::Null;
        schedule
    };
    assert_eq!(definition(&first), definition(&second), "one definition for both seats");
}

#[test]
fn an_event_triggered_schedule_is_unrepresentable() {
    let mut triggered = schedule("after-compaction", "active", Some(LATER), &["s-a"]);
    triggered["events"] = json!(["compaction"]);
    triggered["at"] = Value::Null;
    triggered["interval_seconds"] = Value::Null;
    let mut windowed = schedule("night", "active", Some(LATER), &["s-a"]);
    windowed["window"] = json!({"timezone": "Etc/UTC", "start": "02:00", "end": "06:00", "weekdays": [1]});
    let fragment = classify(&[triggered, windowed], CAPTURED_AT, &map(&[("s-a", "a")]), "a");
    assert_eq!(names(&fragment), [SCHEDULE_UNREPRESENTABLE, SCHEDULE_UNREPRESENTABLE]);
    assert_eq!(fragment.refusals[0].member, "schedule after-compaction.events");
    assert_eq!(fragment.refusals[1].member, "schedule night.window");
    assert!(fragment.destinations.is_empty());
}

#[test]
fn a_live_schedule_for_another_seat_is_counted_not_imported() {
    let other = schedule("theirs", "active", Some(LATER), &["s-b"]);
    let fragment = classify(&[other], CAPTURED_AT, &map(&[("s-a", "a")]), "a");
    assert!(fragment.refusals.is_empty());
    assert!(fragment.destinations.is_empty());
    let counts = fragment.schedule_counts.expect("counts");
    assert_eq!((counts.total, counts.live, counts.not_imported), (1, 1, 1));
    assert!(fragment.excluded[0].reason.contains("no recipient maps to seat a"));
}

#[test]
fn the_seat_binding_delivers_to_the_seats_lys_agent() {
    let mine = schedule("mine", "active", Some(LATER), &["s-w"]);
    let fragment = classify_for(&[mine], CAPTURED_AT, &map(&[("s-w", "waffles")]), "waffles", "agent-7f");
    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    let change = &fragment.destinations[0].change;
    assert_eq!(change["schedule"]["recipients"], json!([{"kind": "agent", "id": "agent-7f"}]));
    assert_eq!(change["bindings"][0]["seat"], "waffles");
    assert_eq!(change["state"], "active");
}
