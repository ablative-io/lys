//! Side-work calls leave the same session's last reported agent context in place.

use crate::budgets_state::{Held, Leaf};

use super::tests::{MODEL, call, limit, live, windows};

const SIDE_MODEL: &str = "side-model";

#[test]
fn a_later_raw_complete_side_work_call_keeps_the_same_sessions_main_context_time_and_model()
-> Result<(), String> {
    let mut held = Held::default();
    held.hold(Leaf::Used(call(
        "main-read",
        "session",
        10_000,
        Some(60_000),
    )))?;
    let mut side = call("side-call", "session", 20_000, Some(90_000));
    side.call.as_mut().ok_or("missing side call")?.model = Some(SIDE_MODEL.to_owned());
    held.hold(Leaf::Used(side))?;
    let mut declared = windows();
    declared.insert(SIDE_MODEL.to_owned(), None);
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &declared, 30_000)?;
    assert_eq!(used.figure, Some(6.into()));
    assert_eq!(used.source.as_deref(), Some("model"));
    assert_eq!(used.unavailable, None);
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.read_at_ms, Some(10_000));
    assert_eq!(context.model.as_deref(), Some(MODEL));
    assert_eq!(context.context_tokens, Some(60_000));
    assert_eq!(context.window_tokens, Some(1_000_000));
    assert_eq!(context.reported_none, 1);
    assert_eq!(context.live_sessions[0].reported_none, 1);
    Ok(())
}

#[test]
fn only_side_work_keeps_a_named_gap_and_call_count_without_creating_an_agent_context_reading()
-> Result<(), String> {
    let mut held = Held::default();
    let mut side = call("side-call", "session", 20_000, Some(90_000));
    side.call.as_mut().ok_or("missing side call")?.model = Some(SIDE_MODEL.to_owned());
    held.hold(Leaf::Used(side))?;
    let mut declared = windows();
    declared.insert(SIDE_MODEL.to_owned(), None);
    let used = held.session_context(&limit(), &live(&[("session", 1)]), &declared, 30_000)?;
    assert_eq!(used.figure, None);
    assert_eq!(used.source, None);
    assert_eq!(
        used.unavailable.as_deref(),
        Some("the call's model is declared as side work, which never sets an agent's context")
    );
    let context = used.context.ok_or("missing context")?;
    assert_eq!(context.read_at_ms, None);
    assert_eq!(context.model.as_deref(), Some(SIDE_MODEL));
    assert_eq!(context.context_tokens, None);
    assert_eq!(context.window_tokens, None);
    assert_eq!(context.reported_none, 1);
    assert_eq!(context.live_sessions[0].figure, None);
    Ok(())
}
