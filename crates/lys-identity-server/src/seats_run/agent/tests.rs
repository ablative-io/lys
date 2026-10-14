#![cfg(test)]

use lys_runner::harness_control::Transport;
use lys_runner::tracking::Harness;
use serde_json::{Value, json};

use super::{native_conversation, require_ended, select_native, transition};
use crate::provisioning_store::Version;
use crate::seats_state::{Added, Held, Line, Seat, Started};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const AGENT: &str = "agent";
const MACHINE: &str = "machine";
const PRIOR_OPERATION: &str = "op-0123456789abcdef0123456789abcdef";
const OPERATION: &str = "op-fedcba9876543210fedcba9876543210";
const UUID: &str = "fedcba98-7654-3210-fedc-ba9876543210";
const PRIOR_UUID: &str = "01234567-89ab-cdef-0123-456789abcdef";
const FOLDER: &str = "/var/declared-link/project";

fn profile(number: u32, name: &str, harness: Option<Harness>, managed: bool) -> Result<Version> {
    let mut declared = crate::harness_description::declared();
    declared["name"] = json!(name);
    if let Some(Harness::Codex) = harness {
        let catalogue: Value = serde_json::from_str(include_str!(
            "../../../../../docs/harness/catalogue/codex.json"
        ))?;
        declared["description"] = catalogue["description"].clone();
        declared["program"] = json!("codex");
    }
    if harness.is_none() {
        declared = Value::Null;
    }
    Ok(serde_json::from_value(json!({
        "number": number,
        "operation": "profile",
        "settings": {
            "model_access": ["fixture-model"], "tools": [], "skills": [], "mcp_servers": [],
            "instructions": "", "note": "", "harness": declared,
            "session": {"stall_after": 300, "requires_controls": managed}
        },
        "set_by": "person", "set_at": 1,
        "reviewed": {"operation": "review", "by": "person", "at": 2}
    }))?)
}

fn seat(name: &str, session: Option<&str>, running: bool) -> Seat {
    Seat {
        qualification: None,
        transport: session.map(|_| "pty".to_owned()),
        added: Added {
            operation: format!("add-{name}"),
            name: name.to_owned(),
            agent: AGENT.to_owned(),
            harness: "Original operator name".to_owned(),
            profile_version: 1,
            machine: MACHINE.to_owned(),
            working_folder: Some("/older/profile/folder".to_owned()),
            account: None,
            by: "person".to_owned(),
            at: 1,
        },
        session: session.map(str::to_owned),
        harness_session: session.map(|_| PRIOR_UUID.to_owned()),
        running,
        revision: 2,
        retired: false,
    }
}

fn view(harness: &str, version: u32) -> Value {
    json!({
        "agent": AGENT, "machine": MACHINE, "session": OPERATION,
        "harness": harness, "provisioning_version": version, "directory": FOLDER,
        "native_conversation": UUID,
        "runner": {"state": "running", "qualification": "unqualified native version"}
    })
}

#[test]
fn managed_only_seats_leave_a_generic_agent_start_on_its_existing_path() -> Result {
    let first = seat("first", None, false);
    let second = seat("second", None, false);
    let managed = profile(1, "Managed", Some(Harness::ClaudeCode), true)?;
    assert!(select_native([(&first, &managed), (&second, &managed)], AGENT, MACHINE)?.is_none());
    Ok(())
}

#[test]
fn one_native_seat_is_selected_before_counting_other_managed_seats() -> Result {
    let terminal = seat("terminal", None, false);
    let first = seat("first-managed", None, false);
    let second = seat("second-managed", None, false);
    let native = profile(1, "Native", Some(Harness::ClaudeCode), false)?;
    let managed = profile(1, "Managed", Some(Harness::ClaudeCode), true)?;
    assert_eq!(
        select_native(
            [
                (&first, &managed),
                (&terminal, &native),
                (&second, &managed)
            ],
            AGENT,
            MACHINE,
        )?,
        Some(&terminal)
    );
    Ok(())
}

#[test]
fn two_native_matching_seats_refuse_by_name_after_managed_seats_are_removed() -> Result {
    let first = seat("first", None, false);
    let second = seat("second", None, false);
    let claude = profile(1, "Claude", Some(Harness::ClaudeCode), false)?;
    let codex = profile(1, "Codex", Some(Harness::Codex), false)?;
    let error = select_native([(&first, &claude), (&second, &codex)], AGENT, MACHINE)
        .err()
        .ok_or("two native seats were silently selected")?;
    assert_eq!(error.name(), "pty_seat_ambiguous");
    Ok(())
}

#[test]
fn retired_unrelated_and_undeclared_native_seats_never_change_generic_selection() -> Result {
    let mut retired = seat("retired", None, false);
    retired.retired = true;
    let mut other_agent = seat("other-agent", None, false);
    other_agent.added.agent = "different-agent".to_owned();
    let mut other_machine = seat("other-machine", None, false);
    other_machine.added.machine = "different-machine".to_owned();
    let undeclared = seat("undeclared", None, false);
    let native = profile(1, "Native", Some(Harness::ClaudeCode), false)?;
    let missing = profile(1, "Undeclared", None, false)?;
    assert!(
        select_native(
            [
                (&retired, &native),
                (&other_agent, &native),
                (&other_machine, &native),
                (&undeclared, &missing),
            ],
            AGENT,
            MACHINE,
        )?
        .is_none()
    );
    Ok(())
}

#[test]
fn generic_start_again_stops_moves_and_starts_the_exact_reviewed_profile_in_order() -> Result {
    let mut held = Held::default();
    let original = seat("terminal", Some(PRIOR_OPERATION), true);
    held.hold(Line::Added(original.added.clone()))?;
    held.hold(Line::Started(Started {
        qualification: None,
        transport: "pty".to_owned(),
        operation: PRIOR_OPERATION.to_owned(),
        name: original.added.name.clone(),
        session: PRIOR_OPERATION.to_owned(),
        harness_session: Some(PRIOR_UUID.to_owned()),
        by: "person".to_owned(),
        at: 2,
    }))?;
    let version = profile(2, "Exact new operator name", Some(Harness::Codex), false)?;
    let current = held.seat("terminal").ok_or("seat missing")?;
    let launch = view("codex", 2);
    let lines = transition(
        current,
        &version,
        &launch,
        Some(UUID.to_owned()),
        "person",
        3,
    )?;
    let [
        Line::Stopped(stopped),
        Line::Moved(moved),
        Line::Started(started),
    ] = lines.as_slice()
    else {
        return Err("the new profile was not selected between stop and start".into());
    };
    assert_eq!(stopped.session, PRIOR_OPERATION);
    assert_eq!(
        stopped.operation,
        super::super::part(OPERATION, "terminal-stop")
    );
    assert_eq!(moved.profile_version, 2);
    assert_eq!(moved.harness, "Exact new operator name");
    assert_eq!(
        moved.operation,
        super::super::part(OPERATION, "terminal-move")
    );
    assert_ne!(stopped.operation, moved.operation);
    assert_ne!(moved.operation, started.operation);
    assert_eq!(started.operation, OPERATION);
    assert_eq!(started.harness_session.as_deref(), Some(UUID));
    assert_eq!(
        started.qualification.as_deref(),
        Some("unqualified native version")
    );
    assert_eq!(
        lines,
        transition(
            current,
            &version,
            &launch,
            Some(UUID.to_owned()),
            "person",
            3
        )?
    );
    for line in lines {
        held.hold(line)?;
    }
    let current = held.seat("terminal").ok_or("updated seat missing")?;
    assert!(current.running);
    assert_eq!(current.added.profile_version, 2);
    assert_eq!(current.added.harness, "Exact new operator name");
    assert_eq!(current.session.as_deref(), Some(OPERATION));
    assert_eq!(current.harness_session.as_deref(), Some(UUID));
    assert_eq!(current.revision, 5);
    assert_eq!(
        current.added.working_folder.as_deref(),
        Some("/older/profile/folder")
    );
    assert_eq!(launch["directory"], FOLDER);
    Ok(())
}

#[test]
fn a_stopped_seat_moves_without_inventing_another_stop_or_canonical_harness_name() -> Result {
    let held = seat("terminal", Some(PRIOR_OPERATION), false);
    let version = profile(
        2,
        "Operator chose this name",
        Some(Harness::ClaudeCode),
        false,
    )?;
    let lines = transition(
        &held,
        &version,
        &view("claude-code", 2),
        Some(UUID.to_owned()),
        "person",
        3,
    )?;
    let [Line::Moved(moved), Line::Started(_)] = lines.as_slice() else {
        return Err("a stopped seat acquired an invented stop".into());
    };
    assert_eq!(moved.harness, "Operator chose this name");
    Ok(())
}

#[test]
fn an_unchanged_exact_profile_starts_without_moving_it_to_the_rendered_harness_slug() -> Result {
    let held = seat("terminal", None, false);
    let version = profile(
        1,
        "Original operator name",
        Some(Harness::ClaudeCode),
        false,
    )?;
    let lines = transition(
        &held,
        &version,
        &view("claude-code", 1),
        Some(UUID.to_owned()),
        "person",
        3,
    )?;
    assert!(matches!(lines.as_slice(), [Line::Started(_)]));
    Ok(())
}

#[test]
fn a_first_codex_native_id_remains_unknown_until_an_actual_notification_is_observed() -> Result {
    let launch = json!({});
    assert_eq!(native_conversation(&launch, Transport::Codex, None)?, None);
    assert_eq!(
        native_conversation(&launch, Transport::Codex, Some(UUID))?.as_deref(),
        Some(UUID)
    );
    let held = seat("terminal", None, false);
    let version = profile(1, "Original operator name", Some(Harness::Codex), false)?;
    let mut launch = view("codex", 1);
    launch
        .as_object_mut()
        .ok_or("view missing")?
        .remove("native_conversation");
    let lines = transition(&held, &version, &launch, None, "person", 3)?;
    let [Line::Started(started)] = lines.as_slice() else {
        return Err("first Codex start unexpectedly selected another profile".into());
    };
    assert_eq!(started.harness_session, None);
    Ok(())
}

#[test]
fn expected_and_observed_native_ids_are_validated_before_a_seat_transition_is_written() -> Result {
    for transport in [Transport::Claude, Transport::Codex] {
        let launch = json!({"native_conversation": UUID});
        assert_eq!(
            native_conversation(&launch, transport, None)?.as_deref(),
            Some(UUID)
        );
        assert_eq!(
            native_conversation(&launch, transport, Some(UUID))?.as_deref(),
            Some(UUID)
        );
        for inconsistent in [
            json!({"native_conversation": UUID, "conversation": PRIOR_UUID}),
            json!({"native_conversation": PRIOR_UUID}),
        ] {
            let before = inconsistent.clone();
            let error = native_conversation(&inconsistent, transport, Some(UUID))
                .err()
                .ok_or("a mismatched native id was remembered")?;
            assert_eq!(error.name(), "pty_conversation_mismatch");
            assert_eq!(inconsistent, before);
        }
        for invalid in ["", "not-a-uuid", OPERATION] {
            let error =
                native_conversation(&json!({"native_conversation": invalid}), transport, None)
                    .err()
                    .ok_or("an invalid native UUID was remembered")?;
            assert_eq!(error.name(), "control_binding_invalid");
            assert!(error.to_string().contains("choose Start again"));
        }
        for invalid in [json!(true), json!(17), json!([]), json!({})] {
            let error = native_conversation(
                &json!({"native_conversation": invalid}),
                transport,
                Some(UUID),
            )
            .err()
            .ok_or("an invalid native field silently used the observed id")?;
            assert_eq!(error.name(), "pty_resume_unavailable");
        }
    }
    Ok(())
}

#[test]
fn a_missing_recorded_folder_or_mismatched_profile_refuses_without_modifying_the_seat() -> Result {
    let held = seat("terminal", None, false);
    let version = profile(
        1,
        "Original operator name",
        Some(Harness::ClaudeCode),
        false,
    )?;
    let mut missing = view("claude-code", 1);
    missing
        .as_object_mut()
        .ok_or("view missing")?
        .remove("directory");
    for launch in [missing, view("codex", 1), view("claude-code", 2)] {
        let before = held.clone();
        let error = transition(&held, &version, &launch, Some(UUID.to_owned()), "person", 3)
            .err()
            .ok_or("an unavailable exact launch was written to the seat")?;
        assert_eq!(error.name(), "LaunchUnrenderable");
        assert_eq!(held, before);
    }
    Ok(())
}

#[test]
fn replacing_a_running_seat_requires_positive_end_evidence_instead_of_an_absent_runner_record()
-> Result {
    require_ended("terminal", PRIOR_OPERATION, true, None)?;
    require_ended("terminal", PRIOR_OPERATION, false, Some((false, true)))?;
    let running = require_ended("terminal", PRIOR_OPERATION, false, Some((true, false)))
        .err()
        .ok_or("an alive prior session was declared stopped")?;
    assert_eq!(running.name(), "seat_running");
    for unknown in [None, Some((false, false))] {
        let error = require_ended("terminal", PRIOR_OPERATION, false, unknown)
            .err()
            .ok_or("an unconfirmed prior session was declared stopped")?;
        assert_eq!(error.name(), "pty_previous_session_unconfirmed");
        assert!(error.to_string().contains(PRIOR_OPERATION));
    }
    Ok(())
}

#[test]
fn a_prior_session_reported_alive_and_ended_refuses_as_a_malformed_runner_answer() -> Result {
    let error = require_ended("terminal", PRIOR_OPERATION, false, Some((true, true)))
        .err()
        .ok_or("an inconsistent prior session was declared stopped")?;
    assert_eq!(error.name(), "runner_reply_malformed");
    Ok(())
}
