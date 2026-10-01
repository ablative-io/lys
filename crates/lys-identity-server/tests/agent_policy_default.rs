//! Missing policies receive a durable restrictive first version.

use std::error::Error;
use std::path::Path;
use std::sync::Arc;

use identity_contract::apps::{Auth, get, login, op, post};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::{Actor, AuthMethod, LoginBinding, OperationId, Profile, Provenance};
use lys_identity_server::agent_policy_store::PolicyStore;
use lys_identity_server::routes::open_directory;
use lys_runner::judge::{Asked, Judgement, PATH_TOOLS, Policy, judge};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

async fn table() -> Result<(Service, [String; 2]), Box<dyn Error>> {
    Service::start_adjusted(
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
            config.runtime_dir = None;
            config.service_accounts_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        |config| {
            let actor = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
            let mut directory = open_directory(config)?;
            let (person, _) = directory.setup_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Owner")?,
                1,
            )?;
            let (missing, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                Profile::new("Missing")?,
                2,
            )?;
            let (explicit, _) = directory.register_agent(
                actor,
                OperationId::generate()?,
                person,
                Profile::new("Explicit")?,
                3,
            )?;
            let mut policies = PolicyStore::open(
                config.policies_dir.as_deref().ok_or("policies missing")?,
                Arc::new(Ed25519Identity::load(&config.event_key_file)?),
            )?;
            for version in 0..2 {
                policies.set(
                    Policy {
                        version: version + 1,
                        agent: explicit.to_string(),
                        rules: Vec::new(),
                    },
                    version,
                )?;
            }
            let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
            drop(
                lys_identity_server::configuration_store::ConfigurationStore::open(
                    &config.log_dir.with_file_name("organisation"),
                    Arc::clone(&key),
                )?,
            );
            drop(lys_identity_server::runner_acts::ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&key),
            )?);
            drop(lys_identity::start::LaunchRecords::open(
                &config.log_dir.with_file_name("launch-records"),
                Ed25519Identity::load(&config.event_key_file)?,
            )?);
            drop(lys_identity_server::apps_api::opened(config, key, &|_| {})?);
            Ok([missing.to_string(), explicit.to_string()])
        },
    )
    .await
}

fn denies_calls(policy: &Policy) {
    for subagent in [false, true] {
        for (tool, field) in PATH_TOOLS {
            for input in [
                json!({field: "/"}),
                json!({field: "missing/../file"}),
                json!({}),
            ] {
                assert!(
                    matches!(
                        judge(
                            policy,
                            &Asked {
                                tool,
                                input: &input,
                                cwd: Path::new("/"),
                                subagent
                            }
                        ),
                        Judgement::Deny { .. }
                    ),
                    "{tool}: {input}"
                );
            }
        }
        for tool in ["WebFetch", "Bash", "mcp__server__tool", "FutureTool"] {
            let input = json!({"url": "https://example.test/", "command": "true"});
            assert!(
                matches!(
                    judge(
                        policy,
                        &Asked {
                            tool,
                            input: &input,
                            cwd: Path::new("/"),
                            subagent
                        }
                    ),
                    Judgement::Deny { .. }
                ),
                "{tool}"
            );
        }
    }
}

#[tokio::test]
async fn an_old_install_gets_only_missing_policies_and_keeps_them_after_restart() -> TestResult {
    let (mut service, [missing, explicit]) = table().await?;
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
    denies_calls(&policy);
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
    let (service, _) = table().await?;
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
    denies_calls(&serde_json::from_value(policy["policy"].clone())?);
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
