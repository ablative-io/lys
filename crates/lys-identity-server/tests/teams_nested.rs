//! Parent and lead change tree reach without granting authority.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

struct Table {
    service: Service,
    seeded: Seeded,
    cookie: String,
}

impl Table {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR])?)).await?;
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
    let table = Table::start().await?;
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
