#![cfg(test)]
//! Bootstrap recovery requires an explicit owner act and preserves revocations.

use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Instant;

use identity_contract::apps::{Auth, get, login, ok, op, post};
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::{Actor, AuthMethod, Directory, LoginBinding, OperationId, Profile, Provenance};
use lys_log_store::{FileLeafStore, LeafStore};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type Outcome = Result<(), Box<dyn Error>>;
const ACCOUNT: &str = "op-91919191919191919191919191919191";

fn model(version: u64) -> String {
    let mut editor = vec!["view", "edit"];
    if version >= 2 {
        editor.push("agent.create");
    }
    if version >= 3 {
        editor.push("agent.stop");
    }
    let mut owner = editor.clone();
    owner.push("grant");
    json!({"version":version,"relations":{"owner":owner,"editor":editor,"viewer":["view"]}})
        .to_string()
}

fn operation(collection: &str, kind: &str, version: u64) -> String {
    let suffix = if version == 1 {
        String::new()
    } else {
        format!("/v{version}")
    };
    let hash = Sha256::digest(format!(
        "lys/import-bootstrap/v1/{ACCOUNT}/{collection}/{kind}{suffix}"
    ));
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    OperationId::from_bytes(bytes).to_string()
}

async fn fixture() -> Result<(Service, String, String), Box<dyn Error>> {
    let started = Instant::now();
    let (service, owner) = Service::start_adjusted(
        &model(1),
        None,
        None,
        None,
        |config| {
            config.certificates_dir = None;
            config.homes_dir = None;
            config.teams_dir = None;
            config.requests_dir = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.network_file = None;
            config.runtime_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.policies_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        |config| {
            FileLeafStore::create(&config.log_dir, &config.log_origin)?;
            let path = config.log_dir.clone();
            let mut directory = Directory::open(
                Box::new(move || FileLeafStore::open(&path)),
                Ed25519Identity::load(&config.event_key_file)?,
            )?;
            let actor = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
            let (person, _) = directory.setup_person(
                actor,
                OperationId::generate()?,
                Profile::new("Owner")?,
                1,
            )?;
            Ok(person.to_string())
        },
    )
    .await?;
    eprintln!("bootstrap fixture startup: {:?}", started.elapsed());
    let cookie = service.sign_in(login(ADMINISTRATOR)).await?;
    let path = service.dir.path().join("loader.credential");
    fs::write(
        &path,
        format!("lys-registrar.{ACCOUNT}.{}", "ab".repeat(32)),
    )?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    Ok((service, owner, cookie))
}

async fn loader(service: &mut Service, enabled: bool) -> Outcome {
    let path = service.dir.path().join("loader.credential");
    let started = Instant::now();
    let result = service
        .restart_adjusted(|config| config.import_credential_file = enabled.then_some(path))
        .await;
    eprintln!(
        "bootstrap restart with loader {enabled}: {:?}",
        started.elapsed()
    );
    result
}

fn extent(service: &Service) -> Result<u64, Box<dyn Error>> {
    Ok(FileLeafStore::open(&service.dir.path().join("grant-log"))?.extent())
}

async fn root(
    service: &Service,
    cookie: &str,
    owner: &str,
    collection: &str,
    version: u64,
) -> Result<Value, Box<dyn Error>> {
    let actions = serde_json::from_str::<Value>(&model(version))?["relations"]["editor"].clone();
    ok(post(
        service,
        "/grants/roots",
        Auth::Cookie(cookie),
        &json!({
            "operation":operation(collection,"root",version),"route":"api","holder":owner,
            "resource":{"kind":"directory","id":collection},"relation":"editor",
            "pass_on":{"kind":"to","actions":actions,"recipients":["service_account"]},
            "window":{"starts_at":0,"ends_at":null}
        }),
    )
    .await?)
}

#[tokio::test]
async fn partial_v1_refuses_then_documented_owner_recovery_is_idempotent() -> Outcome {
    let (mut service, owner, cookie) = fixture().await?;
    for collection in ["apps", "agents"] {
        root(&service, &cookie, &owner, collection, 1).await?;
    }
    fs::write(service.dir.path().join("grant-model.json"), model(2))?;
    let before = extent(&service)?;
    let error = loader(&mut service, true)
        .await
        .err()
        .ok_or("partial bootstrap started")?;
    let text = error.to_string();
    assert!(text.starts_with("BootstrapInterrupted: "), "{text}");
    assert_eq!(extent(&service)?, before);
    let mut plan: Value = serde_json::from_str(
        text.strip_prefix("BootstrapInterrupted: ")
            .ok_or("no recovery")?,
    )?;
    assert_eq!(
        plan["act"],
        "disable import_credential_file, restart, sign in as the configured administrator, record these requests, then restore import_credential_file"
    );
    assert_eq!(plan["recorded_model_version"], 1);
    assert_eq!(plan["current_model_version"], 2);
    loader(&mut service, false).await?;
    for collection in ["apps", "agents"] {
        if collection == "agents" {
            let error = loader(&mut service, true)
                .await
                .err()
                .ok_or("incomplete replacement started")?;
            let text = error.to_string();
            plan = serde_json::from_str(
                text.strip_prefix("BootstrapInterrupted: ")
                    .ok_or("no recovery")?,
            )?;
            loader(&mut service, false).await?;
        }
        assert_eq!(plan["root"]["resource"]["id"], collection);
        assert_eq!(plan["recorded_model_version"], 1);
        assert_eq!(plan["current_model_version"], 2);
        let created = ok(post(
            &service,
            "/grants/roots",
            Auth::Cookie(&cookie),
            &plan["root"],
        )
        .await?)?;
        if collection == "apps" {
            let after_root = extent(&service)?;
            let refused = loader(&mut service, true)
                .await
                .err()
                .ok_or("startup completed a replacement")?;
            assert!(refused.to_string().starts_with("BootstrapInterrupted: "));
            let interrupted: Value = serde_json::from_str(
                refused
                    .to_string()
                    .strip_prefix("BootstrapInterrupted: ")
                    .ok_or("no recovery")?,
            )?;
            assert_eq!(interrupted["recorded_model_version"], 1);
            assert_eq!(interrupted["current_model_version"], 2);
            assert_eq!(extent(&service)?, after_root);
            loader(&mut service, false).await?;
        }
        let mut delegate = plan["delegation"].clone();
        delegate["source"] = created["grant"].clone();
        ok(post(&service, "/grants", Auth::Cookie(&cookie), &delegate).await?)?;
    }
    let before = extent(&service)?;
    loader(&mut service, true).await?;
    assert_eq!(extent(&service)?, before);
    fs::write(service.dir.path().join("grant-model.json"), model(3))?;
    service.restart().await?;
    assert_eq!(extent(&service)?, before);
    let listed = ok(get(&service, "/grants", Auth::Cookie(&cookie)).await?)?;
    let replacements: Vec<_> = listed["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .filter(|grant| grant["holder"] == ACCOUNT)
        .collect();
    assert_eq!(replacements.len(), 2);
    for grant in replacements {
        assert_eq!(grant["model_version"], 2);
        assert!(
            grant["actions"]
                .as_array()
                .ok_or("no actions")?
                .contains(&json!("agent.create"))
        );
    }
    Ok(())
}

async fn revoked_across_upgrade(version: u64) -> Outcome {
    let (mut service, owner, cookie) = fixture().await?;
    if version > 1 {
        fs::write(service.dir.path().join("grant-model.json"), model(version))?;
        service.restart().await?;
    }
    loader(&mut service, true).await?;
    let issued = root(&service, &cookie, &owner, "apps", version).await?;
    let id = issued["grant"].as_str().ok_or("no root id")?;
    ok(post(
        &service,
        &format!("/grants/{id}/revoke"),
        Auth::Cookie(&cookie),
        &json!({"operation":op()?,"route":"api","reason":"withdraw loader authority"}),
    )
    .await?)?;
    let before = extent(&service)?;
    fs::write(
        service.dir.path().join("grant-model.json"),
        model(version + 1),
    )?;
    service.restart().await?;
    assert_eq!(
        extent(&service)?,
        before,
        "startup issued authority after a model bump"
    );
    let held = ok(get(&service, &format!("/grants/{id}"), Auth::Cookie(&cookie)).await?)?;
    assert_eq!(held["standing"]["stands"], false);
    Ok(())
}

#[tokio::test]
async fn revoked_v1_root_is_preserved_across_v2() -> Outcome {
    revoked_across_upgrade(1).await
}

#[tokio::test]
async fn revoked_v2_root_is_preserved_across_v3() -> Outcome {
    revoked_across_upgrade(2).await
}
