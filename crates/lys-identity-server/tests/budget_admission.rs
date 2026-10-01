//! Refused starts leave a counted runner untouched; the counter first receives a signed control.

#[path = "support/harness_description.rs"]
mod harness_description;

use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::budgets_state::{
    Act, Budget, Holder, HolderKind, Length, Measure, Period, Usage,
};
use lys_identity_server::budgets_store::BudgetStore;
use lys_identity_server::routes::open_directory;
use lys_runner::protocol::{Greeting, Reply, verify_request};
use lys_runner::{Act as RunnerAct, Answer};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::oneshot;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct CountedRunner {
    received: Arc<AtomicUsize>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<tokio::task::JoinHandle<Result<(), String>>>,
}

impl CountedRunner {
    fn start(listener: UnixListener, key: [u8; 32]) -> Self {
        let received = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&received);
        let (stop, mut stopped) = oneshot::channel();
        let task = tokio::spawn(async move {
            loop {
                let socket = tokio::select! {
                    _ = &mut stopped => return Ok(()),
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
                verify_request(&line, &key, &greeting).map_err(|error| error.to_string())?;
                counter.fetch_add(1, Ordering::SeqCst);
                let reply = Reply {
                    version: lys_runner::protocol::PROTOCOL_VERSION,
                    answer: Answer::Refused {
                        refusal: "no_live_session".to_owned(),
                        words: "the counted runner starts no process".to_owned(),
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
            received,
            stop: Some(stop),
            task: Some(task),
        }
    }

    async fn finish(mut self) -> TestResult<usize> {
        self.stop
            .take()
            .ok_or("missing runner stop signal")?
            .send(())
            .map_err(|()| "runner ended before its stop signal")?;
        self.task
            .take()
            .ok_or("missing runner task")?
            .await?
            .map_err(std::io::Error::other)?;
        Ok(self.received.load(Ordering::SeqCst))
    }
}

impl Drop for CountedRunner {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

async fn post(service: &Service, cookie: &str, path: &str, body: &Value) -> TestResult<Value> {
    let (status, answer) = service.post(path, Some(cookie), body).await?;
    assert_eq!(status, 200, "{path}: {answer}");
    Ok(answer)
}

fn prepare_cap(config: &lys_identity_server::Config, agent: &str, person: &str) -> TestResult {
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
                id: agent.to_owned(),
            },
            measure: Measure::Tokens,
            limit: 0,
            period: Some(Period {
                length: Length::Day,
                zone: "UTC".to_owned(),
            }),
            act: Act::Stop,
            version: 0,
            by: person.to_owned(),
            at: 1,
        },
        0,
    )?;
    store.charge(Usage {
        event: "already-exhausted".to_owned(),
        agent: agent.to_owned(),
        at_ms: jiff::Timestamp::now().as_millisecond(),
        tokens: 500,
        ..Usage::default()
    })?;
    Ok(())
}

fn prepare_agent(config: &lys_identity_server::Config) -> TestResult<(String, String)> {
    let mut directory = open_directory(config)?;
    let at = jiff::Timestamp::now().as_second().try_into()?;
    let actor = Actor::new(
        config.administrator_binding()?,
        Provenance::new(AuthMethod::Oidc, at),
    );
    let (person, _) = directory.register_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Operator")?,
        at,
    )?;
    directory.bind_login(
        actor.clone(),
        OperationId::generate()?,
        person,
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        at,
    )?;
    directory.transition(
        actor.clone(),
        OperationId::generate()?,
        IdentityId::Person(person),
        Transition::Activate,
        "",
        at,
    )?;
    let (agent, _) = directory.register_agent(
        actor.clone(),
        OperationId::generate()?,
        person,
        Profile::new("Runner")?,
        at,
    )?;
    directory.transition(
        actor,
        OperationId::generate()?,
        IdentityId::Agent(agent),
        Transition::Activate,
        "",
        at,
    )?;
    Ok((agent.to_string(), person.to_string()))
}

async fn control(path: &Path, key: Arc<Ed25519Identity>) -> TestResult {
    let client = lys_runner::client::Client::new(path.to_owned(), key);
    let answer = tokio::task::spawn_blocking(move || {
        client.ask(&RunnerAct::Status {
            session: Some("control".to_owned()),
        })
    })
    .await?;
    assert!(answer.is_err(), "the control runner has no sessions");
    Ok(())
}

#[tokio::test]
async fn an_exhausted_cap_refuses_both_routes_without_a_runner_request() -> TestResult {
    let (service, (agent, key)) = Service::start_with(|config| {
        let (agent, person) = prepare_agent(config)?;
        prepare_cap(config, &agent, &person)?;
        Ok((
            agent,
            Arc::new(Ed25519Identity::load(&config.event_key_file)?),
        ))
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let path = service.dir.path().join("counted-runner.sock");
    let runner = CountedRunner::start(UnixListener::bind(&path)?, key.public_key_bytes());
    control(&path, key).await?;
    assert_eq!(
        runner.received.load(Ordering::SeqCst),
        1,
        "the signed control must be seen"
    );
    runner.received.store(0, Ordering::SeqCst);
    let machine = operation()?;
    post(&service, &cookie, "/network/machines", &json!({"operation": machine, "name": "Runner", "kind": "laptop", "runtime": "sh", "slots": 1, "may_run": [agent], "may_reach": []})).await?;
    post(
        &service,
        &cookie,
        &format!("/network/machines/{machine}/runner"),
        &json!({"runner": {"kind": "socket", "path": path}}),
    )
    .await?;
    let profile = format!("/agents/{agent}/provisioning");
    post(&service, &cookie, &profile, &json!({"operation": operation()?, "from_version": 0, "model_access": ["claude-fable-5-1"], "tools": [], "skills": [], "mcp_servers": [], "harness": harness_description::declared(), "instructions": "", "note": ""})).await?;
    post(
        &service,
        &cookie,
        &format!("{profile}/1/review"),
        &json!({"operation": operation()?}),
    )
    .await?;
    let mut answers = Vec::new();
    for route in ["start-command", "start"] {
        let body = if route == "start-command" {
            json!({"machine": machine, "operation": operation()?})
        } else {
            json!({"machine": machine, "profile_version": "1"})
        };
        answers.push((
            route,
            service
                .post(&format!("/agents/{agent}/{route}"), Some(&cookie), &body)
                .await?,
        ));
    }
    let received = runner.finish().await?;
    assert_eq!(
        received, 0,
        "an exhausted start must send nothing to the counted runner"
    );
    for (route, (status, answer)) in answers {
        assert_eq!(status, 409, "{route}: {answer}");
        assert_eq!(answer["refusal"], "BudgetExhausted", "{route}: {answer}");
    }
    Ok(())
}
