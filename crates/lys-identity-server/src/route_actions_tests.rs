#![cfg(test)]

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

async fn fixture() -> TestResult<(Service, String, String, String, String)> {
    let (service, (pass, person, agent)) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.homes_dir = None;
            config.runtime_dir = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        |config| {
            let log_dir = config.log_dir.clone();
            if !log_dir.exists() {
                lys_log_store::FileLeafStore::create(&log_dir, &config.log_origin)?;
            }
            let key = lys_identity::signer::load_service_key(&config.event_key_file)?;
            lys_identity::directory_migration::migrate(
                lys_log_store::FileLeafStore::open(&log_dir)?,
                &key,
            )?;
            let mut directory = lys_identity::Directory::open(
                Box::new(move || lys_log_store::FileLeafStore::open(&log_dir)),
                key,
            )?;
            let actor = lys_identity::Actor::new(
                lys_identity::LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                lys_identity::Provenance::new(lys_identity::AuthMethod::Oidc, 1),
            );
            let (person, _) = directory.setup_person(
                actor.clone(),
                OperationId::generate()?,
                lys_identity::Profile::new("Owner")?,
                1,
            )?;
            let (agent, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                lys_identity::Profile::new("Agent")?,
                2,
            )?;
            directory.transition(
                actor,
                OperationId::generate()?,
                lys_identity::IdentityId::Agent(agent),
                lys_identity::Transition::Activate,
                "",
                3,
            )?;
            let mut passes = crate::agent_pass_store::Passes::open(
                config.log_dir.with_file_name("agent-passes.json"),
            )?;
            let pass = passes
                .issue(agent, "launch-proof", "session-proof")?
                .to_string();
            Ok((pass, person.to_string(), agent.to_string()))
        },
    )
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "shared@example.test".to_owned(),
        })
        .await?;
    Ok((service, pass, cookie, person, agent))
}

fn client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

async fn get(service: &Service, path: &str, pass: &str) -> TestResult<(u16, Value)> {
    let response = client()
        .get(format!("{}{path}", service.base))
        .header("lys-agent-pass", pass)
        .send()
        .await?;
    Ok((response.status().as_u16(), response.json().await?))
}

async fn grant(service: &Service, cookie: &str, person: &str, agent: &str) -> TestResult<String> {
    let resource = json!({"kind":"configuration", "id":"all"});
    let (status, root) = service.post("/grants/roots", Some(cookie), &json!({
        "operation":operation()?, "route":"api", "holder":person, "resource":resource,
        "relation":"alpha", "pass_on":{"kind":"to", "actions":["read"], "recipients":["agent"]},
        "window":{"starts_at":0,"ends_at":null}
    })).await?;
    assert_eq!(status, 200, "{root}");
    let (status, issued) = service
        .post(
            "/grants",
            Some(cookie),
            &json!({
                "operation":operation()?, "route":"api", "source":root["grant"], "recipient":agent,
                "responsible":person, "resource":resource, "relation":"beta",
                "pass_on":{"kind":"use_only"}, "window":{"starts_at":0,"ends_at":null}
            }),
        )
        .await?;
    assert_eq!(status, 200, "{issued}");
    Ok(issued["grant"].as_str().ok_or("grant missing")?.to_owned())
}

#[tokio::test]
async fn a_pass_exercises_a_live_route_grant_and_ending_it_refuses_the_next_call() -> TestResult {
    let (service, pass, cookie, person, agent) = fixture().await?;
    let refused = get(&service, "/configuration", &pass).await?;
    assert_eq!(refused.0, 403, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "NotHeld");
    let granted = grant(&service, &cookie, &person, &agent).await?;
    assert_eq!(get(&service, "/configuration", &pass).await?.0, 200);
    let (status, revoked) = service
        .post(
            &format!("/grants/{granted}/revoke"),
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
    let response = client()
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
        let response = client()
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

#[tokio::test]
async fn a_granted_call_refuses_when_its_use_cannot_be_recorded() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let (service, pass, cookie, person, agent) = fixture().await?;
    grant(&service, &cookie, &person, &agent).await?;
    let leaves = service.dir.path().join("grant-log/leaves");
    let mode = std::fs::metadata(&leaves)?.permissions();
    let pin = service.dir.path().join("grant-log/state.json");
    let before = std::fs::read(&pin)?;
    std::fs::set_permissions(&leaves, std::fs::Permissions::from_mode(0o500))?;
    let response = get(&service, "/configuration", &pass).await;
    std::fs::set_permissions(&leaves, mode)?;
    let (status, body) = response?;
    assert_eq!(std::fs::read(&pin)?, before);
    assert_eq!(status, 503, "{body}");
    assert_eq!(body["refusal"], "LogUnavailable");
    Ok(())
}
