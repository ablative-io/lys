//! HOME-037 R1: the harness and its build are declared in the profile.
//! A profile records the declared kind, program and package and gives them
//! back as recorded; a start from a profile that declares none is refused
//! by name before anything is rendered.

use std::error::Error;

use axum::Router;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::{Seeded, seed_configured};
use lys_identity_server::secrets_api::SecretsSettings;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

async fn broker(request: Request) -> Response {
    if request.uri().path() != "/_lys/handles" {
        return axum::Json(json!({})).into_response();
    }
    axum::Json(json!({ "holder": "any", "handles": [] })).into_response()
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

struct Table {
    service: Service,
    seeded: Seeded,
    ada: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        tokio::spawn(async move { axum::serve(listener, Router::new().fallback(broker)).await });
        let keys = tempfile::TempDir::new()?;
        let key_file = keys.path().join("secrets-service.key");
        Ed25519Identity::load_or_generate(&key_file)?;
        let settings = SecretsSettings {
            broker: format!("http://{address}"),
            service: "identity".to_owned(),
            service_key_file: key_file,
        };
        let (service, seeded) =
            Service::start_asking(GRANT_MODEL, None, Some(settings), |config| {
                Ok(seed_configured(config, [ADMINISTRATOR, "bea-subject"])?)
            })
            .await?;
        drop(keys);
        let ada = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "ada@example.test".to_owned(),
            })
            .await?;
        Ok(Self {
            service,
            seeded,
            ada,
        })
    }

    fn agent(&self) -> String {
        self.seeded.people[0].agents[0].id.to_string()
    }

    /// Record a profile declaring `harness`, or none when it is null, from `from`.
    async fn record(&self, from: u32, harness: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let mut body = json!({
            "operation": operation()?, "from_version": from,
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "",
        });
        if !harness.is_null() {
            body["harness"] = harness.clone();
        }
        let path = format!("/agents/{}/provisioning", self.agent());
        self.service.post(&path, Some(&self.ada), &body).await
    }

    /// Record `harness`, review it, and ask for a start on a new machine.
    async fn start(&self, from: u32, harness: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let (status, set) = self.record(from, harness).await?;
        assert_eq!(status, 200, "{set}");
        let path = format!("/agents/{}/provisioning/{}/review", self.agent(), from + 1);
        let reviewed = json!({ "operation": operation()? });
        let (status, set) = self.service.post(&path, Some(&self.ada), &reviewed).await?;
        assert_eq!(status, 200, "{set}");
        let machine = operation()?;
        let body = json!({
            "operation": machine, "name": format!("Box {from}"), "kind": "server",
            "runtime": "manifold", "slots": 1, "may_run": [self.agent()], "may_reach": [],
        });
        let (status, named) = self
            .service
            .post("/network/machines", Some(&self.ada), &body)
            .await?;
        assert_eq!(status, 200, "{named}");
        let path = format!("/agents/{}/start-command", self.agent());
        let body = json!({ "machine": machine, "operation": operation()? });
        self.service.post(&path, Some(&self.ada), &body).await
    }
}

fn claude_code() -> Value {
    json!({ "kind": "claude_code", "program": "/opt/seat/bin/claude", "package": "claude-code-seat" })
}

#[tokio::test]
async fn a_profile_records_its_declared_build_and_refuses_one_it_cannot_verify() -> TestResult {
    let table = Table::set().await?;
    let (status, set) = table.record(0, &claude_code()).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["profile"]["harness"], claude_code());
    let mut relative = claude_code();
    relative["program"] = json!("bin/claude");
    let (status, refused) = table.record(1, &relative).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("RequestMalformed")),
        "{refused}"
    );
    let mut unnamed = claude_code();
    unnamed["package"] = json!(" ");
    let (status, refused) = table.record(1, &unnamed).await?;
    assert_eq!(
        (status, &refused["refusal"]),
        (400, &json!("RequestMalformed")),
        "{refused}"
    );
    Ok(())
}

#[tokio::test]
async fn a_start_from_a_profile_with_no_declared_harness_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    let (status, refused) = table.start(0, &Value::Null).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "HarnessUndeclared");
    let (status, start) = table.start(1, &claude_code()).await?;
    assert_eq!(status, 200, "{start}");
    let codex = json!({ "kind": "codex", "program": "/opt/seat/bin/cdx", "package": "codex-seat" });
    let (status, refused) = table.start(2, &codex).await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(
        refused["refusal"], "LaunchUnrenderable",
        "no Codex start is rendered as Claude Code"
    );
    Ok(())
}
