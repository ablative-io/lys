#![cfg(test)]
//! A real pre-rule team log is recovered without erasing memberships or
//! repeating recovery; only an administrator can release a recorded hold.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login as SignIn;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity::signer::load_service_key;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::read_views::Login;
use lys_identity_server::teams_state::{Changed, Created, Line};
use lys_identity_server::teams_store::TeamStore;
use serde_json::{Value, json};

type ResultOf<T = ()> = Result<T, Box<dyn Error>>;
const BEA: &str = "bea-subject";

fn op() -> ResultOf<String> {
    Ok(OperationId::generate()?.to_string())
}

fn login(subject: &str) -> SignIn {
    SignIn {
        subject: subject.to_owned(),
        email: "fixture@example.test".to_owned(),
    }
}

struct Legacy {
    service: Service,
    team: String,
    members: Vec<String>,
}

impl Legacy {
    async fn open() -> ResultOf<Self> {
        let (service, (team, members)) = Service::start_with(|config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
            let team = op()?;
            let members = vec![
                seeded.people[1].id.to_string(),
                seeded.people[1].agents[0].id.to_string(),
                seeded.people[0].id.to_string(),
                seeded.people[0].agents[0].id.to_string(),
            ];
            let by = Login {
                provider: config.issuer.clone(),
                subject: BEA.to_owned(),
            };
            let dir = config
                .teams_dir
                .as_deref()
                .ok_or("teams directory missing")?;
            let key = Arc::new(load_service_key(&config.event_key_file)?);
            let mut store = TeamStore::open(dir, key)?;
            store.keep(Line::Created(Created {
                id: team.clone(),
                owner: members[0].clone(),
                name: "Legacy team".to_owned(),
                description: "Made under the earlier membership rule".to_owned(),
                by: by.clone(),
                at: 1,
            }))?;
            for member in &members {
                store.keep(Line::Added(Changed {
                    operation: op()?,
                    team: team.clone(),
                    member: member.clone(),
                    by: by.clone(),
                    at: 2,
                }))?;
            }
            Ok((team, members))
        })
        .await?;
        Ok(Self {
            service,
            team,
            members,
        })
    }

    async fn read(&self, cookie: &str) -> ResultOf<Value> {
        let (status, answer) = self
            .service
            .get(&format!("/teams/{}", self.team), Some(cookie))
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }
}

#[tokio::test]
async fn legacy_foreign_members_are_held_and_rightful_members_and_history_stay() -> ResultOf {
    let mut table = Legacy::open().await?;
    let admin = table.service.sign_in(login(ADMINISTRATOR)).await?;
    let before = table.read(&admin).await?;
    assert_eq!(before["members"], json!(table.members));
    let held = before["held"]
        .as_array()
        .ok_or("no named held-membership list")?;
    assert_eq!(held.len(), 2, "{before}");
    for member in &table.members[2..] {
        let entry = held
            .iter()
            .find(|entry| entry["member"] == *member)
            .ok_or("foreign member not held")?;
        assert!(
            !entry["reason"]
                .as_str()
                .ok_or("hold has no reason")?
                .is_empty()
        );
    }
    table.service.restart().await?;
    let admin = table.service.sign_in(login(ADMINISTRATOR)).await?;
    assert_eq!(
        table.read(&admin).await?,
        before,
        "restart must not add new holds or change their evidence"
    );
    Ok(())
}

#[tokio::test]
async fn only_the_administrator_confirms_a_held_membership_and_retry_is_one_act() -> ResultOf {
    let table = Legacy::open().await?;
    let admin = table.service.sign_in(login(ADMINISTRATOR)).await?;
    let bea = table.service.sign_in(login(BEA)).await?;
    let path = format!("/teams/{}/members/{}/confirm", table.team, table.members[3]);
    let body = json!({ "operation": op()? });
    let (status, answer) = table.service.post(&path, Some(&bea), &body).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted", "{answer}");
    let (status, confirmed) = table.service.post(&path, Some(&admin), &body).await?;
    assert_eq!(status, 200, "{confirmed}");
    assert_eq!(confirmed["held"].as_array().map(Vec::len), Some(1));
    assert_eq!(confirmed["members"], json!(table.members));
    let (status, again) = table.service.post(&path, Some(&admin), &body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, confirmed);
    Ok(())
}
