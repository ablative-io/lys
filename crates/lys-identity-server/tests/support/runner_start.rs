#![cfg(test)]
//! A declared fixture program reads its config before signalling readiness.

#[path = "harness_description.rs"]
mod harness_description;

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

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

async fn broker(request: Request) -> Response {
    if request.uri().path() != "/_lys/handles" {
        return axum::Json(json!({})).into_response();
    }
    axum::Json(json!({ "holder": "any", "handles": [
        { "id": "h-live", "secret": "git-host token", "max_uses": 10, "used": 1,
          "not_after_ms": 0, "dropped": false, "spend_cap": null, "settled": 0, "parent": null },
    ]}))
    .into_response()
}

pub fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

pub struct Table {
    pub service: Service,
    seeded: Seeded,
    pub ada: String,
    pub dir: tempfile::TempDir,
    pub serving: Option<Serving>,
    pub server_key: Arc<Ed25519Identity>,
}

impl Table {
    pub async fn set() -> Result<Self, Box<dyn Error>> {
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
            GRANT_MODEL,
            None,
            Some(settings),
            None,
            move |config| config.runner_socket = Some(adjusted),
            move |config| {
                let seeded = seed_configured(config, [ADMINISTRATOR, "bea-subject"])?;
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
        };
        table.profile().await?;
        Ok(table)
    }

    pub fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    pub async fn ok(&self, path: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(path, Some(&self.ada), body).await?;
        if status != 200 {
            return Err(format!("{path} answered {status}: {answer}").into());
        }
        Ok(answer)
    }

    pub async fn profile(&self) -> TestResult {
        let path = format!("/agents/{}/provisioning", self.agent());
        let program = self.dir.path().join("declared-harness");
        std::fs::write(
            &program,
            "#!/bin/sh\nprintf 'declared-program\\n'\nprintf '%s\\n' \"$@\"\ncat \"$CLAUDE_CONFIG_DIR/settings.json\" \"$CLAUDE_CONFIG_DIR/mcp.json\" \"$CLAUDE_CONFIG_DIR/instructions.txt\"\nprintf '\\nprofile-read\\n'\nexec cat\n",
        )?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
        let mut harness = harness_description::declared();
        harness["program"] = json!(program);
        let body = json!({
            "operation": operation()?, "from_version": 0,
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "",
            "harness": harness,
        });
        self.ok(&path, &body).await?;
        self.ok(
            &format!("{path}/1/review"),
            &json!({ "operation": operation()? }),
        )
        .await?;
        Ok(())
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

    pub async fn start(&self, agent: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/agents/{agent}/start-command");
        self.service.post(&path, Some(&self.ada), body).await
    }

    pub fn close(mut self) -> TestResult {
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}
