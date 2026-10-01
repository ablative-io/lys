//! Structured account windows cannot be tripped by quoted terminal text.

use std::error::Error;

use lys_runner::rotation::{Rotation, RotationState};

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
