//! Missing context stops the affected sessions and remains visible in budget reads.

use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::budgets_limits::{Limit, Limits};
use lys_identity_server::budgets_state::{Act, Holder, HolderKind, Measure};
use lys_identity_server::budgets_store::BudgetStore;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::RuntimeStore;
use lys_runner::operations::{Operation, OperationRequest};
use lys_runner::protocol::{Greeting, Reply, verify_request};
use lys_runner::{Act as RunnerAct, Answer};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::oneshot;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Runner {
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<Vec<Operation>, String>>>,
}

impl Runner {
    fn start(listener: UnixListener, key: [u8; 32]) -> Self {
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut operations = Vec::new();
            loop {
                let socket = tokio::select! {
                    _ = &mut stopped => return Ok(operations),
                    accepted = listener.accept() => accepted.map_err(|error| error.to_string())?.0,
                };
                let (read, mut write) = socket.into_split();
                let greeting = Greeting::fresh("0123456789abcdef0123456789abcdef");
                write
                    .write_all(format!("{}\n", greeting.line()).as_bytes())
                    .await
                    .map_err(|error| error.to_string())?;
                let line = BufReader::new(read)
                    .lines()
                    .next_line()
                    .await
                    .map_err(|error| error.to_string())?
                    .ok_or("runner request ended before its line")?;
                match verify_request(&line, &key, &greeting).map_err(|error| error.to_string())? {
                    RunnerAct::AsCaller { done, .. } => match *done {
                        RunnerAct::Operate { operation } => operations.push(operation),
                        other => return Err(format!("unexpected act for a caller: {other:?}")),
                    },
                    other => return Err(format!("unexpected runner act: {other:?}")),
                }
                let reply = Reply {
                    version: lys_runner::protocol::PROTOCOL_VERSION,
                    answer: Answer::Refused {
                        refusal: "fixture_stop_observed".to_owned(),
                        words: "the fixture records requests without starting processes".to_owned(),
                        oldest: None,
                    },
                };
                let line = serde_json::to_string(&reply).map_err(|error| error.to_string())?;
                write
                    .write_all(format!("{line}\n").as_bytes())
                    .await
                    .map_err(|error| error.to_string())?;
            }
        });
        Self {
            stop: Some(stop),
            task: Some(task),
        }
    }

    async fn finish(mut self) -> TestResult<Vec<Operation>> {
        self.stop
            .take()
            .ok_or("missing runner stop signal")?
            .send(())
            .map_err(|()| "runner ended before its stop signal")?;
        Ok(self
            .task
            .take()
            .ok_or("missing runner task")?
            .await?
            .map_err(std::io::Error::other)?)
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

struct Table {
    service: Service,
    client: reqwest::Client,
    cookie: String,
    agent: String,
    person: String,
    sessions: Vec<String>,
    other: String,
    runner: Runner,
}

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

struct Prepared {
    agent: String,
    person: String,
    sessions: Vec<String>,
    other: String,
    key: Arc<Ed25519Identity>,
}

fn prepare(config: &lys_identity_server::Config, act: Act, machine: &str) -> TestResult<Prepared> {
    let seed = seed_configured(config, [ADMINISTRATOR, "other-subject"])?;
    let person = seed.people[0].id.to_string();
    let agent = seed.people[0].agents[0].id.to_string();
    let covered_other = seed.people[0].agents[1].id.to_string();
    let sessions = vec![operation()?, operation()?];
    let other = operation()?;
    let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
    let mut runtime = RuntimeStore::open(
        config
            .runtime_dir
            .as_deref()
            .ok_or("no runtime directory")?,
        Arc::clone(&key),
    )?;
    for (session, owner) in sessions
        .iter()
        .map(|session| (session, &agent))
        .chain(std::iter::once((&other, &covered_other)))
    {
        runtime.report(Report {
            operation: operation()?,
            session: session.clone(),
            agent: Some(owner.clone()),
            machine: machine.to_owned(),
            state: Reported::Starting,
            what: "fixture session".to_owned(),
            confirmation: String::new(),
            reported_by: person.clone(),
            at: 1,
            launch: None,
        })?;
        runtime.report(Report {
            operation: operation()?,
            session: session.clone(),
            agent: Some(owner.clone()),
            machine: machine.to_owned(),
            state: Reported::Running,
            what: "fixture running".to_owned(),
            confirmation: String::new(),
            reported_by: person.clone(),
            at: 2,
            launch: None,
        })?;
    }
    let mut budgets = BudgetStore::open(
        config
            .budgets_dir
            .as_deref()
            .ok_or("no budgets directory")?,
        Arc::clone(&key),
    )?;
    budgets.set_limits(
        Limits {
            holder: Holder {
                kind: HolderKind::Person,
                id: person.clone(),
            },
            limits: vec![Limit {
                unit: Measure::ContextPercent,
                amount: 80.into(),
                period: None,
                act,
                zone: None,
            }],
            warn_at: None,
            version: 0,
            by: person.clone(),
            at: 1,
        },
        0,
    )?;
    Ok(Prepared {
        agent,
        person,
        sessions,
        other,
        key,
    })
}

impl Table {
    async fn fresh(act: Act) -> TestResult<Self> {
        let machine = operation()?;
        let (
            service,
            Prepared {
                agent,
                person,
                sessions,
                other,
                key,
            },
        ) = Service::start_with(|config| prepare(config, act, &machine)).await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        let path = service.dir.path().join("context-runner.sock");
        let runner = Runner::start(UnixListener::bind(&path)?, key.public_key_bytes());
        let table = Self {
            service,
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            cookie,
            agent,
            person,
            sessions,
            other,
            runner,
        };
        table
            .post(
                "/network/machines",
                &json!({"operation": machine,
            "name": "Fixture", "kind": "laptop", "runtime": "sh", "slots": 3,
            "may_run": [], "may_reach": []}),
            )
            .await?;
        table
            .post(
                &format!("/network/machines/{machine}/runner"),
                &json!({"runner": {"kind": "socket", "path": path}}),
            )
            .await?;
        Ok(table)
    }

    async fn post(&self, path: &str, body: &Value) -> TestResult<Value> {
        let response = self
            .client
            .post(format!("{}{path}", self.service.base))
            .header(reqwest::header::COOKIE, &self.cookie)
            .json(body)
            .send()
            .await?;
        let status = response.status().as_u16();
        let answer: Value = response.json().await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    async fn report(&self, body: Value) -> TestResult<Value> {
        self.post(&format!("/agents/{}/usage", self.agent), &body)
            .await
    }

    async fn budget(&self) -> TestResult<Value> {
        let response = self
            .client
            .get(format!(
                "{}/budgets/person/{}",
                self.service.base, self.person
            ))
            .header(reqwest::header::COOKIE, &self.cookie)
            .send()
            .await?;
        let status = response.status().as_u16();
        let answer: Value = response.json().await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }
}

fn report(event: &str, session: Option<&str>, context: Option<u64>, at_ms: i64) -> Value {
    json!({"event": event, "at_ms": at_ms, "session": session, "context_percent": context})
}

fn stopped(operations: &[Operation], sessions: &[&str]) {
    assert_eq!(operations.len(), sessions.len(), "{operations:?}");
    for session in sessions {
        let found: Vec<_> = operations
            .iter()
            .filter(|operation| operation.session == *session)
            .collect();
        assert_eq!(found.len(), 1, "{operations:?}");
        assert_eq!(found[0].request, OperationRequest::Stop);
    }
}

fn unavailable_crossings(answer: &Value, expected: usize, reason: &str) -> TestResult {
    let receipts = answer["receipts"].as_array().ok_or("no receipts")?;
    assert_eq!(receipts.len(), expected, "{answer}");
    for receipt in receipts {
        assert!(receipt["crossing"]["figure"].is_null(), "{receipt}");
        assert_eq!(receipt["crossing"]["unavailable"], reason, "{receipt}");
        assert_eq!(receipt["crossing"]["act"], "stop", "{receipt}");
    }
    Ok(())
}

#[tokio::test]
async fn absent_context_stops_the_named_live_session() -> TestResult {
    let table = Table::fresh(Act::Stop).await?;
    let answer = table
        .report(report("missing-context", Some(&table.sessions[0]), None, 1))
        .await?;
    let operations = table.runner.finish().await?;
    stopped(&operations, &[&table.sessions[0]]);
    unavailable_crossings(
        &answer,
        1,
        "context_percent was not reported, so the context Stop cannot be measured",
    )
}

#[tokio::test]
async fn absent_session_stops_all_its_agents_live_sessions_but_no_other_covered_agent() -> TestResult
{
    let table = Table::fresh(Act::Stop).await?;
    let answer = table
        .report(report("missing-session", None, None, 1))
        .await?;
    let operations = table.runner.finish().await?;
    stopped(&operations, &[&table.sessions[0], &table.sessions[1]]);
    assert!(
        operations
            .iter()
            .all(|operation| operation.session != table.other)
    );
    unavailable_crossings(
        &answer,
        2,
        "the usage report names no session, so its context cannot be measured",
    )
}

#[tokio::test]
async fn a_known_context_above_the_limit_still_asks_stop() -> TestResult {
    let table = Table::fresh(Act::Stop).await?;
    let answer = table
        .report(report(
            "known-context",
            Some(&table.sessions[0]),
            Some(90),
            1,
        ))
        .await?;
    let operations = table.runner.finish().await?;
    stopped(&operations, &[&table.sessions[0]]);
    let receipts = answer["receipts"].as_array().ok_or("no receipts")?;
    assert_eq!(receipts.len(), 1, "{answer}");
    assert_eq!(receipts[0]["crossing"]["figure"], 90);
    assert_eq!(receipts[0]["crossing"]["limit"], 80);
    assert_eq!(receipts[0]["crossing"]["act"], "stop");
    Ok(())
}

#[tokio::test]
async fn tell_names_a_missing_figure_after_a_known_one_without_asking_an_act() -> TestResult {
    let table = Table::fresh(Act::Tell).await?;
    table
        .report(report(
            "known-context",
            Some(&table.sessions[0]),
            Some(60),
            1,
        ))
        .await?;
    let before = table.budget().await?;
    let answer = table
        .report(report("missing-context", Some(&table.sessions[0]), None, 2))
        .await?;
    let after = table.budget().await?;
    let operations = table.runner.finish().await?;
    assert!(operations.is_empty(), "{operations:?}");
    assert_eq!(answer["receipts"], json!([]), "{answer}");
    assert_eq!(before["used"][0]["figure"], 60, "{before}");
    assert!(after["used"][0]["figure"].is_null(), "{after}");
    let reason = after["used"][0]["unavailable"]
        .as_str()
        .ok_or("no unavailable reason")?;
    assert!(
        reason.contains("context_percent") && reason.contains("not reported"),
        "{after}"
    );
    Ok(())
}
