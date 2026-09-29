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
