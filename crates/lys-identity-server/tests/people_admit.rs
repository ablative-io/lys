#![cfg(test)]

//! Admitting a person is one act (Scone's queued unit, Waffles 9 Oct 2026
//! 14:5x): register the person, make their sign-in account at the issuer by
//! email, bind that login, activate them and issue their first named root
//! grant, in that order. Each step runs under an operation derived from the
//! act's one operation, so asking again finishes what is left and makes
//! nothing twice; a failed step is answered by its name with the steps that
//! completed before it, and every answer carries one receipt.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_rauthy::FakeRauthy;
use identity_contract::harness::{GRANT_MODEL, Service, session_cookie};
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::configuration_store::ConfigurationStore;
use lys_identity_server::routes::open_directory;
use lys_identity_server::runner_acts::ActStore;
use lys_identity_server::setup::SetupSettings;
use lys_identity_server::sign_in_providers::SignInProvidersSettings;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn Error>>;

const CODE: &str = "people-admit-first-run-code";
const ADMIN_EMAIL: &str = "ada@example.test";
const PASSWORD: &str = "Analytical-Engine-1843";
const BEA: &str = "bea@example.test";
const STEPS: [&str; 5] = ["register", "account", "bind", "activate", "grant"];

/// A service with a fake issuer and Ada as its first-run administrator.
async fn table() -> Result<(Service, FakeRauthy, String), Box<dyn Error>> {
    let rauthy = FakeRauthy::start().await?;
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
            });
        },
        |config| {
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
    Ok((service, rauthy, ada))
}

/// The act admitting Bea with a first grant under `relation`.
fn admit(operation: &str, relation: &str) -> Value {
    json!({
        "operation": operation,
        "display_name": "Bea",
        "email": BEA,
        "grant": {
            "route": "browser",
            "resource": {"kind": "doc", "id": "alpha"},
            "relation": relation,
        },
    })
}

/// The accounts the issuer holds for `email`.
fn accounts_for(rauthy: &FakeRauthy, email: &str) -> Result<usize, Box<dyn Error>> {
    Ok(rauthy
        .users()?
        .iter()
        .filter(|user| user["email"] == email)
        .count())
}

/// The people the directory lists, as the administrator reads them.
async fn people(service: &Service, ada: &str) -> Result<usize, Box<dyn Error>> {
    let (status, listed) = service.get("/directory/people", Some(ada)).await?;
    assert_eq!(status, 200, "{listed}");
    let people = listed["people"]
        .as_array()
        .or_else(|| listed.as_array())
        .ok_or("the directory lists people")?;
    Ok(people.len())
}

#[tokio::test]
async fn one_act_registers_makes_the_account_binds_activates_and_grants() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let operation = OperationId::generate()?.to_string();
    let (status, answer) = service
        .post("/people/admit", Some(&ada), &admit(&operation, "alpha"))
        .await?;
    assert_eq!(status, 200, "{answer}");
    let person = answer["person"].as_str().ok_or("a person")?;
    let subject = answer["subject"].as_str().ok_or("an issuer subject")?;
    assert!(answer["grant"].as_str().is_some(), "{answer}");
    let receipt = &answer["receipt"];
    assert_eq!(receipt["operation"], operation);
    assert_eq!(receipt["completed"], json!(STEPS));
    assert!(receipt["failed"].is_null(), "{receipt}");
    // Every step that writes a log names its leaf: the directory's three and
    // the grant's. The issuer account is written at the issuer, not a log.
    let logged: Vec<(Value, Value)> = receipt["logged"]
        .as_array()
        .ok_or("the receipt names its log leaves")?
        .iter()
        .map(|leaf| (leaf["step"].clone(), leaf["log"].clone()))
        .collect();
    assert_eq!(
        logged,
        [
            (json!("register"), json!("directory")),
            (json!("bind"), json!("directory")),
            (json!("activate"), json!("directory")),
            (json!("grant"), json!("grants")),
        ],
        "{receipt}"
    );
    let directory: Vec<u64> = receipt["logged"]
        .as_array()
        .ok_or("leaves")?
        .iter()
        .filter(|leaf| leaf["log"] == "directory")
        .filter_map(|leaf| leaf["index"].as_u64())
        .collect();
    assert_eq!(directory.len(), 3, "{receipt}");
    assert!(
        directory.windows(2).all(|pair| pair[0] < pair[1]),
        "{receipt}"
    );
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    let (status, read) = service
        .get(&format!("/identities/{person}"), Some(&ada))
        .await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["state"], "active", "{read}");
    let logins = read["logins"].to_string();
    assert!(logins.contains(subject), "the login is bound: {read}");
    Ok(())
}

#[tokio::test]
async fn asking_again_answers_the_same_person_and_makes_nothing_twice() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let body = admit(&operation, "alpha");
    let (status, first) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{first}");
    let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 200, "{again}");
    for member in ["person", "subject", "grant"] {
        assert_eq!(first[member], again[member], "{member}");
    }
    // The receipt is the logs' own: asking again answers the same leaves.
    assert_eq!(first["receipt"]["logged"], again["receipt"]["logged"]);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

#[tokio::test]
async fn a_failed_step_is_named_with_the_steps_completed_before_it() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let body = admit(&operation, "gamma");
    let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 403, "{refused}");
    assert_eq!(refused["refusal"], "RelationUnknown", "{refused}");
    let receipt = &refused["receipt"];
    assert_eq!(receipt["failed"], "grant", "{refused}");
    assert_eq!(receipt["completed"], json!(STEPS[..4]), "{refused}");
    let person = receipt["person"].as_str().ok_or("the registered person")?;
    // Asking again with the same words fails at the same step and repeats
    // none of the four before it.
    let (status, again) = service.post("/people/admit", Some(&ada), &body).await?;
    assert_eq!(status, 403, "{again}");
    assert_eq!(again["receipt"]["failed"], "grant");
    assert_eq!(again["receipt"]["person"], person);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

/// Reuse is caught at the first step whose words differ: here the grant, after
/// the four steps before it replay without writing. (A changed email is caught
/// at the bind, after the issuer has found or made that account: named in the
/// handback as a gap, not hidden.)
#[tokio::test]
async fn the_same_operation_with_other_grant_words_is_refused_at_the_grant() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let (status, first) = service
        .post("/people/admit", Some(&ada), &admit(&operation, "alpha"))
        .await?;
    assert_eq!(status, 200, "{first}");
    let (status, refused) = service
        .post("/people/admit", Some(&ada), &admit(&operation, "beta"))
        .await?;
    assert_ne!(status, 200, "{refused}");
    assert_eq!(refused["receipt"]["failed"], "grant", "{refused}");
    assert_eq!(refused["receipt"]["completed"], json!(STEPS[..4]));
    assert_eq!(refused["receipt"]["person"], first["person"]);
    assert_eq!(accounts_for(&rauthy, BEA)?, 1);
    assert_eq!(people(&service, &ada).await?, before + 1);
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_admits_and_a_refusal_writes_nothing() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let (status, refused) = service
        .post("/people/admit", None, &admit(&operation, "alpha"))
        .await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "NotSignedIn", "{refused}");
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    assert_eq!(people(&service, &ada).await?, before);
    Ok(())
}

#[tokio::test]
async fn a_malformed_email_or_missing_member_is_refused_before_any_step() -> TestResult {
    let (service, rauthy, ada) = table().await?;
    let before = people(&service, &ada).await?;
    let operation = OperationId::generate()?.to_string();
    let mut bad_email = admit(&operation, "alpha");
    bad_email["email"] = json!("not-an-email");
    let mut no_grant = admit(&OperationId::generate()?.to_string(), "alpha");
    no_grant.as_object_mut().ok_or("an object")?.remove("grant");
    for body in [bad_email, no_grant] {
        let (status, refused) = service.post("/people/admit", Some(&ada), &body).await?;
        assert_eq!(status, 400, "{refused}");
        assert_eq!(refused["refusal"], "RequestMalformed", "{refused}");
        assert!(
            refused["receipt"]["completed"]
                .as_array()
                .is_none_or(Vec::is_empty)
        );
    }
    assert_eq!(accounts_for(&rauthy, BEA)?, 0);
    assert_eq!(people(&service, &ada).await?, before);
    Ok(())
}
