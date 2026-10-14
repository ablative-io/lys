//! Repeated cached input is a billing component, while context belongs to one session call.

use std::collections::{BTreeMap, BTreeSet};

use crate::budgets_limits::Limit;
use crate::budgets_state::{Act, CallSeen, Held, Leaf, Length, Measure, Usage};

#[test]
fn billing_components_repeat_cached_input_but_current_context_uses_one_sessions_last_call()
-> Result<(), Box<dyn std::error::Error>> {
    let mut held = Held::default();
    let rows = [
        ("old", 1_791_945_477_347, None),
        ("old", 1_791_945_643_597, Some((2, 14, 58_707, 0))),
        ("current", 1_791_946_762_126, None),
        ("current", 1_791_962_651_252, Some((1_163, 13, 0, 0))),
        ("current", 1_791_962_651_992, Some((2, 14, 58_771, 0))),
        ("current", 1_791_962_675_783, Some((4, 59, 4_232, 54_559))),
    ];
    for (index, (session, at_ms, parts)) in rows.into_iter().enumerate() {
        let mut usage = Usage {
            event: format!("call-{index}"),
            agent: "agent".to_owned(),
            session: Some(session.to_owned()),
            at_ms,
            call: Some(Box::new(CallSeen {
                id: format!("call-{index}"),
                model: Some("claude-opus-5-5".to_owned()),
                input_tokens: parts.map(|(input, _, _, _)| input),
                output_tokens: parts.map(|(_, output, _, _)| output),
                cache_creation_tokens: parts.map(|(_, _, written, _)| written),
                cache_read_tokens: parts.map(|(_, _, _, read)| read),
                run: session.to_owned(),
                record: None,
            })),
            ..Usage::default()
        };
        if let Some((input, output, written, read)) = parts {
            usage.tokens = input + output + written + read;
        } else {
            usage.unavailable.push(lys_runner::tracking::Unavailable {
                figure: "tokens".to_owned(),
                reason: "the response reported no token components".to_owned(),
            });
        }
        held.hold(Leaf::Used(usage))?;
    }
    let limit = Limit {
        unit: Measure::Tokens,
        amount: 1_000_000.into(),
        period: Some(Length::Day),
        act: Act::Tell,
        zone: None,
    };
    let agents = BTreeSet::from(["agent".to_owned()]);
    let at_ms = 1_791_962_700_000;
    let used = crate::budgets_api::current_usage(
        &held,
        &limit,
        &agents,
        "Australia/Melbourne",
        at_ms,
        None,
    )?;
    assert_eq!(used.figure, None);
    let reported = used.reported.ok_or("missing reported billing subtotal")?;
    assert_eq!(reported.figure, 177_540.into());
    assert_eq!(reported.missing, 2);
    let components = used.tokens.ok_or("missing token components")?;
    assert_eq!(components.input_tokens.reported, Some(1_171));
    assert_eq!(components.output_tokens.reported, Some(100));
    assert_eq!(components.cache_creation_tokens.reported, Some(121_710));
    assert_eq!(components.cache_read_tokens.reported, Some(54_559));
    assert_eq!(components.input_tokens.missing_calls, 2);
    assert_eq!(components.output_tokens.missing_calls, 2);
    assert_eq!(components.cache_creation_tokens.missing_calls, 2);
    assert_eq!(components.cache_read_tokens.missing_calls, 2);
    assert_eq!(components.calls, 6);
    assert_eq!(components.sessions, 2);
    assert_eq!(components.calls_without_session, 0);
    assert!(held.crossings.of_agent("agent").is_empty());
    let context = held.session_context(
        &Limit {
            unit: Measure::ContextPercent,
            period: None,
            ..limit
        },
        &BTreeMap::from([(
            "agent".to_owned(),
            BTreeMap::from([("old".to_owned(), 1), ("current".to_owned(), 2)]),
        )]),
        &BTreeMap::from([("claude-opus-5-5".to_owned(), Some(1_000_000))]),
        at_ms,
    )?;
    assert_eq!(
        serde_json::to_value(&context)?["figure"],
        serde_json::json!(5.8795)
    );
    let reading = context.context.ok_or("missing session context")?;
    assert_eq!(reading.session.as_deref(), Some("current"));
    assert_eq!(reading.context_tokens, Some(58_795));
    assert_eq!(reading.read_at_ms, Some(1_791_962_675_783));
    assert_eq!(reading.reported_none, 0);
    assert_eq!(Held::decode(&held.encode()?)?, held);
    Ok(())
}
