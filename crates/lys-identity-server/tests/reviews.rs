//! The review route: a person is shown what the agents answering to them
//! hold now, the root authority is shown every agent's, a grant that no
//! longer stands is not due, and an agent whose person is not active is
//! listed as having no one answering. A grant is kept only by its reviewer
//! or the root authority and only while it stands; the same decision asked
//! again is recorded once. The service runs with only its own
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

#[path = "reviews/table.rs"]
mod table;
use table::Table;
#[path = "reviews/existing.rs"]
mod existing;
use existing::due_ids;
#[path = "reviews/batch.rs"]
mod batch;

#[tokio::test]
async fn service_refuses_zero_review_period_even_with_typed_configuration() -> TestResult {
    let started = Service::start_adjusted(
        identity_contract::harness::GRANT_MODEL,
        None,
        None,
        None,
        |config| config.reviews.period_days = 0,
        |_config| Ok(()),
    )
    .await;
    let refused = started.err().ok_or("the service accepted a zero review period")?;
    assert!(refused.to_string().contains("[reviews] period_days"), "{refused}");
    Ok(())
}

#[tokio::test]
async fn the_period_since_giving_or_keeping_is_shared_by_reviews_and_waiting() -> TestResult {
    let table = Table::set().await?;
    let (_, fresh) = table.lend_at(0, 0, &table.ada, "fresh", 1_800_000_000).await?;
    let (_, old) = table.lend_at(0, 0, &table.ada, "old", 1_800_000_000 - 31 * 86_400).await?;
    assert_eq!(due_ids(&table.reviews(&table.ada).await?)?, BTreeSet::from([old.clone()]));
    assert_eq!(table.waiting().await?[0]["review"]["grants"][0]["id"], old);
    let body = json!({"operation": operation()?, "note": "still needed"});
    assert_eq!(table.keep(&table.ada, &old, &body).await?.0, 200);
    assert!(due_ids(&table.reviews(&table.ada).await?)?.is_empty());
    assert!(table.waiting().await?.is_empty());
    table.clock.set(1_800_000_000 + 30 * 86_400 - 1);
    assert!(due_ids(&table.reviews(&table.ada).await?)?.is_empty());
    assert!(table.waiting().await?.is_empty());
    table.clock.set(1_800_000_000 + 30 * 86_400);
    assert_eq!(due_ids(&table.reviews(&table.ada).await?)?, BTreeSet::from([old, fresh]));
    assert_eq!(table.waiting().await?[0]["review"]["count"], 2);
    Ok(())
}

mod tom_26_october {
    use super::*;

    #[tokio::test]
    async fn six_agents_540_grants_then_two_real_decisions() -> TestResult {
        let table = Table::set().await?;
        let mut agents = Vec::new();
        for number in 0..6 {
            let id = if number < 3 {
                table.seeded.people[0].agents[number].id.to_string()
            } else {
                let (status, agent) = table.service.post("/agents", Some(&table.ada), &json!({
                    "operation": operation()?, "answers_to": table.seeded.people[0].id.to_string(),
                    "display_name": format!("Tom's helper {number}"),
                })).await?;
                assert_eq!(status, 200, "{agent}");
                agent["agent"].as_str().ok_or("agent id")?.to_owned()
            };
            if number != 0 {
                let transition = if number == 2 { "reinstate" } else { "activate" };
                let (status, active) = table.service.post(&format!("/identities/{id}/transitions"), Some(&table.ada), &json!({
                    "operation": operation()?, "transition": transition, "reason": "replay",
                })).await?;
                assert_eq!(status, 200, "{active}");
            }
            for grant in 0..90 {
                let (status, issued) = table.service.post("/grants/roots", Some(&table.ada), &json!({
                    "operation": operation()?, "route": "api", "holder": id,
                    "resource": {"kind": "doc", "id": format!("replay-{number}-{grant}")},
                    "relation": "beta", "pass_on": {"kind": "use_only"},
                    "window": {"starts_at": 1_800_000_000 - 31 * 86_400, "ends_at": null},
                })).await?;
                assert_eq!(status, 200, "{issued}");
            }
            agents.push(id);
        }
        assert_eq!(due_ids(&table.reviews(&table.ada).await?)?.len(), 540);
        let rows = table.waiting().await?;
        assert_eq!(rows.len(), 6, "Tom reads six lines, never 540");
        let mut remaining = 6;
        for row in rows {
            assert_eq!(row["review"]["count"], 90);
            let agent = row["review"]["agent"]["id"].as_str().ok_or("agent")?;
            assert!(agents.iter().any(|id| id == agent));
            let grants: Vec<&Value> = row["review"]["grants"].as_array().ok_or("grants")?.iter().map(|grant| &grant["id"]).collect();
            let (status, kept) = table.keep_all(&table.ada, agent, &json!({"operation": operation()?, "grants": grants})).await?;
            assert_eq!(status, 200, "{kept}");
            remaining -= 1;
            assert_eq!(table.waiting().await?.len(), remaining);
        }
        assert!(table.waiting().await?.is_empty());
        batch::request_and_app_action_decided_from_waiting(&table).await?;
        Ok(())
    }
}
