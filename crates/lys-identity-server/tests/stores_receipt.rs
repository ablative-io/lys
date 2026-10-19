//! No store is ever lost or emptied silently (DIRECTORY-107 R2): after a
//! clean stop the service's receipt counts every store, and a boot that
//! finds a whole store short, of another origin or identity, or absent
//! refuses, makes nothing anew and serves its health alone, saying the
//! store, both heads, the one command and the computer. An honest shrink
//! (a revoke, a removal through the receipt's one removal function) boots;
//! an own store found short refuses only its own routes; and only an
//! operator's acceptance naming the store serves what was found. Every loss
//! here is made by the test while the service is stopped, as a put-back or a
//! hand would make it.

use std::error::Error;
use std::path::{Path, PathBuf};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity::stores_receipt::{self as receipt, Acceptance, Kind};
use lys_identity_server::Config;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

#[path = "stores_receipt/scope.rs"]
mod scope;
#[path = "stores_receipt/words.rs"]
mod words;

type TestResult = Result<(), Box<dyn Error>>;
type Failure = Box<dyn Error>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Failure> {
    Ok(OperationId::generate()?.to_string())
}

/// The service with Ada as the administrator and Bea, each with agents.
async fn seeded() -> Result<(Service, Seeded), Failure> {
    Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?)).await
}

/// Bea's active agent.
fn agent(seeded: &Seeded) -> Result<String, Failure> {
    Ok(seeded.people[1]
        .agents
        .iter()
        .find(|agent| agent.state == lys_identity::LifecycleState::Active)
        .ok_or("Bea has no active agent")?
        .id
        .to_string())
}

/// A root grant of `alpha` on `doc:<id>` to `holder`, answering the grant.
async fn grant(service: &Service, admin: &str, holder: &str, id: &str) -> Result<String, Failure> {
    let body = json!({
        "operation": operation()?,
        "route": "api",
        "holder": holder,
        "resource": { "kind": "doc", "id": id },
        "relation": "alpha",
        "pass_on": { "kind": "use_only" },
        "window": { "starts_at": 0, "ends_at": null },
    });
    let (status, issued) = service.post("/grants/roots", Some(admin), &body).await?;
    assert_eq!(status, 200, "{issued}");
    Ok(issued["grant"].as_str().ok_or("no grant")?.to_owned())
}

/// A machine named `name` that may run `agent`, answering its id.
async fn machine(
    service: &Service,
    admin: &str,
    name: &str,
    agent: &str,
) -> Result<String, Failure> {
    let id = operation()?;
    let body = json!({
        "operation": id, "name": name, "kind": "laptop", "runtime": "local launcher",
        "slots": 2, "may_run": [agent], "may_reach": [],
    });
    let (status, named) = service
        .post("/network/machines", Some(admin), &body)
        .await?;
    assert_eq!(status, 200, "{named}");
    Ok(id)
}

/// Copies the tree at `from` to `to`, which must not exist.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_file() {
        std::fs::copy(from, to)?;
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Puts the copy at `kept` in place of the store at `live`: the store as it
/// was when the copy was taken, its later records gone.
fn put_back(live: &Path, kept: &Path) -> std::io::Result<()> {
    if live.is_dir() {
        std::fs::remove_dir_all(live)?;
    } else if live.exists() {
        std::fs::remove_file(live)?;
    }
    copy_tree(kept, live)
}

/// Where the test keeps its copy of a store.
fn kept(service: &Service, name: &str) -> PathBuf {
    service.dir.path().join("kept").join(name)
}

/// The operator's `lys identity accept-loss --store <store>`, as the
/// installer writes it, answering its instant.
fn accept(config: &Config, store: &str) -> Result<String, Failure> {
    let at = receipt::now();
    receipt::append_acceptances(
        &config.stores_accepted_file,
        &[Acceptance {
            kind: Kind::Accepted,
            store: store.to_owned(),
            held: "what the receipt held".to_owned(),
            found: "what was found".to_owned(),
            at: at.clone(),
            by: "the test's operator".to_owned(),
        }],
    )?;
    Ok(at)
}

/// The stores a refused start names, in its order.
fn refused_stores(health: &Value) -> Vec<String> {
    health["stores"]
        .as_array()
        .map(|stores| {
            stores
                .iter()
                .filter_map(|store| store["store"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// `GET /health` while the start is refused: 503 `refused_to_start`.
async fn refused_health(service: &Service) -> Result<Value, Failure> {
    let (status, health) = service.get("/health", None).await?;
    assert_eq!(status, 503, "{health}");
    assert_eq!(health["refusal"], "refused_to_start", "{health}");
    Ok(health)
}

/// The number after `head ` in a store's words.
fn head(words: &Value) -> Result<u64, Failure> {
    let words = words.as_str().ok_or("no words")?;
    let rest = words.split("head ").nth(1).ok_or("no head")?;
    Ok(rest.split(' ').next().ok_or("no number")?.parse()?)
}

#[tokio::test]
async fn a_lost_whole_store_refuses_the_boot_makes_nothing_anew_and_serves_only_its_health()
-> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    grant(&service, &ada, &seeded.people[1].id.to_string(), "1").await?;
    machine(&service, &ada, "Ward desk", &agent(&seeded)?).await?;
    let mut gone = Vec::new();
    service
        .restart_between(None, |config| {
            let certificates = config.certificates_dir.clone().ok_or("no certificates")?;
            for dir in [
                config.log_dir.clone(),
                config.grant_log_dir.clone(),
                certificates,
            ] {
                std::fs::remove_dir_all(&dir)?;
                gone.push(dir);
            }
            Ok(())
        })
        .await?;
    let health = refused_health(&service).await?;
    let named = refused_stores(&health);
    for store in ["directory-log", "grant-log", "certificates"] {
        assert!(
            named.iter().any(|name| name == store),
            "{store} not named: {health}"
        );
    }
    for store in health["stores"].as_array().ok_or("no stores")? {
        assert_eq!(store["refusal"], "store_lost", "{store}");
        assert!(
            store["held"]
                .as_str()
                .is_some_and(|held| held.starts_with("head ")),
            "{store}"
        );
        assert!(
            store["found"]
                .as_str()
                .is_some_and(|found| found.starts_with("nothing at ")),
            "{store}"
        );
    }
    let command = health["command"].as_str().ok_or("no command")?;
    assert!(
        command.starts_with("lys identity accept-loss --store "),
        "{command}"
    );
    for store in &named {
        assert!(command.contains(&format!("--store {store}")), "{command}");
    }
    let computer = health["computer"].as_str().ok_or("no computer")?;
    assert!(!computer.is_empty());
    let reason = health["reason"].as_str().ok_or("no reason")?;
    assert!(
        reason.starts_with("refused_to_start: Lys refused to start: "),
        "{reason}"
    );
    assert!(
        reason.contains(&format!("run `{command}` on {computer}")),
        "{reason}"
    );
    assert_eq!(health["service"], "lys-identity-server", "{health}");
    for (status, answer) in [
        service.get("/me", Some(&ada)).await?,
        service.get("/grants", Some(&ada)).await?,
        service
            .post("/grants/roots", Some(&ada), &json!({}))
            .await?,
    ] {
        assert_eq!(status, 503, "{answer}");
        assert_eq!(answer["reason"], health["reason"], "{answer}");
    }
    for dir in gone {
        assert!(!dir.exists(), "{} was made anew", dir.display());
    }
    Ok(())
}

#[tokio::test]
async fn a_grant_log_short_of_its_receipt_refuses_the_boot_naming_both_heads() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = seeded.people[1].id.to_string();
    grant(&service, &ada, &bea, "1").await?;
    let copy = kept(&service, "grant-log");
    service
        .restart_between(None, |config| Ok(copy_tree(&config.grant_log_dir, &copy)?))
        .await?;
    grant(&service, &ada, &bea, "2").await?;
    service
        .restart_between(None, |config| Ok(put_back(&config.grant_log_dir, &copy)?))
        .await?;
    let health = refused_health(&service).await?;
    assert_eq!(refused_stores(&health), ["grant-log"], "{health}");
    let store = &health["stores"][0];
    assert_eq!(store["refusal"], "store_lost", "{store}");
    let (held, found) = (head(&store["held"])?, head(&store["found"])?);
    assert!(found < held, "{store}");
    let reason = health["reason"].as_str().ok_or("no reason")?;
    assert!(
        reason.contains(&format!("head {held} ")) && reason.contains(&format!("head {found} ")),
        "{reason}"
    );
    assert_eq!(
        health["command"],
        "lys identity accept-loss --store grant-log"
    );
    Ok(())
}

#[tokio::test]
async fn a_rewritten_grant_log_identity_refuses_the_boot_naming_both_identities() -> TestResult {
    use identity_contract::apps::{Auth, NOTES, ok, post, registered};
    let (mut service, _) = identity_contract::apps::seeded().await?;
    let admin = service.sign_in(login(ADMINISTRATOR)).await?;
    let notes = registered(&service, &admin, NOTES).await?;
    let page = ok(post(
        &service,
        "/grants/changes",
        Auth::Bearer(&notes),
        &json!({"limit": 4}),
    )
    .await?)?;
    let identity = page["log"]["identity"]
        .as_str()
        .ok_or("no identity")?
        .to_owned();
    let elsewhere = format!("{identity}-elsewhere");
    service
        .restart_between(None, |config| {
            let mut name = config
                .grant_log_dir
                .file_name()
                .ok_or("no name")?
                .to_os_string();
            name.push(".identity.json");
            let path = config.grant_log_dir.with_file_name(name);
            let mut recorded: Value = serde_json::from_slice(&std::fs::read(&path)?)?;
            recorded["identity"] = json!(elsewhere);
            std::fs::write(&path, serde_json::to_vec(&recorded)?)?;
            Ok(())
        })
        .await?;
    let health = refused_health(&service).await?;
    assert_eq!(refused_stores(&health), ["grant-log"], "{health}");
    let store = &health["stores"][0];
    assert_eq!(store["refusal"], "grant_log_identity_changed", "{store}");
    assert_eq!(
        store["held"],
        format!("identity {identity} at epoch 0"),
        "{store}"
    );
    assert_eq!(
        store["found"],
        format!("identity {elsewhere} at epoch 0"),
        "{store}"
    );
    Ok(())
}

#[tokio::test]
async fn a_revoke_is_an_honest_shrink_and_the_next_boot_serves() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let given = grant(&service, &ada, &seeded.people[1].id.to_string(), "1").await?;
    let body = json!({"operation": operation()?, "route": "api", "reason": "left the ward"});
    let (status, revoked) = service
        .post(&format!("/grants/{given}/revoke"), Some(&ada), &body)
        .await?;
    assert_eq!(status, 200, "{revoked}");
    service.restart().await?;
    let (status, health) = service.get("/health", None).await?;
    assert_eq!(status, 200, "{health}");
    let (status, me) = service.get("/me", Some(&ada)).await?;
    assert_eq!(status, 200, "{me}");
    Ok(())
}

/// The machines `GET /network` answers, by name.
async fn machine_names(service: &Service, cookie: &str) -> Result<Vec<String>, Failure> {
    let (status, network) = service.get("/network", Some(cookie)).await?;
    assert_eq!(status, 200, "{network}");
    Ok(network["machines"]
        .as_array()
        .ok_or("no machines")?
        .iter()
        .filter_map(|machine| machine["name"].as_str().map(str::to_owned))
        .collect())
}

#[tokio::test]
async fn a_removal_through_the_receipt_boots_and_the_same_removal_without_it_is_refused()
-> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let runs = agent(&seeded)?;
    machine(&service, &ada, "Kept desk", &runs).await?;
    let copy = kept(&service, "network.json");
    service
        .restart_between(None, |config| {
            std::fs::create_dir_all(copy.parent().ok_or("no folder")?)?;
            Ok(copy_tree(
                config.network_file.as_deref().ok_or("no network")?,
                &copy,
            )?)
        })
        .await?;
    machine(&service, &ada, "Pruned desk", &runs).await?;
    service
        .restart_between(None, |config| {
            let live = config.network_file.clone().ok_or("no network")?;
            receipt::remove_records(&config.stores_receipt_file, "network", 1, || {
                put_back(&live, &copy).map_err(|error| Failure::from(error.to_string()))
            })
        })
        .await?;
    assert_eq!(machine_names(&service, &ada).await?, ["Kept desk"]);

    machine(&service, &ada, "Pruned by hand", &runs).await?;
    service
        .restart_between(None, |config| {
            Ok(put_back(
                config.network_file.as_deref().ok_or("no network")?,
                &copy,
            )?)
        })
        .await?;
    let (status, health) = service.get("/health", None).await?;
    assert_eq!(
        status, 200,
        "an own store's loss refuses its routes alone: {health}"
    );
    let (status, refused) = service.get("/network", Some(&ada)).await?;
    assert_ne!(status, 200, "{refused}");
    let reason = refused["reason"].as_str().ok_or("no reason")?;
    assert!(
        reason.contains("Lys lost its computer records: the receipt holds 2 records"),
        "{reason}"
    );
    assert!(
        reason.contains("lys identity accept-loss --store network"),
        "{reason}"
    );
    Ok(())
}

#[tokio::test]
async fn a_lost_machines_file_refuses_only_its_routes_until_its_loss_is_accepted() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let runs = agent(&seeded)?;
    machine(&service, &ada, "Ward desk", &runs).await?;
    let mut network_file = PathBuf::new();
    service
        .restart_between(None, |config| {
            network_file = config.network_file.clone().ok_or("no network")?;
            Ok(std::fs::remove_file(&network_file)?)
        })
        .await?;
    let bea = service.sign_in(login(BEA)).await?;
    for path in ["/me", "/grants"] {
        let (status, answer) = service.get(path, Some(&bea)).await?;
        assert_eq!(status, 200, "{path}: {answer}");
    }
    let body = json!({
        "operation": operation()?, "name": "Second desk", "kind": "laptop",
        "runtime": "local launcher", "slots": 2, "may_run": [runs], "may_reach": [],
    });
    for (status, refused) in [
        service.get("/network", Some(&ada)).await?,
        service.post("/network/machines", Some(&ada), &body).await?,
    ] {
        assert_eq!(status, 503, "{refused}");
        let reason = refused["reason"].as_str().ok_or("no reason")?;
        assert!(
            reason.contains("Lys lost its computer records: the receipt holds 1 records"),
            "{reason}"
        );
        assert!(reason.contains("nothing at "), "{reason}");
        assert!(
            reason.contains("Run `lys identity accept-loss --store network` on "),
            "{reason}"
        );
    }
    assert!(!network_file.exists(), "the machines file was made anew");
    let mut at = String::new();
    service
        .restart_between(None, |config| {
            at = accept(config, "network")?;
            Ok(())
        })
        .await?;
    assert!(machine_names(&service, &ada).await?.is_empty());
    let written = receipt::read_receipt(&service.dir.path().join("install").join("stores.json"))?
        .ok_or("no receipt")?;
    let entry = written.stores.get("network").ok_or("network not counted")?;
    assert_eq!(entry.head, 0, "the receipt counts what was accepted");
    assert!(
        entry.read_at > at,
        "the receipt was written after the acceptance"
    );
    Ok(())
}

#[tokio::test]
async fn an_acceptance_serves_exactly_the_store_it_names() -> TestResult {
    let (mut service, seeded) = seeded().await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    grant(&service, &ada, &seeded.people[1].id.to_string(), "1").await?;
    service
        .restart_between(None, |config| {
            std::fs::remove_dir_all(&config.grant_log_dir)?;
            std::fs::remove_dir_all(
                config
                    .certificates_dir
                    .as_deref()
                    .ok_or("no certificates")?,
            )?;
            Ok(())
        })
        .await?;
    let health = refused_health(&service).await?;
    assert_eq!(refused_stores(&health).len(), 2, "{health}");
    service
        .restart_between(None, |config| {
            accept(config, "grant-log")?;
            Ok(())
        })
        .await?;
    let health = refused_health(&service).await?;
    assert_eq!(refused_stores(&health), ["certificates"], "{health}");
    let mut at = String::new();
    service
        .restart_between(None, |config| {
            at = accept(config, "certificates")?;
            Ok(())
        })
        .await?;
    let (status, health) = service.get("/health", None).await?;
    assert_eq!(status, 200, "{health}");
    let written = receipt::read_receipt(&service.dir.path().join("install").join("stores.json"))?
        .ok_or("no receipt")?;
    for store in ["grant-log", "certificates"] {
        let entry = written.stores.get(store).ok_or("not counted")?;
        assert!(entry.read_at > at, "{store} was not counted again");
    }
    Ok(())
}
