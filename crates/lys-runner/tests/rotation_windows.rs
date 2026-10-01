//! Structured account windows cannot be tripped by quoted terminal text.

use std::error::Error;

use lys_runner::rotation::{Rotation, RotationState};
use lys_runner::tracking_budget::PlanWindow;

type TestResult = Result<(), Box<dyn Error>>;

fn rotation() -> Result<RotationState, Box<dyn Error>> {
    Ok(RotationState::new(serde_json::from_str::<Rotation>(
        r#"{"accounts":["first","second"],"variable":"ACCOUNT","limit":{"signal":"plan_window"}}"#,
    )?)?)
}

#[test]
fn structured_limit_never_reads_quoted_terminal_words() -> TestResult {
    let state = rotation()?;
    assert!(!state.words_in("quoted: usage limit reached"));
    assert_eq!(state.longest_word(), 0);
    assert!(!state.limit_at_exit(Some(1)));
    Ok(())
}

#[test]
fn structured_limit_records_the_declared_signal_after_trip() -> TestResult {
    let mut state = rotation()?;
    state.trip();
    assert!(state.limit_at_exit(None));
    assert_eq!(state.advance(10).as_deref(), Some("second"));
    assert_eq!(state.moves()[0].by, "plan_window");
    assert!(!state.tripped());
    Ok(())
}

fn window(percent: &str, reset: u64) -> Result<PlanWindow, Box<dyn Error>> {
    Ok(PlanWindow {
        duration_minutes: 300,
        used_percent: serde_json::from_str(percent)?,
        resets_at_ms: reset,
    })
}

#[test]
fn only_a_full_live_window_trips_the_structured_limit() -> TestResult {
    let state = rotation()?;
    for percent in [
        "0",
        "40",
        "99.99",
        "99.9999999999999999999",
        "100.01",
        "-100",
    ] {
        assert!(!state.windows_in(&[window(percent, 11)?], 10), "{percent}");
    }
    for percent in ["100", "100.0", "1e2"] {
        assert!(state.windows_in(&[window(percent, 11)?], 10), "{percent}");
        assert!(!state.windows_in(&[window(percent, 10)?], 10));
        assert!(!state.windows_in(&[window(percent, 9)?], 10));
    }
    assert!(!state.windows_in(&[], 10));
    let mut invalid = window("100", 11)?;
    invalid.duration_minutes = 0;
    assert!(!state.windows_in(&[invalid], 10));
    Ok(())
}

#[test]
fn existing_limit_declarations_keep_their_own_signals() -> TestResult {
    for (signal, quoted) in [
        (r#"{"signal":"words","words":["limit"]}"#, true),
        (r#"{"signal":"exit_status","status":7}"#, false),
    ] {
        let declaration =
            format!(r#"{{"accounts":["one"],"variable":"ACCOUNT","limit":{signal}}}"#);
        let parsed: Rotation = serde_json::from_str(&declaration)?;
        assert_eq!(
            serde_json::from_str::<Rotation>(&serde_json::to_string(&parsed)?)?,
            parsed
        );
        let state = RotationState::new(parsed)?;
        assert_eq!(state.words_in("quoted limit"), quoted);
        assert!(!state.windows_in(&[window("100", 11)?], 10));
    }
    Ok(())
}
