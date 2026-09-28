//! The review route: a person is shown what the agents answering to them
//! hold now, the root authority is shown every agent's, a grant that no
//! longer stands is not due, and an agent whose person is not active is
//! listed as having no one answering. The service runs with only its own
//! disposable log and the in-process issuer, and no other server.

use std::collections::BTreeSet;
use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";
const STRANGER: &str = "stranger-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

/// A service with Ada as the root authority and Bea, each with their agents,
/// and the cookies of Ada and Bea.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            seeded,
            ada,
            bea,
        })
    }

    /// Ada issues `person` a root grant on `resource` that may be lent to
    /// agents, and `person` lends it to `agent`. Answers both grant ids.
    async fn lend(
        &self,
        person: usize,
        cookie: &str,
        resource: &str,
    ) -> Result<(String, String), Box<dyn Error>> {
        let holder = &self.seeded.people[person];
        let root = json!({
            "operation": operation()?,
            "route": "api",
            "holder": holder.id.to_string(),
            "resource": { "kind": "doc", "id": resource },
            "relation": "alpha",
            "pass_on": { "kind": "to", "actions": ["read"], "recipients": ["agent"] },
            "window": { "starts_at": 0, "ends_at": null },
        });
        let (status, issued) = self
            .service
            .post("/grants/roots", Some(&self.ada), &root)
            .await?;
        assert_eq!(status, 200, "{issued}");
        let source = issued["grant"].as_str().ok_or("no grant")?.to_owned();
        let lent = json!({
            "operation": operation()?,
            "route": "api",
            "source": source,
            "recipient": holder.agents[0].id.to_string(),
            "responsible": holder.id.to_string(),
            "resource": { "kind": "doc", "id": resource },
            "relation": "beta",
            "pass_on": { "kind": "use_only" },
            "window": { "starts_at": 0, "ends_at": null },
        });
        let (status, issued) = self.service.post("/grants", Some(cookie), &lent).await?;
        assert_eq!(status, 200, "{issued}");
        Ok((
            source,
            issued["grant"].as_str().ok_or("no grant")?.to_owned(),
        ))
    }

    async fn reviews(&self, cookie: &str) -> Result<Value, Box<dyn Error>> {
        let (status, body) = self.service.get("/reviews", Some(cookie)).await?;
        assert_eq!(status, 200, "{body}");
        Ok(body)
    }
}

fn due_ids(view: &Value) -> Result<BTreeSet<String>, Box<dyn Error>> {
    view["due"]
        .as_array()
        .ok_or("due is not a list")?
        .iter()
        .map(|due| Ok(due["grant"]["id"].as_str().ok_or("no grant id")?.to_owned()))
        .collect()
}

#[tokio::test]
async fn a_person_reviews_their_own_agents_and_the_root_authority_every_agent() -> TestResult {
    let table = Table::set().await?;
    let (ada, bea) = (&table.seeded.people[0], &table.seeded.people[1]);
    let (_, of_ada) = table.lend(0, &table.ada, "1").await?;
    let (_, of_bea) = table.lend(1, &table.bea, "2").await?;

    let personal = table.reviews(&table.bea).await?;
    assert_eq!(personal["scope"], "personal");
    assert_eq!(due_ids(&personal)?, BTreeSet::from([of_bea.clone()]));
    let due = &personal["due"][0];
    assert_eq!(due["agent"]["id"], bea.agents[0].id.to_string());
    assert_eq!(due["agent"]["state"], "active");
    assert_eq!(due["reviewer"]["id"], bea.id.to_string());
    assert_eq!(due["grant"]["holder"], bea.agents[0].id.to_string());
    assert_eq!(
        due["grant"]["resource"],
        json!({ "kind": "doc", "id": "2" })
    );
    assert_eq!(personal["unanswered"], json!([]));
    let (_, listed) = table.service.get("/grants", Some(&table.bea)).await?;
    assert_eq!(personal["revision"], listed["revision"]);

    let whole = table.reviews(&table.ada).await?;
    assert_eq!(whole["scope"], "directory");
    assert_eq!(due_ids(&whole)?, BTreeSet::from([of_ada, of_bea]));
    let reviewers: BTreeSet<&str> = whole["due"]
        .as_array()
        .ok_or("due is not a list")?
        .iter()
        .filter_map(|due| due["reviewer"]["id"].as_str())
        .collect();
    let (ada_id, bea_id) = (ada.id.to_string(), bea.id.to_string());
    assert_eq!(
        reviewers,
        BTreeSet::from([ada_id.as_str(), bea_id.as_str()])
    );
    Ok(())
}

#[tokio::test]
async fn a_caller_with_no_person_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    table.lend(1, &table.bea, "1").await?;
    let (status, body) = table.service.get("/reviews", None).await?;
    assert_eq!(status, 401, "{body}");
    assert_eq!(body["refusal"], "NotSignedIn");
    let stranger = table.service.sign_in(login(STRANGER)).await?;
    let (status, body) = table.service.get("/reviews", Some(&stranger)).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "NoPerson");
    assert_eq!(body.get("due"), None, "{body}");
    Ok(())
}

#[tokio::test]
async fn a_grant_whose_source_is_revoked_is_no_longer_due() -> TestResult {
    let table = Table::set().await?;
    let (source, lent) = table.lend(1, &table.bea, "1").await?;
    assert_eq!(
        due_ids(&table.reviews(&table.bea).await?)?,
        BTreeSet::from([lent])
    );
    let revoke = json!({ "operation": operation()?, "route": "api", "reason": "review test" });
    let (status, revoked) = table
        .service
        .post(
            &format!("/grants/{source}/revoke"),
            Some(&table.ada),
            &revoke,
        )
        .await?;
    assert_eq!(status, 200, "{revoked}");
    assert_eq!(due_ids(&table.reviews(&table.bea).await?)?, BTreeSet::new());
    assert_eq!(due_ids(&table.reviews(&table.ada).await?)?, BTreeSet::new());
    Ok(())
}

#[tokio::test]
async fn an_agent_whose_person_is_suspended_has_no_one_answering() -> TestResult {
    let table = Table::set().await?;
    let bea = &table.seeded.people[1];
    assert_eq!(table.reviews(&table.ada).await?["unanswered"], json!([]));
    let moved =
        json!({ "operation": operation()?, "transition": "suspend", "reason": "review test" });
    let (status, body) = table
        .service
        .post(
            &format!("/identities/{}/transitions", bea.id),
            Some(&table.ada),
            &moved,
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let whole = table.reviews(&table.ada).await?;
    let unanswered = whole["unanswered"].as_array().ok_or("not a list")?;
    assert_eq!(
        unanswered.len(),
        1,
        "the retired agent is not listed: {whole}"
    );
    assert_eq!(unanswered[0]["agent"]["id"], bea.agents[0].id.to_string());
    assert_eq!(unanswered[0]["person"]["id"], bea.id.to_string());
    assert_eq!(unanswered[0]["person"]["state"], "suspended");
    Ok(())
}
