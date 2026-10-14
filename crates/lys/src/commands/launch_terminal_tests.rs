#![cfg(test)]

use std::sync::Mutex;

use serde_json::{Value, json};

use super::{Transport, answer, bytes, delivered, end_session, exit, finish};
use crate::commands::error::CliResult;
use crate::commands::seat_client::refused;

fn window(from: u64, data: &[u8], ended: Value) -> Value {
    let mut view = json!({
        "answer": {
            "kind": "bytes",
            "output": {
                "session": "session-1", "from": from,
                "cursor": from + u64::try_from(data.len()).expect("test data length"),
                "oldest": 0, "data": data,
            },
        },
    });
    view["answer"]["output"]["ended"] = ended;
    view
}

fn ended() -> Value {
    json!({"how": "exited", "at": 1, "status": 0, "signal": null})
}

#[test]
fn terminal_bytes_preserve_incomplete_utf8_escape_and_control_sequences() {
    let raw = [0xff, 0xc3, 0x1b, b'[', b'2', b'J', 0, 3];
    let output =
        bytes(window(42, &raw, Value::Null), "session-1", Some(42)).expect("exact byte window");
    assert_eq!(output.data, raw);
    assert_eq!(output.from, 42);
    assert_eq!(output.cursor, 50);
}

#[test]
fn every_terminal_verb_preserves_the_runners_named_refusal() {
    let refusal = json!({"answer": {
        "kind": "refused", "refusal": "session_ended", "words": "the process has ended",
    }});
    for verb in ["read-bytes", "input-bytes", "resize", "end"] {
        let error = answer(refusal.clone(), verb).expect_err("runner refused");
        let words = error.to_string();
        assert!(words.contains("session_ended"), "{words}");
        assert!(words.contains("the process has ended"), "{words}");
    }
}

#[test]
fn input_and_resize_require_delivered_for_the_same_session() {
    let delivered_reply = json!({"answer": {"kind": "delivered", "session": "session-1"}});
    for verb in ["input-bytes", "resize"] {
        delivered(delivered_reply.clone(), "session-1", verb).expect("matching delivery");
        let error = delivered(delivered_reply.clone(), "session-2", verb)
            .expect_err("foreign session cannot confirm delivery");
        assert!(
            error
                .to_string()
                .contains("launch_terminal_answer_unexpected")
        );
        let error = delivered(
            json!({"answer": {"kind": "collected", "words": "ok"}}),
            "session-1",
            verb,
        )
        .expect_err("another answer cannot confirm delivery");
        assert!(
            error
                .to_string()
                .contains("launch_terminal_answer_unexpected")
        );
    }
}

#[test]
fn terminal_replies_require_a_complete_runner_answer() {
    for malformed in [
        json!({}),
        json!({"answer": {"kind": "bytes"}}),
        json!({"answer": {"kind": "unknown"}}),
    ] {
        let error = answer(malformed, "read-bytes").expect_err("malformed answer");
        assert!(
            error
                .to_string()
                .contains("launch_terminal_answer_unreadable")
        );
    }
}

#[test]
fn malformed_terminal_reply_diagnostics_do_not_echo_response_content() {
    let error = answer(
        json!({"answer": {"kind": "private-marker-not-to-print"}}),
        "read-bytes",
    )
    .expect_err("malformed answer");
    let words = error.to_string();
    assert!(words.contains("launch_terminal_answer_unreadable"));
    assert!(!words.contains("private-marker-not-to-print"));
}

#[test]
fn terminal_reads_refuse_session_cursor_and_length_drift() {
    let correct = window(42, &[1, 2, 3], Value::Null);
    let cases = [
        ("session", json!("session-2")),
        ("from", json!(41)),
        ("cursor", json!(46)),
        ("oldest", json!(43)),
    ];
    for (field, value) in cases {
        let mut reply = correct.clone();
        reply["answer"]["output"][field] = value;
        let error = bytes(reply, "session-1", Some(42)).expect_err("changed byte window");
        assert!(error.to_string().contains("launch_terminal_cursor_invalid"));
    }
}

#[test]
fn initial_terminal_read_uses_the_runners_retained_oldest_byte() {
    let mut reply = window(42, &[1, 2, 3], Value::Null);
    reply["answer"]["output"]["oldest"] = json!(42);
    let output = bytes(reply, "session-1", None).expect("initial retained byte window");
    assert_eq!(output.from, 42);
}

#[test]
fn a_follow_must_wait_for_bytes_or_an_observed_end() {
    let error = bytes(window(0, &[], Value::Null), "session-1", Some(0))
        .expect_err("empty running follow must not become a polling loop");
    assert!(error.to_string().contains("launch_terminal_follow_empty"));
    let output = bytes(window(0, &[], ended()), "session-1", Some(0))
        .expect("an observed end can have no remaining bytes");
    assert!(output.ended.is_some());
}

struct Ending {
    answer: Value,
    calls: Mutex<Vec<(String, Value)>>,
}

impl Transport for Ending {
    fn post(&self, path: &str, body: &Value) -> CliResult<Value> {
        self.calls
            .lock()
            .expect("calls lock")
            .push((path.to_owned(), body.clone()));
        Ok(self.answer.clone())
    }
}

#[test]
fn terminal_cleanup_requires_ended_for_its_own_session() {
    for (session, confirmed) in [("session-1", true), ("session-2", false)] {
        let transport = Ending {
            answer: json!({"answer": {"kind": "ended", "session": session, "ended": ended()}}),
            calls: Mutex::new(Vec::new()),
        };
        assert_eq!(end_session(&transport, "session-1").is_ok(), confirmed);
        assert_eq!(
            *transport.calls.lock().expect("calls lock"),
            vec![("/runtime/sessions/session-1/end".to_owned(), json!({}))]
        );
    }
    let transport = Ending {
        answer: json!({"answer": {"kind": "delivered", "session": "session-1"}}),
        calls: Mutex::new(Vec::new()),
    };
    assert!(
        end_session(&transport, "session-1")
            .expect_err("delivery is not an end")
            .to_string()
            .contains("launch_terminal_end_unconfirmed")
    );
}

#[test]
fn terminal_failure_does_not_hide_cleanup_or_restore_failure() {
    let error = finish(
        Err(refused("cursor_expired", "retained output was lost")),
        Err(refused(
            "runner_unreachable",
            "session end was not confirmed",
        )),
        Err(refused(
            "launch_terminal_io_failed",
            "attributes could not be restored",
        )),
    )
    .expect_err("all three failures must be kept");
    let words = error.to_string();
    for name in [
        "cursor_expired",
        "runner_unreachable",
        "launch_terminal_io_failed",
    ] {
        assert!(words.contains(name), "{words}");
    }
    let single = finish(Err(refused("cursor_expired", "lost")), Ok(()), Ok(()))
        .expect_err("original named refusal");
    assert!(single.to_string().starts_with("cursor_expired"));
}

#[test]
fn exit_requires_the_runners_observed_success_status() {
    let mut evidence: lys_runner::Ended = serde_json::from_value(ended()).expect("exit evidence");
    exit(&evidence).expect("observed success");
    evidence.status = Some(17);
    assert!(
        exit(&evidence)
            .expect_err("nonzero exit")
            .to_string()
            .contains("17")
    );
    evidence.status = None;
    assert!(exit(&evidence).is_err());
    evidence.status = Some(0);
    evidence.how = lys_runner::EndedHow::EndedByRunnerRestart;
    assert!(exit(&evidence).is_err());
}
