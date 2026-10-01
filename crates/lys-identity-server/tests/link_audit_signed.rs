//! The link-audit source may sign its requests with an agent's key in place
//! of a browser session. The agent is admitted when the person responsible
//! for it holds the configured link-audit source login; the event's actor is
//! that person, named by that login, by the method agent signature, and the
//! event keeps the agent's id. An agent of anyone else is refused
//! `NotAdmitted`, a signature that does not stand is refused
//! `AgentSignatureRefused`, and a refused request records nothing. A request
//! carrying a session and a signature is judged by the signature.

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Answer, LINK_AUDIT_SOURCE, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::{
    AgentId, AuthMethod, Change, IdentityEvent, IdentityId, LoginBinding, OperationId, verify_event,
};
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::{Seeded, SeededPerson, seed_configured};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const DELIVER: &str = "/link-audit";
const ASK: &str = "/link-audit/person";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
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

fn unhex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    (0..text.len())
        .step_by(2)
        .map(|at| {
            let pair = text.get(at..at + 2).ok_or("an odd number of hex digits")?;
            Ok(u8::from_str_radix(pair, 16)?)
        })
        .collect()
}

fn nonce(seed: u8) -> String {
    hex(&[seed; 16])
}

fn now_ms() -> Result<u64, Box<dyn Error>> {
    Ok(u64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
    )?)
}

fn refused(answer: &Answer, status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
    assert!(answer.1.get("receipt").is_none(), "{}", answer.1);
    assert!(answer.1.get("person").is_none(), "{}", answer.1);
}

/// An active agent holding a certificate over its own key.
struct Signer {
    agent: AgentId,
    key: Arc<Ed25519Identity>,
    serial: String,
    /// The session of the person responsible for the agent.
    responsible: String,
}

impl Signer {
    /// The first agent of `person`, certified over a fresh key as the person
    /// signed in through `subject` asks.
    async fn certified(
        service: &Service,
        person: &SeededPerson,
        subject: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let agent = person.agents[0].id;
        let responsible = service.sign_in(login(subject)).await?;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join(format!("{agent}.key")),
        )?);
        let serial = OperationId::generate()?.to_string();
        let asked = json!({
            "operation": serial,
            "request": STANDARD.encode(create_certificate_request(&key, &agent.to_string())?),
        });
        let (status, issued) = service
            .post(
                &format!("/agents/{agent}/certificates"),
                Some(&responsible),
                &asked,
            )
            .await?;
        assert_eq!(status, 200, "{issued}");
        Ok(Self {
            agent,
            key,
            serial,
            responsible,
        })
    }

    /// The header signing `body` for a POST to `path` under `nonce`.
    fn header(&self, path: &str, body: &[u8], nonce: &str) -> Result<String, Box<dyn Error>> {
        let signed_at = now_ms()?;
        let signed = payload("POST", path, body, signed_at, nonce);
        let cose = sign_attestation(&signed, &self.key).to_cose_bytes();
        Ok(format!("{} {signed_at} {nonce} {}", self.agent, hex(&cose)))
    }
}

/// A seeded service whose second person holds the configured link-audit
/// source login, the active agent of that person, and the active agent of
/// the first person, who does not hold it.
struct Table {
    service: Service,
    seeded: Seeded,
    admitted: Signer,
    other: Signer,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_with(|config| {
            Ok(seed_configured(config, [ADMINISTRATOR, LINK_AUDIT_SOURCE])?)
        })
        .await?;
        let other = Signer::certified(&service, &seeded.people[0], ADMINISTRATOR).await?;
        let admitted = Signer::certified(&service, &seeded.people[1], LINK_AUDIT_SOURCE).await?;
        Ok(Self {
            service,
            seeded,
            admitted,
            other,
        })
    }

    /// A delivery about the first person under `source_operation_id`.
    fn delivery(&self, source_operation_id: &str) -> Vec<u8> {
        json!({
            "person": self.seeded.people[0].id.to_string(),
            "source_operation_id": source_operation_id,
            "change": "linked",
            "issuer": "https://accounts.provider.test",
            "subject": "ada-elsewhere",
            "observer": self.service.issuer.issuer(),
            "observed_at": 1_790_000_200_u64,
        })
        .to_string()
        .into_bytes()
    }

    /// The question of who holds the first person's login.
    fn question(&self) -> Vec<u8> {
        json!({ "issuer": self.service.issuer.issuer(), "subject": ADMINISTRATOR })
            .to_string()
            .into_bytes()
    }

    async fn send(
        &self,
        signer: &Signer,
        path: &str,
        body: Vec<u8>,
        nonce: &str,
    ) -> Result<Answer, Box<dyn Error>> {
        let header = signer.header(path, &body, nonce)?;
        self.service
            .post_signed(path, (HEADER, &header), body)
            .await
    }

    /// How many events the directory log holds.
    async fn recorded(&self) -> Result<u64, Box<dyn Error>> {
        let (status, first) = self.service.get("/receipts/0", None).await?;
        assert_eq!(status, 200, "{first}");
        Ok(first["checkpoint"]["tree_size"]
            .as_u64()
            .ok_or("the checkpoint names no tree size")?)
    }

    /// The event the receipt in `answer` was given for, read back from the
    /// log and verified under the service's key.
    async fn event(&self, answer: &Value) -> Result<IdentityEvent, Box<dyn Error>> {
        let index = answer["receipt"]["log"]["index"]
            .as_u64()
            .ok_or("the receipt names no leaf")?;
        let (status, kept) = self
            .service
            .get(&format!("/receipts/{index}"), None)
            .await?;
        assert_eq!(status, 200, "{kept}");
        let (status, key) = self.service.get("/service-key", None).await?;
        assert_eq!(status, 200, "{key}");
        let key = unhex(key["ed25519"].as_str().ok_or("no service key")?)?;
        let key = <[u8; 32]>::try_from(key).map_err(|_short| "the service key is not 32 bytes")?;
        let message = unhex(kept["message"].as_str().ok_or("no signed message")?)?;
        Ok(verify_event(&message, &key)?.event()?.clone())
    }

    fn source_login(&self) -> Result<LoginBinding, Box<dyn Error>> {
        Ok(LoginBinding::new(
            self.service.issuer.issuer(),
            LINK_AUDIT_SOURCE,
        )?)
    }
}

#[tokio::test]
async fn a_delivery_signed_by_the_sources_agent_is_recorded_as_the_person_and_keeps_the_agent()
-> TestResult {
    let table = Table::set().await?;
    let before = table.recorded().await?;
    let (status, answer) = table
        .send(&table.admitted, DELIVER, table.delivery("op-1"), &nonce(1))
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer["receipt"]["actor"]["subject"], LINK_AUDIT_SOURCE,
        "{answer}"
    );
    assert_eq!(table.recorded().await?, before + 1);

    let event = table.event(&answer).await?;
    let actor = event.actor();
    assert_eq!(actor.binding(), &table.source_login()?);
    assert_eq!(
        actor.provenance().method(),
        AuthMethod::AgentSignature(table.admitted.agent)
    );
    assert_eq!(actor.provenance().agent(), Some(table.admitted.agent));
    assert_eq!(
        event.identity(),
        IdentityId::Person(table.seeded.people[0].id)
    );
    assert!(matches!(event.change(), Change::LinkAudit(_)));
    Ok(())
}

#[tokio::test]
async fn a_signed_delivery_from_an_agent_of_another_person_is_refused_and_records_nothing()
-> TestResult {
    let table = Table::set().await?;
    let before = table.recorded().await?;
    let answer = table
        .send(&table.other, DELIVER, table.delivery("op-1"), &nonce(2))
        .await?;
    refused(&answer, 403, "NotAdmitted");
    let said = answer.1.to_string();
    assert!(
        said.contains("does not hold the configured link-audit source login"),
        "{said}"
    );
    assert!(said.contains("act: "), "{said}");
    assert_eq!(table.recorded().await?, before);
    Ok(())
}

#[tokio::test]
async fn a_signed_delivery_with_another_body_is_refused_and_records_nothing() -> TestResult {
    let table = Table::set().await?;
    let before = table.recorded().await?;
    let header = table
        .admitted
        .header(DELIVER, &table.delivery("op-1"), &nonce(3))?;
    let answer = table
        .service
        .post_signed(DELIVER, (HEADER, &header), table.delivery("op-2"))
        .await?;
    refused(&answer, 401, "AgentSignatureRefused");
    assert_eq!(table.recorded().await?, before);
    Ok(())
}

#[tokio::test]
async fn a_signed_delivery_sent_again_under_its_nonce_is_refused_and_records_nothing_more()
-> TestResult {
    let table = Table::set().await?;
    let before = table.recorded().await?;
    let body = table.delivery("op-1");
    let header = table.admitted.header(DELIVER, &body, &nonce(4))?;
    let (status, answer) = table
        .service
        .post_signed(DELIVER, (HEADER, &header), body.clone())
        .await?;
    assert_eq!(status, 200, "{answer}");
    let again = table
        .service
        .post_signed(DELIVER, (HEADER, &header), body)
        .await?;
    refused(&again, 401, "AgentSignatureRefused");
    assert!(again.1.to_string().contains("already used"), "{}", again.1);
    assert_eq!(table.recorded().await?, before + 1);
    Ok(())
}

#[tokio::test]
async fn a_signed_delivery_under_a_withdrawn_certificate_is_refused_and_records_nothing()
-> TestResult {
    let table = Table::set().await?;
    let withdrawal = format!(
        "/agents/{}/certificates/{}/withdrawal",
        table.admitted.agent, table.admitted.serial
    );
    let (status, withdrawn) = table
        .service
        .post(
            &withdrawal,
            Some(&table.admitted.responsible),
            &json!({"reason": "the key left the machine"}),
        )
        .await?;
    assert_eq!(status, 200, "{withdrawn}");
    let before = table.recorded().await?;
    let answer = table
        .send(&table.admitted, DELIVER, table.delivery("op-1"), &nonce(5))
        .await?;
    refused(&answer, 401, "AgentSignatureRefused");
    assert_eq!(table.recorded().await?, before);
    Ok(())
}

#[tokio::test]
async fn the_question_is_answered_for_the_sources_agent_and_refused_for_another() -> TestResult {
    let table = Table::set().await?;
    let (status, answer) = table
        .send(&table.admitted, ASK, table.question(), &nonce(6))
        .await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer,
        json!({ "person": table.seeded.people[0].id.to_string() })
    );
    let answer = table
        .send(&table.other, ASK, table.question(), &nonce(7))
        .await?;
    refused(&answer, 403, "NotAdmitted");
    Ok(())
}

#[tokio::test]
async fn a_signature_made_for_one_route_is_refused_on_the_other() -> TestResult {
    let table = Table::set().await?;
    let body = table.question();
    let header = table.admitted.header(DELIVER, &body, &nonce(8))?;
    let answer = table
        .service
        .post_signed(ASK, (HEADER, &header), body)
        .await?;
    refused(&answer, 401, "AgentSignatureRefused");
    Ok(())
}

#[tokio::test]
async fn a_session_of_the_source_still_delivers_and_asks() -> TestResult {
    let table = Table::set().await?;
    let session = table.service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
    let delivery: Value = serde_json::from_slice(&table.delivery("op-1"))?;
    let (status, answer) = table
        .service
        .post(DELIVER, Some(&session), &delivery)
        .await?;
    assert_eq!(status, 200, "{answer}");
    let event = table.event(&answer).await?;
    assert_eq!(event.actor().binding(), &table.source_login()?);
    assert_eq!(event.actor().provenance().method(), AuthMethod::Oidc);
    assert_eq!(event.actor().provenance().agent(), None);

    let question: Value = serde_json::from_slice(&table.question())?;
    let (status, answer) = table.service.post(ASK, Some(&session), &question).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        answer,
        json!({ "person": table.seeded.people[0].id.to_string() })
    );
    Ok(())
}

#[tokio::test]
async fn a_request_carrying_a_session_and_a_signature_is_judged_by_the_signature() -> TestResult {
    let table = Table::set().await?;
    let session = table.service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
    let before = table.recorded().await?;

    let body = table.delivery("op-1");
    let header = table.other.header(DELIVER, &body, &nonce(9))?;
    let carried = [(HEADER, header.as_str()), ("cookie", session.as_str())];
    let answer = table.service.post_carrying(DELIVER, &carried, body).await?;
    refused(&answer, 403, "NotAdmitted");
    assert_eq!(table.recorded().await?, before);

    let body = table.delivery("op-1");
    let header = table.admitted.header(DELIVER, &body, &nonce(10))?;
    let carried = [
        (HEADER, header.as_str()),
        ("cookie", table.other.responsible.as_str()),
    ];
    let (status, answer) = table.service.post_carrying(DELIVER, &carried, body).await?;
    assert_eq!(status, 200, "{answer}");
    let event = table.event(&answer).await?;
    assert_eq!(
        event.actor().provenance().agent(),
        Some(table.admitted.agent)
    );
    Ok(())
}

#[tokio::test]
async fn a_request_in_other_words_is_refused_as_malformed_by_either_path() -> TestResult {
    let table = Table::set().await?;
    let session = table.service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
    let before = table.recorded().await?;
    let mut unknown: Value = serde_json::from_slice(&table.delivery("op-1"))?;
    unknown["email"] = json!("shared@example.test");
    let bodies = [
        unknown.to_string().into_bytes(),
        br#"{"person": "#.to_vec(),
        br#"{"source_operation_id": "op-1"}"#.to_vec(),
    ];
    for (seed, body) in (20_u8..).zip(bodies) {
        let answer = table
            .send(&table.admitted, DELIVER, body.clone(), &nonce(seed))
            .await?;
        refused(&answer, 400, "RequestMalformed");
        let carried = [("cookie", session.as_str())];
        let answer = table.service.post_carrying(DELIVER, &carried, body).await?;
        refused(&answer, 400, "RequestMalformed");
    }
    let answer = table
        .send(&table.admitted, ASK, br#"{"issuer": "#.to_vec(), &nonce(30))
        .await?;
    refused(&answer, 400, "RequestMalformed");
    assert_eq!(table.recorded().await?, before);
    Ok(())
}
