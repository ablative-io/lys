use std::error::Error;

use lys_runner::tracking::{
    Accounts, CLAUDE_ADAPTER, CODEX_ADAPTER, Harness, Reading, Tracking, measured, status_figures,
    version_in,
};
use lys_runner::tracking_store::{Body, SourceState};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn status(cost: Value, limits: Value) -> Value {
    json!({
        "session_id": "native-session",
        "cost": {"total_cost_usd": cost, "total_duration_ms": 100},
        "context_window": {"current_usage": {
            "input_tokens": 10, "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0
        }},
        "rate_limits": limits
    })
}

fn tracking() -> Tracking {
    Tracking {
        harness: Harness::ClaudeCode,
        adapter: CLAUDE_ADAPTER.to_owned(),
        version: "2.1.285".to_owned(),
        config_home: "/isolated-home".to_owned(),
        context_window: 200_000,
        profile_version: 1,
        account: Some("shared-account".to_owned()),
        requires_pre_tool: false,
    }
}

fn usage(body: Body) -> Result<Value, Box<dyn Error>> {
    let Body::Usage(record) = body else {
        return Err("the status line produced no usage record".into());
    };
    Ok(serde_json::to_value(record)?)
}

#[test]
fn the_native_versions_are_admitted_without_losing_the_codex_suffix() -> TestResult {
    measured(CLAUDE_ADAPTER, "2.1.285")?;
    measured(CODEX_ADAPTER, "0.161.0-alpha.3")?;
    assert_eq!(
        version_in("codex-cli 0.161.0-alpha.3"),
        Some("0.161.0-alpha.3".to_owned())
    );
    Ok(())
}

#[test]
fn reported_dollars_retain_microdollar_precision() -> TestResult {
    let (figures, _) = status_figures(&status(json!(400.123456), Value::Null));
    let encoded = serde_json::to_value(figures)?;
    assert_eq!(encoded["dollars_micros"], 400_123_456, "{encoded}");
    Ok(())
}

#[test]
fn dollars_are_differences_of_reported_session_totals_and_replays_add_nothing() -> TestResult {
    let contract = tracking();
    let reading = Reading {
        runner: "runner",
        session: "session",
        tracking: &contract,
        accounts: Accounts {
            current: None,
            moves: &[],
            declared: Some("shared-account"),
        },
        now: 1_800_000_000_000,
    };
    let mut source = SourceState::default();
    let first = status(json!(400), Value::Null);
    let initial = usage(
        reading
            .status(&mut source, &first, "first".to_owned())
            .ok_or("no first record")?,
    )?;
    assert_eq!(
        initial["figures"]["dollars_micros"], 400_000_000,
        "{initial}"
    );
    assert!(
        reading
            .status(&mut source, &first, "repeat".to_owned())
            .is_none()
    );
    let changed = status(json!(500), Value::Null);
    let next = usage(
        reading
            .status(&mut source, &changed, "next".to_owned())
            .ok_or("no cost-change record")?,
    )?;
    assert_eq!(next["figures"]["dollars_micros"], 100_000_000, "{next}");
    assert_eq!(next["account"], "shared-account");
    assert!(
        reading
            .status(&mut source, &changed, "repeat-next".to_owned())
            .is_none()
    );
    let mut restored: SourceState = serde_json::from_slice(&serde_json::to_vec(&source)?)?;
    assert!(
        reading
            .status(&mut restored, &changed, "after-restart".to_owned())
            .is_none()
    );
    Ok(())
}

#[test]
fn plan_windows_are_reported_account_levels_with_their_reset_instants() -> TestResult {
    let input = status(
        json!(0),
        json!({
            "five_hour": {"used_percentage": 42.5, "resets_at": 1_800_003_600},
            "seven_day": {"used_percentage": 50, "resets_at": 1_800_604_800}
        }),
    );
    let (figures, _) = status_figures(&input);
    let encoded = serde_json::to_value(figures)?;
    assert_eq!(
        encoded["plan_windows"],
        json!([
            {"duration_minutes": 300, "used_percent": 42.5, "resets_at_ms": 1_800_003_600_000_i64},
            {"duration_minutes": 10080, "used_percent": 50, "resets_at_ms": 1_800_604_800_000_i64}
        ]),
        "{encoded}"
    );
    Ok(())
}

#[test]
fn absent_currency_and_plan_reports_are_named_unavailable_instead_of_zero() -> TestResult {
    let (figures, unavailable) = status_figures(&status(Value::Null, Value::Null));
    let encoded = serde_json::to_value(figures)?;
    assert_eq!(encoded["dollars_micros"], Value::Null, "{encoded}");
    assert_eq!(encoded["plan_windows"], json!([]), "{encoded}");
    for figure in ["dollars_micros", "plan_windows"] {
        assert!(
            unavailable
                .iter()
                .any(|entry| entry.figure == figure && !entry.reason.is_empty()),
            "{unavailable:?}"
        );
    }
    Ok(())
}
