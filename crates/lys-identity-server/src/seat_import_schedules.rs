//! The monitor's scheduled messages of one seat, classified once at the plan's
//! `captured_at` instant (AGENTS-003 R6): finished first, otherwise expired
//! when `until` is at or before `captured_at` (`until` is exclusive),
//! otherwise live. Every schedule is counted; only live ones relevant to
//! this seat are imported, into AGENTS-001's schedules, and every other one
//! is kept as not imported with its id, revision and reason.
//!
//! Paused, failed and uncertain schedules are imported stopped and need a
//! separate deliberate resume. Absolute dates, the completed count, the next
//! due instant and the last delivery's provenance are kept, so a spent
//! occurrence is never sent again. Each legacy recipient maps explicitly to
//! a Lys seat through the manifest, or the plan is refused; a shared
//! schedule is one destination keyed by its source id, each seat's binding
//! confirmed and enabled separately. Event, goal-row and windowed schedules
//! cannot be held by AGENTS-001 and are refused by name, never flattened.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::schedules_state::{INTERVAL_MAX, INTERVAL_MIN, Recipient, Schedule, Source};
use crate::seat_import_monitor::{
    BOUND_EXCEEDED, CREDENTIAL_INLINE, MEMBER_UNSUPPORTED, empty, reached, refusal, secret_shaped,
};
use crate::seat_import_monitor_maps::words_text;
use crate::seat_import_plan::{
    Completeness, DestinationEntry, Excluded, Fragment, Refusal, ScheduleCounts, SourceEntry,
};
use crate::words_state::Slot;

/// A legacy recipient the manifest maps to no Lys seat.
pub const RECIPIENT_UNMAPPED: &str = "import_recipient_unmapped";
/// A schedule AGENTS-001 cannot hold as it is.
pub const SCHEDULE_UNREPRESENTABLE: &str = "import_schedule_unrepresentable";
/// The most live schedules one seat's plan imports.
pub const LIVE_BOUND: usize = 256;
/// The most recipients one schedule names.
pub const RECIPIENT_BOUND: usize = 32;

const NANOS: i128 = 1_000_000_000;

/// How a schedule stands at `captured_at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// The monitor says it has finished.
    Finished,
    /// Its `until` is at or before `captured_at`.
    Expired,
    /// Neither.
    Live,
}

/// The instant `text` names, ISO 8601 with an offset, in nanoseconds since
/// the Unix epoch.
pub fn instant(text: &str) -> Result<i128, String> {
    text.parse::<jiff::Timestamp>()
        .map(jiff::Timestamp::as_nanosecond)
        .map_err(|error| format!("`{text}` is not an instant with an offset: {error}"))
}

/// How `schedule` stands at `captured_at`, decided once: finished first,
/// then expired when its `until` is at or before `captured_at`.
pub fn standing(schedule: &Value, captured_at: u64) -> Result<Standing, String> {
    if schedule.get("state").and_then(Value::as_str) == Some("finished") {
        return Ok(Standing::Finished);
    }
    match schedule.get("until") {
        None | Some(Value::Null) => Ok(Standing::Live),
        Some(Value::String(until)) => {
            if reached(instant(until)?, captured_at) {
                Ok(Standing::Expired)
            } else {
                Ok(Standing::Live)
            }
        }
        Some(other) => Err(format!("until is {other}, not an instant")),
    }
}

/// Count and classify every schedule in `schedules` at `captured_at`, and
/// map the live ones whose recipients `recipient_map` sends to `seat`, for
/// a seat whose Lys agent is named as the seat is.
pub fn classify(
    schedules: &[Value],
    captured_at: u64,
    recipient_map: &BTreeMap<String, String>,
    seat: &str,
) -> Fragment {
    classify_for(schedules, captured_at, recipient_map, seat, seat)
}

/// Count and classify every schedule in `schedules` at `captured_at`, and
/// map the live ones whose recipients `recipient_map` sends to `seat`, the
/// seat's own binding delivering to the Lys agent `agent`.
pub fn classify_for(
    schedules: &[Value],
    captured_at: u64,
    recipient_map: &BTreeMap<String, String>,
    seat: &str,
    agent: &str,
) -> Fragment {
    let mut fragment = empty();
    let mut counts = ScheduleCounts {
        total: 0,
        live: 0,
        expired: 0,
        finished: 0,
        not_imported: 0,
    };
    let mut imported = 0;
    for (index, schedule) in schedules.iter().enumerate() {
        let id = schedule.get("id").and_then(Value::as_str);
        let revision = schedule.get("revision").and_then(Value::as_u64);
        let (Some(id), Some(revision)) = (id, revision) else {
            let member = format!("schedules[{index}]");
            let detail = "a schedule needs a text id and a whole revision";
            fragment
                .refusals
                .push(refusal(MEMBER_UNSUPPORTED, &member, detail));
            continue;
        };
        counts.total += 1;
        let source_id = format!("monitor:schedule:{id}");
        fragment.sources.push(SourceEntry {
            id: source_id.clone(),
            kind: "monitor_schedule".to_owned(),
            locator: format!("/api/scheduled-messages#{id}"),
            scope: id.to_owned(),
            revision_kind: "native".to_owned(),
            source_revision: revision.to_string(),
            completeness: Completeness::Complete,
        });
        let reason = match standing(schedule, captured_at) {
            Err(reason) => {
                let member = format!("schedule {id}.until");
                fragment
                    .refusals
                    .push(refusal(MEMBER_UNSUPPORTED, &member, reason));
                continue;
            }
            Ok(Standing::Finished) => {
                counts.finished += 1;
                "finished in the monitor at captured_at".to_owned()
            }
            Ok(Standing::Expired) => {
                counts.expired += 1;
                format!(
                    "expired: until {} is at or before captured_at {captured_at}, and until is exclusive",
                    schedule.get("until").unwrap_or(&Value::Null)
                )
            }
            Ok(Standing::Live) => {
                counts.live += 1;
                let Some(recipients) = schedule.get("recipients").and_then(Value::as_array) else {
                    let member = format!("schedule {id}.recipients");
                    let refused = refusal(MEMBER_UNSUPPORTED, &member, "not a list of recipients");
                    fragment.refusals.push(refused);
                    continue;
                };
                let relevant = recipients.iter().any(|recipient| {
                    recipient
                        .get("session_id")
                        .and_then(Value::as_str)
                        .and_then(|session| recipient_map.get(session))
                        .is_some_and(|mapped| mapped == seat)
                });
                if relevant {
                    let bound = (recipient_map, seat, agent);
                    match live(schedule, id, recipients, captured_at, bound) {
                        Ok((mut entry, prerequisites)) => {
                            entry.source_entry_ids.push(source_id);
                            fragment.destinations.push(entry);
                            fragment.prerequisites.extend(prerequisites);
                            imported += 1;
                        }
                        Err(refused) => fragment.refusals.extend(refused),
                    }
                    continue;
                }
                format!("live, but no recipient maps to seat {seat}")
            }
        };
        counts.not_imported += 1;
        fragment.excluded.push(Excluded {
            source_id,
            revision: revision.to_string(),
            reason,
        });
    }
    if imported > LIVE_BOUND {
        let detail = format!("{imported} live schedules; one seat imports at most {LIVE_BOUND}");
        let refused = refusal(BOUND_EXCEEDED, "schedules", detail);
        fragment.refusals.push(refused);
    }
    fragment.schedule_counts = Some(counts);
    fragment
}

/// The seconds of the instant at `field`, none when it is absent or null.
fn seconds(schedule: &Value, field: &str, id: &str) -> Result<Option<u64>, Refusal> {
    let member = format!("schedule {id}.{field}");
    let nanos = match schedule.get(field) {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(text)) => instant(text),
        Some(other) => Err(format!("{other} is not an instant")),
    }
    .map_err(|reason| refusal(MEMBER_UNSUPPORTED, &member, reason))?;
    u64::try_from(nanos.div_euclid(NANOS))
        .map(Some)
        .map_err(|error| refusal(MEMBER_UNSUPPORTED, &member, format!("before 1970: {error}")))
}

/// A whole number at `field`, none when it is absent or null.
fn whole(schedule: &Value, field: &str, id: &str) -> Result<Option<u64>, Refusal> {
    match schedule.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            let member = format!("schedule {id}.{field}");
            refusal(
                MEMBER_UNSUPPORTED,
                &member,
                format!("{value} is not a whole number"),
            )
        }),
    }
}

/// One live schedule relevant to `seat`, as its AGENTS-001 destination
/// delivering to the seat's Lys `agent` alone, with every seat's binding and
/// the prerequisites they carry; every reason it cannot be, by name.
fn live(
    schedule: &Value,
    id: &str,
    recipients: &[Value],
    captured_at: u64,
    (recipient_map, seat, agent): (&BTreeMap<String, String>, &str, &str),
) -> Result<(DestinationEntry, Vec<String>), Vec<Refusal>> {
    let member = |field: &str| format!("schedule {id}.{field}");
    let mut refused = Vec::new();
    if let Some(path) = secret_shaped(&member("definition"), schedule) {
        let detail = "a secret-shaped member; its value is not shown";
        refused.push(refusal(CREDENTIAL_INLINE, &path, detail));
    }
    if recipients.len() > RECIPIENT_BOUND {
        let detail = format!("{} recipients; at most {RECIPIENT_BOUND}", recipients.len());
        refused.push(refusal(BOUND_EXCEEDED, &member("recipients"), detail));
    }
    for field in ["events", "rows"] {
        if schedule
            .get(field)
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
        {
            let detail = "AGENTS-001 has no event- or goal-row-triggered schedule, and it is not flattened to an interval";
            refused.push(refusal(SCHEDULE_UNREPRESENTABLE, &member(field), detail));
        }
    }
    for field in ["window", "starts_at"] {
        if schedule.get(field).is_some_and(|value| !value.is_null()) {
            let detail = "AGENTS-001 schedules have no delivery window, so it cannot be kept";
            refused.push(refusal(SCHEDULE_UNREPRESENTABLE, &member(field), detail));
        }
    }
    let mut bound_seats = BTreeSet::new();
    let mut bindings = Vec::new();
    for (index, recipient) in recipients.iter().enumerate() {
        let session = recipient.get("session_id").and_then(Value::as_str);
        let Some((session, mapped)) =
            session.and_then(|session| Some((session, recipient_map.get(session)?)))
        else {
            let who = session.map_or_else(
                || "a recipient with no session_id".to_owned(),
                |session| format!("session {session}"),
            );
            let detail = format!("{who} maps to no Lys seat in the manifest");
            let at = member(&format!("recipients[{index}]"));
            refused.push(refusal(RECIPIENT_UNMAPPED, &at, detail));
            continue;
        };
        bound_seats.insert(mapped.clone());
        bindings.push(json!({
            "source_session": session,
            "seat": mapped,
            "this_import": mapped == seat,
            "enabled": false,
            "last_sent": recipient.get("last_sent"),
        }));
    }
    if !refused.is_empty() {
        return Err(refused);
    }
    let read = (
        seconds(schedule, "at", id),
        seconds(schedule, "until", id),
        seconds(schedule, "next_at", id),
        whole(schedule, "interval_seconds", id),
        whole(schedule, "max_occurrences", id),
        whole(schedule, "occurrences_completed", id),
    );
    let (at, until, next_due, interval, max_occurrences, completed) = match read {
        (Ok(at), Ok(until), Ok(next), Ok(interval), Ok(max), Ok(completed)) => {
            (at, until, next, interval, max, completed)
        }
        (at, until, next, interval, max, completed) => {
            let errors = [
                at.err(),
                until.err(),
                next.err(),
                interval.err(),
                max.err(),
                completed.err(),
            ];
            refused.extend(errors.into_iter().flatten());
            return Err(refused);
        }
    };
    let Some(at) = at else {
        let detail = "a clock schedule names its first instant";
        refused.push(refusal(MEMBER_UNSUPPORTED, &member("at"), detail));
        return Err(refused);
    };
    let Some(completed) = completed else {
        let detail = "absent, so the occurrences already spent cannot be known";
        let at = member("occurrences_completed");
        refused.push(refusal(MEMBER_UNSUPPORTED, &at, detail));
        return Err(refused);
    };
    if interval.is_some_and(|interval| !(INTERVAL_MIN..=INTERVAL_MAX).contains(&interval)) {
        let detail = format!("Lys keeps intervals of {INTERVAL_MIN} to {INTERVAL_MAX} seconds");
        let at = member("interval_seconds");
        refused.push(refusal(MEMBER_UNSUPPORTED, &at, detail));
    }
    let stopped = match schedule.get("state").and_then(Value::as_str) {
        Some("active") => None,
        Some("paused") => Some("paused"),
        Some("failed") => Some("failed"),
        Some("uncertain" | "sending") => Some("uncertain"),
        state => {
            let detail = format!("state {state:?} is not one the monitor documents");
            refused.push(refusal(MEMBER_UNSUPPORTED, &member("state"), detail));
            None
        }
    };
    let author = schedule.get("author").and_then(Value::as_str);
    let text = schedule.get("text").and_then(Value::as_str);
    let source = if schedule.get("use_prompt_profile") == Some(&Value::Bool(true)) {
        Ok(Source::Slot {
            slot: Slot::ScheduledReminder,
        })
    } else {
        match text {
            Some(text) => words_text(&member("text"), text).map(|text| Source::Text { text }),
            None => Err(refusal(MEMBER_UNSUPPORTED, &member("text"), "absent")),
        }
    };
    let source = match source {
        Ok(source) => Some(source),
        Err(refusal_of_text) => {
            refused.push(refusal_of_text);
            None
        }
    };
    if author.is_none() {
        refused.push(refusal(MEMBER_UNSUPPORTED, &member("author"), "absent"));
    }
    let (Some(author), Some(source)) = (author, source) else {
        return Err(refused);
    };
    if !refused.is_empty() {
        return Err(refused);
    }
    let lys = Schedule {
        id: id.to_owned(),
        at,
        until,
        interval,
        max_occurrences,
        recipients: vec![Recipient::Agent {
            id: agent.to_owned(),
        }],
        source,
        author: author.to_owned(),
        set_at: captured_at,
    };
    let prerequisites = bound_seats
        .iter()
        .map(|bound| {
            if bound == seat {
                format!("schedule {id}: seat {bound}'s binding is enabled only after this seat's move proof (AGENTS-002 R5)")
            } else {
                format!("schedule {id}: seat {bound}'s binding is confirmed by that seat's own import and enabled only after its move proof")
            }
        })
        .collect();
    let entry = DestinationEntry {
        record_kind: "schedule".to_owned(),
        record_id: format!("schedule:{id}"),
        expected_revision: None,
        change: json!({
            "schedule": lys,
            "state": if stopped.is_some() { "stopped" } else { "active" },
            "stopped_reason": stopped,
            "resume": stopped.map(|_| "a separate deliberate resume that accounts for the original outcome"),
            "next_due": next_due,
            "occurrences_completed": completed,
            "template": schedule.get("template"),
            "spent": {
                "last_finished_at": schedule.get("last_finished_at"),
                "last_request_id": schedule.get("last_request_id"),
                "last_outcome": schedule.get("last_outcome"),
                "delivery_counts": schedule.get("delivery_counts"),
            },
            "bindings": bindings,
            "source_revision": schedule.get("revision"),
        }),
        source_entry_ids: Vec::new(),
    };
    Ok((entry, prerequisites))
}
