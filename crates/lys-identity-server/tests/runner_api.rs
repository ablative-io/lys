#![cfg(test)]
//! DIRECTORY-050 R4, R5 and R6 through the service's routes, against Lys's
//! own runner started here: a session is typed to, read, waited on, sent
//! keys, resized and compacted, each under the operate relation and each
//! leaving a receipt that carries no typed text; a session printing its
//! usage-limit words moves to its next account and stops
//! `accounts_exhausted` at the list's end, and no account value reaches an
//! answer, a receipt or a record; a wake types into the live session; the
//! emergency stop ends every session through the runner and shows each
//! confirmed with its exit instant. Every wait ends on an answer, never a
//! clock.

#[path = "support/harness_description.rs"]
mod harness_description;
#[path = "support/stub_program.rs"]
mod stub_program;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use axum::Router;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// A file found under a directory, and its bytes.
type Found = (PathBuf, Vec<u8>);

/// Every line the runners and the services of this binary said.
type Heard = Arc<Mutex<Vec<String>>>;

/// Where every log line is kept: the runner's, through its sink, and each
/// service's, through the `say` it is started with.
fn heard() -> &'static Heard {
    static HEARD: OnceLock<Heard> = OnceLock::new();
    HEARD.get_or_init(|| {
        let lines = Heard::default();
        let kept = Arc::clone(&lines);
        lys_runner::error::also_to(Box::new(move |line| {
            kept.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(line.to_owned());
        }));
        lines
    })
}

fn said_lines() -> Vec<String> {
    heard()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

const BEA: &str = "bea-subject";
/// A value the stand-in broker plants beside every handle; it must never
/// reach an answer, a receipt or a record.
const PLANTED: &str = "sk-planted-account-value-7c1e";

async fn broker(request: Request) -> Response {
    if request.uri().path() != "/_lys/handles" {
        return axum::Json(json!({})).into_response();
    }
    axum::Json(json!({ "holder": "any", "handles": [
        { "id": "h-live", "secret": "model account", "max_uses": 10, "used": 1,
          "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0,
          "parent": null, "value": PLANTED, "token": PLANTED },
    ]}))
    .into_response()
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
    dir: tempfile::TempDir,
    serving: Option<Serving>,
    machine: String,
    answers: Vec<Value>,
}

impl Table {
    async fn set(session: &Value) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        tokio::spawn(async move { axum::serve(listener, Router::new().fallback(broker)).await });
        let dir = tempfile::tempdir()?;
        let key_file = dir.path().join("secrets-service.key");
        Ed25519Identity::load_or_generate(&key_file)?;
        let settings = SecretsSettings {
            broker: format!("http://{address}"),
            service: "identity".to_owned(),
            service_key_file: key_file,
        };
        let socket = dir.path().join("runner.sock");
        let state = dir.path().join("runner-state");
        let adjusted = socket.clone();
        let lines = Arc::clone(heard());
        let (service, (seeded, serving)) = Box::pin(Service::start_saying(
            GRANT_MODEL,
            None,
            Some(settings),
            None,
            move |config| config.runner_socket = Some(adjusted),
            Some(Arc::new(move |line: &str| {
                lines
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(format!("service {line}"));
            })),
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, BEA])?;
                let key = Ed25519Identity::load(&config.event_key_file)?;
                let runner = Runner::open(&Options {
                    socket,
                    state,
                    server_key: key.public_key_bytes(),
                    scrollback: 1 << 16,
                })?;
                Ok((seeded, runner.spawn()))
            },
        ))
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let mut table = Self {
            service,
            seeded,
            ada,
            bea,
            dir,
            serving: Some(serving),
            machine: String::new(),
            answers: Vec::new(),
        };
        table.machine = table.named_machine().await?;
        table.profile(session).await?;
        Ok(table)
    }

    fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    /// POST `body` to `path` as `cookie`, keeping the answer.
    async fn sent(
        &mut self,
        path: &str,
        cookie: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(cookie), body).await?;
        self.answers.push(answer.clone());
        Ok((status, answer))
    }

    async fn ok(&mut self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let ada = self.ada.clone();
        let (status, answer) = self.sent(path, &ada, body).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }

    async fn named_machine(&mut self) -> Result<String, Box<dyn Error>> {
        let id = operation()?;
        let body = json!({
            "operation": id, "name": "Runner box", "kind": "laptop", "runtime": "sh",
            "slots": 2, "may_run": [self.agent()], "may_reach": ["cambium.example.test"],
        });
        self.ok("/network/machines", &body).await?;
        let runner = json!({ "runner": { "kind": "lys" } });
        self.ok(&format!("/network/machines/{id}/runner"), &runner)
            .await?;
        Ok(id)
    }

    async fn profile(&mut self, session: &Value) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let program = stub_program::write(self.dir.path(), stub_program::SHELL)?;
        let mut harness = harness_description::declared();
        harness["program"] = json!(program);
        let body = json!({
            "operation": operation()?, "from_version": 0,
            "model_access": ["claude-fable-5-1"], "tools": ["read"], "skills": [],
            "mcp_servers": [{ "name": "cambium", "url": "https://cambium.example.test/mcp" }],
            "harness": harness,
            "instructions": "", "note": "", "session": session,
        });
        self.ok(&path, &body).await?;
        self.ok(
            &format!("{path}/1/review"),
            &json!({ "operation": operation()? }),
        )
        .await?;
        Ok(())
    }

    /// Start the agent on the runner's machine, answering its session.
    async fn start(&mut self) -> Result<String, Box<dyn Error>> {
        let path = format!("/agents/{}/start-command", self.agent());
        let body = json!({ "machine": self.machine, "operation": operation()? });
        let started = self.ok(&path, &body).await?;
        assert_eq!(started["runner"]["state"], "running", "{started}");
        Ok(started["session"].as_str().ok_or("no session")?.to_owned())
    }

    async fn act(
        &mut self,
        session: &str,
        act: &str,
        body: &Value,
    ) -> Result<Value, Box<dyn Error>> {
        self.ok(&format!("/runtime/sessions/{session}/{act}"), body)
            .await
    }

    async fn waited(&mut self, session: &str, pattern: &str) -> Result<Value, Box<dyn Error>> {
        let answer = self
            .act(session, "wait", &json!({ "pattern": pattern, "cursor": 0 }))
            .await?;
        assert_eq!(answer["answer"]["kind"], "matched", "{answer}");
        assert_eq!(answer["answer"]["matched"], pattern);
        Ok(answer)
    }

    /// Follow the session's output until it ends, answering its end.
    async fn until_ended(&mut self, session: &str) -> Result<Value, Box<dyn Error>> {
        let mut cursor = Value::Null;
        loop {
            let body = json!({ "cursor": cursor, "follow": true });
            let read = self.act(session, "read", &body).await?;
            let output = &read["answer"]["output"];
            if !output["ended"].is_null() {
                return Ok(output["ended"].clone());
            }
            cursor = output["cursor"].clone();
        }
    }

    fn close(mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn typing_a_line_and_reading_it_back_shows_the_line_and_its_output() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let session = table.start().await?;
    let typed = table
        .act(
            &session,
            "input",
            &json!({ "text": "echo typed-$((6*7))", "enter": true }),
        )
        .await?;
    assert_eq!(typed["answer"]["kind"], "delivered", "{typed}");
    let waited = table.waited(&session, "typed-42").await?;
    assert_eq!(waited["receipt"]["act"]["act"], "wait");
    let read = table.act(&session, "read", &json!({ "cursor": 0 })).await?;
    let text = read["answer"]["output"]["text"].as_str().ok_or("no text")?;
    assert!(text.contains("echo typed-$((6*7))"), "the line: {text}");
    assert!(text.contains("typed-42"), "its output: {text}");
    table.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn each_session_is_hidden_without_operate_and_each_admitted_act_leaves_a_receipt()
-> TestResult {
    let session_settings = json!({ "compact": "echo compacted-$((2+3))" });
    let mut table = Table::set(&session_settings).await?;
    let session = table.start().await?;
    let acts = [
        (
            "input",
            json!({ "text": "echo receipt-$((1+2))", "enter": true }),
        ),
        ("keys", json!({ "keys": ["ctrl_l"] })),
        ("read", json!({ "lines": 5 })),
        ("wait", json!({ "pattern": "receipt-3", "cursor": 0 })),
        ("resize", json!({ "columns": 100, "rows": 30 })),
        ("compact", json!({})),
    ];
    let bea = table.bea.clone();
    let mut refused = 0;
    for (act, body) in &acts {
        let (status, answer) = table
            .sent(&format!("/runtime/sessions/{session}/{act}"), &bea, body)
            .await?;
        assert_eq!(
            (status, answer["refusal"].as_str()),
            (404, Some("RuntimeSessionUnknown")),
            "{act}: {answer}"
        );
        refused += 1;
    }
    let (status, answer) = table
        .sent(
            &format!("/agents/{}/wake", table.agent()),
            &bea,
            &json!({ "message": "hello" }),
        )
        .await?;
    assert_eq!(
        (status, answer["refusal"].as_str()),
        (403, Some("not_permitted")),
        "{answer}"
    );
    assert_eq!(refused + 1, acts.len() + 1);

    let mut indexes = Vec::new();
    for (act, body) in &acts {
        let answer = table.act(&session, act, body).await?;
        let receipt = &answer["receipt"];
        assert_eq!(receipt["act"]["act"], *act, "{answer}");
        assert_eq!(
            receipt["act"]["caller"],
            table.seeded.people[0].id.to_string()
        );
        indexes.push(receipt["index"].as_u64().ok_or("no index")?);
    }
    table.waited(&session, "compacted-5").await?;
    for (index, (act, _)) in indexes.iter().zip(&acts) {
        let (status, kept) = table
            .service
            .get(&format!("/runner-receipts/{index}"), None)
            .await?;
        assert_eq!(status, 200, "{kept}");
        assert_eq!(kept["act"]["act"], *act);
        let text = kept.to_string();
        assert!(
            !text.contains("receipt-$((1+2))") && !text.contains("compacted-$"),
            "{text}"
        );
    }
    let typed = table
        .service
        .get(&format!("/runner-receipts/{}", indexes[0]), None)
        .await?
        .1;
    assert_eq!(
        typed["act"]["text"]["length"],
        "echo receipt-$((1+2))".len()
    );
    assert_eq!(
        typed["act"]["text"]["sha256"].as_str().map(str::len),
        Some(64)
    );
    table.close()
}

/// Every file under `dir`, read whole.
fn files(dir: &Path) -> Result<Vec<Found>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(&next)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.is_file() {
                found.push((path.clone(), std::fs::read(&path)?));
            }
        }
    }
    Ok(found)
}

#[tokio::test(flavor = "multi_thread")]
async fn a_usage_limit_moves_the_session_to_the_next_account_and_the_list_end_stops_it()
-> TestResult {
    let rotation = json!({ "accounts": {
        "accounts": ["h-account-one", "h-account-two"],
        "variable": "LYS_ACCOUNT_HANDLE",
        "limit": { "signal": "words", "words": ["usage limit reached"] },
        "resume_arguments": ["-c", "echo on-$LYS_ACCOUNT_HANDLE; exec cat"],
    }});
    let mut table = Table::set(&rotation).await?;
    let session = table.start().await?;
    table
        .act(
            &session,
            "input",
            &json!({ "text": "echo on-$LYS_ACCOUNT_HANDLE", "enter": true }),
        )
        .await?;
    table.waited(&session, "on-h-account-one").await?;
    table
        .act(
            &session,
            "input",
            &json!({ "text": "echo usage limit reached", "enter": true }),
        )
        .await?;
    // The session resumes on its next account and says so by itself.
    table.waited(&session, "on-h-account-two").await?;
    let typed = json!({ "text": "usage limit reached", "enter": true });
    let (status, answer) = table
        .service
        .post(
            &format!("/runtime/sessions/{session}/input"),
            Some(&table.ada),
            &typed,
        )
        .await?;
    table.answers.push(answer.clone());
    assert_eq!(status, 200, "{answer}");
    let ended = table.until_ended(&session).await?;
    assert_eq!(ended["how"], "accounts_exhausted", "{ended}");

    let path = format!("/agents/{}/runtime/sessions", table.agent());
    let (_, sessions) = table.service.get(&path, Some(&table.ada)).await?;
    let shown = sessions["sessions"]
        .as_array()
        .and_then(|all| all.iter().find(|s| s["session"] == session.as_str()))
        .ok_or("the session is not listed")?;
    assert_eq!(shown["shown"], "stopped");
    let confirmation = shown["stopped"]["confirmation"]
        .as_str()
        .ok_or("no confirmation")?;
    assert!(
        confirmation.starts_with("accounts_exhausted"),
        "{confirmation}"
    );

    let mut checked = 0;
    for answer in &table.answers {
        assert!(
            !answer.to_string().contains(PLANTED),
            "an answer carries the value: {answer}"
        );
        checked += 1;
    }
    for (path, bytes) in files(table.service.dir.path())?
        .into_iter()
        .chain(files(table.dir.path())?)
    {
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains(PLANTED),
            "{} carries the value",
            path.display()
        );
        checked += 1;
    }
    assert!(
        checked > table.answers.len(),
        "records were read as well as answers"
    );
    let lines = said_lines();
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("lys-runner ") && line.contains("moves to account")),
        "the runner's log of the move was read: {lines:?}"
    );
    assert!(
        lines.iter().any(|line| line.starts_with("service ")),
        "the service's log was read: {lines:?}"
    );
    for line in &lines {
        assert!(
            !line.contains(PLANTED),
            "a log line carries the value: {line}"
        );
    }
    table.close()
}

#[path = "shared/runner_session_security.rs"]
mod runner_session_security;

#[path = "shared/runner_controls.rs"]
mod runner_controls;
