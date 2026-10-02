#![cfg(test)]
//! Budget prompts and due goal reminders are delivered before any viewer opens.

#[path = "support/harness_description.rs"]
mod harness_description;

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use axum::Router;
use identity_contract::apps::{Auth, send};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Act, Answer, Client, Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Held {
    service: Service,
    seeded: Seeded,
    ada: String,
    dir: tempfile::TempDir,
    key: Arc<Ed25519Identity>,
    serving: Option<Serving>,
    machine: String,
    broker: tokio::task::JoinHandle<std::io::Result<()>>,
}

impl Held {
    async fn open() -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let broker = Router::new().fallback(|| async { axum::Json(json!({"handles": []})) });
        let broker = tokio::spawn(async move { axum::serve(listener, broker).await });
        let dir = tempfile::tempdir()?;
        let program = dir.path().join("seat");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf 'ready %s\\n' \"$LYS_LAUNCH_TEMPLATE\"\nwhile IFS= read -r line; do if [ \"$line\" = compact ]; then printf 'compacted\\n'; else printf '%s\\n' \"$line\"; fi; done\n",
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
        let key_file = dir.path().join("broker.key");
        Ed25519Identity::load_or_generate(&key_file)?;
        let settings = SecretsSettings {
            broker: format!("http://{address}"),
            service: "identity".to_owned(),
            service_key_file: key_file,
        };
        let socket = dir.path().join("runner.sock");
        let adjusted = socket.clone();
        let state = dir.path().join("state");
        let (service, (seeded, serving, key)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            Some(settings),
            None,
            move |config| {
                config.runner_socket = Some(adjusted);
            },
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let runner = Runner::open(&Options {
                    socket,
                    state,
                    server_key: key.public_key_bytes(),
                    scrollback: 4096,
                })?;
                Ok((seeded, runner.spawn(), key))
            },
        )
        .await?;
        let ada = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "person@example.test".to_owned(),
            })
            .await?;
        let held = Self {
            service,
            seeded,
            ada,
            dir,
            key,
            serving: Some(serving),
            machine: operation()?,
            broker,
        };
        held.ok("/network/machines", &json!({ "operation": held.machine, "name": "Seat", "kind": "laptop", "runtime": program.display().to_string(), "slots": 2, "may_run": [held.agent()], "may_reach": [] })).await?;
        held.ok(
            &format!("/network/machines/{}/runner", held.machine),
            &json!({ "runner": { "kind": "lys" } }),
        )
        .await?;
        held.profile().await?;
        Ok(held)
    }

    fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    fn client(&self) -> Client {
        Client::new(self.dir.path().join("runner.sock"), Arc::clone(&self.key))
    }

    async fn ok(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        assert_eq!(status, 200, "{path}: {answer}");
        Ok(answer)
    }

    async fn profile(&self) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let mut harness = harness_description::declared();
        harness["program"] = json!(self.dir.path().join("seat").display().to_string());
        self.ok(&path, &json!({ "operation": operation()?, "from_version": 0,
            "model_access": ["model-1"], "tools": [], "skills": [], "mcp_servers": [], "instructions": "", "note": "", "permissions": {"default_mode": "plan"}, "harness": harness, "session": { "compact": "compact" } })).await?;
        self.ok(
            &format!("/agents/{}/provisioning/1/review", self.agent()),
            &json!({ "operation": operation()? }),
        )
        .await?;
        Ok(())
    }

    async fn start(&self) -> Result<Value, Box<dyn Error>> {
        self.ok(
            &format!("/agents/{}/start-command", self.agent()),
            &json!({ "operation": operation()?, "machine": self.machine }),
        )
        .await
    }

    fn status(&self, session: &str) -> Result<lys_runner::protocol::SessionView, Box<dyn Error>> {
        let Answer::Status { status } = self.client().ask(&Act::Status {
            session: Some(session.to_owned()),
        })?
        else {
            return Err("runner answered no status".into());
        };
        status
            .sessions
            .into_iter()
            .next()
            .ok_or_else(|| "runner holds no session".into())
    }

    async fn output(&self, session: &str, pattern: &str) -> TestResult {
        let client = self.client();
        let act = Act::Wait {
            session: session.to_owned(),
            cursor: Some(0),
            pattern: pattern.to_owned(),
            regex: false,
        };
        let answer = tokio::task::spawn_blocking(move || client.ask(&act)).await??;
        assert!(matches!(answer, Answer::Matched { .. }), "{answer:?}");
        Ok(())
    }

    fn close(mut self) -> TestResult {
        self.broker.abort();
        self.serving
            .take()
            .ok_or("runner already stopped")?
            .stop()?;
        Ok(())
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        self.broker.abort();
        if let Some(serving) = self.serving.take() {
            if let Err(error) = serving.stop() {
                eprintln!("runner cleanup failed: {error}");
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn budget_compaction_and_notice_reach_a_session_before_any_viewer_opens() -> TestResult {
    let held = Held::open().await?;
    let started = held.start().await?;
    let session = started["session"].as_str().ok_or("no session")?;
    let second = held.start().await?;
    let other = second["session"].as_str().ok_or("no second session")?;
    let path = format!("/budgets/agent/{}", held.agent());
    let budget = json!({"limits": [{"unit": "context_percent", "amount": 50, "period": null, "act": "compact"}], "warn_at": null, "version": 0});
    let (status, set) = send(
        &held.service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&held.ada),
        Some(&budget),
    )
    .await?;
    assert_eq!(status, 200, "{set}");
    let compacted = held.ok(&format!("/agents/{}/usage", held.agent()), &json!({ "event": "compact", "at_ms": jiff::Timestamp::now().as_millisecond(), "session": session, "context_percent": 60 })).await?;
    assert_eq!(
        compacted["receipts"][0]["acted"]["stands"], "delivered",
        "{compacted}"
    );
    assert!(held.status(other)?.ended.is_none());
    let budget = json!({"limits": [{"unit": "context_percent", "amount": 80, "period": null, "act": "notice"}], "warn_at": null, "version": 1});
    let (status, set) = send(
        &held.service,
        reqwest::Method::PUT,
        &path,
        Auth::Cookie(&held.ada),
        Some(&budget),
    )
    .await?;
    assert_eq!(status, 200, "{set}");
    let noticed = held.ok(&format!("/agents/{}/usage", held.agent()), &json!({ "event": "notice", "at_ms": jiff::Timestamp::now().as_millisecond(), "session": session, "context_percent": 90 })).await?;
    assert_eq!(
        noticed["receipts"][1]["acted"]["stands"], "delivered",
        "{noticed}"
    );
    assert!(held.status(other)?.ended.is_none());
    // Delivery receipts precede the first output consumer.
    held.output(session, "compacted").await?;
    held.output(session, "context is at 90%").await?;
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_due_goal_reminder_reaches_each_session_before_any_viewer_opens() -> TestResult {
    let held = Held::open().await?;
    let first = held.start().await?;
    let second = held.start().await?;
    let deadline = u64::try_from(jiff::Timestamp::now().as_second())? + 3600;
    let path = format!("/agents/{}/goals", held.agent());
    let body = json!({ "operation": operation()?, "kind": "goal", "words": "remember-the-review", "deadline": deadline, "reminders": [{ "when": "before", "seconds": 7200 }] });
    let goal = held.ok(&path, &body).await?;
    let sent = goal["fired"][0]["sent"]
        .as_array()
        .ok_or("no reminder deliveries")?;
    assert_eq!(sent.len(), 2, "{goal}");
    for delivery in sent {
        assert_eq!(delivery["state"], "delivered", "{delivery}");
    }
    let again = held.ok(&path, &body).await?;
    assert_eq!(
        again["fired"].as_array().ok_or("no fired reminders")?.len(),
        1
    );
    // Both deliveries and the replay settle before any output consumer.
    for started in [&first, &second] {
        let session = started["session"].as_str().ok_or("no session")?;
        held.output(session, "remember-the-review").await?;
    }
    held.close()
}
