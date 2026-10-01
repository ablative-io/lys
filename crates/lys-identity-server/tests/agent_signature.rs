//! An agent reports its own session by signing the request with the key its
//! certificate names: a signature over the exact request is taken; one made
//! with a withdrawn certificate's key, a nonce sent twice, a signing time
//! more than a minute off and a body other than the one signed are each
//! refused `AgentSignatureRefused`.

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
use lys_identity::{
    Actor, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance, Transition,
};
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::certificates_store::{CertificateStore, Issued};
use lys_identity_server::routes::open_directory;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), reason: &str) {
    assert_eq!(answer.0, 401, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "AgentSignatureRefused", "{}", answer.1);
    assert!(
        answer.1.to_string().contains(reason),
        "expected `{reason}` in {}",
        answer.1
    );
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

fn now_ms() -> Result<u64, Box<dyn Error>> {
    Ok(u64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

/// Bea's agent, a machine it may run on, a certificate issued over `key`,
/// and the cookies to act as Bea.
struct Table {
    service: Service,
    agent: String,
    machine: String,
    bea: String,
    key: Arc<Ed25519Identity>,
    serial: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        Self::set_certificate(None).await
    }

    async fn set_certificate(der: Option<&'static [u8]>) -> Result<Self, Box<dyn Error>> {
        let serial = operation()?;
        let entered_serial = serial.clone();
        let (service, (agent, key)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.roles_file = None;
                config.provisioning_file = None;
                config.service_accounts_dir = None;
                config.teams_dir = None;
                config.stops_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.reviews_dir = None;
            },
            move |config| {
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let actor = Actor::new(
                    LoginBinding::new(&config.issuer, BEA)?,
                    Provenance::new(AuthMethod::Oidc, 1),
                );
                let mut directory = open_directory(config)?;
                let (person, _) = directory.setup_person(
                    actor.clone(),
                    OperationId::generate()?,
                    Profile::new("Owner")?,
                    1,
                )?;
                let (agent, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Agent")?,
                    2,
                )?;
                directory.transition(
                    actor,
                    OperationId::generate()?,
                    IdentityId::Agent(agent),
                    Transition::Activate,
                    "",
                    3,
                )?;
                if let Some(der) = der {
                    let mut certificates = CertificateStore::open(
                        config
                            .certificates_dir
                            .as_deref()
                            .ok_or("certificate directory missing")?,
                        Arc::clone(&key),
                    )?;
                    certificates.issue(Issued {
                        serial: entered_serial,
                        agent: agent.to_string(),
                        person: person.to_string(),
                        claims: json!({}),
                        der: STANDARD.encode(der),
                        issued_at: 0,
                    })?;
                }
                drop(
                    lys_identity_server::configuration_store::ConfigurationStore::open(
                        &config.log_dir.with_file_name("organisation"),
                        Arc::clone(&key),
                    )?,
                );
                drop(lys_identity_server::runner_acts::ActStore::open(
                    &config.log_dir.with_file_name("runner-acts"),
                    Arc::clone(&key),
                )?);
                drop(lys_identity::start::LaunchRecords::open(
                    &config.log_dir.with_file_name("launch-records"),
                    Ed25519Identity::load(&config.event_key_file)?,
                )?);
                drop(lys_identity_server::apps_api::opened(
                    config,
                    Arc::clone(&key),
                    &|_| {},
                )?);
                Ok((agent.to_string(), key))
            },
        )
        .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let machine = operation()?;
        let body = json!({
            "operation": machine, "name": "Laptop 2", "kind": "laptop", "runtime": "local launcher",
            "slots": 2, "may_run": [agent], "may_reach": [],
        });
        let (status, named) = service.post("/network/machines", Some(&ada), &body).await?;
        assert_eq!(status, 200, "{named}");
        if der.is_none() {
            let body = json!({
                "operation": serial,
                "request": STANDARD.encode(create_certificate_request(&key, &agent)?),
            });
            let (status, issued) = service
                .post(&format!("/agents/{agent}/certificates"), Some(&bea), &body)
                .await?;
            assert_eq!(status, 200, "{issued}");
        }
        Ok(Self {
            service,
            agent,
            machine,
            bea,
            key,
            serial,
        })
    }

    fn path(&self, session: &str) -> String {
        format!("/agents/{}/runtime/sessions/{session}/reports", self.agent)
    }

    fn body(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(json!({
            "operation": operation()?, "state": "starting", "machine": self.machine,
            "what": "the agent's own session",
        })
        .to_string()
        .into_bytes())
    }

    /// The header signing `body` for `path` at `signed_at` under `nonce`.
    fn header(&self, path: &str, body: &[u8], signed_at: u64, nonce: &str) -> String {
        let signed = payload("POST", path, body, signed_at, nonce);
        let cose = sign_attestation(&signed, &self.key).to_cose_bytes();
        format!("{} {signed_at} {nonce} {}", self.agent, hex(&cose))
    }

    async fn send(
        &self,
        path: &str,
        header: &str,
        body: Vec<u8>,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        self.service.post_signed(path, (HEADER, header), body).await
    }
}

fn nonce(seed: u8) -> String {
    hex(&[seed; 16])
}

#[tokio::test]
async fn a_signed_report_is_taken_as_the_agents_own() -> TestResult {
    let table = Table::set().await?;
    let path = table.path(&operation()?);
    let body = table.body()?;
    let header = table.header(&path, &body, now_ms()?, &nonce(1));
    let (status, taken) = table.send(&path, &header, body).await?;
    assert_eq!(status, 200, "{taken}");
    assert_eq!(taken["agent"], table.agent, "{taken}");
    Ok(())
}

#[tokio::test]
async fn a_nonce_sent_twice_is_refused() -> TestResult {
    let table = Table::set().await?;
    let path = table.path(&operation()?);
    let body = table.body()?;
    let header = table.header(&path, &body, now_ms()?, &nonce(2));
    let (status, taken) = table.send(&path, &header, body.clone()).await?;
    assert_eq!(status, 200, "{taken}");
    refused(&table.send(&path, &header, body).await?, "already used");
    Ok(())
}

#[tokio::test]
async fn a_stale_signing_time_is_refused() -> TestResult {
    let table = Table::set().await?;
    let path = table.path(&operation()?);
    let body = table.body()?;
    let stale = now_ms()? - 5 * 60_000;
    let header = table.header(&path, &body, stale, &nonce(3));
    refused(
        &table.send(&path, &header, body).await?,
        "more than a minute",
    );
    Ok(())
}

#[tokio::test]
async fn a_body_other_than_the_one_signed_is_refused() -> TestResult {
    let table = Table::set().await?;
    let path = table.path(&operation()?);
    let header = table.header(&path, &table.body()?, now_ms()?, &nonce(4));
    refused(
        &table.send(&path, &header, table.body()?).await?,
        "does not verify",
    );
    Ok(())
}

#[tokio::test]
async fn a_withdrawn_certificates_key_is_refused() -> TestResult {
    let table = Table::set().await?;
    let withdrawal = format!(
        "/agents/{}/certificates/{}/withdrawal",
        table.agent, table.serial
    );
    let (status, withdrawn) = table
        .service
        .post(
            &withdrawal,
            Some(&table.bea),
            &json!({"reason": "the key left the machine"}),
        )
        .await?;
    assert_eq!(status, 200, "{withdrawn}");
    let path = table.path(&operation()?);
    let body = table.body()?;
    let header = table.header(&path, &body, now_ms()?, &nonce(5));
    refused(&table.send(&path, &header, body).await?, "does not verify");
    Ok(())
}

#[tokio::test]
async fn a_signed_agent_report_is_refused_when_its_responsible_person_is_suspended() -> TestResult {
    let table = Table::set().await?;
    let admin = table.service.sign_in(login(ADMINISTRATOR)).await?;
    let (_, me) = table.service.get("/me", Some(&table.bea)).await?;
    let person = me["person"]["id"].as_str().ok_or("no responsible person")?;
    let response = table.service.post(
        &format!("/identities/{person}/transitions"),
        Some(&admin),
        &json!({"operation": operation()?, "transition": "suspend", "reason": "no further authority"}),
    ).await?;
    assert_eq!(response.0, 200, "{}", response.1);
    let path = table.path(&operation()?);
    let body = table.body()?;
    let header = table.header(&path, &body, now_ms()?, &nonce(43));
    let answer = table.send(&path, &header, body).await?;
    assert_eq!(answer.0, 403, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "inactive");
    assert!(
        answer.1["reason"]
            .as_str()
            .ok_or("no reason")?
            .contains("suspended")
    );
    Ok(())
}

#[tokio::test]
async fn a_valid_agent_signature_cannot_bypass_a_bad_operator_header() -> TestResult {
    let table = Table::set().await?;
    let path = table.path(&operation()?);
    let body = table.body()?;
    let header = table.header(&path, &body, now_ms()?, &nonce(42));
    let answer = table
        .service
        .post_carrying(
            &path,
            &[(HEADER, &header), ("lys-operator", "wrong")],
            body.clone(),
        )
        .await?;
    assert_eq!(answer.0, 401, "{}", answer.1);
    assert_eq!(answer.1["refusal"], "OperatorRefused");
    // Admission refused before consuming the valid signature's nonce.
    let accepted = table.send(&path, &header, body).await?;
    assert_eq!(accepted.0, 200, "{}", accepted.1);
    Ok(())
}

#[tokio::test]
async fn a_runtime_signature_covers_the_query_as_well_as_the_path() -> TestResult {
    let table = Table::set().await?;
    let path = table.path(&operation()?);
    let body = table.body()?;
    let header = table.header(&path, &body, now_ms()?, &nonce(44));
    refused(
        &table
            .send(&format!("{path}?scope=changed"), &header, body.clone())
            .await?,
        "does not verify",
    );
    let target = format!("{path}?scope=original");
    let header = table.header(&target, &body, now_ms()?, &nonce(45));
    let accepted = table.send(&target, &header, body).await?;
    assert_eq!(accepted.0, 200, "{}", accepted.1);
    Ok(())
}

#[tokio::test]
async fn a_goal_signature_covers_the_query_as_well_as_the_path() -> TestResult {
    let table = Table::set().await?;
    let path = format!("/goals/{}/mark", operation()?);
    let body = json!({"operation": operation()?, "standing": "met", "words": "completed"})
        .to_string()
        .into_bytes();
    let header = table.header(&path, &body, now_ms()?, &nonce(46));
    refused(
        &table
            .send(&format!("{path}?scope=changed"), &header, body.clone())
            .await?,
        "does not verify",
    );
    let target = format!("{path}?scope=original");
    let header = table.header(&target, &body, now_ms()?, &nonce(47));
    let answer = table.send(&target, &header, body).await?;
    assert_eq!(answer.1["refusal"], "goal_unknown", "{}", answer.1);
    Ok(())
}

#[tokio::test]
async fn an_expired_certificate_cannot_sign_a_fresh_request() -> TestResult {
    let table = Table::set_certificate(Some(include_bytes!("fixtures/expired-agent.der"))).await?;
    let path = format!("/goals/{}/mark", operation()?);
    let body = json!({"operation": operation()?, "standing": "met", "words": "completed"})
        .to_string()
        .into_bytes();
    let header = table.header(&path, &body, now_ms()?, &nonce(48));
    let answer = table.send(&path, &header, body).await?;
    refused(&answer, "does not verify");
    Ok(())
}
