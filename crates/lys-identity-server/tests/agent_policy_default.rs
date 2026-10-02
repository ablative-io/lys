//! Initial policies preserve the reviewed program's native permission boundary.

use std::error::Error;

use identity_contract::apps::{Auth, get, login, op, post};
use identity_contract::harness::ADMINISTRATOR;
use lys_runner::judge::Policy;
use serde_json::json;

#[path = "support/agent_policy.rs"]
mod support;
use support::table;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn an_old_install_gets_only_missing_policies_and_keeps_them_after_restart() -> TestResult {
    let (mut service, [missing, explicit]) = table(true).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, answer) = get(
        &service,
        &format!("/agents/{missing}/policy"),
        Auth::Cookie(&cookie),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["policy"]["version"], 1, "{answer}");
    let policy: Policy = serde_json::from_value(answer["policy"].clone())?;
    assert!(policy.rules.is_empty());
    service.restart().await?;
    let (_, restarted) = get(
        &service,
        &format!("/agents/{missing}/policy"),
        Auth::Cookie(&cookie),
    )
    .await?;
    assert_eq!(restarted, answer);
    let (_, existing) = get(
        &service,
        &format!("/agents/{explicit}/policy"),
        Auth::Cookie(&cookie),
    )
    .await?;
    assert_eq!(existing["policy"]["version"], 2);
    assert_eq!(existing["policy"]["rules"], json!([]));
    Ok(())
}

#[tokio::test]
async fn registration_keeps_a_default_policy_before_answering_and_retry_keeps_its_version()
-> TestResult {
    let (service, _) = table(true).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let body = json!({"operation": op()?, "display_name": "New"});
    let (status, answer) = post(&service, "/agents", Auth::Cookie(&cookie), &body).await?;
    assert_eq!(status, 200, "{answer}");
    let agent = answer["agent"].as_str().ok_or("no agent")?;
    let (_, policy) = get(
        &service,
        &format!("/agents/{agent}/policy"),
        Auth::Cookie(&cookie),
    )
    .await?;
    assert_eq!(policy["policy"]["version"], 1, "{policy}");
    assert_eq!(policy["policy"]["rules"], json!([]));
    let (status, retry) = post(&service, "/agents", Auth::Cookie(&cookie), &body).await?;
    assert_eq!(status, 200, "{retry}");
    assert_eq!(retry["agent"], agent);
    let (_, again) = get(
        &service,
        &format!("/agents/{agent}/policy"),
        Auth::Cookie(&cookie),
    )
    .await?;
    assert_eq!(again, policy);
    Ok(())
}

#[tokio::test]
async fn registration_without_a_policy_store_refuses_before_creating_an_agent() -> TestResult {
    let (mut service, _) = table(true).await?;
    service
        .restart_adjusted(|config| config.policies_dir = None)
        .await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let before = service.log_size().await?;
    let (status, answer) = post(
        &service,
        "/agents",
        Auth::Cookie(&cookie),
        &json!({"operation": op()?, "display_name": "Refused"}),
    )
    .await?;
    assert_eq!(status, 503, "{answer}");
    assert_eq!(answer["refusal"], "PolicyUnavailable");
    assert_eq!(service.log_size().await?, before);
    Ok(())
}
