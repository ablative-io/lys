#![cfg(test)]
//! Start from an older installation with ordinary v1 identity records and
//! no loader, then restart with the configuration an upgrade adds. Exercise
//! the real HTTP owners and reopen their durable logs; no live estate data.

use std::error::Error;
use std::fs;
use std::os::unix::fs::PermissionsExt;

use identity_contract::apps::{Auth, get, ok, op, post, workspace_schema};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity_server::dev_seed::seed_configured;
use lys_log_store::{FileLeafStore, LeafStore};
use serde_json::{Value, json};

type Outcome = Result<(), Box<dyn Error>>;
const ACCOUNT: &str = "op-91919191919191919191919191919191";
const MODEL: &str = r#"{"version":1,"relations":{"owner":["view","edit","grant"],"editor":["view","edit"],"viewer":["view"]}}"#;

fn credential() -> String {
    format!("lys-registrar.{ACCOUNT}.{}", "ab".repeat(32))
}

async fn upgraded() -> Result<Service, Box<dyn Error>> {
    let (mut service, _) = Service::start_judging(MODEL, None, |config| {
        Ok(seed_configured(config, [ADMINISTRATOR, "second-person"])?)
    })
    .await?;
    let path = service.dir.path().join("loader.credential");
    fs::write(&path, credential())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    service
        .restart_adjusted(|config| config.import_credential_file = Some(path))
        .await?;
    Ok(service)
}

fn extents(service: &Service) -> Result<Vec<u64>, Box<dyn Error>> {
    ["log", "grant-log", "apps"]
        .iter()
        .map(|name| Ok(FileLeafStore::open(&service.dir.path().join(name))?.extent()))
        .collect()
}

fn document() -> Value {
    json!({"apps":[{"id":"fixture_import","name":"Import fixture","redirects":[],"schema":workspace_schema("fixture_import")}],
        "agents":[{"display_name":"Import fixture agent"}]})
}

#[tokio::test]
async fn old_install_gains_loader_and_reimport_after_restart_writes_nothing() -> Outcome {
    let mut service = upgraded().await?;
    let credential = credential();
    let first = ok(post(
        &service,
        "/identity/import",
        Auth::Bearer(&credential),
        &document(),
    )
    .await?)?;
    assert_eq!(first["by"], json!({"kind":"service_account","id":ACCOUNT}));
    assert_eq!(first["completed"].as_array().ok_or("no entries")?.len(), 2);
    let app = &first["completed"][0]["result"];
    assert_eq!(
        app["registered_by"],
        json!({"kind":"service_account","id":ACCOUNT})
    );
    let actor = &first["completed"][1]["result"]["receipt"]["actor"];
    assert_eq!(actor["authentication"], "service_account_bearer");
    assert_eq!(actor["service_account"], ACCOUNT);
    assert_eq!(first["completed"][1]["result"]["receipt"]["version"], 2);
    let before = extents(&service)?;
    service.restart().await?;
    let again = ok(post(
        &service,
        "/identity/import",
        Auth::Bearer(&credential),
        &document(),
    )
    .await?)?;
    assert_eq!(again, first);
    assert_eq!(extents(&service)?, before);
    assert_eq!(
        fs::read_to_string(service.dir.path().join("loader.credential"))?,
        credential
    );
    Ok(())
}

#[tokio::test]
async fn refused_credential_and_revoked_grant_never_mutate_the_directory() -> Outcome {
    let mut service = upgraded().await?;
    let before = extents(&service)?;
    let bad = format!("lys-registrar.{ACCOUNT}.{}", "cd".repeat(32));
    let refused = post(
        &service,
        "/identity/import",
        Auth::Bearer(&bad),
        &document(),
    )
    .await?;
    assert_eq!(refused.1["refusal"], "credential_refused", "{}", refused.1);
    assert_ne!(refused.0, 200);
    assert_eq!(extents(&service)?, before);
    let token = credential();
    let grants = ok(get(&service, "/grants", Auth::Bearer(&token)).await?)?;
    let grant = grants["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .find(|grant| grant["holder"] == ACCOUNT && grant["resource"]["id"] == "apps")
        .ok_or("the installed loader has no ordinary apps grant")?;
    let id = grant["id"].as_str().ok_or("grant lacks id")?;
    let admin = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "fixture@example.test".to_owned(),
        })
        .await?;
    ok(post(
        &service,
        &format!("/grants/{id}/revoke"),
        Auth::Cookie(&admin),
        &json!({"operation":op()?,"route":"api","reason":"remove import authority"}),
    )
    .await?)?;
    service.restart().await?;
    let before = extents(&service)?;
    let refused = post(
        &service,
        "/identity/import",
        Auth::Bearer(&token),
        &document(),
    )
    .await?;
    assert_ne!(refused.0, 200);
    assert_eq!(refused.1["entry"], "apps/fixture_import");
    assert!(
        matches!(refused.1["refusal"].as_str(), Some("Revoked" | "NotHeld")),
        "{}",
        refused.1
    );
    assert_eq!(extents(&service)?, before);
    Ok(())
}

#[tokio::test]
async fn root_entry_never_uses_the_service_accounts_owner_as_caller() -> Outcome {
    let service = upgraded().await?;
    let before = extents(&service)?;
    let document = json!({"root_grants":[{"name":"forged-root","route":"api","holder":"@owner", "resource":{"kind":"directory","id":"agents"},"relation":"editor","pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}}]});
    let refused = post(
        &service,
        "/identity/import",
        Auth::Bearer(&credential()),
        &document,
    )
    .await?;
    assert_eq!(
        refused.1["refusal"], "RootAuthorityRefused",
        "{}",
        refused.1
    );
    assert_eq!(refused.1["entry"], "root_grants/forged-root");
    assert_eq!(refused.1["audit"]["account"], ACCOUNT);
    assert_eq!(refused.1["audit"]["refusal"], "RootAuthorityRefused");
    assert_eq!(extents(&service)?, before);
    Ok(())
}

#[tokio::test]
async fn refusal_receipt_is_readable_after_restart_and_repeat_adds_no_audit_leaf() -> Outcome {
    let mut service = upgraded().await?;
    let document = json!({"root_grants":[{"name":"refused","route":"api","holder":"@owner","resource":{"kind":"directory","id":"agents"},"relation":"editor","pass_on":{"kind":"use_only"},"window":{"starts_at":0,"ends_at":null}}]});
    let first = post(
        &service,
        "/identity/import",
        Auth::Bearer(&credential()),
        &document,
    )
    .await?;
    assert_eq!(first.1["refusal"], "RootAuthorityRefused");
    let count = FileLeafStore::open(&service.dir.path().join("service-accounts"))?.extent();
    service.restart().await?;
    let again = post(
        &service,
        "/identity/import",
        Auth::Bearer(&credential()),
        &document,
    )
    .await?;
    assert_eq!(again.1["audit"], first.1["audit"]);
    assert_eq!(
        FileLeafStore::open(&service.dir.path().join("service-accounts"))?.extent(),
        count
    );
    let admin = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "fixture@example.test".to_owned(),
        })
        .await?;
    let listed = ok(get(&service, "/service-accounts", Auth::Cookie(&admin)).await?)?;
    let account = listed["service_accounts"]
        .as_array()
        .ok_or("no accounts")?
        .iter()
        .find(|a| a["id"] == ACCOUNT)
        .ok_or("no loader")?;
    assert_eq!(account["import_refusals"][0], first.1["audit"]);
    Ok(())
}

#[tokio::test]
async fn installed_estate_flow_is_admin_only_and_runs_as_real_loader() -> Outcome {
    let service = upgraded().await?;
    fs::write(
        service.dir.path().join("estate-approval.json"),
        br#"{"version":1,"agents":[],"resources":[]}"#,
    )?;
    let admin = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let other = service
        .sign_in(identity_contract::apps::login("second-person"))
        .await?;
    let before = extents(&service)?;
    assert_eq!(
        get(&service, "/identity/estate-plan", Auth::Cookie(&other))
            .await?
            .0,
        403
    );
    assert_eq!(
        post(
            &service,
            "/identity/estate-apply",
            Auth::Cookie(&other),
            &document()
        )
        .await?
        .0,
        403
    );
    assert_eq!(extents(&service)?, before);
    let read = ok(get(&service, "/identity/estate-plan", Auth::Cookie(&admin)).await?)?;
    assert_eq!(read["loader"], ACCOUNT);
    assert!(!read.to_string().contains(&credential()));
    assert_eq!(extents(&service)?, before, "preview is read-only");
    let first = ok(post(
        &service,
        "/identity/estate-apply",
        Auth::Cookie(&admin),
        &document(),
    )
    .await?)?;
    assert_eq!(first["by"], json!({"kind":"service_account","id":ACCOUNT}));
    let after = extents(&service)?;
    let repeat = ok(post(
        &service,
        "/identity/estate-apply",
        Auth::Cookie(&admin),
        &document(),
    )
    .await?)?;
    assert_eq!(repeat, first);
    assert_eq!(extents(&service)?, after);
    Ok(())
}
