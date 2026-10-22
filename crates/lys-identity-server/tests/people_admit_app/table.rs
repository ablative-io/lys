//! The admission table the /people/admit product suites stand on: a
//! service with a fake issuer, Ada as the first-run administrator of Acme,
//! and the fixture app approved; the admitting body, and what an act leaves.

use std::error::Error;
use std::sync::Arc;

use identity_contract::apps::registered;
use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::configuration_store::ConfigurationStore;
use lys_identity_server::provider::{CODE_SECONDS, PASS_SECONDS, ProviderSettings};
use lys_identity_server::routes::open_directory;
use lys_identity_server::runner_acts::ActStore;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const CODE: &str = "people-admit-app-first-run-code";
const ADMIN_EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";
pub(super) const BEA: &str = "bea@example.test";
pub(super) const APP: &str = "fixture_product";
pub(super) const STEPS: [&str; 6] = ["register", "account", "bind", "activate", "grant", "letter"];

/// A service with a fake issuer, no mail settings, Lys's provider on, Ada
/// as the first-run administrator of Acme, and the fixture app approved.
pub(super) async fn table() -> Result<(Service, FakeRauthy, String), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
    let broker = identity_contract::app_custody::start().await?;
    let settings = SignInProvidersSettings {
        api: rauthy.api().to_owned(),
        api_key_file: rauthy.api_key_file(),
    };
    let (service, ()) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        Some(settings),
        move |config| {
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
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
            config.administrator = None;
            config.setup = Some(SetupSettings {
                code_file: config.log_dir.with_file_name("setup-code"),
                administrator_file: config.log_dir.with_file_name("administrator.json"),
                email: None,
                organisation: "Acme".to_owned(),
            });
            config.secrets = Some(SecretsSettings {
                broker,
                service: "identity".to_owned(),
                service_key_file: config.event_key_file.clone(),
            });
            config.provider = Some(ProviderSettings {
                key_file: config.log_dir.with_file_name("provider.key"),
                code_seconds: CODE_SECONDS,
                pass_seconds: PASS_SECONDS,
                rights_bytes: None,
            });
        },
        |config| {
            std::fs::write(config.log_dir.with_file_name("provider.key"), [6u8; 32])?;
            let setup = config.setup.as_ref().ok_or("setup is configured")?;
            let digest: Vec<String> = Sha256::digest(CODE.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            let pending = json!({ "purpose": "first-run", "sha256": digest.concat() });
            std::fs::write(&setup.code_file, pending.to_string())?;
            let key = Arc::new(load_service_key(&config.event_key_file)?);
            drop(open_directory(config)?);
            drop(ConfigurationStore::open(
                &config.log_dir.with_file_name("organisation"),
                Arc::clone(&key),
            )?);
            drop(ActStore::open(
                &config.log_dir.with_file_name("runner-acts"),
                Arc::clone(&key),
            )?);
            Ok(())
        },
    )
    .await?;
    rauthy.link(service.issuer.clone())?;
    let made = reqwest::Client::new()
        .post(format!("{}/setup/administrator", service.base))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            json!({
                "code": CODE,
                "operation": OperationId::generate()?.to_string(),
                "display_name": "Ada Lovelace",
                "email": ADMIN_EMAIL,
                "password": PASSWORD,
            })
            .to_string(),
        )
        .send()
        .await?;
    let ada = session_cookie(made).await?;
    registered(&service, &ada, APP).await?;
    Ok((service, rauthy, ada))
}

/// POST `body` to /people/admit with `pass` as the bearer.
pub(super) async fn admit_with(
    service: &Service,
    pass: &str,
    body: &Value,
) -> Result<(u16, Value), Box<dyn Error>> {
    let answer = reqwest::Client::new()
        .post(format!("{}/people/admit", service.base))
        .bearer_auth(pass)
        .json(body)
        .send()
        .await?;
    let status = answer.status().as_u16();
    Ok((status, answer.json().await?))
}

/// The act admitting Bea, with `grant` as its first grant.
pub(super) fn bea(operation: &str, grant: Value) -> Value {
    let mut asked = json!({"operation": operation, "display_name": "Bea", "email": BEA});
    asked["grant"] = grant;
    asked
}

/// A member grant on `kind` `ws-1`, asked on the API route.
pub(super) fn member_of(kind: &str) -> Value {
    json!({"route": "api", "resource": {"kind": kind, "id": "ws-1"}, "relation": "member"})
}

/// The people the directory lists, as Ada reads them.
pub(super) async fn people(service: &Service, ada: &str) -> Result<usize, Box<dyn Error>> {
    let (status, listed) = service.get("/directory/people", Some(ada)).await?;
    assert_eq!(status, 200, "{listed}");
    let people = listed["people"]
        .as_array()
        .or_else(|| listed.as_array())
        .ok_or("the directory lists people")?;
    Ok(people.len())
}

/// The accounts the issuer holds for `email`.
pub(super) fn accounts_for(rauthy: &FakeRauthy, email: &str) -> Result<usize, Box<dyn Error>> {
    Ok(rauthy
        .users()?
        .iter()
        .filter(|user| user["email"] == email)
        .count())
}
