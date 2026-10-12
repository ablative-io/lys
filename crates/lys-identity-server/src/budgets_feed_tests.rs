#![cfg(test)]
use super::*;
use lys_runner::tracking::Figures;

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
async fn budgets_usage_get_serves_the_context_declaration_source()
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
        for (index, window, source, percent) in [(1, 1000, "profile", 5), (2, 0, "model", 25)] {
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
            assert_eq!(context["figure"], percent);
            assert_eq!(context["source"], source);
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
fn budgets_context_keeps_the_winning_source_through_ties_replacement_and_gaps()
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
    for (event, session, at, figure, source, expected, declared) in [
        (
            "first",
            "one",
            1,
            Some(60),
            Some("profile"),
            Some(60),
            Some("profile"),
        ),
        (
            "tie",
            "two",
            2,
            Some(60),
            Some("model"),
            Some(60),
            Some("profile"),
        ),
        (
            "lower",
            "one",
            3,
            Some(40),
            Some("profile"),
            Some(60),
            Some("model"),
        ),
        ("gap", "two", 4, None, None, None, None),
        (
            "restore",
            "two",
            5,
            Some(20),
            Some("model"),
            Some(40),
            Some("profile"),
        ),
        (
            "older",
            "two",
            1,
            Some(90),
            Some("model"),
            Some(40),
            Some("profile"),
        ),
    ] {
        let usage: Usage = serde_json::from_value(serde_json::json!({
            "event":event, "agent":"agent", "at_ms":at, "tokens":0, "running_ms":0,
            "session":session, "context_percent":figure, "context_source":source
        }))?;
        held.hold(Leaf::Used(usage))?;
        let answer = serde_json::to_value(held.context_availability.used(&limit, &agents, 10))?;
        assert_eq!(answer["figure"], serde_json::json!(expected), "{event}");
        assert_eq!(answer["source"], serde_json::json!(declared), "{event}");
        if event == "tie" {
            let historical = crate::budgets_usage::figure(&held, &limit, &agents, "UTC", 10, None)?;
            assert_eq!(serde_json::to_value(historical)?["source"], "profile");
        }
    }
    held.context_availability.remove("agent", "one")?;
    let answer = serde_json::to_value(held.context_availability.used(&limit, &agents, 10))?;
    assert_eq!(answer["figure"], 20);
    assert_eq!(answer["source"], "model");
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
