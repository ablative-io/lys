#![cfg(test)]

//! AGENTS-001 R2: variables per agent and per session with revision and
//! expiry. Two patches with the same revision: the second is refused by
//! name; a key with an expiry reads as absent after it and is named
//! expired; the agent patches its own variables by its signed request and
//! another agent's patch is refused with the needed grant named; a restart
//! keeps the map; a run's own routes take its pass and nothing else; and
//! without a variables directory the routes answer `variables_unavailable`.

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::OperationId;
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::seed_configured;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

fn now() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0x0f)],
            ]
        })
        .map(char::from)
        .collect()
}

/// Ada (administrator) and Bea, each with an agent holding a certificate
/// over a key the test signs with; the cookies of both.
struct Table {
    service: Service,
    beas_agent: String,
    adas_agent: String,
    ada: String,
    bea: String,
    beas_key: Arc<Ed25519Identity>,
    adas_key: Arc<Ed25519Identity>,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let beas_agent = seeded.people[1].agents[0].id.to_string();
        let adas_agent = seeded.people[0].agents[0].id.to_string();
        let beas_key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join("bea-agent.key"),
        )?);
        let adas_key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join("ada-agent.key"),
        )?);
        for (agent, key, cookie) in [(&beas_agent, &beas_key, &bea), (&adas_agent, &adas_key, &ada)] {
            let issue = json!({
                "operation": OperationId::generate()?.to_string(),
                "request": STANDARD.encode(create_certificate_request(key, agent)?),
            });
            let (status, answer) = service
                .post(&format!("/agents/{agent}/certificates"), Some(cookie), &issue)
                .await?;
            assert_eq!(status, 200, "{answer}");
        }
        Ok(Self {
            service,
            beas_agent,
            adas_agent,
            ada,
            bea,
            beas_key,
            adas_key,
        })
    }

    async fn patch(
        &self,
        cookie: &str,
        agent: &str,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .post(&format!("/agents/{agent}/variables"), Some(cookie), body)
            .await
    }

    async fn read(&self, cookie: &str, agent: &str) -> Result<(u16, Value), Box<dyn Error>> {
        self.service
            .get(&format!("/agents/{agent}/variables"), Some(cookie))
            .await
    }

    /// A patch of `agent`'s variables signed by `key` as `signer`.
    async fn signed_patch(
        &self,
        signer: &str,
        key: &Ed25519Identity,
        agent: &str,
        nonce: u8,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/agents/{agent}/variables");
        let bytes = body.to_string().into_bytes();
        let signed_at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = hex(&[nonce; 16]);
        let signed = payload("POST", &path, &bytes, signed_at, &nonce);
        let cose = sign_attestation(&signed, key).to_cose_bytes();
        let header = format!("{signer} {signed_at} {nonce} {}", hex(&cose));
        self.service
            .post_signed(&path, (HEADER, &header), bytes)
            .await
    }
}

#[tokio::test]
async fn a_patch_carries_the_revision_read_and_a_stale_one_is_refused() -> TestResult {
    let table = Table::set().await?;
    let body = json!({ "revision": 0, "values": { "focus": "the door install", "step": 3 } });
    let (status, answer) = table.patch(&table.bea, &table.beas_agent, &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["revision"], 1);
    assert_eq!(answer["values"]["focus"]["value"], "the door install");
    assert_eq!(answer["values"]["step"]["value"], 3);
    assert_eq!(answer["values"]["focus"]["revision"], 1);
    assert!(answer["values"]["focus"]["author"]
        .as_str()
        .is_some_and(|author| author.starts_with("person-")));

    let (status, answer) = table.patch(&table.bea, &table.beas_agent, &body).await?;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["refusal"], "variables_stale");

    let next = json!({ "revision": 1, "values": { "step": null, "next": "readback" } });
    let (status, answer) = table.patch(&table.ada, &table.beas_agent, &next).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["revision"], 2);
    assert!(answer["values"].get("step").is_none(), "{answer}");
    assert_eq!(answer["values"]["focus"]["revision"], 1, "an omitted key is unchanged");
    assert_eq!(answer["values"]["next"]["revision"], 2);

    let (status, read) = table.read(&table.bea, &table.beas_agent).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["revision"], 2);
    assert_eq!(read["scope"], json!({ "kind": "agent", "id": table.beas_agent }));
    Ok(())
}

#[tokio::test]
async fn an_expired_key_reads_as_absent_and_is_named() -> TestResult {
    let table = Table::set().await?;
    let soon = now()? + 1;
    let body = json!({ "revision": 0, "values": { "lantern": "lit" }, "expires_at": soon });
    let (status, answer) = table.patch(&table.ada, &table.beas_agent, &body).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["values"]["lantern"]["expires_at"], soon);
    tokio::time::sleep(std::time::Duration::from_millis(1600)).await;
    let (status, read) = table.read(&table.ada, &table.beas_agent).await?;
    assert_eq!(status, 200, "{read}");
    assert!(read["values"].get("lantern").is_none(), "{read}");
    assert_eq!(read["expired"], json!(["lantern"]));
    assert_eq!(read["revision"], 1, "the revision stands");

    let past = json!({ "revision": 1, "values": { "x": 1 }, "expires_at": 1 });
    let (status, answer) = table.patch(&table.ada, &table.beas_agent, &past).await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "variables_malformed");
    Ok(())
}

#[tokio::test]
async fn a_malformed_name_or_an_empty_patch_is_refused_by_name() -> TestResult {
    let table = Table::set().await?;
    for values in [json!({ "Focus": 1 }), json!({ "9lives": 1 }), json!({ "a-b": 1 }), json!({})] {
        let (status, answer) = table
            .patch(&table.ada, &table.beas_agent, &json!({ "revision": 0, "values": values }))
            .await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], "variables_malformed", "{answer}");
    }
    let (status, answer) = table
        .patch(&table.ada, &table.beas_agent, &json!({ "revision": 0 }))
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "RequestMalformed");
    Ok(())
}

#[tokio::test]
async fn the_agent_patches_its_own_and_another_agent_is_refused_naming_the_grant() -> TestResult {
    let table = Table::set().await?;
    let body = json!({ "revision": 0, "values": { "focus": "mine" } });
    let (status, answer) = table
        .signed_patch(&table.beas_agent, &table.beas_key, &table.beas_agent, 1, &body)
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["values"]["focus"]["author"], table.beas_agent);

    let body = json!({ "revision": 1, "values": { "focus": "theirs" } });
    let (status, answer) = table
        .signed_patch(&table.adas_agent, &table.adas_key, &table.beas_agent, 2, &body)
        .await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "not_permitted");
    assert!(
        answer["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("variables.set")),
        "{answer}"
    );

    // Bea, responsible for her agent only, is refused on Ada's agent.
    let (status, answer) = table.patch(&table.bea, &table.adas_agent, &body).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "not_permitted");
    let (status, answer) = table.read(&table.bea, &table.adas_agent).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "not_permitted");

    let (status, answer) = table
        .service
        .get("/runtime/sessions/session-nobody/variables", Some(&table.ada))
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "SessionUnknown");

    let (status, answer) = table
        .read(&table.ada, "agent-not-an-id")
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "AgentNotVisible");
    Ok(())
}

#[tokio::test]
async fn a_restart_keeps_the_map_and_its_revision() -> TestResult {
    let mut table = Table::set().await?;
    let body = json!({ "revision": 0, "values": { "focus": "kept", "count": [1, 2] } });
    let (status, answer) = table.patch(&table.ada, &table.beas_agent, &body).await?;
    assert_eq!(status, 200, "{answer}");
    table.service.restart().await?;
    let ada = table.service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, read) = table.read(&ada, &table.beas_agent).await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["revision"], 1);
    assert_eq!(read["values"]["focus"]["value"], "kept");
    assert_eq!(read["values"]["count"]["value"], json!([1, 2]));
    Ok(())
}

#[tokio::test]
async fn a_runs_own_routes_take_its_pass_and_nothing_else() -> TestResult {
    let table = Table::set().await?;
    let (status, answer) = table.service.get("/me/variables", Some(&table.ada)).await?;
    assert_eq!(status, 403, "{answer}");
    assert_eq!(answer["refusal"], "NotAdmitted");

    let (status, answer) = table
        .service
        .post_signed("/me/session/variables", ("lys-agent-pass", "short"), b"{}".to_vec())
        .await?;
    assert_eq!(status, 401, "{answer}");
    assert_eq!(answer["refusal"], "AgentPassRefused");
    Ok(())
}

#[tokio::test]
async fn without_a_variables_directory_the_routes_answer_variables_unavailable() -> TestResult {
    let (service, seeded) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.variables_dir = None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR])?),
    )
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let (status, answer) = service
        .get(&format!("/agents/{agent}/variables"), Some(&ada))
        .await?;
    assert_eq!(status, 503, "{answer}");
    assert_eq!(answer["refusal"], "variables_unavailable");
    Ok(())
}
