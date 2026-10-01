//! Nested team totals count each member once and retain unavailable sources.

use std::error::Error;
use std::sync::Arc;

use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::RuntimeStore;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Table {
    service: Service,
    cookie: String,
    agents: Vec<String>,
    sessions: Vec<String>,
}

impl Table {
    async fn fresh() -> TestResult<Self> {
        Self::with_session(false).await
    }

    async fn with_session(has_session: bool) -> TestResult<Self> {
        Self::with_sessions(usize::from(has_session)).await
    }

    async fn with_sessions(count: usize) -> TestResult<Self> {
        let (service, (seeded, sessions)) = Service::start_with(move |config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, "other-subject"])?;
            let mut sessions = Vec::with_capacity(count);
            if count > 0 {
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let mut runtime = RuntimeStore::open(
                    config
                        .runtime_dir
                        .as_deref()
                        .ok_or("no runtime directory")?,
                    key,
                )?;
                for _ in 0..count {
                    let session = OperationId::generate()?.to_string();
                    runtime.report(Report {
                        operation: OperationId::generate()?.to_string(),
                        session: session.clone(),
                        agent: Some(seeded.people[0].agents[2].id.to_string()),
                        machine: OperationId::generate()?.to_string(),
                        state: Reported::Starting,
                        what: "tracked session".to_owned(),
                        confirmation: String::new(),
                        reported_by: seeded.people[0].id.to_string(),
                        at: 1,
                        launch: None,
                    })?;
                    sessions.push(session);
                }
            }
            Ok((seeded, sessions))
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
            sessions,
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

    async fn charged_team(&self) -> TestResult<(String, String, Value)> {
        let parent = self.team(None).await?;
        let child = self.team(Some(&parent)).await?;
        self.member(&parent, &self.agents[0]).await?;
        for agent in &self.agents[..2] {
            self.member(&child, agent).await?;
        }
        let at = jiff::Timestamp::now().as_millisecond();
        for (agent, tokens, running, dollars, percent) in [
            (&self.agents[0], 200, 100, 400_000_000, 49),
            (&self.agents[1], 300, 150, 100_000_000, 50),
        ] {
            self.post(&format!("/agents/{agent}/usage"), &json!({"event": agent, "at_ms": at, "tokens": tokens, "running_ms": running, "dollars_micros": dollars, "account": "shared-account", "plan_windows": [{"duration_minutes": 10_080, "used_percent": percent, "resets_at_ms": at + 604_800_000}]})).await?;
        }
        let limits = json!([
            {"unit": "tokens", "amount": 600, "period": "week", "act": "stop"},
            {"unit": "running_ms", "amount": 300, "period": "week", "act": "tell"},
            {"unit": "dollars", "amount": 600, "period": "week", "act": "stop"},
            {"unit": "plan_percent", "amount": 60, "period": "week", "act": "stop"}
        ]);
        Ok((parent, child, limits))
    }

    async fn start(&self, agent: &str, route: &str) -> TestResult<(u16, Value)> {
        let machine = OperationId::generate()?.to_string();
        let body = if route == "start-command" {
            json!({"machine": machine, "operation": OperationId::generate()?.to_string()})
        } else {
            json!({"machine": machine, "profile_version": "1"})
        };
        self.service
            .post(
                &format!("/agents/{agent}/{route}"),
                Some(&self.cookie),
                &body,
            )
            .await
    }
}

#[tokio::test]
async fn nested_team_usage_is_deduplicated_and_appears_in_each_agents_within_rows() -> TestResult {
    let table = Table::fresh().await?;
    let (parent, child, limits) = table.charged_team().await?;
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
    assert_eq!(unknown["used"][2]["figure"], 500);
    assert_eq!(unknown["used"][2]["unavailable"], Value::Null);
    assert_eq!(unknown["used"][3]["figure"], 50);
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

#[tokio::test]
async fn a_member_with_a_session_and_no_dollar_report_refuses_both_starts() -> TestResult {
    let table = Table::with_session(true).await?;
    let (parent, child, limits) = table.charged_team().await?;
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
    let member = &table.agents[2];
    table.member(&child, member).await?;
    let unknown = table.get(&path).await?;
    let starts = [
        ("start-command", table.start(member, "start-command").await?),
        ("start", table.start(member, "start").await?),
    ];
    assert_eq!(unknown["used"][2]["figure"], Value::Null, "{unknown}");
    assert!(
        unknown["used"][2]["unavailable"]
            .as_str()
            .is_some_and(|reason| reason.contains(member)),
        "{unknown}"
    );
    assert_eq!(unknown["used"][3]["figure"], Value::Null, "{unknown}");
    for (route, (status, refused)) in starts {
        assert_eq!(status, 503, "{route}: {refused}");
        assert_eq!(
            refused["refusal"], "BudgetsUnavailable",
            "{route}: {refused}"
        );
        let reason = refused["reason"].as_str().ok_or("no refusal reason")?;
        assert!(
            reason.contains("dollars") && reason.contains(member),
            "{route}: {refused}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_reported_live_session_does_not_hide_an_unreported_live_session() -> TestResult {
    let table = Table::with_sessions(2).await?;
    let (parent, child, limits) = table.charged_team().await?;
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
    let member = &table.agents[2];
    let reported = table.sessions.first().ok_or("no reported session")?;
    let missing = table.sessions.get(1).ok_or("no unreported session")?;
    let at = jiff::Timestamp::now().as_millisecond();
    table.post(&format!("/agents/{member}/usage"), &json!({
        "event": "known-live-cost", "session": reported, "at_ms": at,
        "dollars_micros": 25_000_000, "account": "shared-account",
        "plan_windows": [{"duration_minutes": 10_080, "used_percent": 50, "resets_at_ms": at + 604_800_000}]
    })).await?;
    table.member(&child, member).await?;
    let unknown = table.get(&path).await?;
    let starts = [
        ("start-command", table.start(member, "start-command").await?),
        ("start", table.start(member, "start").await?),
    ];
    assert_eq!(unknown["used"][2]["figure"], Value::Null, "{unknown}");
    let reason = unknown["used"][2]["unavailable"]
        .as_str()
        .ok_or("no dollar gap")?;
    assert!(
        reason.contains(member) && reason.contains(missing),
        "{unknown}"
    );
    assert_eq!(unknown["used"][3]["figure"], 50, "{unknown}");
    for (route, (status, refused)) in starts {
        assert_eq!(status, 503, "{route}: {refused}");
        assert_eq!(
            refused["refusal"], "BudgetsUnavailable",
            "{route}: {refused}"
        );
        assert_eq!(
            refused["reason"],
            format!("BudgetsUnavailable: {reason}"),
            "{route}: {refused}"
        );
    }
    Ok(())
}
