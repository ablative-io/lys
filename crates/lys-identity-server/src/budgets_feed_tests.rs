#![cfg(test)]
use super::*;
use lys_runner::tracking::Figures;

#[path = "budgets_feed_ingestion_tests.rs"]
mod ingestion;

/// A proxy spend whose call had `context` tokens in use, under a profile
/// that declared `window`.
fn spend(model: Option<&str>, context: u64, window: u64) -> UsageRecord {
    UsageRecord {
        version: RECORD_VERSION,
        id: "call".to_owned(),
        runner: "0123456789abcdef0123456789abcdef".to_owned(),
        session: "session".to_owned(),
        generation: 1,
        offset: None,
        turn: None,
        observed_at: 5,
        measure: Measure::Spend,
        figures: Figures {
            input_tokens: Some(1),
            output_tokens: Some(1),
            cache_creation_tokens: Some(0),
            cache_read_tokens: Some(0),
            context_tokens: Some(context),
            ..Figures::default()
        },
        unavailable: Vec::new(),
        adapter: "lys-proxy-usage/1".to_owned(),
        model: model.map(str::to_owned),
        account: None,
        account_unknown: None,
        context_window: window,
        profile_version: 1,
        run: None,
        record: None,
    }
}

fn kept(
    record: &UsageRecord,
    windows: &Windows,
) -> Result<(Option<u64>, Vec<String>), ServerError> {
    let usage = convert(
        "machine",
        "agent",
        record,
        &crate::budgets_state::Held::default(),
        windows,
    )?;
    let said = usage
        .unavailable
        .iter()
        .filter(|gap| gap.figure == "context_percent")
        .map(|gap| gap.reason.clone())
        .collect();
    Ok((usage.context_percent, said))
}

#[test]
fn a_calls_context_is_held_against_the_window_declared_for_its_model() -> Result<(), ServerError> {
    let windows = Windows::from([("main".to_owned(), Some(200)), ("small".to_owned(), None)]);
    // The profile declaration takes precedence over the model window.
    assert_eq!(
        kept(&spend(Some("main"), 50, 0), &windows)?,
        (Some(25), vec![])
    );
    assert_eq!(
        kept(&spend(Some("main"), 50, 1000), &windows)?,
        (Some(5), vec![])
    );
    // A profile window also precedes a side-work model declaration.
    assert_eq!(
        kept(&spend(Some("small"), 50, 1000), &windows)?,
        (Some(5), vec![])
    );
    // Without a profile window, side work never sets a context.
    assert_eq!(
        kept(&spend(Some("small"), 50, 0), &windows)?,
        (
            None,
            vec![
                "the call's model is declared as side work, which never sets an agent's context"
                    .to_owned()
            ]
        )
    );
    // A model with no row falls to the profile's window, and with none there says so by name.
    assert_eq!(
        kept(&spend(Some("other"), 35, 100), &windows)?,
        (Some(35), vec![])
    );
    assert_eq!(kept(&spend(None, 35, 100), &windows)?, (Some(35), vec![]));
    assert_eq!(
        kept(&spend(Some("other"), 35, 0), &windows)?,
        (
            None,
            vec!["no context window is declared for model other".to_owned()]
        )
    );
    assert_eq!(
        kept(&spend(None, 35, 0), &windows)?,
        (
            None,
            vec!["no context window is declared for model (not named)".to_owned()]
        )
    );
    // A context larger than the window a person declared is said, and the use is still kept.
    assert_eq!(
        kept(&spend(Some("main"), 201, 0), &windows)?,
        (
            None,
            vec![
                "the call's context of 201 tokens is larger than the window of 200 declared for its model"
                    .to_owned()
            ]
        )
    );
    // One larger than the profile's own window is refused, as it was.
    assert!(kept(&spend(Some("other"), 101, 100), &windows).is_err());
    Ok(())
}

#[test]
fn budgets_feed_preserves_the_context_declaration_source() -> Result<(), Box<dyn std::error::Error>>
{
    let windows = Windows::from([("main".to_owned(), Some(200))]);
    for (window, source, percent) in [(1000, "profile", 5), (0, "model", 25)] {
        let usage = convert(
            "machine",
            "agent",
            &spend(Some("main"), 50, window),
            &crate::budgets_state::Held::default(),
            &windows,
        )?;
        let encoded = serde_json::to_value(&usage)?;
        assert_eq!(encoded["context_percent"], percent);
        assert_eq!(encoded["context_source"], source);
        let mut held = crate::budgets_state::Held::default();
        held.hold(crate::budgets_state::Leaf::Used(usage))?;
        let limit = crate::budgets_limits::Limit {
            unit: crate::budgets_state::Measure::ContextPercent,
            amount: 100.into(),
            period: None,
            act: crate::budgets_state::Act::Tell,
            zone: None,
        };
        let agents = std::collections::BTreeSet::from(["agent".to_owned()]);
        let current = held.context_availability.used(&limit, &agents, 5);
        assert_eq!(serde_json::to_value(current)?["source"], source);
        let historic = crate::budgets_usage::figure(&held, &limit, &agents, "UTC", 5, None)?;
        assert_eq!(serde_json::to_value(historic)?["source"], source);
    }
    Ok(())
}

#[tokio::test]
async fn legacy_spend_never_supplies_context_for_a_live_session_without_completed_calls()
-> Result<(), Box<dyn std::error::Error>> {
    use std::sync::Arc;
    let fixture = super::entry_tests::fixture().await?;
    fixture.track("session", "machine")?;
    let binding = lys_identity::LoginBinding::new(fixture.state.oidc.issuer(), "owner")?;
    let actor = lys_identity::Actor::new(
        binding,
        lys_identity::Provenance::new(lys_identity::AuthMethod::Oidc, 1),
    );
    let cookie = fixture.state.sessions.begin(actor)?;
    let cookie = cookie.split(';').next().ok_or("missing fixture cookie")?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = crate::budgets_act::routes().with_state(Arc::clone(&fixture.state));
    let server = tokio::spawn(async move { axum::serve(listener, app).await });
    let result = async {
        let windows = Windows::from([("main".to_owned(), Some(200))]);
        let client = reqwest::Client::new();
        for (index, window) in [(1, 1000), (2, 0)] {
            let mut record = spend(Some("main"), 50, window);
            record.id = format!("source-{index}");
            record.observed_at = index;
            let usage = convert(
                "machine",
                &fixture.agent,
                &record,
                &crate::budgets_state::Held::default(),
                &windows,
            )?;
            crate::budgets_enforce::keep(&fixture.state, usage).await?;
            let answer = client
                .get(format!("http://{address}/agents/{}/usage", fixture.agent))
                .header(reqwest::header::COOKIE, cookie)
                .send()
                .await?;
            assert_eq!(answer.status(), reqwest::StatusCode::OK);
            let answer: serde_json::Value = answer.json().await?;
            let context = answer["figures"]
                .as_array()
                .ok_or("missing figures")?
                .iter()
                .find(|figure| figure["unit"] == "context_percent")
                .ok_or("missing context")?;
            assert!(context["figure"].is_null());
            assert!(context["source"].is_null());
            assert_eq!(context["unavailable"], crate::budgets_context::NO_CONTEXT);
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;
    server.abort();
    match server.await {
        Ok(result) => result?,
        Err(error) if error.is_cancelled() => {}
        Err(error) => return Err(error.into()),
    }
    fixture.finish().await?;
    result
}

#[test]
fn budgets_context_keeps_ties_and_replacement_because_gaps_preserve_the_last_reading()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::budgets_state::{Act, Held, Leaf, Usage};
    let limit = crate::budgets_limits::Limit {
        unit: crate::budgets_state::Measure::ContextPercent,
        amount: 100.into(),
        period: None,
        act: Act::Tell,
        zone: None,
    };
    let agents = std::collections::BTreeSet::from(["agent".to_owned()]);
    let mut held = Held::default();
    for (event, session, at, figure, source, expected, declared, read_at) in context_readings() {
        let usage: Usage = serde_json::from_value(serde_json::json!({
            "event":event, "agent":"agent", "at_ms":at, "tokens":0, "running_ms":0,
            "session":session, "context_percent":figure, "context_source":source
        }))?;
        held.hold(Leaf::Used(usage))?;
        let answer = serde_json::to_value(held.context_availability.used(&limit, &agents, 10))?;
        assert_eq!(answer["figure"], serde_json::json!(expected), "{event}");
        assert_eq!(answer["source"], serde_json::json!(declared), "{event}");
        assert_eq!(answer["context"]["read_at_ms"], read_at, "{event}");
        if event == "tie" {
            let historical = crate::budgets_usage::figure(&held, &limit, &agents, "UTC", 10, None)?;
            assert_eq!(serde_json::to_value(historical)?["source"], "profile");
        }
    }
    held.context_availability.remove("agent", "one")?;
    let answer = serde_json::to_value(held.context_availability.used(&limit, &agents, 10))?;
    assert_eq!(answer["figure"], 20);
    assert_eq!(answer["source"], "model");
    assert_eq!(answer["context"]["read_at_ms"], 5);
    held.hold(Leaf::Used(Usage {
        event: "never-reported".to_owned(),
        agent: "agent".to_owned(),
        at_ms: 6,
        session: Some("never".to_owned()),
        ..Usage::default()
    }))?;
    let answer = serde_json::to_value(held.context_availability.used(&limit, &agents, 10))?;
    assert!(answer["figure"].is_null());
    assert_eq!(answer["unavailable"], crate::budgets_context::NO_CONTEXT);
    assert_eq!(answer["context"]["session"], "never");
    assert!(answer["context"]["read_at_ms"].is_null());
    held.context_availability.remove("agent", "never")?;
    let answer = serde_json::to_value(held.context_availability.used(&limit, &agents, 10))?;
    assert_eq!(answer["figure"], 20);
    assert_eq!(answer["source"], "model");
    assert_eq!(answer["context"]["read_at_ms"], 5);
    held.context_availability.remove("agent", "two")?;
    let answer = serde_json::to_value(held.context_availability.used(&limit, &agents, 10))?;
    assert!(answer["figure"].is_null());
    assert!(answer["source"].is_null());
    let old: Usage = serde_json::from_str(
        r#"{"event":"old","agent":"old-agent","at_ms":1,"tokens":0,"running_ms":0,"session":"old-session","context_percent":20}"#,
    )?;
    assert_eq!(old.context_source, None);
    Ok(())
}

#[test]
fn every_reported_subset_keeps_gaps_except_codexs_defined_zero_cache_creation()
-> Result<(), Box<dyn std::error::Error>> {
    let names = [
        "input_tokens",
        "output_tokens",
        "cache_creation_tokens",
        "cache_read_tokens",
    ];
    for adapter in ["lys-proxy-usage/1", lys_runner::tracking::CODEX_ADAPTER] {
        for supplied in [[2_u64, 3, 5, 7], [0; 4]] {
            for mask in 0_u8..16 {
                let values: [Option<u64>; 4] = std::array::from_fn(|index| {
                    (mask & (1 << index) != 0).then_some(supplied[index])
                });
                let mut record = spend(Some("main"), 50, 100);
                record.adapter = adapter.to_owned();
                record.run = Some(record.runner.clone());
                record.figures.input_tokens = values[0];
                record.figures.output_tokens = values[1];
                record.figures.cache_creation_tokens = values[2];
                record.figures.cache_read_tokens = values[3];
                let usage = convert(
                    "machine",
                    "agent",
                    &record,
                    &crate::budgets_state::Held::default(),
                    &Windows::new(),
                )?;
                assert_eq!(usage.tokens, values.into_iter().flatten().sum::<u64>());
                let gaps: Vec<_> = usage
                    .unavailable
                    .iter()
                    .filter(|gap| gap.figure == "tokens")
                    .map(|gap| gap.reason.clone())
                    .collect();
                let expected: Vec<_> = names
                    .iter()
                    .zip(values)
                    .filter(|(name, value)| {
                        value.is_none()
                            && !(adapter == lys_runner::tracking::CODEX_ADAPTER
                                && **name == "cache_creation_tokens")
                    })
                    .map(|(name, _)| format!("native response does not report {name}"))
                    .collect();
                assert_eq!(gaps, expected, "adapter={adapter} mask={mask}");
                let call = usage.call.ok_or("converted spend has no call")?;
                assert_eq!(
                    [
                        call.input_tokens,
                        call.output_tokens,
                        call.cache_creation_tokens,
                        call.cache_read_tokens
                    ],
                    {
                        let mut expected = values;
                        if adapter == lys_runner::tracking::CODEX_ADAPTER {
                            expected[2] = expected[2].or(Some(0));
                        }
                        expected
                    },
                    "adapter={adapter} mask={mask}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn reported_token_components_refuse_checked_overflow_even_when_other_components_are_missing()
-> Result<(), Box<dyn std::error::Error>> {
    for values in [
        [Some(u64::MAX), Some(1), None, None],
        [None, Some(u64::MAX), Some(1), None],
        [None, None, Some(u64::MAX), Some(1)],
    ] {
        let mut record = spend(Some("main"), 50, 100);
        record.figures.input_tokens = values[0];
        record.figures.output_tokens = values[1];
        record.figures.cache_creation_tokens = values[2];
        record.figures.cache_read_tokens = values[3];
        let error = convert(
            "machine",
            "agent",
            &record,
            &crate::budgets_state::Held::default(),
            &Windows::new(),
        )
        .err()
        .ok_or("reported token component sum did not overflow")?;
        assert!(error.to_string().contains("native token spend overflows"));
    }
    Ok(())
}

#[test]
fn partial_calls_keep_their_independent_components_and_gaps_through_a_budget_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::budgets_state::{Act, Held, Leaf, Length};
    let mut held = Held::default();
    for (id, values) in [
        ("first", [Some(11), None, Some(0), Some(7)]),
        ("second", [None, Some(3), None, Some(5)]),
    ] {
        let mut record = spend(Some("main"), 50, 100);
        record.id = id.to_owned();
        record.run = Some(record.runner.clone());
        record.figures.input_tokens = values[0];
        record.figures.output_tokens = values[1];
        record.figures.cache_creation_tokens = values[2];
        record.figures.cache_read_tokens = values[3];
        let usage = convert("machine", "agent", &record, &held, &Windows::new())?;
        held.hold(Leaf::Used(usage.clone()))?;
        held.hold(Leaf::Used(usage))?;
    }
    let held = Held::decode(&held.encode()?)?;
    assert_eq!(held.uses.len(), 2);
    assert_eq!(held.reported_calls("agent")?, 2);
    assert_eq!(held.uses.iter().map(|usage| usage.tokens).sum::<u64>(), 26);
    assert_eq!(
        held.uses[0].unavailable[0].reason,
        "native response does not report output_tokens"
    );
    assert_eq!(
        held.uses[1]
            .unavailable
            .iter()
            .map(|gap| gap.reason.as_str())
            .collect::<Vec<_>>(),
        [
            "native response does not report input_tokens",
            "native response does not report cache_creation_tokens"
        ]
    );
    let limit = crate::budgets_limits::Limit {
        unit: crate::budgets_state::Measure::Tokens,
        amount: 100.into(),
        period: Some(Length::Day),
        act: Act::Tell,
        zone: None,
    };
    let parts = held.token_components(
        &limit,
        &std::collections::BTreeSet::from(["agent".to_owned()]),
        "UTC",
        5,
    )?;
    assert_eq!(parts.calls, 2);
    assert_eq!(parts.sessions, 1);
    for (part, reported, missing) in [
        (parts.input_tokens, Some(11), 1),
        (parts.output_tokens, Some(3), 1),
        (parts.cache_creation_tokens, Some(0), 1),
        (parts.cache_read_tokens, Some(12), 0),
    ] {
        assert_eq!(part.reported, reported);
        assert_eq!(part.missing_calls, missing);
    }
    Ok(())
}

type ContextCase = (
    &'static str,
    &'static str,
    i64,
    Option<u64>,
    Option<&'static str>,
    Option<u64>,
    Option<&'static str>,
    i64,
);
fn context_readings() -> Vec<ContextCase> {
    vec![
        (
            "first",
            "one",
            1,
            Some(60),
            Some("profile"),
            Some(60),
            Some("profile"),
            1,
        ),
        (
            "tie",
            "two",
            2,
            Some(60),
            Some("model"),
            Some(60),
            Some("profile"),
            1,
        ),
        (
            "lower",
            "one",
            3,
            Some(40),
            Some("profile"),
            Some(60),
            Some("model"),
            2,
        ),
        ("gap", "two", 4, None, None, Some(60), Some("model"), 2),
        (
            "restore",
            "two",
            5,
            Some(20),
            Some("model"),
            Some(40),
            Some("profile"),
            3,
        ),
        (
            "older",
            "two",
            1,
            Some(90),
            Some("model"),
            Some(40),
            Some("profile"),
            3,
        ),
    ]
}
