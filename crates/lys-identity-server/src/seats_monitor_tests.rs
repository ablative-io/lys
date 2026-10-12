//! Seat states come from the runner; retired monitor settings refuse admission.

use std::error::Error;
use std::sync::atomic::AtomicBool;

use lys_runner::{Launch, Sessions};
use serde_json::json;

use super::check;
use crate::seats_acts::Live;
use crate::seats_state::Seat;
use crate::seats_views::{Reading, view};

fn projected(live: &lys_runner::liveness::LivenessView) -> Live {
    Live {
        session: live.session.clone(),
        pid: live.pid,
        alive: live.alive,
        managed: live.managed,
        harness_session: live.harness_session.clone(),
        turn_active: live.turn_active,
        last_signal_at: live.last_signal_at,
        started_at: live.started_at,
        ended: live.ended,
    }
}

#[test]
fn the_seat_runner_says_online_offline_or_unknown() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let runner = Sessions::open(dir.path(), 4096)?;
    let session = "monitor-seat";
    let launch: Launch = serde_json::from_value(json!({
        "session": session, "program": "/bin/cat", "arguments": [],
        "directory": dir.path(), "environment": {}, "columns": 80, "rows": 24,
    }))?;
    runner.start(launch)?;
    let seat: Seat = serde_json::from_value(json!({
        "added": {"operation":"op-a", "name":"waffles", "agent":"agent",
            "harness":"claude", "profile_version":1, "machine":"op-1",
            "working_folder":null, "account":null, "by":"person", "at":1},
        "session": session, "harness_session":null, "running":true,
        "revision":2, "retired":false,
    }))?;
    for (event, expected) in [
        ("UserPromptSubmit", "online-working"),
        ("Stop", "online-idle"),
    ] {
        runner.collect(
            session,
            &lys_runner::peer::Collected::Hook {
                event: event.to_owned(),
                input: json!({}),
            },
        )?;
        let live = runner.liveness(Some(session))?;
        assert_eq!(live.len(), 1);
        assert!(live[0].alive);
        assert!(!live[0].managed);
        assert_eq!(
            view(&seat, None, &Reading::Read(Some(&projected(&live[0])))).state,
            expected
        );
    }
    runner.end(session, &AtomicBool::new(false))?;
    let dead = runner.liveness(Some(session))?;
    assert!(!dead[0].alive);
    assert!(dead[0].ended);
    assert_eq!(
        view(&seat, None, &Reading::Read(Some(&projected(&dead[0])))).state,
        "offline"
    );
    assert_eq!(
        view(&seat, None, &Reading::Read(None)).state,
        "offline"
    );
    let unknown = view(&seat, None, &Reading::Unread("runner_unreachable"));
    assert_eq!(unknown.state, "unknown");
    assert_eq!(unknown.reason.as_deref(), Some("runner_unreachable"));
    let error = runner
        .liveness(Some("absent"))
        .expect_err("unknown session reported live");
    assert_eq!(error.name(), "session_unknown");
    Ok(())
}

#[test]
fn a_seat_refuses_retired_monitor_settings_without_reading_their_values()
-> Result<(), Box<dyn Error>> {
    const KEYS: [&str; 3] = [
        "LYS_MONITOR_URL",
        "LYS_MONITOR_SECRET",
        "LYS_MONITOR_SECRET_HEADER",
    ];
    const PROBE: &str = "LYS_F2_MONITOR_PROBE";
    if let Some(key) = std::env::var_os(PROBE) {
        let key = key.to_str().ok_or("invalid probe name")?;
        assert!(KEYS.contains(&key));
        let error = check("waffles").expect_err("retired monitor setting accepted");
        assert_eq!(error.name(), "ConfigInvalid");
        let words = error.to_string();
        assert!(words.contains(key), "{words}");
        assert!(words.contains("delete the line"), "{words}");
        assert!(words.contains("liveness comes from the runner"), "{words}");
        assert!(!words.contains("private-monitor-value"), "{words}");
        return Ok(());
    }
    for key in KEYS {
        let mut child = std::process::Command::new(std::env::current_exe()?);
        child.args([
            "--exact",
            "seats_monitor::tests::a_seat_refuses_retired_monitor_settings_without_reading_their_values",
            "--nocapture",
        ]);
        for retired in KEYS {
            child.env_remove(retired);
        }
        let output = child
            .env(PROBE, key)
            .env(key, "private-monitor-value")
            .output()?;
        assert!(
            output.status.success(),
            "{key}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    }
    let note = check("waffles")?;
    assert!(note.contains("liveness is read from its runner"), "{note}");
    Ok(())
}
