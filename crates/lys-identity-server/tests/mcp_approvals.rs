#![cfg(test)]
//! Approval copies a reviewed server only within the caller's reach and remit.

#[path = "support/harness_description.rs"]
mod harness_description;
#[path = "support/mcp_approvals.rs"]
mod support;

use lys_identity_server::launch_template::{Start, from_template, render};
use serde_json::{Value, json};
use support::{Fixture, TestResult, approval, operation, server};

#[tokio::test]
async fn a_lead_approves_from_its_own_latest_reviewed_profile() -> TestResult {
    let fixture = Fixture::new().await?;
    let asked = fixture.ask("dot").await?;
    let body = approval(&operation()?);
    let approved = fixture
        .approve(
            asked["id"].as_str().ok_or("no request id")?,
            &fixture.lead_cookie,
            &body,
            200,
        )
        .await?;
    assert_eq!(approved["id"], asked["id"]);
    assert_eq!(approved["profile_version"], 1);
    assert_eq!(approved["state"], "approved");
    assert_eq!(approved["decision"]["by"], fixture.lead);
    assert_eq!(approved["decision"]["note"], body["note"]);
    assert_eq!(approved["decision"]["profile_version"], 2);
    assert!(
        approved["decision"]["decided_at"]
            .as_u64()
            .is_some_and(|at| at > 0)
    );
    let versions = fixture.versions()?;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[1].number, 2);
    assert_eq!(versions[1].set_by, fixture.lead);
    assert_eq!(
        versions[1]
            .reviewed
            .as_ref()
            .ok_or("new version not reviewed")?
            .by,
        fixture.lead
    );
    assert_eq!(
        serde_json::to_value(&versions[1].settings.mcp_servers)?,
        json!([server("https://lead.example.test/mcp")])
    );
    let mut expected = versions[0].settings.clone();
    expected.mcp_servers = versions[1].settings.mcp_servers.clone();
    assert_eq!(versions[1].settings, expected);
    let (status, listed) = fixture
        .service
        .get(
            &format!("/agents/{}/mcp-requests", fixture.target),
            Some(&fixture.administrator),
        )
        .await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed, json!({"requests": [approved]}));
    Ok(())
}

#[tokio::test]
async fn a_server_outside_agent_remit_names_approver_target_and_server() -> TestResult {
    let fixture = Fixture::new().await?;
    let before = fixture.versions()?;
    let asked = fixture.ask("waffles").await?;
    let refused = fixture
        .approve(
            asked["id"].as_str().ok_or("no request id")?,
            &fixture.narrow_cookie,
            &approval(&operation()?),
            403,
        )
        .await?;
    assert_eq!(refused["refusal"], "mcp_beyond_remit", "{refused}");
    let reason = refused["reason"].as_str().ok_or("no refusal reason")?;
    for name in [&fixture.narrow, &fixture.target, "waffles"] {
        assert!(reason.contains(name), "{refused}");
    }
    assert_eq!(fixture.versions()?, before);
    Ok(())
}

#[tokio::test]
async fn approval_preserves_the_previous_profile_version_across_reopen() -> TestResult {
    let mut fixture = Fixture::new().await?;
    let before = fixture.versions()?;
    let asked = fixture.ask("dot").await?;
    let id = asked["id"].as_str().ok_or("no request id")?;
    let body = approval(&operation()?);
    let approved = fixture
        .approve(id, &fixture.lead_cookie, &body, 200)
        .await?;
    fixture.service.restart().await?;
    let after = fixture.versions()?;
    assert_eq!(after.len(), 2);
    assert_eq!(after[0], before[0]);
    let replay = fixture
        .approve(id, &fixture.lead_cookie, &body, 200)
        .await?;
    assert_eq!(replay, approved);
    assert_eq!(fixture.versions()?, after);
    Ok(())
}

#[tokio::test]
async fn the_approved_profile_renders_the_server_in_native_mcp_config() -> TestResult {
    let fixture = Fixture::new().await?;
    let asked = fixture.ask("dot").await?;
    fixture
        .approve(
            asked["id"].as_str().ok_or("no request id")?,
            &fixture.lead_cookie,
            &approval(&operation()?),
            200,
        )
        .await?;
    let versions = fixture.versions()?;
    let version = versions.last().ok_or("no approved version")?;
    let rendered = render(
        &Start {
            agent: &fixture.target,
            session: "rendered-session",
            machine: "rendered-machine",
            runtime: "unused-runtime",
            version,
            skills: &[],
            policy: None,
        },
        &[],
    )?;
    let launch = from_template(version, &rendered.template, &rendered.template_sha256)?;
    let file = launch
        .files
        .iter()
        .find(|file| file.path == "mcp.json")
        .ok_or("no mcp config")?;
    let config: Value = serde_json::from_str(&file.text)?;
    assert_eq!(
        config["mcpServers"]["dot"]["url"],
        "https://lead.example.test/mcp"
    );
    assert!(
        launch
            .arguments
            .windows(2)
            .any(|args| args == ["--channels", "server:dot"])
    );
    Ok(())
}

#[tokio::test]
async fn an_outside_approver_is_refused_before_server_remit_is_examined() -> TestResult {
    let fixture = Fixture::new().await?;
    let before = fixture.versions()?;
    for server in ["dot", "waffles"] {
        let asked = fixture.ask(server).await?;
        let refused = fixture
            .approve(
                asked["id"].as_str().ok_or("no request id")?,
                &fixture.outside_cookie,
                &approval(&operation()?),
                404,
            )
            .await?;
        assert_eq!(refused["refusal"], "AgentNotVisible", "{refused}");
    }
    assert_eq!(fixture.versions()?, before);
    Ok(())
}

#[tokio::test]
async fn approval_operations_replay_exactly_and_refuse_unknown_or_decided_requests() -> TestResult {
    let fixture = Fixture::new().await?;
    let unknown = fixture
        .approve(
            &operation()?,
            &fixture.lead_cookie,
            &approval(&operation()?),
            404,
        )
        .await?;
    assert_eq!(unknown["refusal"], "RequestUnknown", "{unknown}");
    let asked = fixture.ask("dot").await?;
    let id = asked["id"].as_str().ok_or("no request id")?;
    let body = approval(&operation()?);
    let approved = fixture
        .approve(id, &fixture.lead_cookie, &body, 200)
        .await?;
    assert_eq!(
        fixture
            .approve(id, &fixture.lead_cookie, &body, 200)
            .await?,
        approved
    );
    let mut changed = body.clone();
    changed["note"] = json!("changed words");
    let reused = fixture
        .approve(id, &fixture.lead_cookie, &changed, 409)
        .await?;
    assert_eq!(reused["refusal"], "RequestReused", "{reused}");
    let decided = fixture
        .approve(id, &fixture.lead_cookie, &approval(&operation()?), 409)
        .await?;
    assert_eq!(decided["refusal"], "RequestDecided", "{decided}");
    assert_eq!(fixture.versions()?.len(), 2);
    Ok(())
}

#[tokio::test]
async fn a_directory_scope_person_approves_from_a_reviewed_estate_profile() -> TestResult {
    let fixture = Fixture::new().await?;
    let asked = fixture.ask("waffles").await?;
    let approved = fixture
        .approve(
            asked["id"].as_str().ok_or("no request id")?,
            &fixture.administrator,
            &approval(&operation()?),
            200,
        )
        .await?;
    assert_eq!(approved["state"], "approved");
    let versions = fixture.versions()?;
    assert_eq!(
        serde_json::to_value(&versions[1].settings.mcp_servers)?,
        json!([
            {"name": "waffles", "url": "https://waffles.example.test/mcp"}
        ])
    );
    assert_eq!(
        versions[1].reviewed.as_ref().ok_or("no review")?.by,
        approved["decision"]["by"]
    );
    Ok(())
}
