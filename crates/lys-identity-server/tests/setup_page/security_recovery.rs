//! Recovery is auditable without exposing bearer codes or replacing factors.

use super::{CODE, EMAIL, PASSWORD, TestResult, administrator, post_raw, table, write_code};
use identity_contract::fake_rauthy::API_KEY;
use serde_json::json;

#[tokio::test]
async fn a_completed_password_reset_records_security_events_visible_to_the_administrator()
-> TestResult {
    let table = table().await?;
    let (status, text, cookie) = post_raw(
        &table.service,
        "/setup/administrator",
        &administrator(CODE, EMAIL)?,
    )
    .await?;
    assert_eq!(status, 200, "{text}");
    let cookie = cookie.ok_or("setup did not begin a session")?;
    write_code(&table.code_file, "password", "new-recovery-code")?;
    let (status, body) = table
        .service
        .post(
            "/setup/password",
            None,
            &json!({
                "code":"new-recovery-code", "password":"Another-Password-99"
            }),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let (status, events) = table.service.get("/security-events", Some(&cookie)).await?;
    assert_eq!(status, 200, "{events}");
    assert_eq!(events["administrator"], true);
    let kinds: std::collections::BTreeSet<_> = events["events"]
        .as_array()
        .ok_or("security events missing")?
        .iter()
        .filter_map(|event| event["kind"].as_str())
        .collect();
    assert_eq!(
        kinds,
        std::collections::BTreeSet::from(["password_reset_started", "password_reset_completed"])
    );
    for secret in ["new-recovery-code", "Another-Password-99", PASSWORD] {
        assert!(!events.to_string().contains(secret));
    }
    assert_eq!(table.service.get("/security-events", None).await?.0, 401);
    assert!(!table.code_file.exists());
    Ok(())
}

#[tokio::test]
async fn an_unwritable_security_event_store_refuses_recovery_before_the_password_changes()
-> TestResult {
    let table = table().await?;
    let (status, text, _) = post_raw(
        &table.service,
        "/setup/administrator",
        &administrator(CODE, EMAIL)?,
    )
    .await?;
    assert_eq!(status, 200, "{text}");
    write_code(&table.code_file, "password", "new-recovery-code")?;
    std::fs::write(
        table.code_file.with_file_name("recovery-events"),
        b"not an event directory",
    )?;
    let (status, body) = table
        .service
        .post(
            "/setup/password",
            None,
            &json!({
                "code":"new-recovery-code","password":"Another-Password-99"
            }),
        )
        .await?;
    assert_eq!(status, 503, "{body}");
    assert_eq!(body["refusal"], "SetupUnavailable");
    assert!(
        body["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("RecoveryEventsUnavailable"))
    );
    assert!(table.code_file.exists());
    assert!(
        !table
            .service
            .sign_in_with(EMAIL, PASSWORD)
            .await?
            .is_empty()
    );
    Ok(())
}

#[tokio::test]
async fn a_completed_reset_is_recorded_and_spent_even_when_the_account_cannot_sign_in() -> TestResult
{
    let table = table().await?;
    let (status, text, cookie) = post_raw(
        &table.service,
        "/setup/administrator",
        &administrator(CODE, EMAIL)?,
    )
    .await?;
    assert_eq!(status, 200, "{text}");
    let cookie = cookie.ok_or("setup did not begin a session")?;
    let mut user = table
        .rauthy
        .users()?
        .into_iter()
        .next()
        .ok_or("administrator missing")?;
    let id = user["id"]
        .as_str()
        .ok_or("administrator id missing")?
        .to_owned();
    user["enabled"] = json!(false);
    let disabled = reqwest::Client::new()
        .put(format!("{}/users/{id}", table.rauthy.api()))
        .header(reqwest::header::AUTHORIZATION, format!("API-Key {API_KEY}"))
        .json(&user)
        .send()
        .await?;
    assert_eq!(disabled.status(), 200);
    write_code(&table.code_file, "password", "new-recovery-code")?;
    let (status, body) = table
        .service
        .post(
            "/setup/password",
            None,
            &json!({"code":"new-recovery-code","password":"Another-Password-99"}),
        )
        .await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "SignInRefused");
    assert!(
        !table.code_file.exists(),
        "the issuer accepted the password before sign-in was refused"
    );
    let (status, events) = table.service.get("/security-events", Some(&cookie)).await?;
    assert_eq!(status, 200, "{events}");
    assert!(
        events["events"]
            .as_array()
            .ok_or("events missing")?
            .iter()
            .any(|event| event["kind"] == "password_reset_completed")
    );
    Ok(())
}
