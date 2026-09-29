#![cfg(test)]
//! Operator authority is tested through the actual HTTP admission path and
//! the independently verified signed leaf, never inferred from a 200 alone.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use identity_contract::apps::{FILES, NOTES, login, registration, workspace_schema};
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::signer::verify_event;
use lys_identity::{AuthMethod, OperationId};
use lys_identity_server::setup::SetupSettings;
use lys_log_store::{FileLeafStore, LeafStore};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;
const TOKEN: &str = "operator-fixture-secret-0123456789abcdef";

async fn service(administrator: bool) -> Result<Service, Box<dyn Error>> {
    Ok(Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| {
            config.operator_token_file = Some(config.log_dir.with_file_name("operator.token"));
            config.operator_upgrade_file = Some(config.log_dir.with_file_name("upgrade.json"));
            if !administrator {
                config.administrator = None;
                config.setup = Some(SetupSettings {
                    code_file: config.log_dir.with_file_name("setup-code"),
                    administrator_file: config.log_dir.with_file_name("administrator.json"),
                    email: None,
                });
            }
        },
        |config| {
            let path = config.operator_token_file.as_ref().ok_or("no token path")?;
            std::fs::write(path, TOKEN)?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
            Ok(())
        },
    )
    .await?
    .0)
}

fn setup() -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({"operation": OperationId::generate()?.to_string(), "display_name": "Operator fixture"}),
    )
}

#[tokio::test]
async fn matching_token_records_operator_provenance_and_exact_retry_keeps_one_leaf() -> TestResult {
    let service = service(true).await?;
    let body = serde_json::to_vec(&setup()?)?;
    let headers = [("lys-operator", TOKEN)];
    let (status, first) = service
        .post_carrying("/setup", &headers, body.clone())
        .await?;
    assert_eq!(status, 200, "{first}");
    let repeated = service.post_carrying("/setup", &headers, body).await?;
    assert_eq!(repeated, (200, first));
    let store = FileLeafStore::open_read_only(&service.dir.path().join("log"))?;
    assert_eq!(store.extent(), 1);
    let bytes = store.leaf(0)?.ok_or("setup leaf missing")?;
    let key = Ed25519Identity::load(&service.dir.path().join("service.key"))?;
    let signed = verify_event(&bytes, &key.public_key_bytes())?;
    assert_eq!(signed.event().actor().binding().subject(), ADMINISTRATOR);
    assert_eq!(
        signed.event().actor().provenance().method(),
        AuthMethod::Operator
    );
    assert_eq!(signed.event().actor().provenance().agent(), None);
    Ok(())
}

#[tokio::test]
async fn wrong_token_refuses_even_with_a_valid_administrator_cookie() -> TestResult {
    let service = service(true).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    for wrong in ["wrong", "operator-fixture-secret-0123456789abcdeg"] {
        let (status, answer) = service
            .post_carrying(
                "/setup",
                &[("lys-operator", wrong), ("cookie", &cookie)],
                serde_json::to_vec(&setup()?)?,
            )
            .await?;
        assert_eq!(status, 401, "{answer}");
        assert_eq!(answer["refusal"], "OperatorRefused");
        assert!(!answer.to_string().contains(wrong));
        assert!(!answer.to_string().contains(TOKEN));
    }
    assert_eq!(service.log_size().await?, 0);
    Ok(())
}

#[tokio::test]
async fn absent_header_keeps_the_existing_cookie_admission() -> TestResult {
    let service = service(true).await?;
    let body = setup()?;
    assert_eq!(service.post("/setup", None, &body).await?.0, 401);
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, answer) = service.post("/setup", Some(&cookie), &body).await?;
    assert_eq!(status, 200, "{answer}");
    let store = FileLeafStore::open_read_only(&service.dir.path().join("log"))?;
    let bytes = store.leaf(0)?.ok_or("setup leaf missing")?;
    let key = Ed25519Identity::load(&service.dir.path().join("service.key"))?;
    assert_eq!(
        verify_event(&bytes, &key.public_key_bytes())?
            .event()
            .actor()
            .provenance()
            .method(),
        AuthMethod::Oidc
    );
    Ok(())
}

#[tokio::test]
async fn a_token_needs_both_configured_secret_and_administrator() -> TestResult {
    let absent = Service::start().await?;
    let unconfigured = service(false).await?;
    for service in [&absent, &unconfigured] {
        let (status, answer) = service
            .post_carrying(
                "/setup",
                &[("lys-operator", TOKEN)],
                serde_json::to_vec(&setup()?)?,
            )
            .await?;
        assert_eq!(status, 401, "{answer}");
        assert_eq!(answer["refusal"], "OperatorRefused");
        assert_eq!(service.log_size().await?, 0);
    }
    Ok(())
}

#[tokio::test]
async fn operator_admission_reaches_the_app_registration_route() -> TestResult {
    let mut service = service(true).await?;
    let headers = [("lys-operator", TOKEN)];
    let (status, answer) = service
        .post_carrying("/setup", &headers, serde_json::to_vec(&setup()?)?)
        .await?;
    assert_eq!(status, 200, "{answer}");
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let old_body = registration(FILES, &workspace_schema(FILES))?;
    let (status, old) = service.post("/apps", Some(&cookie), &old_body).await?;
    assert_eq!(status, 200, "{old}");
    assert_eq!(old["registered_by"]["kind"], "person");
    let store = FileLeafStore::open_read_only(&service.dir.path().join("apps"))?;
    let old_leaves = (0..store.extent())
        .map(|index| store.leaf(index))
        .collect::<Result<Vec<_>, _>>()?;
    drop(store);
    let body = registration(NOTES, &workspace_schema(NOTES))?;
    let (status, answer) = service
        .post_carrying("/apps", &headers, serde_json::to_vec(&body)?)
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["id"], NOTES);
    assert_eq!(answer["registered_by"]["kind"], "operator");
    assert_eq!(answer["registered_by"]["login"]["subject"], ADMINISTRATOR);
    service.restart().await?;
    let client = reqwest::Client::new();
    for (id, original) in [(FILES, old), (NOTES, answer)] {
        let response = client
            .get(format!("{}/apps/{id}", service.base))
            .header("lys-operator", TOKEN)
            .send()
            .await?;
        assert_eq!(response.status(), 200);
        assert_eq!(
            serde_json::from_str::<Value>(&response.text().await?)?,
            original
        );
    }
    let store = FileLeafStore::open_read_only(&service.dir.path().join("apps"))?;
    assert_eq!(store.extent(), u64::try_from(old_leaves.len())? + 1);
    for (index, bytes) in old_leaves.into_iter().enumerate() {
        assert_eq!(
            store.leaf(u64::try_from(index)?)?,
            bytes,
            "old leaves never rewritten"
        );
    }
    Ok(())
}

#[test]
fn old_app_actor_shapes_round_trip_without_a_migration_or_relabelling() -> TestResult {
    use lys_identity_server::apps_state::By;
    for original in [
        r#"{"kind":"person","login":{"provider":"https://issuer.test","subject":"administrator"}}"#,
        r#"{"kind":"service_account","id":"account-1"}"#,
        r#"{"kind":"start"}"#,
    ] {
        let actor: By = serde_json::from_str(original)?;
        assert!(!matches!(actor, By::Operator { .. }));
        assert_eq!(serde_json::to_string(&actor)?, original);
    }
    Ok(())
}

#[test]
fn openapi_names_the_operator_header_and_keeps_oauth_token_authority_separate() -> TestResult {
    let document = lys_identity_server::openapi::document()?;
    assert_eq!(
        document["components"]["securitySchemes"]["lys_operator"]["name"],
        "lys-operator"
    );
    for (path, method) in [("/setup", "post"), ("/apps", "post"), ("/apps", "get")] {
        let security = document["paths"][path][method]["security"]
            .as_array()
            .ok_or("no security")?;
        assert!(
            security
                .iter()
                .any(|entry| entry.get("lys_operator").is_some())
        );
    }
    for (path, method) in [
        ("/oauth/authorize", "get"),
        ("/oauth/token", "post"),
        ("/oauth/userinfo", "get"),
    ] {
        let security = document["paths"][path][method]["security"]
            .as_array()
            .ok_or("no security")?;
        assert!(
            !security
                .iter()
                .any(|entry| entry.get("lys_operator").is_some())
        );
    }
    Ok(())
}

#[tokio::test]
async fn wrong_operator_cannot_be_bypassed_by_an_app_bearer() -> TestResult {
    let service = service(true).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    assert_eq!(
        service.post("/setup", Some(&cookie), &setup()?).await?.0,
        200
    );
    let body = registration(NOTES, &workspace_schema(NOTES))?;
    assert_eq!(service.post("/apps", Some(&cookie), &body).await?.0, 200);
    let path = format!("/apps/{NOTES}/approve");
    let (status, approved) = service
        .post(
            &path,
            Some(&cookie),
            &json!({"operation": OperationId::generate()?.to_string()}),
        )
        .await?;
    assert_eq!(status, 200, "{approved}");
    let credential = approved["client"]["credential"]
        .as_str()
        .ok_or("no credential")?;
    let client = reqwest::Client::new();
    let url = format!("{}/apps/me", service.base);
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(credential)
            .send()
            .await?
            .status(),
        200
    );
    let response = client
        .get(&url)
        .bearer_auth(credential)
        .header("lys-operator", "wrong")
        .send()
        .await?;
    assert_eq!(response.status(), 401);
    assert_eq!(
        serde_json::from_str::<Value>(&response.text().await?)?["refusal"],
        "OperatorRefused"
    );
    Ok(())
}

#[tokio::test]
async fn every_start_route_preserves_the_operator_refusal() -> TestResult {
    let service = service(true).await?;
    let client = reqwest::Client::new();
    for (method, path) in [
        (reqwest::Method::POST, "/agents/agent/start"),
        (reqwest::Method::POST, "/launch-records/record/start-again"),
        (reqwest::Method::POST, "/launch-records/record/withdraw"),
        (reqwest::Method::GET, "/launch-records/record/state"),
    ] {
        let response = client
            .request(method, format!("{}{path}", service.base))
            .header("lys-operator", "wrong")
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await?;
        assert_eq!(response.status(), 401, "{path}");
        assert_eq!(
            serde_json::from_str::<Value>(&response.text().await?)?["refusal"],
            "OperatorRefused",
            "{path}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn the_operator_has_no_personal_account_even_with_an_administrator_cookie() -> TestResult {
    let service = service(true).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let client = reqwest::Client::new();
    for (method, path) in [
        (reqwest::Method::GET, "/me"),
        (reqwest::Method::GET, "/me/account"),
        (reqwest::Method::POST, "/me/account/email"),
        (reqwest::Method::POST, "/me/account/password"),
    ] {
        let response = client
            .request(method, format!("{}{path}", service.base))
            .header("lys-operator", TOKEN)
            .header("cookie", &cookie)
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await?;
        assert_eq!(response.status(), 401, "{path}");
        assert_eq!(
            serde_json::from_str::<Value>(&response.text().await?)?["refusal"],
            "OperatorRefused",
            "{path}"
        );
    }
    assert_eq!(service.log_size().await?, 0);
    Ok(())
}

#[tokio::test]
async fn reversible_upgrade_blocks_operator_writes_until_the_intent_is_cleared() -> TestResult {
    let service = service(true).await?;
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    assert_eq!(
        service.post("/setup", Some(&cookie), &setup()?).await?.0,
        200
    );
    let intent = service.dir.path().join("upgrade.json");
    std::fs::write(&intent, "unfinished upgrade")?;
    let store_path = service.dir.path().join("apps");
    let before = FileLeafStore::open_read_only(&store_path)?.extent();
    let body = serde_json::to_vec(&registration(NOTES, &workspace_schema(NOTES))?)?;
    let response = service
        .post_carrying("/apps", &[("lys-operator", TOKEN)], body.clone())
        .await?;
    assert_eq!(response.0, 401, "{}", response.1);
    assert_eq!(response.1["refusal"], "OperatorRefused");
    assert_eq!(FileLeafStore::open_read_only(&store_path)?.extent(), before);
    // The service remains live for readiness and sessions during the upgrade.
    assert_eq!(service.get("/authority", None).await?.0, 200);
    assert_eq!(service.get("/apps", Some(&cookie)).await?.0, 200);
    std::fs::remove_file(intent)?;
    let response = service
        .post_carrying("/apps", &[("lys-operator", TOKEN)], body)
        .await?;
    assert_eq!(response.0, 200, "{}", response.1);
    assert_eq!(response.1["registered_by"]["kind"], "operator");
    assert_eq!(
        FileLeafStore::open_read_only(&store_path)?.extent(),
        before + 1
    );
    Ok(())
}

#[test]
fn every_route_publishes_the_global_operator_refusal_but_me_has_no_operator_authority() -> TestResult
{
    let document = lys_identity_server::openapi::document()?;
    let paths = document["paths"].as_object().ok_or("no paths")?;
    let mut count = 0;
    for (path, methods) in paths {
        for operation in methods.as_object().ok_or("no methods")?.values() {
            let refusals = operation["x-refusals"].as_array().ok_or("no refusals")?;
            assert!(
                refusals.iter().any(|name| name == "OperatorRefused"),
                "{path}"
            );
            if path == "/me" || path.starts_with("/me/") {
                let security = operation["security"].as_array().ok_or("no security")?;
                assert!(
                    !security
                        .iter()
                        .any(|entry| entry.get("lys_operator").is_some()),
                    "{path}"
                );
            }
            count += 1;
        }
    }
    assert!(count > 100);
    Ok(())
}
