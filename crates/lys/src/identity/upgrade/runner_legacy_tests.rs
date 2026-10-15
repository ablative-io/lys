//! Older runners refuse unknown acts through the typed client error.

use super::*;
use lys_runner::protocol::{read_reply, reply_line};

fn legacy_refusal() -> Result<Answer, lys_runner::error::RunnerError> {
    read_reply(&reply_line(Answer::Refused {
        refusal: "runner_request_malformed".into(),
        words: "the act does not read at line 1, column 25".into(),
        oldest: None,
    }))
}

#[test]
fn an_empty_legacy_runner_can_be_upgraded_after_its_unknown_act_refusal()
-> Result<(), Box<dyn std::error::Error>> {
    let (sessions, fenced) = captured(legacy_refusal(), true, Path::new("runner.sock"))?;
    assert!(sessions.is_empty());
    assert!(!fenced, "the legacy runner never accepted a start fence");
    Ok(())
}

#[test]
fn a_live_legacy_runner_is_never_stopped_without_capturing_its_launches() {
    let answer = captured(legacy_refusal(), false, Path::new("runner.sock"));
    assert!(answer.is_err_and(|error| {
        error
            .to_string()
            .contains("runner_upgrade_capture_unavailable")
    }));
}

#[test]
fn an_empty_runner_with_an_unrelated_refusal_still_refuses_the_upgrade() {
    for (refusal, words) in [
        (
            "runner_state_unavailable",
            "the session store could not be read",
        ),
        (
            "runner_request_malformed",
            "the signature envelope does not read",
        ),
        ("runner_request_unsigned", "the act does not read"),
    ] {
        let answer = read_reply(&reply_line(Answer::Refused {
            refusal: refusal.into(),
            words: words.into(),
            oldest: None,
        }));
        assert!(captured(answer, true, Path::new("runner.sock")).is_err());
    }
}
