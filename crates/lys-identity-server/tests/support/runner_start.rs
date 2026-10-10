#![cfg(test)]
//! A declared fixture program reads its config before signalling readiness.

#[path = "harness_description.rs"]
mod harness_description;

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::{Arc, OnceLock};

use axum::Router;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, IdentityId, LifecycleState, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::dev_seed::{SeededAgent, SeededPerson};
use lys_identity_server::routes::open_directory;
use lys_identity_server::secrets_api::SecretsSettings;
use lys_runner::{Options, Runner, Serving};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

/// A planted broker value that must never reach a rendered start.
pub const PLANTED: &str = "sk-planted-credential-value";

async fn broker(request: Request) -> Response {
    if request.uri().path() != "/_lys/handles" {
        return axum::Json(json!({})).into_response();
    }
    axum::Json(json!({ "holder": "any", "handles": [
        { "id": "h-live", "secret": "git-host token", "max_uses": 10, "used": 1,
          "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0,
          "parent": null, "value": PLANTED, "token": PLANTED },
        { "id": "h-gone", "secret": "old key", "max_uses": 1, "used": 1,
          "not_after_ms": 0, "dropped": true, "spend_cap": null, "settled": 0,
          "parent": null },
    ]}))
    .into_response()
}

/// A fresh operation for a fixture request.
pub fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

/// One service, broker and real runner shared by a scenario.
pub struct Table {
    /// The directory service.
    pub service: Service,
    seeded: Seeded,
    /// The administrator session.
    pub ada: String,
    /// The declared program and runner storage.
    pub dir: tempfile::TempDir,
    /// The runner serving its socket.
    pub serving: Option<Serving>,
    /// The key used to ask the runner directly.
    pub server_key: Arc<Ed25519Identity>,
    /// The one machine of this table naming the install's own runner.
    lys_machine: OnceLock<String>,
}

impl Table {
    /// Start with a reviewed profile for the declared fixture program.
    pub async fn set() -> Result<Self, Box<dyn Error>> {
        let table = Self::unprofiled().await?;
        table.profile().await?;
        Ok(table)
    }

    /// Start once, leaving the scenario to record and review its own profile.
    pub async fn unprofiled() -> Result<Self, Box<dyn Error>> {
        Self::unprofiled_with(false, None).await
    }

    /// Command scenarios seed a second sign-in subject only when they use one.
    pub async fn unprofiled_with(
        commands_only: bool,
        other_subject: Option<&str>,
    ) -> Result<Self, Box<dyn Error>> {
        Self::unprofiled_judging(GRANT_MODEL, commands_only, other_subject).await
    }

    /// Start with a reviewed profile, judging grants by `model`.
    pub async fn set_judging(model: &'static str) -> Result<Self, Box<dyn Error>> {
        let table = Self::unprofiled_judging(model, false, None).await?;
        table.profile().await?;
        Ok(table)
    }

    /// As [`Table::unprofiled_with`], judging grants by `model`.
    pub async fn unprofiled_judging(
        model: &'static str,
        commands_only: bool,
        other_subject: Option<&str>,
    ) -> Result<Self, Box<dyn Error>> {
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
        let (service, (seeded, serving, server_key)) = Service::start_adjusted(
            model,
            None,
            Some(settings),
            None,
            move |config| {
                config.runner_socket = Some(adjusted);
                if commands_only {
                    config.sessions_file = None;
                    config.requests_dir = None;
                    config.roles_file = None;
                    config.service_accounts_dir = None;
                    config.teams_dir = None;
                    config.stops_dir = None;
                    config.budgets_dir = None;
                    config.goals_dir = None;
                    config.reviews_dir = None;
                }
            },
            move |config| {
                let seeded = if commands_only {
                    command_seed(config, other_subject)?
                } else {
                    seed_configured(config, [ADMINISTRATOR, "bea-subject"])?
                };
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let runner = Runner::open(&Options {
                    socket,
                    state,
                    server_key: key.public_key_bytes(),
                    scrollback: 1 << 16,
                })?;
                Ok((seeded, runner.spawn(), key))
            },
        )
        .await?;
        let ada = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "ada@example.test".to_owned(),
            })
            .await?;
        let table = Self {
            service,
            seeded,
            ada,
            dir,
            serving: Some(serving),
            server_key,
            lys_machine: OnceLock::new(),
        };
        table.declare_program()?;
        Ok(table)
    }

    /// The seeded agent that the administrator answers for.
    pub fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    /// The seeded person at `index`: the administrator first, then bea.
    pub fn person(&self, index: usize) -> String {
        self.seeded.people[index].id.to_string()
    }

    /// The first agent of the seeded person at `index`, when there is one.
    pub fn agent_of(&self, index: usize) -> Option<String> {
        self.seeded
            .people
            .get(index)?
            .agents
            .first()
            .map(|agent| agent.id.to_string())
    }

    /// Send a setup request and propagate any refusal.
    pub async fn ok(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }

    fn declare_program(&self) -> TestResult {
        let program = self.dir.path().join("declared-harness");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf 'declared-program\\n'\nprintf '%s\\n' \"$@\"\nwhile [ $# -gt 1 ]; do case \"$1\" in --settings) s=$2;; --mcp-config) m=$2;; --append-system-prompt-file|--system-prompt-file) i=$2;; esac; shift; done\ncat \"$s\" \"$m\" \"$i\"\nprintf '\\nprofile-read\\n'\nexec cat\n",
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
        Ok(())
    }

    /// The declared harness with the exact executable created for this scenario.
    pub fn harness(&self) -> Value {
        let mut harness = harness_description::declared();
        harness["program"] = json!(self.dir.path().join("declared-harness"));
        harness
    }

    /// Record and review the default profile once.
    pub async fn profile(&self) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let harness = self.harness();
        let body = json!({
            "operation": operation()?, "from_version": 0,
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "",
            "harness": harness,
            "permissions": {"default_mode": "plan"},
            "working_folder": self.dir.path(),
        });
        self.ok(&path, &body).await?;
        self.ok(
            &format!("{path}/1/review"),
            &json!({ "operation": operation()? }),
        )
        .await?;
        Ok(())
    }

    /// The one machine of this table that names the install's own runner:
    /// named `name` on the first ask, answered as it is after. An install's
    /// own runner is named by one live machine only (AGENTS-002 D3, a second
    /// refused `runner_lys_taken`), so every start of a table shares it.
    pub async fn lys_machine(&self, name: &str) -> Result<String, Box<dyn Error>> {
        if let Some(machine) = self.lys_machine.get() {
            return Ok(machine.clone());
        }
        let machine = operation()?;
        let body = json!({
            "operation": machine, "name": name, "kind": "server",
            "runtime": "manifold", "slots": 1, "may_run": [self.agent()], "may_reach": [],
        });
        let (status, named) = self
            .service
            .post("/network/machines", Some(&self.ada), &body)
            .await?;
        assert_eq!(status, 200, "{named}");
        self.ok(
            &format!("/network/machines/{machine}/runner"),
            &json!({"runner": {"kind": "lys"}}),
        )
        .await?;
        self.lys_machine
            .set(machine.clone())
            .map_err(|held| format!("the table's lys machine was named twice: {held}"))?;
        Ok(machine)
    }

    /// Name the requested machine, recording its runner when one is given.
    pub async fn machine(
        &self,
        body: &Value,
        runner: Option<Value>,
    ) -> Result<String, Box<dyn Error>> {
        let id = body["operation"]
            .as_str()
            .ok_or("machine body has no operation")?;
        self.ok("/network/machines", body).await?;
        if let Some(runner) = runner {
            self.ok(
                &format!("/network/machines/{id}/runner"),
                &json!({ "runner": runner }),
            )
            .await?;
        }
        Ok(id.to_owned())
    }

    /// Send the exact start request for the given agent.
    pub async fn start(&self, agent: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/agents/{agent}/start-command");
        self.service.post(&path, Some(&self.ada), body).await
    }

    /// Stop the runner and propagate any shutdown error.
    pub fn close(mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}

fn command_seed(
    config: &lys_identity_server::Config,
    other_subject: Option<&str>,
) -> Result<Seeded, Box<dyn Error>> {
    let actor = Actor::new(
        LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, 1),
    );
    let mut directory = open_directory(config)?;
    let (owner, _) = directory.setup_person(
        actor.clone(),
        OperationId::generate()?,
        Profile::new("Owner")?,
        1,
    )?;
    let (agent, _) = directory.register_agent(
        actor.clone(),
        OperationId::generate()?,
        owner,
        Profile::new("Agent")?,
        2,
    )?;
    directory.transition(
        actor.clone(),
        OperationId::generate()?,
        IdentityId::Agent(agent),
        Transition::Activate,
        "",
        3,
    )?;
    if let Some(subject) = other_subject {
        let (other, _) = directory.register_person(
            actor.clone(),
            OperationId::generate()?,
            Profile::new("Other")?,
            4,
        )?;
        directory.bind_login(
            actor.clone(),
            OperationId::generate()?,
            other,
            LoginBinding::new(&config.issuer, subject)?,
            5,
        )?;
        directory.transition(
            actor,
            OperationId::generate()?,
            IdentityId::Person(other),
            Transition::Activate,
            "",
            6,
        )?;
    }
    Ok(Seeded {
        people: vec![SeededPerson {
            id: owner,
            display_name: "Owner".to_owned(),
            subject: ADMINISTRATOR.to_owned(),
            agents: vec![SeededAgent {
                id: agent,
                display_name: "Agent".to_owned(),
                state: LifecycleState::Active,
            }],
        }],
        tree_size: directory.log()?.head()?.0,
    })
}
