#![cfg(test)]
//! `GET /dashboard`: each part is what its own route answers, the waiting
//! counts are counted from `/requests`, `/drafts` and `/reviews`, and a caller
//! is shown only their own agents that are not retired.

use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use serde_json::{Value, json};

type Result<T = ()> = std::result::Result<T, Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String> {
    Ok(OperationId::generate()?.to_string())
}

/// Ada, the administrator and root authority, and Bea, with their seeded agents.
struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self> {
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

    async fn read(&self, path: &str, cookie: &str) -> Result<Value> {
        let (status, answer) = self.service.get(path, Some(cookie)).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    fn agents(&self, person: usize) -> Vec<String> {
        self.seeded.people[person]
            .agents
            .iter()
            .filter(|agent| agent.state != lys_identity::LifecycleState::Retired)
            .map(|agent| agent.id.to_string())
            .collect()
    }
}

fn shown(dashboard: &Value) -> Result<Vec<String>> {
    dashboard["agents"]
        .as_array()
        .ok_or("no agents")?
        .iter()
        .map(|row| {
            row["agent"]["id"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "a row has no agent id".into())
        })
        .collect()
}

#[tokio::test]
async fn an_agents_usage_and_goals_are_what_their_own_routes_answer() -> Result {
    let table = Table::set().await?;
    let agent = table.seeded.people[0].agents[0].id.to_string();
    let budget = json!({
        "limits": [{"unit": "context_percent", "amount": 40, "period": null, "act": "tell"}],
        "warn_at": null, "version": 0,
    });
    let (status, set) = identity_contract::apps::send(
        &table.service,
        reqwest::Method::PUT,
        &format!("/budgets/agent/{agent}"),
        identity_contract::apps::Auth::Cookie(&table.ada),
        Some(&budget),
    )
    .await?;
    assert_eq!(status, 200, "{set}");
    let goal = json!({ "operation": operation()?, "kind": "expectation", "words": "every screen reviewed" });
    let (status, set) = table
        .service
        .post(&format!("/agents/{agent}/goals"), Some(&table.ada), &goal)
        .await?;
    assert_eq!(status, 200, "{set}");

    // The agent's own usage is read first, settling what that read settles;
    // the dashboard then answers the same recorded usage without asking a runner.
    let usage = table
        .read(&format!("/agents/{agent}/usage"), &table.ada)
        .await?;
    let dashboard = table.read("/dashboard", &table.ada).await?;
    let goals = table
        .read(&format!("/agents/{agent}/goals"), &table.ada)
        .await?;
    let row = dashboard["agents"]
        .as_array()
        .ok_or("no agents")?
        .iter()
        .find(|row| row["agent"]["id"] == agent.as_str())
        .ok_or("the agent is not on the dashboard")?;
    assert_eq!(row["usage"], usage);
    assert_eq!(row["goals"], goals);
    let budget = table
        .read(&format!("/budgets/agent/{agent}"), &table.ada)
        .await?;
    assert_eq!(row["budget"], budget);
    assert_eq!(budget["limits"][0]["amount"], 40, "{budget}");
    assert!(
        !usage["used"].as_array().ok_or("no used")?.is_empty(),
        "{usage}"
    );
    assert_eq!(goals["goals"].as_array().ok_or("no goals")?.len(), 1);
    assert_eq!(row["sessions"], json!([]));
    assert_eq!(row["teams"], json!([]));
    assert_eq!(row["agent"]["display_name"], "Scribe");
    Ok(())
}

#[tokio::test]
async fn a_caller_sees_only_their_own_agents_and_never_a_retired_one() -> Result {
    let table = Table::set().await?;
    let ada = table.read("/dashboard", &table.ada).await?;
    let mut adas = table.agents(0);
    adas.sort();
    assert_eq!(shown(&ada)?, adas, "the administrator's own agents alone");
    let bea = table.read("/dashboard", &table.bea).await?;
    let retired = table.seeded.people[1]
        .agents
        .iter()
        .find(|agent| agent.state == lys_identity::LifecycleState::Retired)
        .ok_or("no retired agent seeded")?
        .id
        .to_string();
    assert_eq!(shown(&bea)?, table.agents(1));
    assert!(!shown(&bea)?.contains(&retired), "{bea}");

    let first = table.read("/dashboard?limit=2", &table.ada).await?;
    assert_eq!(shown(&first)?, adas[..2].to_vec());
    assert_eq!(first["total"], adas.len());
    let next = first["next"].as_str().ok_or("no next page")?;
    let rest = table
        .read(&format!("/dashboard?limit=2&after={next}"), &table.ada)
        .await?;
    assert_eq!(shown(&rest)?, adas[2..].to_vec());
    assert_eq!(rest["next"], Value::Null);
    Ok(())
}

/// The counts `/requests`, `/drafts` and `/reviews` answer for `cookie`.
async fn counted(table: &Table, cookie: &str) -> Result<Value> {
    let requests = table.read("/requests", cookie).await?;
    let requests = requests["requests"]
        .as_array()
        .ok_or("no requests")?
        .iter()
        .filter(|request| request["state"] == "waiting" && request["can_decide"] == true)
        .count();
    let drafts = table.read("/drafts", cookie).await?;
    let drafts = drafts["drafts"].as_array().ok_or("no drafts")?.len();
    let reviews = table.read("/reviews", cookie).await?;
    let reviews = reviews["due"].as_array().ok_or("no due")?.len();
    Ok(json!({ "requests": requests, "drafts": drafts, "reviews": reviews }))
}

#[tokio::test]
async fn the_waiting_counts_are_the_routes_own() -> Result {
    let table = Table::set().await?;
    let ask = json!({
        "operation": operation()?,
        "resource": { "kind": "doc", "id": "quarter" },
        "relation": "beta",
        "ends_at": null,
        "why": "to read the quarter's figures",
    });
    let (status, asked) = table
        .service
        .post("/requests", Some(&table.bea), &ask)
        .await?;
    assert_eq!(status, 200, "{asked}");
    for cookie in [&table.ada, &table.bea] {
        let dashboard = table.read("/dashboard", cookie).await?;
        assert_eq!(dashboard["waiting"], counted(&table, cookie).await?);
    }
    let ada = table.read("/dashboard", &table.ada).await?;
    assert_eq!(
        ada["waiting"]["requests"], 1,
        "the root authority decides it"
    );
    let bea = table.read("/dashboard", &table.bea).await?;
    assert_eq!(
        bea["waiting"]["requests"], 0,
        "the asker does not decide it"
    );
    Ok(())
}
