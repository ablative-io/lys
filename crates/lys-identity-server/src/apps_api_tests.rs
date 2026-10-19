#![cfg(test)]
use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use serde_json::json;

use crate::apps_binding::sha256_hex;
use crate::apps_state::{Approved, By, Client, Decided, Line, Registered, SignInSet};
use crate::apps_store::AppStore;
use identity_contract::lys_identity_server::Config;
use identity_contract::lys_identity_server::dev_seed::seed_configured;

type Outcome = Result<(), Box<dyn Error>>;

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "scope@example.test".to_owned(),
    }
}

fn fixture(config: &Config) -> Result<(), Box<dyn Error>> {
    seed_configured(config, [ADMINISTRATOR, "member"])?;
    let mut apps = AppStore::open(
        &config.apps_dir(),
        Arc::new(Ed25519Identity::load(&config.event_key_file)?),
    )?;
    for id in ["ready", "no_settings", "retired", "pending"] {
        apps.keep(Line::Registered(Registered {
            operation: format!("register-{id}"),
            app: id.to_owned(),
            name: id.to_owned(),
            redirects: vec!["https://app.example.test/callback".to_owned()],
            // The smallest schema Lys reads back: one kind, one action, one relation.
            schema: json!({"kinds": {format!("{id}.doc"): {
                "actions": ["read"], "relations": {"reader": ["read"]}, "parents": []}}}),
            service_account: None,
            by: By::Start,
            at: 1,
        }))?;
        if id == "pending" {
            continue;
        }
        apps.keep(Line::Approved(Approved {
            operation: format!("approve-{id}"),
            app: id.to_owned(),
            client: Client {
                client_id: id.to_owned(),
                secret_sha256: sha256_hex("fixture-client-secret"),
            },
            binding: None,
            by: By::Start,
            at: 2,
        }))?;
        if id != "no_settings" {
            apps.keep(Line::SignInSet(SignInSet {
                operation: format!("sign-in-{id}"),
                app: id.to_owned(),
                redirects: vec!["https://app.example.test/callback".to_owned()],
                profile: false,
                by: By::Start,
                at: 2,
            }))?;
        }
        if id == "retired" {
            apps.keep(Line::Retired(Decided {
                operation: "retire".to_owned(),
                app: id.to_owned(),
                reason: "withdrawn".to_owned(),
                by: By::Start,
                at: 3,
            }))?;
        }
    }
    Ok(())
}

#[tokio::test]
async fn personal_apps_only_name_sign_in_ready_apps_without_registry_details() -> Outcome {
    let (service, ()) = Service::start_with(fixture).await?;
    let member = service.sign_in(login("member")).await?;
    let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, personal) = service.get("/apps", Some(&member)).await?;
    assert_eq!(status, 200, "{personal}");
    assert_eq!(
        personal["apps"],
        json!([{
            "id": "ready",
            "name": "ready",
            "state": "approved",
            "address": "https://app.example.test",
        }])
    );
    // Beside the apps, only Lys's own public address and why it has none
    // (DIRECTORY-102 R2): nothing of any app's registry.
    let mut keys: Vec<&String> = personal
        .as_object()
        .ok_or("not an object")?
        .keys()
        .collect();
    keys.sort();
    assert_eq!(keys, ["apps", "public_address", "public_address_reason"]);
    let (status, one) = service.get("/apps/ready", Some(&member)).await?;
    assert_eq!(status, 200, "{one}");
    assert_eq!(one, personal["apps"][0]);
    for id in ["no_settings", "retired", "pending"] {
        let (status, hidden) = service.get(&format!("/apps/{id}"), Some(&member)).await?;
        assert_eq!(status, 404, "{hidden}");
        assert_eq!(hidden["refusal"], "app_unknown");
    }
    let (status, all) = service.get("/apps", Some(&administrator)).await?;
    assert_eq!(status, 200, "{all}");
    let apps = all["apps"].as_array().ok_or("apps missing")?;
    assert_eq!(apps.len(), 5);
    assert!(
        apps.iter()
            .all(|app| app.get("schema").is_some() && app.get("redirects").is_some())
    );
    Ok(())
}
