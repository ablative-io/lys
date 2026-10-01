#![cfg(test)]

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

async fn fixture() -> TestResult<(Service, String, String, String, String)> {
    let (service, (pass, person, agent)) = Service::start_with(|config| {
        let seeded = crate::dev_seed::seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
        let person = seeded.people[0].id;
        let agent = seeded.people[0].agents[0].id;
        let mut passes = crate::agent_pass_store::Passes::open(
            config.log_dir.with_file_name("agent-passes.json"),
        )?;
        let pass = passes
            .issue(agent, "launch-proof", "session-proof")?
            .to_string();
        Ok((pass, person.to_string(), agent.to_string()))
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "shared@example.test".to_owned(),
        })
        .await?;
    Ok((service, pass, cookie, person, agent))
}

async fn get(service: &Service, path: &str, pass: &str) -> TestResult<(u16, Value)> {
    let response = reqwest::Client::new()
        .get(format!("{}{path}", service.base))
        .header("lys-agent-pass", pass)
        .send()
        .await?;
    Ok((response.status().as_u16(), response.json().await?))
}

#[tokio::test]
async fn a_pass_exercises_a_live_route_grant_and_ending_it_refuses_the_next_call() -> TestResult {
    let (service, pass, cookie, person, agent) = fixture().await?;
    let refused = get(&service, "/configuration", &pass).await?;
    assert_eq!(refused.0, 403, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "NotHeld");
    let resource = json!({"kind":"configuration", "id":"all"});
    let (status, root) = service.post("/grants/roots", Some(&cookie), &json!({
        "operation":operation()?, "route":"api", "holder":person, "resource":resource,
        "relation":"alpha", "pass_on":{"kind":"to", "actions":["read"], "recipients":["agent"]},
        "window":{"starts_at":0,"ends_at":null}
    })).await?;
    assert_eq!(status, 200, "{root}");
    let (status, issued) = service
        .post(
            "/grants",
            Some(&cookie),
            &json!({
                "operation":operation()?, "route":"api", "source":root["grant"], "recipient":agent,
                "responsible":person, "resource":resource, "relation":"beta",
                "pass_on":{"kind":"use_only"}, "window":{"starts_at":0,"ends_at":null}
            }),
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    assert_eq!(get(&service, "/configuration", &pass).await?.0, 200);
    let (status, revoked) = service
        .post(
            &format!(
                "/grants/{}/revoke",
                issued["grant"].as_str().ok_or("grant missing")?
            ),
            Some(&cookie),
            &json!({"operation":operation()?,"route":"api","reason":"end"}),
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    assert_eq!(get(&service, "/configuration", &pass).await?.0, 403);
    Ok(())
}

#[tokio::test]
async fn a_pass_cannot_borrow_a_cookie_or_reach_an_undeclared_own_account() -> TestResult {
    let (service, pass, cookie, _, _) = fixture().await?;
    let response = reqwest::Client::new()
        .get(format!("{}/configuration", service.base))
        .header("lys-agent-pass", &pass)
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 401);
    assert_eq!(
        response.json::<Value>().await?["refusal"],
        "AgentPassRefused"
    );
    let response = get(&service, "/me", &pass).await?;
    assert_eq!(response.0, 403, "{}", response.1);
    assert_eq!(response.1["refusal"], "TokenScopeUndeclared");
    Ok(())
}

#[tokio::test]
async fn a_kept_responsibility_refuses_before_reading_an_agent_body() -> TestResult {
    let (service, pass, _, _, _) = fixture().await?;
    for (_, path) in crate::kept_responsibilities::KEPT {
        let response = reqwest::Client::new()
            .post(format!("{}{path}", service.base))
            .header("lys-agent-pass", &pass)
            .body("invalid body")
            .send()
            .await?;
        assert_eq!(response.status().as_u16(), 403);
        let body: Value = response.json().await?;
        assert_eq!(body["refusal"], "ResponsibilityKept");
        assert_eq!(body["fields"]["route"], *path);
    }
    assert_eq!(crate::kept_responsibilities::KEPT.len(), 6);
    Ok(())
}
