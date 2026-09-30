#![cfg(test)]
//! A restart waits on the old session's exit and keeps the reviewed launch it starts.

#[path = "support/harness_description.rs"]
mod harness_description;

use std::collections::BTreeMap;
use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use axum::Router;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Act, Answer, Client, Launch, Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Held {
    service: Service,
    seeded: Seeded,
    ada: String,
    bea: String,
    dir: tempfile::TempDir,
    key: Arc<Ed25519Identity>,
    serving: Serving,
    machine: String,
}

impl Held {
    async fn open() -> Result<Self, Box<dyn Error>> {
        Self::open_with_runtime(true).await
    }

    async fn open_with_runtime(runtime: bool) -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let broker = Router::new().fallback(|| async { axum::Json(json!({"handles": []})) });
        tokio::spawn(async move { axum::serve(listener, broker).await });
        let dir = tempfile::tempdir()?;
        let program = dir.path().join("seat");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf 'ready %s\\n' \"$LYS_LAUNCH_TEMPLATE\"\nwhile IFS= read -r line; do printf '%s\\n' \"$line\"; done\n",
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
                if !runtime {
                    config.runtime_dir = None;
                }
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
        let mut cookies = Vec::new();
        for subject in [ADMINISTRATOR, "bea-subject"] {
            cookies.push(
                service
                    .sign_in(Login {
                        subject: subject.to_owned(),
                        email: "person@example.test".to_owned(),
                    })
                    .await?,
            );
        }
        let mut held = Self {
            service,
            seeded,
            ada: cookies.remove(0),
            bea: cookies.remove(0),
            dir,
            key,
            serving,
            machine: operation()?,
        };
        held.ok("/network/machines", &json!({ "operation": held.machine, "name": "Seat", "kind": "laptop", "runtime": program.display().to_string(), "slots": 2, "may_run": [held.agent()], "may_reach": [] })).await?;
        held.ok(
            &format!("/network/machines/{}/runner", held.machine),
            &json!({ "runner": { "kind": "lys" } }),
        )
        .await?;
        held.profile(0, false).await?;
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

    async fn profile(&mut self, from: u32, reviewed: bool) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let mut harness = harness_description::declared();
        harness["program"] = json!(self.dir.path().join("seat").display().to_string());
        self.ok(&path, &json!({ "operation": operation()?, "from_version": from,
            "model_access": [format!("model-{}", from + 1)], "tools": [], "skills": [], "mcp_servers": [], "instructions": "", "note": "", "harness": harness })).await?;
        if reviewed {
            self.review(from + 1).await?;
        }
        Ok(())
    }

    async fn review(&self, version: u32) -> TestResult {
        self.ok(
            &format!("/agents/{}/provisioning/{version}/review", self.agent()),
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

    async fn ready(&self, session: &str, hash: &str) -> TestResult {
        let client = self.client();
        let act = Act::Wait {
            session: session.to_owned(),
            cursor: Some(0),
            pattern: format!("ready {hash}"),
            regex: false,
        };
        let answer = tokio::task::spawn_blocking(move || client.ask(&act)).await??;
        assert!(matches!(answer, Answer::Matched { .. }), "{answer:?}");
        Ok(())
    }

    fn close(self) -> TestResult {
        self.serving.stop()?;
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_ends_the_old_session_and_keeps_the_latest_reviewed_launch() -> TestResult {
    let mut held = Held::open().await?;
    held.review(1).await?;
    let before = held.start().await?;
    let old = before["session"].as_str().ok_or("no old session")?;
    held.ready(old, before["template_sha256"].as_str().ok_or("no hash")?)
        .await?;
    held.profile(1, true).await?;
    held.profile(2, false).await?;
    let body = json!({ "session": old, "operation": operation()? });
    let path = format!("/agents/{}/restart", held.agent());
    let started = held.ok(&path, &body).await?;
    assert_eq!(started["provisioning_version"], 2, "{started}");
    assert_ne!(started["session"], before["session"]);
    assert!(
        held.status(old)?.ended.is_some(),
        "old process still running"
    );
    let new = started["session"].as_str().ok_or("no new session")?;
    held.ready(
        new,
        started["template_sha256"].as_str().ok_or("no new hash")?,
    )
    .await?;
    assert!(held.status(new)?.ended.is_none());
    let replayed = held.ok(&path, &body).await?;
    assert_eq!(replayed["session"], started["session"]);
    assert_eq!(replayed["template_sha256"], started["template_sha256"]);
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn no_reviewed_profile_refuses_before_ending_the_running_session() -> TestResult {
    let held = Held::open().await?;
    let session = operation()?;
    held.client().ask(&Act::Start {
        launch: Box::new(Launch {
            session: session.clone(),
            program: "/bin/cat".to_owned(),
            arguments: Vec::new(),
            directory: "/".to_owned(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        }),
    })?;
    held.ok(&format!("/agents/{}/runtime/sessions/{session}/reports", held.agent()), &json!({ "operation": operation()?, "machine": held.machine, "state": "running", "what": "process started" })).await?;
    let (status, refused) = held
        .service
        .post(
            &format!("/agents/{}/restart", held.agent()),
            Some(&held.ada),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    assert_ne!(status, 200);
    assert_eq!(
        refused["refusal"], "profile_version_unreviewed",
        "{refused}"
    );
    assert!(held.status(&session)?.ended.is_none());
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn another_person_cannot_restart_the_agents_session() -> TestResult {
    let held = Held::open().await?;
    held.review(1).await?;
    let started = held.start().await?;
    let session = started["session"].as_str().ok_or("no session")?;
    let (status, refused) = held
        .service
        .post(
            &format!("/agents/{}/restart", held.agent()),
            Some(&held.bea),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    assert_eq!(status, 403, "{refused}");
    assert!(held.status(session)?.ended.is_none());
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn restart_refuses_reused_ids_missing_sessions_and_inactive_agents() -> TestResult {
    let held = Held::open().await?;
    held.review(1).await?;
    let started = held.start().await?;
    let session = started["session"].as_str().ok_or("no session")?;
    let path = format!("/agents/{}/restart", held.agent());
    let (status, reused) = held
        .service
        .post(
            &path,
            Some(&held.ada),
            &json!({ "session": session, "operation": session }),
        )
        .await?;
    assert_ne!(status, 200);
    assert_eq!(reused["refusal"], "RuntimeReportReused", "{reused}");
    let absent = operation()?;
    held.ok(
        &format!("/agents/{}/runtime/sessions/{absent}/reports", held.agent()),
        &json!({ "operation": operation()?, "machine": held.machine, "state": "running" }),
    )
    .await?;
    let (status, missing) = held
        .service
        .post(
            &path,
            Some(&held.ada),
            &json!({ "session": absent, "operation": operation()? }),
        )
        .await?;
    assert_eq!(status, 404, "{missing}");
    assert_eq!(missing["refusal"], "session_unknown", "{missing}");
    held.ok(&format!("/identities/{}/transitions", held.agent()), &json!({ "operation": operation()?, "transition": "suspend", "reason": "suspend the agent" })).await?;
    let (status, inactive) = held
        .service
        .post(
            &path,
            Some(&held.ada),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    assert_ne!(status, 200);
    assert_eq!(inactive["refusal"], "AgentNotActive", "{inactive}");
    assert!(held.status(session)?.ended.is_none());
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unconfigured_runtime_refuses_restart_by_name() -> TestResult {
    let held = Held::open_with_runtime(false).await?;
    let (status, refused) = held
        .service
        .post(
            &format!("/agents/{}/restart", held.agent()),
            Some(&held.ada),
            &json!({ "session": operation()?, "operation": operation()? }),
        )
        .await?;
    assert_ne!(status, 200);
    assert_eq!(refused["refusal"], "RuntimeUnavailable", "{refused}");
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_end_without_exit_evidence_does_not_start_another_session() -> TestResult {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    let held = Held::open().await?;
    held.review(1).await?;
    let started = held.start().await?;
    let session = started["session"].as_str().ok_or("no session")?.to_owned();
    let socket = held.dir.path().join("other.sock");
    let listener = UnixListener::bind(&socket)?;
    let reply_session = session.clone();
    let answering = std::thread::spawn(move || -> std::io::Result<()> {
        let (stream, _) = listener.accept()?;
        let mut writer = &stream;
        writer.write_all(b"{\"version\":1,\"runner\":\"00000000000000000000000000000000\",\"challenge\":\"00000000000000000000000000000000\"}\n")?;
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line)?;
        let answer = serde_json::to_vec(&Answer::Delivered {
            session: reply_session,
        })?;
        writer.write_all(&answer)?;
        writer.write_all(b"\n")?;
        Ok(())
    });
    held.ok(
        &format!("/network/machines/{}/runner", held.machine),
        &json!({ "runner": { "kind": "socket", "path": socket.display().to_string() } }),
    )
    .await?;
    let (status, refused) = held
        .service
        .post(
            &format!("/agents/{}/restart", held.agent()),
            Some(&held.ada),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    answering.join().expect("runner stand-in panicked")?;
    assert_eq!(status, 502, "{refused}");
    assert_eq!(refused["refusal"], "runner_reply_malformed", "{refused}");
    assert!(held.status(&session)?.ended.is_none());
    held.close()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_machine_without_a_runner_refuses_restart_before_ending() -> TestResult {
    let held = Held::open().await?;
    held.review(1).await?;
    let started = held.start().await?;
    let session = started["session"].as_str().ok_or("no session")?;
    held.ok(
        &format!("/network/machines/{}/runner", held.machine),
        &json!({ "runner": null }),
    )
    .await?;
    let (status, refused) = held
        .service
        .post(
            &format!("/agents/{}/restart", held.agent()),
            Some(&held.ada),
            &json!({ "session": session, "operation": operation()? }),
        )
        .await?;
    assert_ne!(status, 200);
    assert_eq!(refused["refusal"], "runner_absent", "{refused}");
    assert!(held.status(session)?.ended.is_none());
    held.close()
}
