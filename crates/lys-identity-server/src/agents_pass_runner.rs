use std::error::Error;
use std::os::unix::fs::PermissionsExt;

use axum::{Json, Router};
use lys_core::Ed25519Identity;
use lys_runner::{Options, Runner, Serving};
use serde_json::{Value, json};

use super::fixture::{Table, operation};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

pub(super) struct Running {
    serving: Option<Serving>,
    broker: tokio::task::JoinHandle<Result<(), std::io::Error>>,
    pub(super) machine: String,
}

impl Running {
    pub(super) fn close(mut self) -> TestResult {
        self.broker.abort();
        if let Some(serving) = self.serving.take() {
            serving.stop()?;
        }
        Ok(())
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.broker.abort();
        if let Some(serving) = self.serving.take()
            && let Err(error) = serving.stop()
        {
            tracing::error!("fixture runner shutdown refused: {error}");
        }
    }
}

impl Table {
    pub(super) async fn launch_ready(&mut self) -> TestResult<Running> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let broker = tokio::spawn(async move {
            axum::serve(
                listener,
                Router::new().fallback(|| async { Json(json!({"handles":[]})) }),
            )
            .await
        });
        let root = self.service.dir.path();
        let key_file = root.join("service.key");
        let key = Ed25519Identity::load(&key_file)?;
        let socket = root.join("runner.sock");
        let runner = Runner::open(&Options {
            socket: socket.clone(),
            state: root.join("runner-state"),
            server_key: key.public_key_bytes(),
            scrollback: 4096,
        })?;
        let machine = operation()?;
        let running = Running {
            serving: Some(runner.spawn()),
            broker,
            machine: machine.clone(),
        };
        let settings = serde_json::from_value(json!({
            "broker":format!("http://{address}"), "service":"identity", "service_key_file":key_file
        }))?;
        self.service
            .restart_adjusted(|config| {
                config.network_file = Some(config.log_dir.with_file_name("network.json"));
                config.runtime_dir = Some(config.log_dir.with_file_name("runtime"));
                config.runner_socket = Some(socket);
                config.secrets = Some(settings);
            })
            .await?;
        let program = self.service.dir.path().join("declared-harness");
        std::fs::write(&program, "#!/bin/sh\nexec cat\n")?;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))?;
        let catalogue: Value = serde_json::from_str(include_str!(
            "../../../docs/harness/catalogue/claude-code.json"
        ))?;
        let harness = json!({"name":"Recorded harness", "program":program, "package":"fixture",
            "description":catalogue["description"]});
        let path = format!("/agents/{}/provisioning", self.target);
        self.setup(
            &path,
            &json!({"operation":operation()?, "from_version":0,
            "model_access":["model"], "tools":[], "skills":[], "mcp_servers":[],
            "instructions":"", "note":"", "harness":harness}),
        )
        .await?;
        self.setup(
            &format!("{path}/1/review"),
            &json!({"operation":operation()?}),
        )
        .await?;
        self.setup(
            "/network/machines",
            &json!({"operation":machine, "name":"Fixture",
            "kind":"server", "runtime":"declared", "slots":1, "may_run":[self.target],
            "may_reach":[]}),
        )
        .await?;
        self.setup(
            &format!("/network/machines/{machine}/runner"),
            &json!({"runner":{"kind":"lys"}}),
        )
        .await?;
        Ok(running)
    }

    pub(super) async fn runtime(&self) -> TestResult<Value> {
        let (status, answer) = self
            .service
            .get(
                &format!("/agents/{}/runtime/sessions", self.target),
                Some(&self.cookie),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }

    async fn setup(&self, path: &str, body: &Value) -> TestResult {
        let (status, answer) = self.service.post(path, Some(&self.cookie), body).await?;
        assert_eq!(status, 200, "{answer}");
        Ok(())
    }
}
