//! Reset spend stays charged; unknown stop figures refuse before either start handler.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::budgets_state::{
    Act, Budget, Holder, HolderKind, Length, Measure, Period, Usage,
};
use lys_identity_server::budgets_store::BudgetStore;
use lys_identity_server::dev_seed::seed_configured;
use lys_runner::tracking::{Figures, Unavailable};
use lys_runner::tracking_budget::cost_delta;
use lys_runner::tracking_store::SourceState;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Table {
    service: Service,
    cookie: String,
    agent: String,
}

impl Table {
    async fn with_usage(uses: Vec<Usage>) -> TestResult<Self> {
        let (service, agent) = Service::start_with(move |config| {
            let seed = seed_configured(config, [ADMINISTRATOR, "other-subject"])?;
            let agent = seed.people[0].agents[0].id.to_string();
            let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
            let mut store = BudgetStore::open(
                config
                    .budgets_dir
                    .as_deref()
                    .ok_or("no budgets directory")?,
                key,
            )?;
            store.set(
                Budget {
                    holder: Holder {
                        kind: HolderKind::Agent,
                        id: agent.clone(),
                    },
                    measure: Measure::Dollars,
                    limit: 50,
                    period: Some(Period {
                        length: Length::Week,
                        zone: "UTC".to_owned(),
                    }),
                    act: Act::Stop,
                    version: 0,
                    by: seed.people[0].id.to_string(),
                    at: 1,
                },
                0,
            )?;
            for usage in uses {
                store.charge(Usage {
                    agent: agent.clone(),
                    ..usage
                })?;
            }
            Ok(agent)
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
            agent,
        })
    }

    async fn start(&self, route: &str) -> TestResult<(u16, Value)> {
        let machine = OperationId::generate()?.to_string();
        let body = if route == "start-command" {
            json!({"machine": machine, "operation": OperationId::generate()?.to_string()})
        } else {
            json!({"machine": machine, "profile_version": "1"})
        };
        self.service
            .post(
                &format!("/agents/{}/{route}", self.agent),
                Some(&self.cookie),
                &body,
            )
            .await
    }

    async fn figure(&self) -> TestResult<Value> {
        let (status, body) = self
            .service
            .get(
                &format!("/budgets/agent/{}", self.agent),
                Some(&self.cookie),
            )
            .await?;
        assert_eq!(status, 200, "{body}");
        let used = body["used"].as_array().ok_or("no used figures")?;
        assert_eq!(used.len(), 1, "{body}");
        assert_eq!(used[0]["unit"], "dollars", "{body}");
        Ok(used[0].clone())
    }
}

#[tokio::test]
async fn a_mid_period_cost_reset_keeps_an_exhausted_cap_and_counts_the_new_total() -> TestResult {
    let at_ms = jiff::Timestamp::now().as_millisecond();
    let mut source = SourceState::default();
    let mut observations = Vec::new();
    for (event, total) in [
        ("cap", 50_000_000),
        ("reset", 10_000_000),
        ("later", 15_000_000),
    ] {
        let mut figures = Figures {
            dollars_micros: Some(total),
            ..Figures::default()
        };
        let mut notes = Vec::new();
        cost_delta(&mut source, &mut figures, &mut notes);
        observations.push(Usage {
            event: event.to_owned(),
            at_ms,
            session: Some("provider-session".to_owned()),
            dollars_micros: figures.dollars_micros,
            native_snapshot: true,
            unavailable: notes,
            ..Usage::default()
        });
    }
    let deltas: Vec<_> = observations
        .iter()
        .map(|usage| usage.dollars_micros)
        .collect();
    let reset_notes = observations[1].unavailable.clone();
    let table = Table::with_usage(observations).await?;
    for route in ["start-command", "start"] {
        let (status, answer) = table.start(route).await?;
        assert_eq!(status, 409, "{route}: {answer}");
        assert_eq!(answer["refusal"], "BudgetExhausted", "{route}: {answer}");
        let words = answer.to_string();
        assert!(
            words.contains("dollars") && words.contains("week") && words.contains("resets"),
            "{route}: {answer}"
        );
    }
    assert_eq!(
        deltas,
        vec![Some(50_000_000), Some(10_000_000), Some(5_000_000)]
    );
    assert_eq!(source.reported_cost_micros, Some(15_000_000));
    assert!(
        reset_notes
            .iter()
            .any(|note| note.figure == "dollars_micros" && note.reason.contains("reset")),
        "{reset_notes:?}"
    );
    let used = table.figure().await?;
    assert_eq!(used["figure"], 65, "{used}");
    assert_eq!(used["unavailable"], Value::Null, "{used}");
    Ok(())
}

#[tokio::test]
async fn an_unavailable_stop_figure_refuses_both_starts_with_the_source_reason() -> TestResult {
    let reason = "reported_cost_absent";
    let table = Table::with_usage(vec![Usage {
        event: "missing-cost".to_owned(),
        at_ms: jiff::Timestamp::now().as_millisecond(),
        session: Some("provider-session".to_owned()),
        native_snapshot: true,
        unavailable: vec![Unavailable {
            figure: "dollars_micros".to_owned(),
            reason: reason.to_owned(),
        }],
        ..Usage::default()
    }])
    .await?;
    let used = table.figure().await?;
    assert_eq!(used["figure"], Value::Null, "{used}");
    assert_eq!(used["unavailable"], reason, "{used}");
    for route in ["start-command", "start"] {
        let (status, answer) = table.start(route).await?;
        assert_eq!(status, 503, "{route}: {answer}");
        assert_eq!(answer["refusal"], "BudgetsUnavailable", "{route}: {answer}");
        assert!(
            answer["reason"]
                .as_str()
                .ok_or("no refusal reason")?
                .contains(reason),
            "{route}: {answer}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn a_fresh_agent_has_explicit_zero_spend_and_passes_start_admission() -> TestResult {
    let table = Table::with_usage(Vec::new()).await?;
    let used = table.figure().await?;
    assert_eq!(used["figure"], 0, "{used}");
    assert_eq!(used["unavailable"], Value::Null, "{used}");
    let (status, answer) = table.start("start-command").await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(
        answer["refusal"], "LaunchRecordMissing",
        "admitted past the budget check: {answer}"
    );
    let (status, answer) = table.start("start").await?;
    assert_eq!(status, 409, "{answer}");
    let checks = answer["checks"].as_array().ok_or("no start checks")?;
    assert_eq!(checks.len(), 5, "admitted past the budget check: {answer}");
    assert_eq!(checks[0]["result"], "passed", "{answer}");
    for check in &checks[1..] {
        assert_eq!(check["result"], "check_record_missing", "{answer}");
    }
    let refused = answer["refused"].as_array().ok_or("no start refusals")?;
    assert_eq!(refused.len(), 4, "{answer}");
    assert!(
        refused
            .iter()
            .all(|refusal| refusal["refusal"] == "check_record_missing"),
        "{answer}"
    );
    Ok(())
}
