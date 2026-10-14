#![cfg(test)]

use std::sync::{Arc, Condvar, Mutex};

use lys_runner::operations::TextDigest;
use serde_json::{Value, json};

use super::{Lines, Transport, admitted, follow, page, render};
use crate::commands::error::CliResult;
use crate::commands::launch_terminal::end_session;

fn frames(session: &str, cursor: u64, ended: bool, lines: Value) -> Value {
    json!({"session": session, "cursor": cursor, "ended": ended, "lines": lines})
}

fn line(kind: &str, text: &str) -> Value {
    json!({"at": 1, "kind": kind, "text": text})
}

fn receipt(state: &str, delivered: bool) -> Value {
    if delivered {
        return json!({"session": "session-1", "answer": {"kind": "delivered", "session": "session-1"}});
    }
    json!({"session": "session-1", "answer": {"kind": "operation", "outcome": {
        "operation": "operation-1", "session": "session-1", "request": "notice", "state": state,
        "at": 1, "words": "observed", "text": TextDigest::of("hello"), "ended": null,
    }}})
}

#[test]
fn canonical_input_submits_complete_nonempty_lines_without_trimming_tabs_or_utf8() {
    let mut lines = Lines::default();
    assert_eq!(
        lines
            .receive("hello\tworld\n \t\nüber\n".as_bytes())
            .expect("complete lines"),
        vec!["hello\tworld", "über"]
    );
    lines.ended().expect("no unfinished input");
}

#[test]
fn canonical_input_waits_for_a_newline_even_when_utf8_spans_reads() {
    let mut lines = Lines::default();
    assert!(
        lines
            .receive(&[0xc3])
            .expect("unfinished UTF-8 is retained")
            .is_empty()
    );
    assert!(
        lines
            .receive(&[0xbc, b'b', b'e', b'r'])
            .expect("unfinished line is retained")
            .is_empty()
    );
    assert_eq!(
        lines.receive(b"\n").expect("complete Unicode line"),
        vec!["über"]
    );
    lines.ended().expect("complete line consumed");
}

#[test]
fn canonical_input_refuses_invalid_utf8_or_terminal_control_bytes() {
    for (data, name) in [
        (b"\xff\n".as_slice(), "launch_managed_input_not_utf8"),
        (
            b"secret\x1b[2J\n".as_slice(),
            "launch_managed_input_control",
        ),
        (b"secret\r\n".as_slice(), "launch_managed_input_control"),
    ] {
        let error = Lines::default()
            .receive(data)
            .expect_err("invalid managed input");
        assert!(error.to_string().contains(name));
        assert!(!error.to_string().contains("secret"));
    }
}

#[test]
fn canonical_eof_never_submits_or_silently_drops_a_partial_line() {
    let mut lines = Lines::default();
    assert!(
        lines
            .receive(b"partial")
            .expect("retained input")
            .is_empty()
    );
    assert!(
        lines
            .ended()
            .expect_err("partial input cannot be sent")
            .to_string()
            .contains("launch_managed_input_incomplete")
    );
    assert!(Lines::default().ended().is_ok());
}

#[test]
fn an_unfinished_canonical_line_has_a_named_resource_bound() {
    let error = Lines::default()
        .receive(&vec![b'x'; 1_048_577])
        .expect_err("unfinished line is bounded");
    assert!(error.to_string().contains("launch_managed_input_too_long"));
}

#[test]
fn every_managed_page_must_name_the_launched_session_and_exact_frame_cursor() {
    let value = frames(
        "session-1",
        5,
        false,
        json!([line("assistant", "hello"), line("status", "ready")]),
    );
    assert_eq!(
        page(value.clone(), "session-1", Some(3))
            .expect("exact page")
            .cursor,
        5
    );
    assert!(
        page(value.clone(), "session-2", Some(3))
            .expect_err("seat moved")
            .to_string()
            .contains("launch_managed_session_changed")
    );
    assert!(
        page(value.clone(), "session-1", Some(4))
            .expect_err("cursor skipped a frame")
            .to_string()
            .contains("launch_managed_cursor_invalid")
    );
    assert!(page(value, "session-1", Some(6)).is_err());
}

#[test]
fn a_managed_follow_answers_only_on_frames_or_an_end() {
    assert!(
        page(frames("session-1", 0, false, json!([])), "session-1", None)
            .expect_err("running empty follow cannot poll")
            .to_string()
            .contains("launch_managed_follow_empty")
    );
    assert!(
        page(
            frames("session-1", 5, true, json!([])),
            "session-1",
            Some(5)
        )
        .expect("observed end")
        .ended
    );
    assert!(
        page(
            frames("session-1", 0, false, json!([line("assistant", "hello")])),
            "session-1",
            None
        )
        .is_err()
    );
}

#[test]
fn unknown_or_malformed_managed_frames_are_named_without_echoing_response_values() {
    let error = page(
        frames(
            "session-1",
            1,
            false,
            json!([line("private-marker", "private-text")]),
        ),
        "session-1",
        None,
    )
    .expect_err("unknown frame");
    assert!(error.to_string().contains("launch_managed_frame_unknown"));
    assert!(!error.to_string().contains("private-marker"));
    let error =
        page(json!({"session": "private-marker"}), "session-1", None).expect_err("malformed page");
    assert!(
        error
            .to_string()
            .contains("launch_managed_answer_unreadable")
    );
    assert!(!error.to_string().contains("private-marker"));
}

#[test]
fn managed_frame_rendering_preserves_human_text_and_escapes_terminal_control_bytes() {
    let lines = vec![
        serde_json::from_value(line("assistant", "hello\nworld\t!\x1b[2J"))
            .expect("assistant frame"),
        serde_json::from_value(line("tool_result", "done\rchanged")).expect("tool frame"),
    ];
    let mut out = Vec::new();
    render(&lines, &mut out).expect("human frame output");
    let shown = String::from_utf8(out).expect("output is UTF-8");
    assert_eq!(
        shown,
        "hello\nworld\t!\\u{1b}[2J\ntool_result: done\\rchanged\n"
    );
    assert!(!shown.contains('\x1b'));
    assert!(!shown.contains('\r'));
}

#[test]
fn exact_session_input_requires_delivered_because_a_notice_operation_is_a_different_route() {
    admitted(receipt("delivered", true), "session-1").expect("matching exact-session admission");
    for name in ["accepted", "delivering", "delivered", "confirmed"] {
        assert!(
            admitted(receipt(name, false), "session-1")
                .expect_err("notice outcome cannot prove exact-session input")
                .to_string()
                .contains("launch_terminal_answer_unexpected")
        );
    }
}

#[test]
fn exact_session_input_binds_both_session_fields_and_refuses_other_answer_shapes() {
    let original = receipt("delivered", true);
    for (pointer, changed) in [
        ("/session", json!("session-2")),
        ("/answer/session", json!("session-2")),
        ("/answer", json!({"kind": "collected", "words": "observed"})),
        (
            "/answer",
            json!({"kind": "ended", "session": "session-1", "ended": {"how": "exited", "at": 1, "status": 0, "signal": null}}),
        ),
        ("/answer", json!({"kind": "private-marker"})),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(pointer).expect("receipt field") = changed;
        let words = admitted(value, "session-1")
            .expect_err("mismatched admission")
            .to_string();
        assert!(
            words.contains("answer_mismatch")
                || words.contains("answer_unexpected")
                || words.contains("answer_unreadable")
        );
        assert!(!words.contains("private-marker"));
    }
}

#[test]
fn refused_or_uncertain_exact_session_admissions_never_claim_delivery() {
    for name in ["session_ended", "control_input_uncertain"] {
        let value = json!({"session": "session-1", "answer": {"kind": "refused", "refusal": name, "words": "the runner did not confirm admission"}});
        assert_eq!(
            admitted(value, "session-1")
                .expect_err("named admission refusal")
                .to_string(),
            format!("{name}: the runner did not confirm admission")
        );
    }
}

#[derive(Default)]
struct FakeState {
    following: bool,
    ended: bool,
    calls: Vec<(String, Value)>,
}

#[derive(Default)]
struct Signalled {
    state: Mutex<FakeState>,
    changed: Condvar,
}

impl Transport for Signalled {
    fn post(&self, route: &str, body: &Value) -> CliResult<Value> {
        let mut state = self.state.lock().expect("state lock");
        state.calls.push((route.to_owned(), body.clone()));
        if route.ends_with("/attach") {
            state.following = true;
            self.changed.notify_all();
            drop(
                self.changed
                    .wait_while(state, |state| !state.ended)
                    .expect("end signal"),
            );
            return Ok(frames("session-1", 0, true, json!([])));
        }
        if route.ends_with("/end") {
            state.ended = true;
            self.changed.notify_all();
            return Ok(
                json!({"answer": {"kind": "ended", "session": "session-1", "ended": {"how": "exited", "at": 1, "status": 0, "signal": null}}}),
            );
        }
        assert_eq!(route, "/runtime/sessions/session-1/input");
        assert_eq!(body, &json!({"text": "hello", "enter": true}));
        Ok(receipt("delivered", true))
    }
}

#[test]
fn managed_follow_cannot_block_exact_session_input_and_end_wakes_its_reader_without_resize() {
    let transport = Arc::new(Signalled::default());
    let (mut pages, reader) =
        follow(Arc::clone(&transport), "seat-1", "session-1").expect("managed reader");
    let state = transport.state.lock().expect("state lock");
    drop(
        transport
            .changed
            .wait_while(state, |state| !state.following)
            .expect("follow signal"),
    );
    let received = super::request(
        Arc::clone(&transport),
        "/runtime/sessions/session-1/input".to_owned(),
        json!({"text": "hello", "enter": true}),
    )
    .expect("type request")
    .blocking_recv()
    .expect("receipt channel")
    .expect("type success");
    admitted(received, "session-1").expect("exact-session admission");
    end_session(transport.as_ref(), "session-1").expect("runtime ended");
    assert!(
        pages
            .blocking_recv()
            .expect("end page")
            .expect("valid page")
            .ended
    );
    reader.join().expect("reader joined");
    let state = transport.state.lock().expect("state lock");
    assert_eq!(state.calls.len(), 3);
    assert_eq!(
        state.calls[0],
        (
            "/seats/seat-1/attach".to_owned(),
            json!({"cursor": null, "follow": true})
        )
    );
    assert_eq!(state.calls[2].0, "/runtime/sessions/session-1/end");
    assert!(
        state
            .calls
            .iter()
            .all(|(route, _)| !route.ends_with("/resize"))
    );
}

#[derive(Default)]
struct Awaiting {
    state: Mutex<(bool, bool)>,
    changed: Condvar,
}

impl Transport for Awaiting {
    fn post(&self, route: &str, body: &Value) -> CliResult<Value> {
        assert_eq!(route, "/runtime/sessions/session-1/input");
        assert_eq!(body, &json!({"text": "hello", "enter": true}));
        let mut state = self.state.lock().expect("admission state");
        state.0 = true;
        self.changed.notify_all();
        drop(
            self.changed
                .wait_while(state, |state| !state.1)
                .expect("output was rendered"),
        );
        Ok(receipt("delivered", true))
    }
}

#[test]
fn managed_output_is_rendered_while_exact_session_input_awaits_boundary_admission() {
    let transport = Arc::new(Awaiting::default());
    let (sent, mut pages) = tokio::sync::mpsc::channel(1);
    let held_pages = sent.clone();
    let feeder = {
        let transport = Arc::clone(&transport);
        std::thread::spawn(move || {
            let state = transport.state.lock().expect("admission state");
            drop(
                transport
                    .changed
                    .wait_while(state, |state| !state.0)
                    .expect("input started"),
            );
            sent.blocking_send(Ok(page(
                frames(
                    "session-1",
                    1,
                    false,
                    json!([line("assistant", "still working")]),
                ),
                "session-1",
                None,
            )
            .expect("valid page")))
                .expect("output page");
        })
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("event runtime");
    runtime
        .block_on(async {
            let mut signals = super::Signals::new().expect("signal watchers");
            super::wait_for_admission(
                Arc::clone(&transport),
                "session-1",
                "hello",
                &mut signals.stop,
                &mut pages,
                |lines| {
                    assert_eq!(lines.len(), 1);
                    assert_eq!(lines[0].text, "still working");
                    let mut state = transport.state.lock().expect("admission state");
                    assert!(state.0);
                    state.1 = true;
                    transport.changed.notify_all();
                    Ok(())
                },
            )
            .await
        })
        .expect("admission follows rendered output");
    drop(held_pages);
    feeder.join().expect("output feeder joined");
    assert_eq!(
        *transport.state.lock().expect("admission state"),
        (true, true)
    );
}
