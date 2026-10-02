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
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::routes::open_directory;
use lys_log_store::{FileLeafStore, LeafStore};
use serde_json::{Value, json};

type Outcome = Result<(), Box<dyn Error>>;
const ACCOUNT: &str = "op-91919191919191919191919191919191";
const MODEL: &str = r#"{"version":1,"relations":{"owner":["view","edit","grant"],"editor":["view","edit"],"viewer":["view"]}}"#;

fn credential() -> String {
    format!("lys-registrar.{ACCOUNT}.{}", "ab".repeat(32))
}

enum Seed {
    Administrator,
    SecondPerson,
    ActiveAgent,
}

fn prepare(config: &lys_identity_server::Config, seed: Seed) -> Outcome {
    let actor = Actor::new(
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let mut directory = open_directory(config)?;
    let (owner, _) = directory.register_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Administrator")?,
        1,
    )?;
    directory.bind_login(
        actor.clone(),
        OperationId::generate()?,
        owner,
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        2,
    )?;
    directory.transition(
        actor.clone(),
        OperationId::generate()?,
        IdentityId::Person(owner),
        Transition::Activate,
        "",
        3,
    )?;
    match seed {
        Seed::Administrator => {}
        Seed::SecondPerson => {
            let (other, _) = directory.register_person(
                actor.clone(),
                OperationId::generate()?,
                Profile::new("Other")?,
                4,
            )?;
            directory.bind_login(
                actor.clone(),
                OperationId::generate()?,
                other,
                LoginBinding::new(&config.issuer, "second-person")?,
                5,
            )?;
            directory.transition(
                actor,
                OperationId::generate()?,
                IdentityId::Person(other),
                Transition::Activate,
                "",
                6,
            )?;
        }
        Seed::ActiveAgent => {
            let (agent, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                owner,
                Profile::new("Agent")?,
                4,
            )?;
            directory.transition(
                actor,
                OperationId::generate()?,
                IdentityId::Agent(agent),
                Transition::Activate,
                "",
                5,
            )?;
        }
    }
    Ok(())
}

async fn upgraded(seed: Seed) -> Result<Service, Box<dyn Error>> {
    let (mut service, ()) =
        Service::start_judging(MODEL, None, move |config| prepare(config, seed)).await?;
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
    let mut service = upgraded(Seed::Administrator).await?;
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
    let mut service = upgraded(Seed::Administrator).await?;
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
    let service = upgraded(Seed::Administrator).await?;
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
    let mut service = upgraded(Seed::Administrator).await?;
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
    let service = upgraded(Seed::SecondPerson).await?;
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

#[tokio::test]
async fn estate_without_installed_plan_names_configuration_refusal() -> Outcome {
    let service = upgraded(Seed::Administrator).await?;
    let admin = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let before = extents(&service)?;
    let answer = get(&service, "/identity/estate-plan", Auth::Cookie(&admin)).await?;
    assert_eq!(answer.1["refusal"], "ConfigInvalid");
    assert_ne!(answer.0, 200);
    assert_eq!(extents(&service)?, before);
    Ok(())
}

#[tokio::test]
async fn import_preserves_named_action_and_recipient_refusals_without_grant_writes() -> Outcome {
    let service = upgraded(Seed::ActiveAgent).await?;
    let admin = service
        .sign_in(identity_contract::apps::login(ADMINISTRATOR))
        .await?;
    let me = ok(get(&service, "/me", Auth::Cookie(&admin)).await?)?;
    let owner = me["person"]["id"].as_str().ok_or("no owner")?;
    let people = ok(get(&service, "/directory/people", Auth::Cookie(&admin)).await?)?;
    let person = people["people"]
        .as_array()
        .ok_or("no people")?
        .iter()
        .find(|p| p["id"] == owner)
        .ok_or("owner missing")?;
    let agent = person["agents"]
        .as_array()
        .ok_or("no agents")?
        .iter()
        .find(|a| a["state"] == "active")
        .ok_or("no active agent")?["id"]
        .as_str()
        .ok_or("no agent id")?;
    let resource = json!({"kind":"directory","id":"agents"});
    let root=ok(post(&service,"/grants/roots",Auth::Cookie(&admin),&json!({"operation":op()?,"route":"browser","holder":owner,"resource":resource,"relation":"editor","window":{"starts_at":0,"ends_at":null},"pass_on":{"kind":"to","actions":["view","edit"],"recipients":["service_account","agent"]}})).await?)?;
    let given=ok(post(&service,"/grants",Auth::Cookie(&admin),&json!({"operation":op()?,"route":"browser","source":root["grant"],"recipient":ACCOUNT,"responsible":owner,"resource":resource,"relation":"editor","window":{"starts_at":0,"ends_at":null},"pass_on":{"kind":"to","actions":["view"],"recipients":["agent"]}})).await?)?;
    let before = extents(&service)?;
    for (recipient, relation, name) in [
        (agent, "editor", "ActionsOutside"),
        (owner, "viewer", "RecipientRefused"),
    ] {
        let answer=post(&service,"/identity/import",Auth::Bearer(&credential()),&json!({"delegations":[{"name":name,"route":"api","source":given["grant"],"recipient":recipient,"responsible":owner,"resource":resource,"relation":relation,"window":{"starts_at":0,"ends_at":null},"pass_on":{"kind":"use_only"}}]})).await?;
        assert_eq!(answer.1["refusal"], name, "{}", answer.1);
        assert_ne!(answer.0, 200);
        assert_eq!(extents(&service)?, before);
    }
    Ok(())
}

/// Restart `service` on the model Lys ships, as an upgrade brings it.
async fn on_the_shipped_model(service: &mut Service) -> Outcome {
    fs::write(
        service.dir.path().join("grant-model.json"),
        lys_identity::grants::shipped_model(),
    )?;
    service.restart().await
}

#[tokio::test]
async fn an_upgraded_install_replays_its_loader_grants_and_issues_none() -> Outcome {
    let mut service = upgraded(Seed::Administrator).await?;
    let before = extents(&service)?;
    on_the_shipped_model(&mut service).await?;
    let after = extents(&service)?;
    assert_eq!(after[1], before[1], "the upgrade issues no grant");
    let token = credential();
    let grants = ok(get(&service, "/grants", Auth::Bearer(&token)).await?)?;
    assert!(
        grants["grants"]
            .as_array()
            .ok_or("no grants")?
            .iter()
            .any(|grant| grant["holder"] == ACCOUNT && grant["resource"]["id"] == "apps"),
        "{grants}"
    );
    service.restart().await?;
    assert_eq!(
        extents(&service)?[1],
        before[1],
        "a later start issues none either"
    );
    Ok(())
}

#[tokio::test]
async fn a_revoked_loader_grant_stays_revoked_through_the_upgrade() -> Outcome {
    let mut service = upgraded(Seed::Administrator).await?;
    let token = credential();
    let grants = ok(get(&service, "/grants", Auth::Bearer(&token)).await?)?;
    let id = grants["grants"]
        .as_array()
        .ok_or("no grants")?
        .iter()
        .find(|grant| grant["holder"] == ACCOUNT && grant["resource"]["id"] == "apps")
        .and_then(|grant| grant["id"].as_str())
        .ok_or("the installed loader has no apps grant")?
        .to_owned();
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
    let before = extents(&service)?;
    on_the_shipped_model(&mut service).await?;
    assert_eq!(
        extents(&service)?[1],
        before[1],
        "the upgrade restores nothing"
    );
    let refused = post(
        &service,
        "/identity/import",
        Auth::Bearer(&token),
        &document(),
    )
    .await?;
    assert_ne!(refused.0, 200, "{}", refused.1);
    Ok(())
}
