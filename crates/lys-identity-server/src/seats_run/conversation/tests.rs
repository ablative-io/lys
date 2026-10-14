//! Native conversation choices are explicit for first starts, resumes and fresh starts.

use super::{Harness, Plan, pty};
use crate::seats_state::{Added, Seat};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const OPERATION: &str = "op-fedcba9876543210fedcba9876543210";
const PRIOR_OPERATION: &str = "op-0123456789abcdef0123456789abcdef";
const PRIOR_UUID: &str = "01234567-89ab-cdef-0123-456789abcdef";
const NEW_UUID: &str = "fedcba98-7654-3210-fedc-ba9876543210";
const SELECTED_UUID: &str = "aaaaaaaa-bbbb-4ccc-addd-eeeeeeeeeeee";

fn seat(session: Option<&str>, native: Option<&str>) -> Seat {
    Seat {
        qualification: None,
        transport: session.map(|_| "pty".to_owned()),
        added: Added {
            operation: "op-11111111111111111111111111111111".to_owned(),
            name: "terminal".to_owned(),
            agent: "agent".to_owned(),
            harness: "claude-code".to_owned(),
            profile_version: 1,
            machine: "machine".to_owned(),
            working_folder: Some("/project".to_owned()),
            account: None,
            by: "person".to_owned(),
            at: 1,
        },
        session: session.map(str::to_owned),
        harness_session: native.map(str::to_owned),
        running: false,
        revision: 1,
        retired: false,
    }
}

fn resumed(conversation: &str) -> Plan {
    Plan {
        resume: Some(conversation.to_owned()),
        expected: Some(conversation.to_owned()),
    }
}

#[test]
fn the_first_claude_start_binds_the_operation_uuid_before_any_native_session_exists() -> Result {
    let held = seat(None, None);
    let before = held.clone();
    assert_eq!(
        pty(&held, OPERATION, None, false, Some(Harness::ClaudeCode))?,
        Plan {
            resume: None,
            expected: Some(NEW_UUID.to_owned()),
        }
    );
    assert_eq!(held, before);
    Ok(())
}

#[test]
fn an_ordinary_claude_or_codex_start_resumes_the_remembered_native_conversation() -> Result {
    let held = seat(Some(PRIOR_OPERATION), Some(PRIOR_UUID));
    for harness in [Harness::ClaudeCode, Harness::Codex] {
        assert_eq!(
            pty(&held, OPERATION, None, false, Some(harness))?,
            resumed(PRIOR_UUID)
        );
    }
    Ok(())
}

#[test]
fn an_explicit_native_conversation_is_selected_for_first_and_existing_seats() -> Result {
    for held in [
        seat(None, None),
        seat(Some(PRIOR_OPERATION), Some(PRIOR_UUID)),
    ] {
        for harness in [Harness::ClaudeCode, Harness::Codex] {
            assert_eq!(
                pty(&held, OPERATION, Some(SELECTED_UUID), false, Some(harness))?,
                resumed(SELECTED_UUID)
            );
        }
    }
    Ok(())
}

#[test]
fn start_again_binds_a_new_claude_uuid_and_ignores_the_previous_native_id() -> Result {
    for prior in [None, Some(PRIOR_UUID), Some("legacy-invalid-native-id")] {
        assert_eq!(
            pty(
                &seat(Some(PRIOR_OPERATION), prior),
                OPERATION,
                None,
                true,
                Some(Harness::ClaudeCode),
            )?,
            Plan {
                resume: None,
                expected: Some(NEW_UUID.to_owned()),
            }
        );
    }
    Ok(())
}

#[test]
fn a_first_or_explicitly_fresh_codex_start_keeps_its_unreported_native_id_unavailable() -> Result {
    for (held, fresh) in [
        (seat(None, None), false),
        (seat(Some(PRIOR_OPERATION), Some(PRIOR_UUID)), true),
    ] {
        assert_eq!(
            pty(&held, OPERATION, None, fresh, Some(Harness::Codex))?,
            Plan {
                resume: None,
                expected: None,
            }
        );
    }
    Ok(())
}

#[test]
fn a_legacy_terminal_without_its_native_id_names_start_again_instead_of_starting_fresh() -> Result {
    let held = seat(Some(PRIOR_OPERATION), None);
    for harness in [Harness::ClaudeCode, Harness::Codex] {
        let error = pty(&held, OPERATION, None, false, Some(harness))
            .err()
            .ok_or("the existing terminal silently began a new conversation")?;
        assert_eq!(error.name(), "pty_resume_unavailable");
        assert!(error.to_string().contains("Start again"));
        assert_eq!(
            pty(&held, OPERATION, Some(SELECTED_UUID), false, Some(harness))?,
            resumed(SELECTED_UUID)
        );
    }
    Ok(())
}

#[test]
fn start_again_refuses_a_conflicting_explicit_resume_selection() -> Result {
    let held = seat(Some(PRIOR_OPERATION), Some(PRIOR_UUID));
    let error = pty(
        &held,
        OPERATION,
        Some(SELECTED_UUID),
        true,
        Some(Harness::ClaudeCode),
    )
    .err()
    .ok_or("Start again accepted an existing conversation selection")?;
    assert_eq!(error.name(), "pty_resume_invalid");
    Ok(())
}

#[test]
fn malformed_operations_and_native_ids_are_refused_before_a_plan_is_returned() -> Result {
    for invalid in ["", "op-123", "op-gggggggggggggggggggggggggggggggg"] {
        let error = pty(
            &seat(None, None),
            invalid,
            None,
            false,
            Some(Harness::Codex),
        )
        .err()
        .ok_or("an invalid operation selected a conversation")?;
        assert_eq!(error.name(), "IdentifierMalformed");
    }
    for invalid in ["", "../thread", "not-a-uuid"] {
        for harness in [Harness::ClaudeCode, Harness::Codex] {
            for (held, explicit) in [
                (seat(None, None), Some(invalid)),
                (seat(Some(PRIOR_OPERATION), Some(invalid)), None),
            ] {
                let error = pty(&held, OPERATION, explicit, false, Some(harness))
                    .err()
                    .ok_or("an invalid native id selected a conversation")?;
                assert_eq!(error.name(), "control_binding_invalid");
            }
        }
    }
    Ok(())
}

#[test]
fn a_retained_operation_requires_its_kept_launch_instead_of_inferring_resume_intent() -> Result {
    let held = seat(Some(OPERATION), Some(NEW_UUID));
    for fresh in [false, true] {
        let error = pty(&held, OPERATION, None, fresh, Some(Harness::ClaudeCode))
            .err()
            .ok_or("a replay inferred a new launch plan from the folded seat")?;
        assert_eq!(error.name(), "pty_launch_replay_unresolved");
    }
    Ok(())
}
