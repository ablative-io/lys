#![cfg(test)]
//! A confirmed terminal stop retains only a verified native conversation for that session.

use super::{EndRecord, end_record};
use crate::seats_state::{Added, Held, Line, Seat, Started};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SESSION: &str = "op-0123456789abcdef0123456789abcdef";
const STOP: &str = "op-fedcba9876543210fedcba9876543210";
const NATIVE: &str = "019bcd60-1a27-7000-8000-123456789abc";
const OTHER: &str = "019bcd60-1a27-7000-8000-abcdef123456";

fn seat(native: Option<&str>, transport: &str) -> Seat {
    Seat {
        qualification: Some("0.161.0-alpha.3".to_owned()),
        transport: Some(transport.to_owned()),
        added: Added {
            operation: "op-11111111111111111111111111111111".to_owned(),
            name: "terminal".to_owned(),
            agent: "agent".to_owned(),
            harness: "codex".to_owned(),
            profile_version: 1,
            machine: "machine".to_owned(),
            working_folder: Some("/project".to_owned()),
            account: None,
            by: "person".to_owned(),
            at: 1,
        },
        session: Some(SESSION.to_owned()),
        harness_session: native.map(str::to_owned),
        running: true,
        revision: 2,
        retired: false,
    }
}

fn fold(held: &Seat, record: &EndRecord) -> Result<Held, Box<dyn std::error::Error>> {
    let mut state = Held::default();
    state.hold(Line::Added(held.added.clone()))?;
    state.hold(Line::Started(Started {
        qualification: held.qualification.clone(),
        transport: held.transport.clone().ok_or("transport absent")?,
        operation: SESSION.to_owned(),
        name: held.added.name.clone(),
        session: SESSION.to_owned(),
        harness_session: held.harness_session.clone(),
        by: "starter".to_owned(),
        at: 2,
    }))?;
    for line in &record.lines {
        state.hold(line.clone())?;
    }
    Ok(state)
}

#[test]
fn a_late_native_uuid_restates_the_same_lys_session_and_stops_it_in_one_ordered_batch() -> TestResult
{
    let held = seat(None, "pty");
    let record = end_record(&held, Some(NATIVE), ("stopper", STOP, true, 10))?;
    assert!(record.refusal.is_none());
    let [Line::Started(started), Line::Stopped(stopped)] = record.lines.as_slice() else {
        return Err("expected one native-identity restatement followed by the real stop".into());
    };
    assert_eq!(started.session, SESSION);
    assert_eq!(started.harness_session.as_deref(), Some(NATIVE));
    assert_eq!(started.qualification, held.qualification);
    assert_eq!(started.transport, "pty");
    assert_eq!(
        started.operation,
        super::super::part(STOP, "native-conversation")
    );
    assert_ne!(started.operation, stopped.operation);
    assert_eq!(started.by, "stopper");
    assert_eq!(started.at, 10);
    assert_eq!(stopped.operation, STOP);
    assert_eq!(stopped.session, SESSION);
    assert!(stopped.forced);
    assert_eq!(stopped.by, "stopper");
    assert_eq!(stopped.at, 10);
    let folded = fold(&held, &record)?;
    let ended = folded.seat("terminal").ok_or("seat absent")?;
    assert_eq!(ended.session.as_deref(), Some(SESSION));
    assert_eq!(ended.harness_session.as_deref(), Some(NATIVE));
    assert!(!ended.running);
    assert_eq!(ended.revision, 4);
    assert!(
        matches!(folded.operation(SESSION), Some(Line::Started(started)) if started.harness_session.is_none())
    );
    Ok(())
}

#[test]
fn a_known_native_uuid_is_retained_when_the_ended_runner_reports_it_or_reports_nothing()
-> TestResult {
    for observed in [Some(NATIVE), None] {
        let held = seat(Some(NATIVE), "pty");
        let record = end_record(&held, observed, ("stopper", STOP, false, 10))?;
        assert!(record.refusal.is_none());
        assert!(matches!(record.lines.as_slice(), [Line::Stopped(_)]));
        let folded = fold(&held, &record)?;
        let ended = folded.seat("terminal").ok_or("seat absent")?;
        assert!(!ended.running);
        assert_eq!(ended.harness_session.as_deref(), Some(NATIVE));
    }
    Ok(())
}

#[test]
fn a_differing_native_uuid_refuses_capture_but_keeps_the_real_stop_and_prior_identity() -> TestResult
{
    let held = seat(Some(NATIVE), "pty");
    let record = end_record(&held, Some(OTHER), ("stopper", STOP, false, 10))?;
    assert_eq!(
        record.refusal.as_ref().map(|error| error.name()),
        Some("pty_conversation_mismatch")
    );
    assert!(matches!(record.lines.as_slice(), [Line::Stopped(_)]));
    let folded = fold(&held, &record)?;
    let ended = folded.seat("terminal").ok_or("seat absent")?;
    assert!(!ended.running);
    assert_eq!(ended.session.as_deref(), Some(SESSION));
    assert_eq!(ended.harness_session.as_deref(), Some(NATIVE));
    assert!(matches!(folded.operation(STOP), Some(Line::Stopped(_))));
    Ok(())
}

#[test]
fn an_unknown_native_uuid_remains_unknown_after_stop_without_a_session_derived_substitute()
-> TestResult {
    let held = seat(None, "pty");
    let record = end_record(&held, None, ("stopper", STOP, false, 10))?;
    assert!(record.refusal.is_none());
    assert!(matches!(record.lines.as_slice(), [Line::Stopped(_)]));
    let folded = fold(&held, &record)?;
    let ended = folded.seat("terminal").ok_or("seat absent")?;
    assert!(!ended.running);
    assert_eq!(ended.harness_session, None);
    assert_eq!(ended.session.as_deref(), Some(SESSION));
    Ok(())
}

#[test]
fn an_invalid_terminal_native_id_refuses_capture_without_losing_the_confirmed_stop() -> TestResult {
    for (known, observed) in [(None, Some("native")), (Some("legacy-invalid"), None)] {
        let held = seat(known, "pty");
        let record = end_record(&held, observed, ("stopper", STOP, false, 10))?;
        assert_eq!(
            record.refusal.as_ref().map(|error| error.name()),
            Some("control_binding_invalid")
        );
        assert!(matches!(record.lines.as_slice(), [Line::Stopped(_)]));
        let folded = fold(&held, &record)?;
        let ended = folded.seat("terminal").ok_or("seat absent")?;
        assert!(!ended.running);
        assert_eq!(ended.harness_session.as_deref(), known);
    }
    Ok(())
}

#[test]
fn managed_stops_preserve_their_prior_identity_without_terminal_observation_or_validation()
-> TestResult {
    for known in [None, Some("legacy-managed-id")] {
        let held = seat(known, "managed");
        let record = end_record(&held, Some(OTHER), ("stopper", STOP, false, 10))?;
        assert!(record.refusal.is_none());
        assert!(matches!(record.lines.as_slice(), [Line::Stopped(_)]));
        let folded = fold(&held, &record)?;
        let ended = folded.seat("terminal").ok_or("seat absent")?;
        assert!(!ended.running);
        assert_eq!(ended.harness_session.as_deref(), known);
        assert_eq!(ended.transport.as_deref(), Some("managed"));
    }
    Ok(())
}
