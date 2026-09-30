//! The person who holds the link-audit source login answers for every
//! link-audit request, whether they signed in themselves or their agent
//! signed the request. While that person is suspended, or once they are
//! retired, neither path is admitted: the request is refused `NotAdmitted`
//! naming the state, and nothing is recorded. A reinstated person answers
//! again.

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
use lys_identity::{AgentId, OperationId};
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::{Seeded, seed_configured};
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

fn nonce(seed: u8) -> String {
    hex(&[seed; 16])
}

/// A seeded service whose second person holds the configured link-audit
/// source login, that person's session and active certified agent, and the
/// administrator's session.
struct Table {
    service: Service,
    seeded: Seeded,
    administrator: String,
    source: String,
    agent: AgentId,
    key: Arc<Ed25519Identity>,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_with(|config| {
            Ok(seed_configured(config, [ADMINISTRATOR, LINK_AUDIT_SOURCE])?)
        })
        .await?;
        let administrator = service.sign_in(login(ADMINISTRATOR)).await?;
        let source = service.sign_in(login(LINK_AUDIT_SOURCE)).await?;
        let agent = seeded.people[1].agents[0].id;
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join(format!("{agent}.key")),
        )?);
        let asked = json!({
            "operation": OperationId::generate()?.to_string(),
            "request": STANDARD.encode(create_certificate_request(&key, &agent.to_string())?),
        });
        let (status, issued) = service
            .post(
                &format!("/agents/{agent}/certificates"),
                Some(&source),
                &asked,
            )
            .await?;
        assert_eq!(status, 200, "{issued}");
        Ok(Self {
            service,
            seeded,
            administrator,
            source,
            agent,
            key,
        })
    }

    /// The administrator moves the holder of the source login by `transition`.
    async fn moved(&self, transition: &str) -> TestResult {
        let asked = json!({
            "operation": OperationId::generate()?.to_string(),
            "transition": transition,
            "reason": "link-audit standing test",
        });
        let (status, body) = self
            .service
            .post(
                &format!("/identities/{}/transitions", self.seeded.people[1].id),
                Some(&self.administrator),
                &asked,
            )
            .await?;
        assert_eq!(status, 200, "{body}");
        Ok(())
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

    /// `body` posted to `path`, signed by the holder's agent under `nonce`.
    async fn signed(
        &self,
        path: &str,
        body: Vec<u8>,
        nonce: &str,
    ) -> Result<Answer, Box<dyn Error>> {
        let signed_at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let cose = sign_attestation(&payload("POST", path, &body, signed_at, nonce), &self.key)
            .to_cose_bytes();
        let header = format!("{} {signed_at} {nonce} {}", self.agent, hex(&cose));
        self.service
            .post_signed(path, (HEADER, &header), body)
            .await
    }

    /// `body` posted to `path` under the holder's own session.
    async fn in_session(&self, path: &str, body: Vec<u8>) -> Result<Answer, Box<dyn Error>> {
        let carried = [("cookie", self.source.as_str())];
        self.service.post_carrying(path, &carried, body).await
    }

    /// How many events the directory log holds.
    async fn recorded(&self) -> Result<u64, Box<dyn Error>> {
        let (status, first) = self.service.get("/receipts/0", None).await?;
        assert_eq!(status, 200, "{first}");
        Ok(first["checkpoint"]["tree_size"]
            .as_u64()
            .ok_or("the checkpoint names no tree size")?)
    }

    /// Both routes by both paths are refused, each naming `state`, and
    /// nothing is recorded.
    async fn refuses_all(&self, state: &str, seed: u8) -> TestResult {
        let before = self.recorded().await?;
        let signed_delivery = self
            .signed(DELIVER, self.delivery("op-1"), &nonce(seed))
            .await?;
        let signed_question = self.signed(ASK, self.question(), &nonce(seed + 1)).await?;
        let session_delivery = self.in_session(DELIVER, self.delivery("op-1")).await?;
        let session_question = self.in_session(ASK, self.question()).await?;
        let answers = [
            signed_delivery,
            signed_question,
            session_delivery,
            session_question,
        ];
        for answer in answers {
            assert_eq!(answer.0, 403, "{}", answer.1);
            assert_eq!(answer.1["refusal"], "NotAdmitted", "{}", answer.1);
            assert!(answer.1.get("receipt").is_none(), "{}", answer.1);
            assert!(answer.1.get("person").is_none(), "{}", answer.1);
            let said = answer.1.to_string();
            assert!(said.contains(state), "{said}");
            assert!(said.contains("act: "), "{said}");
        }
        assert_eq!(self.recorded().await?, before);
        Ok(())
    }
}

#[tokio::test]
async fn a_suspended_holder_answers_for_nothing_until_reinstated() -> TestResult {
    let table = Table::set().await?;
    table.moved("suspend").await?;
    table.refuses_all("suspended", 40).await?;

    table.moved("reinstate").await?;
    let answer = table
        .signed(DELIVER, table.delivery("op-1"), &nonce(50))
        .await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let answer: Answer = table.in_session(ASK, table.question()).await?;
    assert_eq!(answer.0, 200, "{}", answer.1);
    let expected: Value = json!({ "person": table.seeded.people[0].id.to_string() });
    assert_eq!(answer.1, expected);
    Ok(())
}

#[tokio::test]
async fn a_retired_holder_answers_for_nothing() -> TestResult {
    let table = Table::set().await?;
    table.moved("retire").await?;
    table.refuses_all("retired", 60).await
}
