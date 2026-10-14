#![cfg(test)]

use std::sync::{Arc, Condvar, Mutex};

use serde_json::{Value, json};

use super::{Signals, Transport, dimensions, finish_resize, follow, request};
use crate::commands::error::CliResult;
use crate::commands::launch_terminal::{delivered, end_session};
use crate::commands::seat_client::refused;

#[derive(Default)]
struct State {
    following: bool,
    ended: bool,
    calls: Vec<(String, Value)>,
}

#[derive(Default)]
struct Signalled {
    state: Mutex<State>,
    changed: Condvar,
}

impl Signalled {
    fn until_following(&self) {
        let state = self.state.lock().expect("state lock");
        drop(
            self.changed
                .wait_while(state, |state| !state.following)
                .expect("follow signal"),
        );
    }
}

impl Transport for Signalled {
    fn post(&self, path: &str, body: &Value) -> CliResult<Value> {
        let mut state = self.state.lock().expect("state lock");
        state.calls.push((path.to_owned(), body.clone()));
        let ended = json!({"how": "exited", "at": 1, "status": 0, "signal": null});
        if path.ends_with("/read-bytes") {
            state.following = true;
            self.changed.notify_all();
            let ended_state = self
                .changed
                .wait_while(state, |state| !state.ended)
                .expect("end signal");
            assert!(ended_state.ended);
            return Ok(json!({"answer": {"kind": "bytes", "output": {
                "session": "session-1", "from": 0, "cursor": 0, "oldest": 0, "data": [], "ended": ended,
            }}}));
        }
        if path.ends_with("/end") {
            state.ended = true;
            self.changed.notify_all();
            return Ok(
                json!({"answer": {"kind": "ended", "session": "session-1", "ended": ended}}),
            );
        }
        Ok(json!({"answer": {"kind": "delivered", "session": "session-1"}}))
    }
}

#[test]
fn blocked_follow_keeps_input_and_resize_available_and_end_wakes_the_reader() {
    let transport = Arc::new(Signalled::default());
    let (mut pages, reader) = follow(Arc::clone(&transport), "session-1").expect("start reader");
    transport.until_following();
    for (verb, body) in [
        ("input-bytes", json!({"data": [0, 3, 255]})),
        ("resize", json!({"columns": 101, "rows": 37})),
    ] {
        let route = format!("/runtime/sessions/session-1/{verb}");
        let reply = request(Arc::clone(&transport), route, body)
            .expect("start control request")
            .blocking_recv()
            .expect("control answer")
            .expect("control success");
        delivered(reply, "session-1", verb).expect("confirmed delivery");
    }
    end_session(transport.as_ref(), "session-1").expect("confirmed end");
    let page = pages
        .blocking_recv()
        .expect("reader answer")
        .expect("read success");
    assert!(page.ended.is_some());
    reader.join().expect("reader joined");
    let state = transport.state.lock().expect("state lock");
    assert_eq!(state.calls.len(), 4);
    assert_eq!(state.calls[0].1, json!({"cursor": null, "follow": true}));
    assert_eq!(state.calls[1].1, json!({"data": [0, 3, 255]}));
    assert_eq!(state.calls[2].1, json!({"columns": 101, "rows": 37}));
    assert_eq!(state.calls[3].1, json!({}));
}

#[derive(Default)]
struct Refusing {
    calls: std::sync::atomic::AtomicUsize,
}

impl Transport for Refusing {
    fn post(&self, path: &str, body: &Value) -> CliResult<Value> {
        assert!(path.ends_with("/read-bytes"));
        assert_eq!(body, &json!({"cursor": null, "follow": true}));
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Err(refused(
            "runner_disconnected",
            "the machine has no connected runner",
        ))
    }
}

#[test]
fn follower_preserves_transport_failure_and_exits_without_a_retry() {
    let transport = Arc::new(Refusing::default());
    let (mut pages, reader) = follow(Arc::clone(&transport), "session-1").expect("start reader");
    let error = pages
        .blocking_recv()
        .expect("failure answer")
        .expect_err("transport refusal");
    assert!(error.to_string().contains("runner_disconnected"));
    assert!(pages.blocking_recv().is_none());
    reader.join().expect("reader joined");
    assert_eq!(transport.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn terminal_window_must_supply_both_dimensions_without_defaults() {
    assert_eq!(dimensions(121, 42).expect("real dimensions"), (121, 42));
    for (columns, rows) in [(0, 42), (121, 0), (0, 0)] {
        assert!(
            dimensions(columns, rows)
                .expect_err("missing dimensions")
                .to_string()
                .contains("launch_terminal_size_missing")
        );
    }
}

struct ImmediatelyEnded;

impl Transport for ImmediatelyEnded {
    fn post(&self, path: &str, body: &Value) -> CliResult<Value> {
        if path.ends_with("/resize") {
            assert_eq!(body, &json!({"columns": 80, "rows": 24}));
            return Err(refused("session_ended", "the process has already exited"));
        }
        assert!(path.ends_with("/read-bytes"));
        assert_eq!(body, &json!({"cursor": null, "follow": true}));
        Ok(json!({"answer": {"kind": "bytes", "output": {
            "session": "session-1", "from": 0, "cursor": 5, "oldest": 0, "data": [104, 105, 255, 13, 10],
            "ended": {"how": "exited", "at": 1, "status": 0, "signal": null},
        }}}))
    }
}

#[test]
fn an_immediate_normal_exit_preserves_final_bytes_when_resize_observes_session_ended() {
    let transport = Arc::new(ImmediatelyEnded);
    let (mut pages, reader) =
        follow(Arc::clone(&transport), "session-1").expect("start final reader");
    let resized = transport
        .post(
            "/runtime/sessions/session-1/resize",
            &json!({"columns": 80, "rows": 24}),
        )
        .map(drop);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("event runtime");
    let mut output = Vec::new();
    let ended = runtime
        .block_on(async {
            let mut signals = Signals::new().expect("signal watchers");
            finish_resize(resized, &mut pages, &mut signals.stop, |data| {
                output.extend_from_slice(data);
                Ok(())
            })
            .await
        })
        .expect("observed successful exit");
    assert!(ended);
    assert_eq!(output, [104, 105, 255, 13, 10]);
    reader.join().expect("final reader joined");
}
