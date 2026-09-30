//! Parent and lead change tree reach without granting authority.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

pub use lys_identity_server::{config, error, read_views, routes};

#[path = "support/teams_before_nesting_state.rs"]
pub mod teams_state;
#[path = "support/teams_before_nesting_store.rs"]
pub mod teams_store;

type TestResult = Result<(), Box<dyn Error>>;

struct Table {
    service: Service,
    seeded: Seeded,
    cookie: String,
}

impl Table {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_with(|config| {
            Ok(seed_configured(config, [ADMINISTRATOR, "another-subject"])?)
        })
        .await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        Ok(Self {
            service,
            seeded,
            cookie,
        })
    }

    async fn post(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.cookie), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    async fn team(&self, name: &str, parent: Option<&str>) -> Result<String, Box<dyn Error>> {
        let operation = OperationId::generate()?.to_string();
        let answer = self
            .post(
                "/teams",
                &json!({
                    "operation": operation, "name": name, "parent": parent, "lead": null,
                }),
            )
            .await?;
        assert_eq!(answer["id"], operation);
        assert_eq!(answer["parent"], json!(parent));
        assert!(
            answer
                .as_object()
                .is_some_and(|object| object.contains_key("lead"))
        );
        Ok(operation)
    }

    async fn refused(&self, path: &str, body: &Value, name: &str) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.cookie), body).await?;
        assert_eq!(status, 409, "{path}: {answer}");
        assert_eq!(answer["refusal"], name, "{answer}");
        Ok(answer)
    }
}

fn nesting(parent: Option<&str>, lead: Option<&str>) -> Result<Value, Box<dyn Error>> {
    Ok(json!({ "operation": OperationId::generate()?.to_string(), "parent": parent, "lead": lead }))
}

#[tokio::test]
async fn a_team_keeps_its_parent_and_a_lead_who_is_a_member() -> TestResult {
    let mut table = Table::start().await?;
    let root = table.team("root", None).await?;
    let child = table.team("child", Some(&root)).await?;
    let lead = table.seeded.people[0].agents[0].id.to_string();
    table
        .post(
            &format!("/teams/{child}/members"),
            &json!({
                "operation": OperationId::generate()?.to_string(), "member": lead,
            }),
        )
        .await?;
    let body = nesting(Some(&root), Some(&lead))?;
    let route = format!("/teams/{child}/nesting");
    let changed = table.post(&route, &body).await?;
    assert_eq!(changed["parent"], root);
    assert_eq!(changed["lead"], lead);
    assert_eq!(
        table.post(&route, &body).await?,
        changed,
        "retry writes nothing"
    );
    table.service.restart().await?;
    let (status, reopened) = table
        .service
        .get(&format!("/teams/{child}"), Some(&table.cookie))
        .await?;
    assert_eq!(status, 200, "{reopened}");
    assert_eq!(reopened["parent"], root);
    assert_eq!(reopened["lead"], lead);
    let removed = table
        .post(
            &format!("/teams/{child}/members/{lead}/remove"),
            &json!({"operation": OperationId::generate()?.to_string()}),
        )
        .await?;
    assert_eq!(removed["lead"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn a_parent_cycle_names_both_teams_and_keeps_the_previous_tree() -> TestResult {
    let table = Table::start().await?;
    let root = table.team("root", None).await?;
    let child = table.team("child", Some(&root)).await?;
    let answer = table
        .refused(
            &format!("/teams/{root}/nesting"),
            &nesting(Some(&child), None)?,
            "team_parent_cycle",
        )
        .await?;
    let reason = answer["reason"].as_str().ok_or("refusal has no reason")?;
    assert!(reason.contains(&root), "{answer}");
    assert!(reason.contains(&child), "{answer}");
    let (status, unchanged) = table
        .service
        .get(&format!("/teams/{root}"), Some(&table.cookie))
        .await?;
    assert_eq!(status, 200, "{unchanged}");
    assert_eq!(unchanged["parent"], Value::Null);
    table
        .refused(
            &format!("/teams/{root}/nesting"),
            &nesting(Some(&root), None)?,
            "team_parent_cycle",
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn a_lead_outside_the_membership_is_refused_without_changing_the_team() -> TestResult {
    let table = Table::start().await?;
    let team = table.team("root", None).await?;
    let lead = table.seeded.people[0].agents[0].id.to_string();
    let answer = table
        .refused(
            &format!("/teams/{team}/nesting"),
            &nesting(None, Some(&lead))?,
            "team_lead_not_member",
        )
        .await?;
    assert!(
        answer["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains(&lead)),
        "{answer}"
    );
    let (status, unchanged) = table
        .service
        .get(&format!("/teams/{team}"), Some(&table.cookie))
        .await?;
    assert_eq!(status, 200, "{unchanged}");
    assert_eq!(unchanged["lead"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn the_archived_writer_store_and_snapshot_read_as_top_level_without_a_lead() -> TestResult {
    use std::sync::Arc;

    use lys_core::Ed25519Identity;
    use lys_log_store::{FileLeafStore, FrontierLog};
    use teams_state::{Created, DOMAIN, Held, Line};

    assert_eq!(teams_state::SOURCE_COMMIT, teams_store::SOURCE_COMMIT);
    let team = OperationId::generate()?.to_string();
    let old_team = team.clone();
    let (service, before) = Service::start_with(move |config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, "another-subject"])?;
        let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
        let path = config.teams_dir.as_deref().ok_or("no teams directory")?;
        let mut store = teams_store::TeamStore::open(path, Arc::clone(&key))?;
        store.keep(Line::Created(Created {
            id: old_team,
            owner: seeded.people[0].id.to_string(),
            name: "old team".to_owned(),
            description: "kept by the archived writer".to_owned(),
            by: read_views::Login {
                provider: config.link_audit_source.issuer.clone(),
                subject: ADMINISTRATOR.to_owned(),
            },
            at: 1,
        }))?;
        let held = Held {
            teams: store.teams().to_vec(),
            checked: None,
        };
        let bytes = held.encode()?;
        let sealed: Value = serde_json::from_slice(&bytes)?;
        let old = sealed["held"]["teams"][0]
            .as_object()
            .ok_or("no old team")?;
        assert!(!old.contains_key("parent"));
        assert!(!old.contains_key("lead"));
        drop(store);
        let (mut log, tail) = FrontierLog::open(FileLeafStore::open(path)?)?;
        assert_eq!(tail.leaves.len(), 1);
        log.write_snapshot(DOMAIN, &bytes, &key)?;
        Ok(sealed)
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let (status, answer) = service
        .get(&format!("/teams/{team}"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["name"],
        before["held"]["teams"][0]["created"]["name"]
    );
    let object = answer.as_object().ok_or("team answer is not an object")?;
    assert!(
        object.contains_key("parent"),
        "old team has no explicit parent answer"
    );
    assert!(
        object.contains_key("lead"),
        "old team has no explicit lead answer"
    );
    assert_eq!(answer["parent"], Value::Null);
    assert_eq!(answer["lead"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn a_nested_teams_owner_cannot_change_its_parent() -> TestResult {
    let other = "another-subject";
    let (service, seeded) =
        Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, other])?)).await?;
    assert_eq!(seeded.people.len(), 2);
    let owner = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let other = service
        .sign_in(Login {
            subject: other.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let parent = OperationId::generate()?.to_string();
    let child = OperationId::generate()?.to_string();
    for (id, cookie) in [(&parent, &owner), (&child, &other)] {
        let (status, answer) = service
            .post(
                "/teams",
                Some(cookie),
                &json!({"operation":id,"name":"team"}),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
    }
    for (route, body) in [
        (
            "/teams".to_owned(),
            json!({"operation": OperationId::generate()?.to_string(), "name":"child", "parent":parent}),
        ),
        (
            format!("/teams/{child}/nesting"),
            nesting(Some(&parent), None)?,
        ),
        (
            format!("/teams/{child}/nesting"),
            nesting(Some(&OperationId::generate()?.to_string()), None)?,
        ),
    ] {
        let (status, answer) = service.post(&route, Some(&other), &body).await?;
        assert_eq!(status, 403, "{answer}");
        assert_eq!(answer["refusal"], "NotAdmitted");
    }
    let (status, answer) = service
        .post(
            &format!("/teams/{child}/nesting"),
            Some(&owner),
            &nesting(Some(&parent), None)?,
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let (status, answer) = service
        .post(
            &format!("/teams/{parent}/nesting"),
            Some(&other),
            &nesting(None, None)?,
        )
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");
    Ok(())
}

#[tokio::test]
async fn plain_creation_keeps_the_old_event_while_nesting_refuses_a_reversible_upgrade()
-> TestResult {
    use identity_contract::harness::GRANT_MODEL;
    use lys_core::Ed25519Identity;
    use lys_identity_server::teams_state::Line as CurrentLine;
    use lys_identity_server::teams_store::TeamStore as CurrentStore;
    use std::sync::Arc;

    let temporary = tempfile::tempdir()?;
    let intent = temporary.path().join("upgrade.intent");
    let (service, (teams, key)) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.operator_upgrade_file = Some(intent.clone()),
        |config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, "another-subject"])?;
            assert_eq!(seeded.people.len(), 1);
            Ok((
                config.teams_dir.clone().ok_or("no teams directory")?,
                config.event_key_file.clone(),
            ))
        },
    )
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    std::fs::write(&intent, b"pending")?;
    let id = OperationId::generate()?.to_string();
    let (status, answer) = service
        .post(
            "/teams",
            Some(&cookie),
            &json!({"operation":id,"name":"plain team"}),
        )
        .await?;
    assert_eq!(status, 200, "{answer}");
    let stored = CurrentStore::open(&teams, Arc::new(Ed25519Identity::load(&key)?))?;
    assert!(matches!(
        stored.recorded(&id),
        Some(CurrentLine::Created(_))
    ));
    drop(stored);
    let (status, refused) = service.post("/teams",Some(&cookie),&json!({"operation":OperationId::generate()?.to_string(),"name":"nested team","parent":id})).await?;
    assert_eq!(status, 503, "{refused}");
    assert_eq!(refused["refusal"], "TeamsUnavailable");
    let (status, refused) = service
        .post(
            &format!("/teams/{id}/nesting"),
            Some(&cookie),
            &nesting(None, None)?,
        )
        .await?;
    assert_eq!(status, 503, "{refused}");
    assert_eq!(refused["refusal"], "TeamsUnavailable");
    Ok(())
}
