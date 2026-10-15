#![cfg(test)]
//! Native terminal argument binding preserves configuration and refuses ambiguous identities.

use super::{claude_arguments, conversation_uuid};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

const SESSION: &str = "op-00112233445566778899aabbccddeeff";
const UUID: &str = "00112233-4455-6677-8899-aabbccddeeff";

#[test]
fn the_complete_operation_hex_becomes_a_stable_native_uuid() -> TestResult {
    assert_eq!(conversation_uuid(SESSION)?, UUID);
    assert_eq!(
        conversation_uuid("op-00112233445566778899AABBCCDDEEFF")?,
        UUID
    );
    Ok(())
}

#[test]
fn invalid_operation_ids_are_refused_instead_of_getting_a_new_identity() {
    for session in [
        "",
        UUID,
        "op-00112233445566778899aabbccddeef",
        "op-00112233445566778899aabbccddeeff0",
        "op-00112233445566778899aabbccddeefg",
        " op-00112233445566778899aabbccddeeff",
        "op-00112233445566778899aabbccddeeff\n",
        "op-00112233445566778899aabbccddeeé",
    ] {
        let error = conversation_uuid(session).unwrap_err();
        assert_eq!(error.name(), "pty_conversation_invalid", "{error}");
    }
}

#[test]
fn fresh_and_resumed_terminal_arguments_preserve_every_configuration_argument() -> TestResult {
    let given = [
        "--settings",
        "/chosen/settings.json",
        "--mcp-config=/chosen/mcp.json",
        "--append-system-prompt",
        "The existing instructions.",
        "--model",
        "chosen-model",
    ]
    .map(str::to_owned);
    for (resume, flag) in [(false, "--session-id"), (true, "--resume")] {
        let answer = claude_arguments(&given, UUID, resume)?;
        assert_eq!(&answer[..given.len()], &given);
        assert_eq!(&answer[given.len()..], [flag, UUID]);
        assert_eq!(answer.len(), given.len() + 2);
    }
    Ok(())
}

#[test]
fn native_identity_and_persistence_conflicts_are_refused_without_overwriting_arguments() {
    for conflict in [
        "--session-id",
        "--session-id=another-id",
        "--resume",
        "--resume=another-id",
        "-r",
        "-ranother-id",
        "--continue",
        "-c",
        "--fork-session",
        "--no-session-persistence",
        "--",
    ] {
        let given = vec![
            "--model".to_owned(),
            "chosen-model".to_owned(),
            conflict.to_owned(),
        ];
        let before = given.clone();
        for resume in [false, true] {
            let error = claude_arguments(&given, UUID, resume).unwrap_err();
            assert_eq!(error.name(), "pty_conversation_conflict", "{error}");
        }
        assert_eq!(given, before);
    }
}

#[test]
fn native_argument_binding_refuses_a_missing_or_non_uuid_conversation() {
    for conversation in ["", SESSION, "00112233-4455-6677-8899-aabbccddeefg"] {
        for resume in [false, true] {
            let error = claude_arguments(&[], conversation, resume).unwrap_err();
            assert_eq!(error.name(), "pty_conversation_invalid", "{error}");
        }
    }
}

#[test]
fn terminal_claude_resume_refuses_picker_fork_duplicate_and_conflicting_native_flags() -> TestResult
{
    for arguments in [
        vec!["--resume"],
        vec!["--continue"],
        vec!["-c"],
        vec!["--fork-session", "--resume", UUID],
        vec!["--resume", UUID, "--resume", UUID],
        vec!["--resume", UUID, "--session-id", UUID],
        vec!["--resume", UUID, "--no-session-persistence"],
        vec!["--", "--resume", UUID],
    ] {
        let arguments = arguments.into_iter().map(str::to_owned).collect::<Vec<_>>();
        let error = super::claude_resume(&arguments)
            .err()
            .ok_or("malformed resume admitted")?;
        assert_eq!(error.name(), "pty_resume_arguments_invalid", "{error}");
    }
    Ok(())
}
