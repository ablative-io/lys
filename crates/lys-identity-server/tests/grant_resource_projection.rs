//! Resource grants are checked after a restart, and unprojectable roots
//! are refused before the grant log changes.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_identity::grants::{
    Action, Grant, GrantId, GrantParts, Grants, MemoryRelationships, PassOn, Relation, Resource,
    Source, Window,
};
use lys_identity::signer::load_service_key;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, PersonId, Profile, Provenance,
    Transition,
};
use lys_identity_server::routes::open_directory;
use lys_identity_server::spicedb::SpiceDbSettings;
use lys_log_store::{FileLeafStore, LeafStore};
use serde_json::{Value, json};

#[path = "support/grant_resource_engine.rs"]
mod grant_resource_engine;

type Outcome = Result<(), Box<dyn Error>>;

fn prepare(config: &lys_identity_server::Config) -> Result<PersonId, Box<dyn Error>> {
    let actor = Actor::new(
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let mut directory = open_directory(config)?;
    let person = directory
        .found_organisation(
            actor.clone(),
            OperationId::generate()?,
            identity_contract::fixtures::organisation()?,
            Profile::new("Administrator")?,
            1,
        )?
        .founder;
    directory.bind_login(
        actor.clone(),
        OperationId::generate()?,
        person,
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        2,
    )?;
    directory.transition(
        actor,
        OperationId::generate()?,
        IdentityId::Person(person),
        Transition::Activate,
        "",
        3,
    )?;
    Ok(person)
}

fn old_grant(config: &lys_identity_server::Config, person: PersonId) -> Outcome {
    let path = config.grant_log_dir.clone();
    FileLeafStore::create(&path, &config.grant_log_origin)?;
    let mut grants = Grants::open(
        Box::new(move || FileLeafStore::open(&path)),
        load_service_key(&config.event_key_file)?,
        MemoryRelationships::default(),
        config.grant_model()?,
        person,
    )?;
    // A root of the store before organisations, written as that build
    // wrote it: one grant, at index zero, with no founding grant before it.
    grants.record_unjudged(
        Grant::new(GrantParts {
            id: GrantId::generate()?,
            issuer: IdentityId::Person(person),
            holder: IdentityId::Person(person),
            responsible: person,
            resource: Resource::new("person", "target")?,
            relation: Relation::new("alpha")?,
            actions: [Action::new("read")?, Action::new("write")?]
                .into_iter()
                .collect(),
            pass_on: PassOn::UseOnly,
            source: Source::Root,
            window: Window::new(0, None)?,
            model_version: 1,
            operation: OperationId::generate()?,
        })?,
        4,
    )?;
    assert_eq!(grants.revision(), 1);
    Ok(())
}

async fn started(
    preexisting: bool,
) -> Result<(grant_resource_engine::Engine, Service, PersonId, String), Box<dyn Error>> {
    let engine = grant_resource_engine::Engine::start(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicBool::new(false)),
    )?;
    let key = tempfile::NamedTempFile::new()?;
    std::fs::write(key.path(), "fixture-only")?;
    let (service, person) = Service::start_adjusted(
        GRANT_MODEL,
        Some(SpiceDbSettings {
            endpoint: engine.endpoint.clone(),
            key_file: key.path().to_owned(),
            mirror: "resource_fixture".to_owned(),
        }),
        None,
        None,
        |config| {
            if let Some(settings) = &mut config.spicedb {
                settings.key_file = config.log_dir.with_file_name("engine.key");
            }
            config.requests_dir = None;
            config.certificates_dir = None;
            config.network_file = None;
            config.roles_file = None;
            config.provisioning_file = None;
            config.homes_dir = None;
            config.runtime_dir = None;
            config.teams_dir = None;
            config.stops_dir = None;
            config.budgets_dir = None;
            config.goals_dir = None;
            config.reviews_dir = None;
        },
        move |config| {
            let path = &config
                .spicedb
                .as_ref()
                .ok_or("engine configuration missing")?
                .key_file;
            std::fs::write(path, "fixture-only")?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
            let person = prepare(config)?;
            if preexisting {
                old_grant(config, person)?;
            }
            Ok(person)
        },
    )
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "fixture@example.test".to_owned(),
        })
        .await?;
    Ok((engine, service, person, cookie))
}

fn root(person: PersonId, kind: &str) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(), "route": "browser",
        "holder": person.to_string(), "resource": {"kind": kind, "id": "target"},
        "relation": "alpha", "pass_on": {"kind": "use_only"},
        "window": {"starts_at": 0, "ends_at": null},
    }))
}

async fn restart_grant(kind: &str) -> Outcome {
    let (mut engine, mut service, person, cookie) = started(false).await?;
    let body = root(person, kind)?;
    let issued = service.post("/grants/roots", Some(&cookie), &body).await?;
    assert_eq!(issued.0, 200, "{kind}: {}", issued.1);
    let grant = issued.1["grant"].as_str().ok_or("issued grant missing")?;
    service.restart().await?;
    let (status, answer) = service
        .post(
            "/grants/check",
            Some(&cookie),
            &json!({
                "route": "browser", "resource": {"kind": kind, "id": "target"}, "action": "read",
            }),
        )
        .await?;
    assert_eq!(status, 200, "{kind} after restart: {answer}");
    assert_eq!(answer["grant"], grant, "{kind} after restart: {answer}");
    service.close()?;
    engine.stop()?;
    Ok(())
}

#[tokio::test]
async fn a_person_resource_root_survives_restart_and_checks() -> Outcome {
    restart_grant("person").await
}

#[tokio::test]
async fn an_agent_resource_root_survives_restart_and_checks() -> Outcome {
    restart_grant("agent").await
}

#[tokio::test]
async fn a_service_account_resource_root_keeps_its_restart_behavior() -> Outcome {
    restart_grant("service_account").await
}

#[tokio::test]
async fn an_unprojectable_root_names_its_kind_and_owner_without_committing() -> Outcome {
    let (mut engine, mut service, person, cookie) = started(false).await?;
    let (status, opened) = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(status, 200, "{opened}");
    let path = service.dir.path().join("grant-log");
    let before = FileLeafStore::open(&path)?.pinned();
    for kind in [
        "grant",
        "lys_revision",
        "lys_mirror",
        "invalid-kind",
        "xy",
        "ends_",
    ] {
        let body = root(person, kind)?;
        let (status, answer) = service.post("/grants/roots", Some(&cookie), &body).await?;
        assert_eq!(status, 400, "{kind}: {answer}");
        assert_eq!(answer["refusal"], "ResourceKindUnheld", "{kind}: {answer}");
        let words = answer["reason"].as_str().ok_or("refusal reason missing")?;
        assert!(words.contains(kind), "{words}");
        assert!(
            words.contains("Lys administrator") || words.contains("Lys maintainer"),
            "{words}"
        );
        assert_eq!(
            FileLeafStore::open(&path)?.pinned(),
            before,
            "{kind} committed"
        );
    }
    service.restart().await?;
    service.close()?;
    engine.stop()?;
    Ok(())
}

#[tokio::test]
async fn a_preexisting_person_grant_starts_against_an_old_engine_schema() -> Outcome {
    let (mut engine, mut service, _, cookie) = started(true).await?;
    let path = service.dir.path().join("grant-log");
    let before = FileLeafStore::open(&path)?.pinned();
    // The person's grant, the boot's stamp of it with the organisation it
    // predates, and the founder's administrator grant (DIRECTORY-109 R3).
    assert_eq!(FileLeafStore::open(&path)?.extent(), 3);
    let (status, answer) = service.get("/grants", Some(&cookie)).await?;
    assert_eq!(status, 200, "old person grant at start: {answer}");
    assert_eq!(
        FileLeafStore::open(&path)?.pinned(),
        before,
        "start wrote a replacement grant"
    );
    let question = json!({"route": "browser", "resource": {"kind": "person", "id": "target"}, "action": "read"});
    for again in [false, true] {
        if again {
            service.restart().await?;
        }
        let (status, answer) = service
            .post("/grants/check", Some(&cookie), &question)
            .await?;
        assert_eq!(status, 200, "preexisting person grant: {answer}");
    }
    service.close()?;
    engine.stop()?;
    Ok(())
}
