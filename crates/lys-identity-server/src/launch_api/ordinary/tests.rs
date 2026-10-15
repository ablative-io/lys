//! An ordinary terminal start resumes only the newest matching recorded launch.

use lys_runner::tracking::Harness;
use serde_json::{Value, json};

use super::{known_native, latest, prior, recorded_directory, replay, saved_native};
use crate::runtime_state::{Report, Reported, Tracked};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const AGENT: &str = "agent";
const MACHINE: &str = "machine";
const OPERATION: &str = "op-fedcba9876543210fedcba9876543210";
const PRIOR_OPERATION: &str = "op-00112233445566778899aabbccddeeff";
const PRIOR_FOLDER: &str = "/var/recorded-link/prior-project";
const CURRENT_FOLDER: &str = "/different/current-profile-project";
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";
const OTHER_UUID: &str = "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee";

fn view(harness: &str) -> Value {
    json!({
        "machine": MACHINE,
        "session": PRIOR_OPERATION,
        "harness": harness,
        "directory": PRIOR_FOLDER,
        "working_folder": CURRENT_FOLDER
    })
}

fn tracked(session: &str, agent: Option<&str>, machine: &str, launch: Option<Value>) -> Tracked {
    Tracked {
        session: session.to_owned(),
        agent: agent.map(str::to_owned),
        machine: machine.to_owned(),
        reports: vec![Report {
            operation: session.to_owned(),
            session: session.to_owned(),
            agent: agent.map(str::to_owned),
            machine: machine.to_owned(),
            state: Reported::Running,
            what: "the runner started process 42".to_owned(),
            confirmation: String::new(),
            reported_by: format!("the runner of machine {machine}"),
            at: 1,
            launch,
        }],
    }
}

#[test]
fn the_latest_prior_is_selected_on_the_same_agent_and_machine_excluding_the_current_operation()
-> Result {
    let sessions = [
        tracked(
            PRIOR_OPERATION,
            Some(AGENT),
            MACHINE,
            Some(view("claude-code")),
        ),
        tracked(
            "other-agent",
            Some("other"),
            MACHINE,
            Some(view("claude-code")),
        ),
        tracked(
            "other-machine",
            Some(AGENT),
            "other",
            Some(view("claude-code")),
        ),
        tracked(OPERATION, Some(AGENT), MACHINE, Some(view("claude-code"))),
    ];
    let selected = latest(&sessions, AGENT, MACHINE, OPERATION)
        .ok_or("the exact prior agent and machine were not selected")?;
    assert_eq!(selected.session, PRIOR_OPERATION);
    Ok(())
}

#[test]
fn no_prior_on_the_selected_machine_keeps_the_first_start_without_a_resume_source() {
    let sessions = [tracked(
        PRIOR_OPERATION,
        Some(AGENT),
        "other-machine",
        Some(view("claude-code")),
    )];
    assert!(latest(&sessions, AGENT, MACHINE, OPERATION).is_none());
}

#[test]
fn a_newer_session_without_launch_metadata_starts_fresh_instead_of_resuming_an_older_launch()
-> Result {
    let sessions = [
        tracked("older", Some(AGENT), MACHINE, Some(view("claude-code"))),
        tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, None),
    ];
    let selected = latest(&sessions, AGENT, MACHINE, OPERATION)
        .ok_or("the newest prior session was skipped")?;
    assert_eq!(selected.session, PRIOR_OPERATION);
    let chosen = prior(selected, Harness::ClaudeCode, None)?;
    assert_eq!(chosen.native, None);
    assert_eq!(chosen.directory, None);
    Ok(())
}

#[test]
fn missing_first_launch_metadata_starts_fresh_even_if_a_later_report_names_a_launch() -> Result {
    let mut held = tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, None);
    let mut later = held.reports[0].clone();
    later.launch = Some(view("claude-code"));
    held.reports.push(later);
    let chosen = prior(&held, Harness::ClaudeCode, None)?;
    assert_eq!(chosen.native, None);
    assert_eq!(chosen.directory, None);
    Ok(())
}

#[test]
fn a_matching_claude_or_codex_prior_keeps_its_exact_recorded_folder_and_session() -> Result {
    for (name, harness) in [
        ("claude-code", Harness::ClaudeCode),
        ("codex", Harness::Codex),
    ] {
        let mut launch = view(name);
        launch["native_conversation"] = json!(UUID);
        let held = tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, Some(launch));
        let selected = prior(&held, harness, Some(PRIOR_FOLDER))?;
        assert_eq!(selected.session, PRIOR_OPERATION);
        assert_eq!(selected.directory.as_deref(), Some(PRIOR_FOLDER));
        assert_eq!(selected.native.as_deref(), Some(UUID));
    }
    Ok(())
}

#[test]
fn a_changed_native_harness_or_explicit_folder_refuses_and_names_start_again() -> Result {
    for (recorded, current) in [
        ("claude-code", Harness::Codex),
        ("codex", Harness::ClaudeCode),
    ] {
        let mut launch = view(recorded);
        launch["native_conversation"] = json!(UUID);
        let held = tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, Some(launch));
        let error = prior(&held, current, None)
            .err()
            .ok_or("a changed harness resumed the prior native conversation")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("Start again"));
    }
    let error = recorded_directory(&view("claude-code"), Some(CURRENT_FOLDER))
        .err()
        .ok_or("a resume silently moved to the current profile folder")?;
    assert_eq!(error.name(), "pty_resume_unavailable");
    assert!(error.to_string().contains("Start again"));
    Ok(())
}

#[test]
fn a_missing_or_invalid_recorded_folder_refuses_without_a_profile_default() -> Result {
    let mut missing = view("claude-code");
    missing
        .as_object_mut()
        .ok_or("the fixture is not an object")?
        .remove("directory");
    let mut cases = vec![missing];
    for invalid in [
        Value::Null,
        json!(true),
        json!(17),
        json!([]),
        json!("relative"),
    ] {
        let mut invalid_view = view("claude-code");
        invalid_view["directory"] = invalid;
        cases.push(invalid_view);
    }
    for invalid in cases {
        let error = recorded_directory(&invalid, None)
            .err()
            .ok_or("an unavailable recorded folder acquired a profile default")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("Start again"));
    }
    Ok(())
}

#[test]
fn the_saved_native_identity_precedes_the_kept_resume_identity_for_both_harnesses() -> Result {
    for (name, harness) in [
        ("claude-code", Harness::ClaudeCode),
        ("codex", Harness::Codex),
    ] {
        let mut launch = view(name);
        launch["native_conversation"] = json!(UUID);
        launch["conversation"] = json!(OTHER_UUID);
        assert_eq!(saved_native(&launch, harness)?.as_deref(), Some(UUID));
        launch["native_conversation"] = Value::Null;
        assert_eq!(saved_native(&launch, harness)?.as_deref(), Some(OTHER_UUID));
        launch
            .as_object_mut()
            .ok_or("the fixture is not an object")?
            .remove("native_conversation");
        assert_eq!(saved_native(&launch, harness)?.as_deref(), Some(OTHER_UUID));
    }
    Ok(())
}

#[test]
fn an_old_launch_without_saved_native_identity_requires_runner_recovery_without_guessing_an_id()
-> Result {
    for harness in [Harness::ClaudeCode, Harness::Codex] {
        assert_eq!(saved_native(&view("claude-code"), harness)?, None);
        let mut launch = view("claude-code");
        launch["native_conversation"] = Value::Null;
        launch["conversation"] = Value::Null;
        assert_eq!(saved_native(&launch, harness)?, None);
    }
    Ok(())
}

#[test]
fn no_remembered_or_recovered_native_id_starts_fresh_but_an_invalid_known_id_still_refuses()
-> Result {
    for harness in [Harness::ClaudeCode, Harness::Codex] {
        assert_eq!(known_native(harness, None)?, None);
        assert_eq!(
            known_native(harness, Some(OTHER_UUID.to_owned()))?.as_deref(),
            Some(OTHER_UUID)
        );
        let error = known_native(harness, Some("not-a-uuid".to_owned()))
            .err()
            .ok_or("an invalid runner conversation was resumed")?;
        assert_eq!(error.name(), "control_binding_invalid");
    }
    Ok(())
}

#[test]
fn invalid_saved_native_fields_refuse_instead_of_falling_back_to_another_identity() -> Result {
    for field in ["native_conversation", "conversation"] {
        for invalid in [json!(true), json!(17), json!([]), json!({})] {
            let mut launch = view("claude-code");
            if field == "native_conversation" {
                launch["conversation"] = json!(OTHER_UUID);
            }
            launch[field] = invalid;
            let error = saved_native(&launch, Harness::ClaudeCode)
                .err()
                .ok_or("an invalid saved identity silently selected another identity")?;
            assert_eq!(error.name(), "pty_resume_unavailable");
            assert!(error.to_string().contains("Start again"));
        }
    }
    for harness in [Harness::ClaudeCode, Harness::Codex] {
        let mut launch = view("claude-code");
        launch["native_conversation"] = json!("not-a-uuid");
        launch["conversation"] = json!(OTHER_UUID);
        let error = saved_native(&launch, harness)
            .err()
            .ok_or("an invalid native UUID silently selected a different UUID")?;
        assert_eq!(error.name(), "control_binding_invalid");
    }
    Ok(())
}

#[test]
fn replay_keeps_original_fresh_or_resume_intent_and_the_recorded_directory() -> Result {
    for conversation in [Value::Null, json!(OTHER_UUID)] {
        let mut launch = view("claude-code");
        launch["native_conversation"] = json!(UUID);
        launch["conversation"] = conversation.clone();
        let selected = replay(&launch, MACHINE, None)?;
        assert!(selected.profile.is_none());
        assert_eq!(selected.directory.as_deref(), Some(PRIOR_FOLDER));
        assert_eq!(selected.conversation.as_deref(), conversation.as_str());
    }
    let selected = replay(&view("claude-code"), MACHINE, None)?;
    assert_eq!(selected.conversation, None);
    Ok(())
}

#[test]
fn replay_refuses_a_different_machine_folder_or_malformed_kept_conversation() -> Result {
    for (machine, directory) in [("other-machine", None), (MACHINE, Some(CURRENT_FOLDER))] {
        let error = replay(&view("claude-code"), machine, directory)
            .err()
            .ok_or("a replay silently changed its machine or working folder")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("Start again"));
    }
    let mut launch = view("claude-code");
    launch["conversation"] = json!(17);
    let error = replay(&launch, MACHINE, None)
        .err()
        .ok_or("a malformed kept conversation became a fresh replay")?;
    assert_eq!(error.name(), "pty_resume_unavailable");
    Ok(())
}

#[test]
fn no_known_conversation_uses_the_new_folder_instead_of_refusing_an_old_folder_change() -> Result {
    let held = tracked(
        PRIOR_OPERATION,
        Some(AGENT),
        MACHINE,
        Some(view("claude-code")),
    );
    let chosen = prior(&held, Harness::ClaudeCode, Some(CURRENT_FOLDER))?;
    assert_eq!(chosen.native, None);
    assert_eq!(chosen.directory, None);
    Ok(())
}

#[test]
fn an_admitted_uuid_is_not_a_remembered_conversation_until_the_runner_started_the_child() -> Result
{
    let mut launch = view("claude-code");
    launch["native_conversation"] = json!(UUID);
    let mut held = tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, Some(launch));
    held.reports[0].state = Reported::Starting;
    let chosen = prior(&held, Harness::ClaudeCode, None)?;
    assert_eq!(chosen.native, None);
    assert_eq!(chosen.directory, None);
    Ok(())
}

#[test]
fn a_failed_start_does_not_hide_the_last_successfully_started_conversation() -> Result {
    let mut launch = view("claude-code");
    launch["native_conversation"] = json!(UUID);
    let successful = tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, Some(launch));
    let mut failed = tracked("failed", Some(AGENT), MACHINE, Some(view("claude-code")));
    failed.reports[0].state = Reported::Starting;
    let mut stopped = failed.reports[0].clone();
    stopped.state = Reported::Stopped;
    failed.reports.push(stopped);
    let sessions = [successful, failed];
    let selected = latest(&sessions, AGENT, MACHINE, OPERATION)
        .ok_or("the successful conversation was hidden by a failed start")?;
    assert_eq!(selected.session, PRIOR_OPERATION);
    assert_eq!(
        prior(selected, Harness::ClaudeCode, None)?
            .native
            .as_deref(),
        Some(UUID)
    );
    Ok(())
}

#[test]
fn a_different_requested_folder_selects_a_fresh_conversation_for_each_native_harness() -> Result {
    for (name, harness) in [
        ("claude-code", Harness::ClaudeCode),
        ("codex", Harness::Codex),
    ] {
        let mut launch = view(name);
        launch["native_conversation"] = json!(UUID);
        let held = tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, Some(launch));
        let selected = prior(&held, harness, Some(CURRENT_FOLDER))?;
        assert_eq!(selected.native, None);
        assert_eq!(selected.directory.as_deref(), Some(PRIOR_FOLDER));
    }
    Ok(())
}

#[test]
fn a_codex_thread_report_survives_runtime_replay_after_the_process_ends() -> Result {
    let mut held = tracked(PRIOR_OPERATION, Some(AGENT), MACHINE, Some(view("codex")));
    held.reports[0].state = Reported::Starting;
    let mut running = held.reports[0].clone();
    running.operation = "running".to_owned();
    running.state = Reported::Running;
    running.launch = None;
    held.reports.push(running.clone());
    running.operation = "native".to_owned();
    running.confirmation = "the runner identified the native conversation".to_owned();
    running.launch = Some(json!({"native_conversation": UUID}));
    held.reports.push(running.clone());
    running.operation = "stopped".to_owned();
    running.state = Reported::Stopped;
    running.launch = None;
    held.reports.push(running);
    let mut runtime = crate::runtime_state::Held::default();
    for report in held.reports {
        runtime.hold(report)?;
    }
    let reopened = crate::runtime_state::Held::decode(&runtime.encode()?)?;
    let selected = latest(&reopened.sessions, AGENT, MACHINE, OPERATION)
        .ok_or("the ended Codex session was lost")?;
    assert_eq!(
        prior(selected, Harness::Codex, None)?.native.as_deref(),
        Some(UUID)
    );
    Ok(())
}
