//! Nested team totals count each member once and retain unavailable sources.

use std::error::Error;

use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Table {
    service: Service,
    cookie: String,
    agents: Vec<String>,
}

impl Table {
    async fn fresh() -> TestResult<Self> {
        let (service, seeded) = Service::start_with(|config| {
            Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
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
            cookie,
            agents: seeded.people[0]
                .agents
                .iter()
                .map(|agent| agent.id.to_string())
                .collect(),
        })
    }
    async fn post(&self, path: &str, body: &Value) -> TestResult<Value> {
        let (status, answer) = self.service.post(path, Some(&self.cookie), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }
    async fn team(&self, parent: Option<&str>) -> TestResult<String> {
        let id = OperationId::generate()?.to_string();
        self.post(
            "/teams",
            &json!({"operation": id, "name": "Budget team", "parent": parent}),
        )
        .await?;
        Ok(id)
    }
    async fn member(&self, team: &str, agent: &str) -> TestResult {
        self.post(
            &format!("/teams/{team}/members"),
            &json!({"operation": OperationId::generate()?.to_string(), "member": agent}),
        )
        .await?;
        Ok(())
    }
    async fn get(&self, path: &str) -> TestResult<Value> {
        let (status, answer) = self.service.get(path, Some(&self.cookie)).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }
}

#[tokio::test]
async fn nested_team_usage_is_deduplicated_and_appears_in_each_agents_within_rows() -> TestResult {
    let table = Table::fresh().await?;
    let parent = table.team(None).await?;
    let child = table.team(Some(&parent)).await?;
    table.member(&parent, &table.agents[0]).await?;
    for agent in &table.agents[..2] {
        table.member(&child, agent).await?;
    }
    let at = jiff::Timestamp::now().as_millisecond();
    for (agent, tokens, running, dollars, percent) in [
        (&table.agents[0], 200, 100, 400_000_000, 49),
        (&table.agents[1], 300, 150, 100_000_000, 50),
    ] {
        table.post(&format!("/agents/{agent}/usage"), &json!({"event": agent, "at_ms": at, "tokens": tokens, "running_ms": running, "dollars_micros": dollars, "account": "shared-account", "plan_windows": [{"duration_minutes": 10_080, "used_percent": percent, "resets_at_ms": at + 604_800_000}]})).await?;
    }
    let limits = json!([
        {"unit": "tokens", "amount": 600, "period": "week", "act": "stop"},
        {"unit": "running_ms", "amount": 300, "period": "week", "act": "tell"},
        {"unit": "dollars", "amount": 600, "period": "week", "act": "stop"},
        {"unit": "plan_percent", "amount": 60, "period": "week", "act": "stop"}
    ]);
    let path = format!("/teams/{parent}/budget");
    let (status, answer) = send(
        &table.service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&table.cookie),
        Some(&json!({"limits": limits, "warn_at": null, "version": 0})),
    )
    .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["used"]
            .as_array()
            .ok_or("no used rows")?
            .iter()
            .map(|row| row["figure"].clone())
            .collect::<Vec<_>>(),
        vec![json!(500), json!(250), json!(500), json!(50)]
    );
    assert_eq!(
        table.get(&format!("/budgets/team/{parent}")).await?["used"],
        answer["used"]
    );
    for agent in &table.agents[..2] {
        let view = table.get(&format!("/budgets/agent/{agent}")).await?;
        let within = view["within"].as_array().ok_or("no within rows")?;
        assert_eq!(within.len(), 2, "{view}");
        let row = within
            .iter()
            .find(|row| row["team"] == parent)
            .ok_or("no parent budget")?;
        assert_eq!(row["used"], answer["used"]);
        assert_eq!(row["limits"], limits);
    }
    table.member(&child, &table.agents[2]).await?;
    let unknown = table.get(&path).await?;
    assert_eq!(unknown["used"][2]["figure"], Value::Null);
    assert!(
        unknown["used"][2]["unavailable"]
            .as_str()
            .is_some_and(|reason| reason.contains(&table.agents[2]))
    );
    assert_eq!(unknown["used"][3]["figure"], Value::Null);
    let (status, refused) = send(
        &table.service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&table.cookie),
        Some(&json!({"limits": limits, "warn_at": null, "version": 1})),
    )
    .await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "BudgetUnitUnavailable");
    assert_eq!(table.get(&path).await?["version"], 1);
    Ok(())
}
