//! Reported cost deltas and account windows retain precision, attribution and replay safety.

use std::error::Error;

use lys_runner::tracking::{
    Accounts, CLAUDE_ADAPTER, CODEX_ADAPTER, Harness, Reading, Tracking, measured, status_figures,
    version_in,
};
use lys_runner::tracking_store::{Body, SourceState};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn status(cost: &Value, limits: &Value) -> Value {
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
    let (figures, _) = status_figures(&status(&json!(400.123_456), &Value::Null));
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
    let first = status(&json!(400), &Value::Null);
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
    let changed = status(&json!(500), &Value::Null);
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
        &json!(0),
        &json!({
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
            {"duration_minutes": 10_080, "used_percent": 50, "resets_at_ms": 1_800_604_800_000_i64}
        ]),
        "{encoded}"
    );
    Ok(())
}

#[test]
fn absent_currency_and_plan_reports_are_named_unavailable_instead_of_zero() -> TestResult {
    let (figures, unavailable) = status_figures(&status(&Value::Null, &Value::Null));
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

#[test]
fn an_omitted_cost_report_does_not_erase_the_previous_cumulative_baseline() -> TestResult {
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
    reading
        .status(
            &mut source,
            &status(&json!(400), &Value::Null),
            "first".to_owned(),
        )
        .ok_or("no first report")?;
    reading
        .status(
            &mut source,
            &status(&Value::Null, &Value::Null),
            "omitted".to_owned(),
        )
        .ok_or("no omitted report")?;
    let mut restored: SourceState = serde_json::from_slice(&serde_json::to_vec(&source)?)?;
    let next = usage(
        reading
            .status(
                &mut restored,
                &status(&json!(500), &Value::Null),
                "next".to_owned(),
            )
            .ok_or("no next report")?,
    )?;
    assert_eq!(next["figures"]["dollars_micros"], 100_000_000, "{next}");
    Ok(())
}

#[test]
fn a_cost_reset_is_named_and_never_becomes_a_negative_or_repeated_charge() -> TestResult {
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
    reading
        .status(
            &mut source,
            &status(&json!(400), &Value::Null),
            "first".to_owned(),
        )
        .ok_or("no first report")?;
    let reset = usage(
        reading
            .status(
                &mut source,
                &status(&json!(10), &Value::Null),
                "reset".to_owned(),
            )
            .ok_or("no reset report")?,
    )?;
    assert_eq!(reset["figures"]["dollars_micros"], 10_000_000);
    assert!(
        reset["unavailable"]
            .as_array()
            .ok_or("no unavailable reasons")?
            .iter()
            .any(|entry| entry["reason"] == "reported_session_cost_reset"),
        "{reset}"
    );
    let next = usage(
        reading
            .status(
                &mut source,
                &status(&json!(15), &Value::Null),
                "next".to_owned(),
            )
            .ok_or("no next report")?,
    )?;
    assert_eq!(next["figures"]["dollars_micros"], 5_000_000);
    Ok(())
}

#[test]
fn expired_windows_are_unavailable_even_when_the_source_repeats_them() -> TestResult {
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
    let input = status(
        &json!(0),
        &json!({"seven_day": {"used_percentage": 50, "resets_at": 1_800_000_000}}),
    );
    let record = usage(
        reading
            .status(&mut SourceState::default(), &input, "expired".to_owned())
            .ok_or("no expiration record")?,
    )?;
    assert_eq!(record["figures"]["plan_windows"], json!([]));
    assert!(
        record["unavailable"]
            .as_array()
            .ok_or("no unavailable reasons")?
            .iter()
            .any(|entry| entry["reason"] == "10080_minute_window_expired"),
        "{record}"
    );
    Ok(())
}

#[test]
fn codex_reads_explicit_window_durations_without_a_token_increase() -> TestResult {
    let contract = Tracking {
        harness: Harness::Codex,
        adapter: CODEX_ADAPTER.to_owned(),
        version: "0.161.0-alpha.3".to_owned(),
        ..tracking()
    };
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
    let input = json!({"type": "event_msg", "payload": {"type": "token_count", "info": null, "rate_limits": {
        "primary": {"used_percent": 50, "window_minutes": 10_080, "resets_at": 1_800_604_800},
        "secondary": {"used_percent": 25, "window_minutes": 300, "resets_at": 1_800_003_600}
    }}});
    let mut source = SourceState::default();
    let bodies = reading.codex(&mut source, 10, &input);
    assert_eq!(bodies.len(), 1);
    let record = usage(bodies.into_iter().next().ok_or("no plan record")?)?;
    assert_eq!(
        record["figures"]["plan_windows"][0]["duration_minutes"],
        10_080
    );
    assert_eq!(
        record["figures"]["plan_windows"][1]["duration_minutes"],
        300
    );
    assert_eq!(record["figures"]["dollars_micros"], Value::Null);
    assert!(
        record["unavailable"]
            .as_array()
            .ok_or("no unavailable reasons")?
            .iter()
            .any(|entry| entry["reason"]
                == "Codex reports dollars only through its app-server; Lys does not read it yet"),
        "{record}"
    );
    assert!(reading.codex(&mut source, 20, &input).is_empty());
    Ok(())
}
