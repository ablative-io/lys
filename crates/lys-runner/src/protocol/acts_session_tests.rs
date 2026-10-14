//! Which session an act is about: what a server routes to the runner that
//! holds it, a seat owner's or the machine's own.

use super::Act;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn act(json: &serde_json::Value) -> Result<Act, serde_json::Error> {
    serde_json::from_value(json.clone())
}

/// A read, input, keys, operation, end or liveness about a session names
/// it, through an act done as a caller too; an act about no one session
/// names none. Fails if any of them is routed to the machine's runner
/// when an owner holds the session.
#[test]
fn an_act_about_a_session_names_it_and_any_other_names_none() -> TestResult {
    let read = act(&serde_json::json!({
        "act": "read", "session": "op-a1", "cursor": null, "lines": null, "bytes": null,
        "follow": false
    }));
    let named = [
        serde_json::json!({"act": "end", "session": "op-a1"}),
        serde_json::json!({"act": "keys", "session": "op-a1", "keys": ["enter"]}),
        serde_json::json!({"act": "liveness", "session": "op-a1"}),
        serde_json::json!({"act": "control_status", "session": "op-a1"}),
        serde_json::json!({"act": "as_caller", "caller": "someone",
            "done": {"act": "end", "session": "op-a1"}}),
    ];
    for json in &named {
        assert_eq!(act(json)?.session(), Some("op-a1"), "{json}");
    }
    assert_eq!(read?.session(), Some("op-a1"));
    for json in [
        serde_json::json!({"act": "owned"}),
        serde_json::json!({"act": "liveness"}),
        serde_json::json!({"act": "status"}),
    ] {
        assert_eq!(act(&json)?.session(), None, "{json}");
    }
    Ok(())
}

#[test]
fn an_upgrade_can_capture_running_launches_before_stopping_the_runner() -> TestResult {
    let request = act(&serde_json::json!({"act": "upgrade_sessions"}))?;
    assert_eq!(request.session(), None);
    Ok(())
}
