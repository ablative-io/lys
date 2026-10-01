#![cfg(test)]

use std::error::Error;

use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::OperationId;
use serde_json::{Value, json};

const PASS_MODEL: &str = r#"{"version":1,"relations":{"alpha":["read","write","configuration.zone.set"],"beta":["read"]}}"#;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

fn prepare_stores(
    log: &std::path::Path,
    origin: &str,
    key_file: &std::path::Path,
    apps: &std::path::Path,
) -> TestResult {
    let key = std::sync::Arc::new(lys_identity::signer::load_service_key(key_file)?);
    if !log.exists() {
        lys_log_store::FileLeafStore::create(log, origin)?;
    }
    let configuration = crate::configuration_store::ConfigurationStore::open(
        &log.with_file_name("organisation"),
        std::sync::Arc::clone(&key),
    )?;
    let acts = crate::runner_acts::ActStore::open(
        &log.with_file_name("runner-acts"),
        std::sync::Arc::clone(&key),
    )?;
    let launches = lys_identity::start::LaunchRecords::open(
        &log.with_file_name("launch-records"),
        lys_identity::signer::load_service_key(key_file)?,
    )?;
    let apps = crate::apps_store::AppStore::open(apps, key)?;
    if let Some(reason) = [
        acts.snapshot_failure(),
        launches.snapshot_failure(),
        apps.snapshot_failure(),
    ]
    .into_iter()
    .flatten()
    .next()
    {
        return Err(format!("fixture snapshot failed: {reason}").into());
    }
    drop(configuration);
    Ok(())
}

async fn fixture() -> TestResult<(Service, String, String, String, String)> {
    let (service, (pass, cookie, person, agent)) = Service::start_adjusted(
        PASS_MODEL,
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
            config.runtime_dir = Some(config.log_dir.with_file_name("runtime"));
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        |config| {
            prepare_stores(
                &config.log_dir,
                &config.log_origin,
                &config.event_key_file,
                &config.apps_dir(),
            )?;
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
                actor.clone(),
                OperationId::generate()?,
                lys_identity::IdentityId::Agent(agent),
                lys_identity::Transition::Activate,
                "",
                3,
            )?;
            let mut runtime = crate::runtime_store::RuntimeStore::open(
                &config
                    .runtime_dir
                    .clone()
                    .ok_or("fixture needs runtime records")?,
                std::sync::Arc::new(lys_identity::signer::load_service_key(
                    &config.event_key_file,
                )?),
            )?;
            runtime.report(crate::runtime_state::Report {
                operation: operation()?,
                session: "session-proof".to_owned(),
                agent: Some(agent.to_string()),
                machine: "fixture-machine".to_owned(),
                state: crate::runtime_state::Reported::Starting,
                what: "starting".to_owned(),
                confirmation: String::new(),
                reported_by: person.to_string(),
                at: 1,
                launch: None,
            })?;
            if let Some(reason) = runtime.snapshot_failure() {
                return Err(format!("fixture runtime snapshot failed: {reason}").into());
            }
            let mut passes = crate::agent_pass_store::Passes::open(
                config.log_dir.with_file_name("agent-passes.json"),
            )?;
            let pass = passes
                .issue(agent, "launch-proof", "session-proof")?
                .to_string();
            let sessions = crate::session::Sessions::open(
                config
                    .sessions_file
                    .clone()
                    .ok_or("fixture needs persisted sessions")?,
                config.session_seconds,
                config.secure_cookie,
            )?;
            let cookie = sessions.begin(actor)?;
            let cookie = cookie
                .split(';')
                .next()
                .ok_or("session cookie missing")?
                .to_owned();
            Ok((pass, cookie, person.to_string(), agent.to_string()))
        },
    )
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
async fn a_pass_exercises_get_and_head_and_refuses_a_revoked_grant_and_an_ungranted_mcp_tool()
-> TestResult {
    let (service, pass, cookie, person, agent) = fixture().await?;
    let refused = get(&service, "/configuration", &pass).await?;
    assert_eq!(refused.0, 403, "{}", refused.1);
    assert_eq!(refused.1["refusal"], "NotHeld");
    assert_eq!(
        refused.1["can_grant"],
        json!([{
            "id":person, "kind":"person", "display_name":"Owner"
        }]),
        "{}",
        refused.1
    );
    let path = format!("{}/configuration", service.base);
    assert_eq!(
        client()
            .head(&path)
            .header("lys-agent-pass", &pass)
            .send()
            .await?
            .status()
            .as_u16(),
        403
    );
    let pin = service.dir.path().join("organisation/state.json");
    let before = std::fs::read(&pin)?;
    let response = client()
        .post(format!("{}/mcp", service.base))
        .header("lys-agent-pass", &pass)
        .header("accept", "application/json, text/event-stream")
        .json(
            &json!({"jsonrpc":"2.0", "id":1, "method":"tools/call", "params":{
                "name":"change", "arguments":{"method":"PUT", "path":"/configuration",
                "body":{"operation":operation()?, "zone":"Europe/London", "version":1}}
            }}),
        )
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = response.json().await?;
    assert_eq!(body["result"]["isError"], true, "{body}");
    assert_eq!(body["result"]["structuredContent"]["status"], 403, "{body}");
    assert_eq!(
        body["result"]["structuredContent"]["body"]["refusal"], "NotHeld",
        "{body}"
    );
    assert_eq!(std::fs::read(&pin)?, before);
    let granted = grant(&service, &cookie, &person, &agent).await?;
    assert_eq!(get(&service, "/configuration", &pass).await?.0, 200);
    assert_eq!(
        client()
            .head(&path)
            .header("lys-agent-pass", &pass)
            .send()
            .await?
            .status()
            .as_u16(),
        200
    );
    refuses_an_unrecorded_use(&service, &pass).await?;
    let (status, revoked) = service
        .post(
            &format!("/grants/{granted}/revoke"),
            Some(&cookie),
            &json!({"operation":operation()?, "route":"api", "reason":"end"}),
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    assert_eq!(get(&service, "/configuration", &pass).await?.0, 403);
    Ok(())
}

#[tokio::test]
async fn a_pass_cannot_borrow_credentials_or_reach_undeclared_or_kept_routes() -> TestResult {
    let (service, pass, cookie, _, _) = fixture().await?;
    for (name, value) in [
        ("cookie", cookie.as_str()),
        ("lys-agent-signature", "other"),
        ("authorization", "other"),
    ] {
        let response = client()
            .get(format!("{}/configuration", service.base))
            .header("lys-agent-pass", &pass)
            .header(name, value)
            .send()
            .await?;
        assert_eq!(response.status().as_u16(), 401);
        assert_eq!(
            response.json::<Value>().await?["refusal"],
            "AgentPassRefused"
        );
    }
    let response = get(&service, "/me", &pass).await?;
    assert_eq!(response.0, 403, "{}", response.1);
    assert_eq!(response.1["refusal"], "TokenScopeUndeclared");
    let defaults: Vec<Value> = serde_json::from_str(crate::kept_responsibilities::DEFAULT)?;
    assert_eq!(defaults.len(), 6);
    for entry in &defaults {
        let path = entry["path"].as_str().ok_or("kept path missing")?;
        let response = client()
            .post(format!("{}{path}", service.base))
            .header("lys-agent-pass", &pass)
            .body("invalid body")
            .send()
            .await?;
        assert_eq!(response.status().as_u16(), 403);
        let body: Value = response.json().await?;
        assert_eq!(body["refusal"], "ResponsibilityKept");
        assert_eq!(body["can_grant"], json!([]), "{body}");
        assert_eq!(body["fields"]["route"], path);
    }
    Ok(())
}

async fn refuses_an_unrecorded_use(service: &Service, pass: &str) -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let leaves = service.dir.path().join("grant-log/leaves");
    let mode = std::fs::metadata(&leaves)?.permissions();
    let pin = service.dir.path().join("grant-log/state.json");
    let before = std::fs::read(&pin)?;
    std::fs::set_permissions(&leaves, std::fs::Permissions::from_mode(0o500))?;
    let response = get(service, "/configuration", pass).await;
    std::fs::set_permissions(&leaves, mode)?;
    let (status, body) = response?;
    assert_eq!(std::fs::read(&pin)?, before);
    assert_eq!(status, 503, "{body}");
    assert_eq!(body["refusal"], "LogUnavailable");
    Ok(())
}

#[tokio::test]
async fn an_unknown_kept_route_refuses_service_startup() -> TestResult {
    let result = Service::start_adjusted(
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
            prepare_stores(
                &config.log_dir,
                &config.log_origin,
                &config.event_key_file,
                &config.apps_dir(),
            )?;
            std::fs::write(
                config.log_dir.with_file_name("kept-responsibilities.json"),
                r#"[{"method":"POST","path":"/missing-route"}]"#,
            )?;
            Ok(())
        },
    )
    .await;
    let Err(error) = result else {
        return Err("service accepted an unknown kept route".into());
    };
    assert!(error.to_string().contains("unknown kept route"), "{error}");
    Ok(())
}
