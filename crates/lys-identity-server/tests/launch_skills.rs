//! HOME-037 R3: each skill a profile names is a skill Lys keeps by name and
//! content hash. The administrator keeps a skill's text; a profile version
//! pins the text it was recorded with, so a later text reaches only later
//! versions; a skill Lys does not keep is refused when the profile is
//! recorded.

#[path = "support/harness_description.rs"]
mod harness_description;

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
use sha2::{Digest, Sha256};

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

    /// Record a Claude Code profile naming `skills`, from `from`.
    async fn record(&self, from: u32, skills: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let operation = operation()?;
        self.record_as(&operation, from, skills, "").await
    }

    /// Record it under `operation` with `note`.
    async fn record_as(
        &self,
        operation: &str,
        from: u32,
        skills: &Value,
        note: &str,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({
            "operation": operation, "from_version": from,
            "model_access": ["claude-fable-5-1"], "tools": [], "skills": skills,
            "mcp_servers": [], "instructions": "", "note": note,
            "harness": harness_description::declared(),
        });
        let path = format!("/agents/{}/provisioning", self.agent());
        self.service.post(&path, Some(&self.ada), &body).await
    }

    /// Keep `text` as the skill `name`.
    async fn keep(&self, name: &str, text: &str) -> Result<(u16, Value), Box<dyn Error>> {
        let body = json!({ "name": name, "text": text });
        self.service.post("/skills", Some(&self.ada), &body).await
    }
}

fn sha256(text: &str) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    Sha256::digest(text.as_bytes())
        .iter()
        .flat_map(|byte| {
            [
                char::from(DIGITS[usize::from(byte >> 4)]),
                char::from(DIGITS[usize::from(byte & 0x0f)]),
            ]
        })
        .collect()
}

#[tokio::test]
async fn a_kept_skill_is_listed_by_hash_and_an_unknown_one_is_refused_when_recorded() -> TestResult
{
    let table = Table::set().await?;
    let text = "Read the change against its brief.\n";
    let (status, kept) = table.keep("review", text).await?;
    assert_eq!(status, 200, "{kept}");
    assert_eq!(
        kept["skills"],
        json!([{ "name": "review", "len": text.len(), "sha256": sha256(text) }])
    );
    for bad in ["", ".hidden", "-flag", "two/parts", "Upper"] {
        let (status, refused) = table.keep(bad, text).await?;
        assert_eq!(
            (status, &refused["refusal"]),
            (400, &json!("RequestMalformed")),
            "{bad}"
        );
    }
    let (status, refused) = table.record(0, &json!(["review", "unkept"])).await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "SkillUnknown");
    assert!(
        refused["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("unkept"))
    );
    Ok(())
}

#[tokio::test]
async fn each_version_pins_the_text_it_was_recorded_with() -> TestResult {
    let table = Table::set().await?;
    let (first, second) = ("Check the brief.\n", "Check the brief and the tests.\n");
    table.keep("review", first).await?;
    let (status, set) = table.record(0, &json!(["review"])).await?;
    assert_eq!(status, 200, "{set}");
    let pinned =
        |text: &str| json!([{ "name": "review", "len": text.len(), "sha256": sha256(text) }]);
    assert_eq!(set["profile"]["skill_pins"], pinned(first));
    table.keep("review", second).await?;
    let (status, set) = table.record(1, &json!(["review"])).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["profile"]["skill_pins"], pinned(second));
    let (status, kept) = table.keep("review", first).await?;
    assert_eq!(status, 200, "{kept}");
    assert_eq!(
        kept["skills"].as_array().map(Vec::len),
        Some(2),
        "a text is kept once"
    );
    Ok(())
}

#[tokio::test]
async fn a_request_sent_again_after_a_newer_text_is_kept_answers_as_first_recorded() -> TestResult {
    let table = Table::set().await?;
    let (first, second) = ("Check the brief.\n", "Check the brief and the tests.\n");
    table.keep("review", first).await?;
    let once = operation()?;
    let (status, set) = table.record_as(&once, 0, &json!(["review"]), "").await?;
    assert_eq!(status, 200, "{set}");
    table.keep("review", second).await?;
    let (status, again) = table.record_as(&once, 0, &json!(["review"]), "").await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again["recorded"], set["recorded"]);
    assert_eq!(again["profile"]["skill_pins"], set["profile"]["skill_pins"]);
    let (status, changed) = table
        .record_as(&once, 0, &json!(["review"]), "changed")
        .await?;
    assert_eq!(status, 409, "{changed}");
    assert_eq!(changed["refusal"], "ProvisioningReused");
    Ok(())
}
