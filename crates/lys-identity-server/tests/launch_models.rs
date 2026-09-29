//! HOME-037 R2: every model a profile lists is carried or refused by name.
//! Claude Code starts on the first and falls back through the rest in
//! order; a model the declared harness cannot carry is refused when the
//! profile is recorded, and nothing is reported left out.

use std::error::Error;

use axum::Router;
use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_home::harness::claude_code::template::parse_template;
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

    /// Record a profile of `models` declaring `harness`, from `from`.
    async fn record(
        &self,
        from: u32,
        harness: &Value,
        models: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({
            "operation": operation()?, "from_version": from,
            "model_access": models, "tools": [], "skills": [],
            "mcp_servers": [], "instructions": "", "note": "", "harness": harness,
        });
        let path = format!("/agents/{}/provisioning", self.agent());
        self.service.post(&path, Some(&self.ada), &body).await
    }

    /// Record `models` for `harness`, review them, and ask for a start on a new machine.
    async fn start(
        &self,
        from: u32,
        harness: &Value,
        models: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let (status, set) = self.record(from, harness, models).await?;
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

fn harness(kind: &str) -> Value {
    json!({ "kind": kind, "program": format!("/opt/seat/bin/{kind}"), "package": format!("{kind}-seat") })
}

#[tokio::test]
async fn three_models_start_claude_code_on_the_first_and_fall_back_through_the_rest() -> TestResult
{
    let table = Table::set().await?;
    let models = json!(["claude-fable-5-1", "claude-opus-5-5", "claude-sonnet-5"]);
    let (status, start) = table.start(0, &harness("claude_code"), &models).await?;
    assert_eq!(status, 200, "{start}");
    let template = parse_template(start["template"].as_str().ok_or("no template")?.as_bytes())?;
    let at = template
        .flags
        .iter()
        .position(|flag| flag == "--model")
        .ok_or("no --model")?;
    assert_eq!(
        template.flags[at..at + 4],
        [
            "--model",
            "claude-fable-5-1",
            "--fallback-model",
            "claude-opus-5-5,claude-sonnet-5"
        ]
    );
    assert_eq!(start["left_out"], json!([]));
    Ok(())
}

#[tokio::test]
async fn a_model_the_declared_harness_cannot_carry_is_refused_when_recorded() -> TestResult {
    let table = Table::set().await?;
    let models = json!(["gpt-seat-1", "gpt-seat-2"]);
    let (status, refused) = table.record(0, &harness("codex"), &models).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "ModelUnrepresentable");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("gpt-seat-2"))
    );
    let listed = json!(["claude-fable-5-1", "claude-opus-5-5,claude-sonnet-5"]);
    let (status, refused) = table.record(0, &harness("claude_code"), &listed).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "ModelUnrepresentable");
    let (status, set) = table
        .record(0, &harness("codex"), &json!(["gpt-seat-1"]))
        .await?;
    assert_eq!(status, 200, "{set}");
    Ok(())
}
