//! Usage cannot charge or operate on another agent's session, even after reopening.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{Login, Table, TestResult, Value, json, now_ms, operation};

fn bytes_under(path: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, Box<dyn std::error::Error>> {
    let mut held = BTreeMap::new();
    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            held.extend(bytes_under(&path)?);
        } else {
            held.insert(path.clone(), std::fs::read(path)?);
        }
    }
    Ok(held)
}

#[tokio::test(flavor = "multi_thread")]
async fn foreign_found_and_unknown_usage_sessions_leave_no_charge_or_runner_act() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let victim = table.start().await?;
    let attacker = table.seeded.people[1].agents[0].id.to_string();
    let bea = table
        .service
        .sign_in(Login {
            subject: "bea-subject".to_owned(),
            email: "shared@example.test".to_owned(),
        })
        .await?;
    let cap = json!({ "measure": "tokens", "limit": 1, "act": "stop", "version": 0,
        "period": { "length": "day", "zone": "UTC" } });
    let (status, body) = table
        .call(
            reqwest::Method::PUT,
            &format!("/budgets/agent/{attacker}"),
            Some(&cap),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let found = operation()?;
    table
        .ok(
            &format!("/runtime/found/{found}/reports"),
            &json!({
                "operation": operation()?, "machine": table.machine, "state": "running",
                "what": "unidentified process", "confirmation": "",
            }),
        )
        .await?;
    let budget_path = table.service.dir.path().join("budgets");
    let before = bytes_under(&budget_path)?;
    let mut refused = 0;
    for session in [&victim, &found, &operation()?] {
        let (status, body) = table
            .service
            .post(
                &format!("/agents/{attacker}/usage"),
                Some(&bea),
                &json!({ "event": "reusable", "at_ms": now_ms(), "tokens": 5,
                "session": session, "context_percent": 99 }),
            )
            .await?;
        assert_eq!(status, 400, "{body}");
        assert_eq!(body["refusal"], "RequestMalformed", "{body}");
        let reason = body["reason"].as_str().ok_or("no reason")?;
        assert!(
            reason.contains(session) && reason.contains(&attacker),
            "{reason}"
        );
        assert_eq!(
            bytes_under(&budget_path)?,
            before,
            "no charge, crossing or dedupe write"
        );
        refused += 1;
    }
    assert_eq!(refused, 3);
    table.service.restart().await?;
    let (_, usage) = table
        .service
        .get(&format!("/agents/{attacker}/usage"), Some(&bea))
        .await?;
    assert_eq!(usage["last_reported_ms"], Value::Null, "{usage}");
    assert_eq!(usage["receipts"], json!([]), "{usage}");
    table
        .ok(
            &format!("/runtime/sessions/{victim}/input"),
            &json!({ "text": "echo victim-still-running", "enter": true }),
        )
        .await?;
    table.waited(&victim, "victim-still-running").await?;
    table.end(&victim).await?;
    table.stop_runner()
}
