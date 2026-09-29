#![cfg(test)]
//! DIRECTORY-051 R3 through the service's routes, against Lys's own runner
//! started here: a use that reaches a budget does the budget's act once per
//! crossing, under an operation id kept before it is asked. A context
//! crossing types one compaction; a configured context notice appears in
//! the live session; a hard token cap ends the session, shown confirmed
//! only on the exit the runner saw, and the stopped session does nothing
//! more; an act whose answer cannot be had stays unconfirmed and is asked
//! again under its own id. Every wait ends on an answer, never a clock.

use std::error::Error;
use std::path::PathBuf;

use axum::Router;
use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
    socket: PathBuf,
    state: PathBuf,
    key: [u8; 32],
    serving: Option<Serving>,
    machine: String,
    _dir: tempfile::TempDir,
}

impl Table {
    async fn set(session: &Value) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let broker = Router::new()
            .fallback(|| async { axum::Json(json!({ "holder": "any", "handles": [] })) });
        tokio::spawn(async move { axum::serve(listener, broker).await });
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
        let (adjusted, opened) = (socket.clone(), (socket.clone(), state.clone()));
        let (service, (seeded, serving, key)) = Service::start_saying(
            GRANT_MODEL,
            None,
            Some(settings),
            None,
            move |config| config.runner_socket = Some(adjusted),
            None,
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
                let key = Ed25519Identity::load(&config.event_key_file)?.public_key_bytes();
                let runner = Runner::open(&Options {
                    socket: opened.0,
                    state: opened.1,
                    server_key: key,
                    scrollback: 1 << 16,
                })?;
                Ok((seeded, runner.spawn(), key))
            },
        )
        .await?;
        let login = Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "shared@example.test".to_owned(),
        };
        let ada = service.sign_in(login).await?;
        let mut table = Self {
            service,
            seeded,
            ada,
            socket,
            state,
            key,
            serving: Some(serving),
            machine: String::new(),
            _dir: dir,
        };
        table.machine = table.named_machine().await?;
        table.profile(session).await?;
        Ok(table)
    }

    fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        send(&self.service, method, path, Auth::Cookie(&self.ada), body).await
    }

    async fn ok(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.call(reqwest::Method::POST, path, Some(body)).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }

    async fn named_machine(&self) -> Result<String, Box<dyn Error>> {
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

    async fn profile(&self, session: &Value) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let body = json!({
            "operation": operation()?, "from_version": 0,
            "model_access": ["claude-fable-5-1"], "tools": ["read"], "skills": [],
            "mcp_servers": [{ "name": "cambium", "url": "https://cambium.example.test/mcp" }],
            "harness": { "kind": "claude_code", "program": "/opt/seat/bin/claude", "package": "claude-code-seat" },
            "instructions": "", "note": "", "session": session,
        });
        self.ok(&path, &body).await?;
        let review = json!({ "operation": operation()? });
        self.ok(&format!("{path}/1/review"), &review).await?;
        Ok(())
    }

    async fn start(&self) -> Result<String, Box<dyn Error>> {
        let path = format!("/agents/{}/start-command", self.agent());
        let body = json!({ "machine": self.machine, "operation": operation()? });
        let started = self.ok(&path, &body).await?;
        Ok(started["session"].as_str().ok_or("no session")?.to_owned())
    }

    async fn budget(&self, budget: Value) -> TestResult {
        let path = format!("/budgets/agent/{}", self.agent());
        let (status, answer) = self
            .call(reqwest::Method::PUT, &path, Some(&budget))
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(())
    }

    async fn used(&self, used: Value) -> Result<Value, Box<dyn Error>> {
        self.ok(&format!("/agents/{}/usage", self.agent()), &used)
            .await
    }

    async fn receipts(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        let path = format!("/agents/{}/usage", self.agent());
        let (status, answer) = self.call(reqwest::Method::GET, &path, None).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer["receipts"].as_array().ok_or("no receipts")?.clone())
    }

    async fn waited(&self, session: &str, pattern: &str) -> TestResult {
        let body = json!({ "pattern": pattern, "cursor": 0 });
        let answer = self
            .ok(&format!("/runtime/sessions/{session}/wait"), &body)
            .await?;
        assert_eq!(answer["answer"]["matched"], pattern, "{answer}");
        Ok(())
    }

    /// Everything the session showed, once it has ended.
    async fn until_ended(&self, session: &str) -> Result<String, Box<dyn Error>> {
        let (mut cursor, mut text) = (json!(0), String::new());
        loop {
            let body = json!({ "cursor": cursor, "follow": true });
            let read = self
                .ok(&format!("/runtime/sessions/{session}/read"), &body)
                .await?;
            let output = &read["answer"]["output"];
            text.push_str(output["text"].as_str().unwrap_or_default());
            if !output["ended"].is_null() {
                return Ok(text);
            }
            cursor = output["cursor"].clone();
        }
    }

    async fn end(&self, session: &str) -> Result<String, Box<dyn Error>> {
        self.ok(&format!("/runtime/sessions/{session}/end"), &json!({}))
            .await?;
        self.until_ended(session).await
    }

    fn stop_runner(&mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }

    fn start_runner(&mut self) -> TestResult {
        let runner = Runner::open(&Options {
            socket: self.socket.clone(),
            state: self.state.clone(),
            server_key: self.key,
            scrollback: 1 << 16,
        })?;
        self.serving = Some(runner.spawn());
        Ok(())
    }
}

fn now_ms() -> i64 {
    jiff::Timestamp::now().as_millisecond()
}

fn context(event: &str, session: &str, figure: u64) -> Value {
    json!({ "event": event, "at_ms": now_ms(), "session": session, "context_percent": figure })
}

#[tokio::test(flavor = "multi_thread")]
async fn a_context_crossing_types_one_compaction() -> TestResult {
    let mut table = Table::set(&json!({ "compact": "echo compacted-$((2+3))" })).await?;
    let session = table.start().await?;
    table
        .budget(
            json!({ "measure": "context_percent", "limit": 80, "act": "compact", "version": 0 }),
        )
        .await?;
    let path = format!("/agents/{}/usage", table.agent());
    let (_, unreported) = table.call(reqwest::Method::GET, &path, None).await?;
    assert!(unreported["last_reported_ms"].is_null(), "{unreported}");
    let first = context("e1", &session, 60);
    let under = table.used(first.clone()).await?;
    assert_eq!(under["receipts"], json!([]), "{under}");
    assert_eq!(under["last_reported_ms"], first["at_ms"], "{under}");
    let crossed = table.used(context("e2", &session, 85)).await?;
    let receipt = &crossed["receipts"][0];
    assert_eq!(receipt["crossing"]["figure"], 85, "{crossed}");
    assert_eq!(receipt["crossing"]["limit"], 80);
    assert!(receipt["crossing"]["at_ms"].is_i64());
    table.waited(&session, "compacted-5").await?;
    table.used(context("e2", &session, 85)).await?;
    table.used(context("e3", &session, 90)).await?;
    assert_eq!(
        table.receipts().await?.len(),
        1,
        "one crossing, one receipt"
    );
    let shown = table.end(&session).await?;
    assert_eq!(shown.matches("compacted-5").count(), 1, "{shown:?}");
    table.stop_runner()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_configured_context_notice_appears_in_the_live_session() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let session = table.start().await?;
    table
        .budget(json!({ "measure": "context_percent", "limit": 50, "act": "notice", "version": 0 }))
        .await?;
    table.used(context("e1", &session, 64)).await?;
    table.waited(&session, "context is at 64%").await?;
    table.end(&session).await?;
    table.stop_runner()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_hard_token_cap_ends_the_session_confirmed_on_its_exit() -> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let session = table.start().await?;
    let cap = json!({ "measure": "tokens", "limit": 1000, "act": "stop", "version": 0,
        "period": { "length": "day", "zone": "UTC" } });
    table.budget(cap).await?;
    let tokens =
        |event: &str, tokens: u64| json!({ "event": event, "at_ms": now_ms(), "tokens": tokens });
    table.used(tokens("e1", 400)).await?;
    assert!(table.receipts().await?.is_empty());
    table.used(tokens("e2", 700)).await?;
    table.until_ended(&session).await?;
    let receipts = table.receipts().await?;
    assert_eq!(receipts.len(), 1, "{receipts:?}");
    assert_eq!(receipts[0]["crossing"]["figure"], 1100);
    assert_eq!(receipts[0]["acted"]["stands"], "confirmed", "{receipts:?}");
    assert!(
        !receipts[0]["acted"]["ended"].is_null(),
        "confirmed on the exit seen"
    );
    table.used(tokens("e3", 50)).await?;
    assert_eq!(
        table.receipts().await?.len(),
        1,
        "a reached cap crosses once"
    );
    let typed = json!({ "text": "echo after", "enter": true });
    let (status, refused) = table
        .call(
            reqwest::Method::POST,
            &format!("/runtime/sessions/{session}/input"),
            Some(&typed),
        )
        .await?;
    assert_ne!(status, 200, "{refused}");
    assert_eq!(refused["refusal"], "session_ended", "{refused}");
    table.stop_runner()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_act_whose_answer_cannot_be_had_stays_unconfirmed_and_is_asked_again_under_its_id()
-> TestResult {
    let mut table = Table::set(&json!({})).await?;
    let session = table.start().await?;
    table
        .budget(json!({ "measure": "context_percent", "limit": 70, "act": "stop", "version": 0 }))
        .await?;
    table.stop_runner()?;
    let crossed = table.used(context("e1", &session, 75)).await?;
    let receipt = &crossed["receipts"][0];
    let id = receipt["crossing"]["operation"].clone();
    assert!(receipt["acted"].is_null(), "shown unconfirmed: {crossed}");
    table.start_runner()?;
    let receipts = table.receipts().await?;
    assert_eq!(receipts.len(), 1, "{receipts:?}");
    assert_eq!(
        receipts[0]["crossing"]["operation"], id,
        "the same operation"
    );
    assert!(
        !receipts[0]["acted"].is_null(),
        "answered once reachable: {receipts:?}"
    );
    assert_ne!(receipts[0]["acted"]["stands"], "confirmed", "{receipts:?}");
    table.stop_runner()
}
