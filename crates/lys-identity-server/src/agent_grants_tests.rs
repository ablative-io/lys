//! Agent grant reads authenticate the agent and never borrow a session.
use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

struct Fixture {
    service: Service,
    cookie: String,
    agent: String,
    other: String,
    person: String,
    key: Arc<Ed25519Identity>,
}

impl Fixture {
    async fn fresh() -> TestResult<Self> {
        let (service, (agent, other, person, key)) = Service::start_with(|config| {
            lys_log_store::FileLeafStore::create(&config.log_dir, &config.log_origin)?;
            let path = config.log_dir.clone();
            let key = Ed25519Identity::load(&config.event_key_file)?;
            lys_identity::directory_migration::migrate(
                lys_log_store::FileLeafStore::open(&path)?,
                &key,
            )?;
            let mut directory = Directory::open(
                Box::new(move || lys_log_store::FileLeafStore::open(&path)),
                key,
            )?;
            let actor = Actor::new(
                LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                Provenance::new(AuthMethod::Oidc, 1),
            );
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
                Profile::new("Caller")?,
                2,
            )?;
            let (other, _) = directory.register_agent(
                actor.clone(),
                OperationId::generate()?,
                person,
                Profile::new("Other")?,
                2,
            )?;
            for id in [agent, other] {
                directory.transition(
                    actor.clone(),
                    OperationId::generate()?,
                    IdentityId::Agent(id),
                    Transition::Activate,
                    "",
                    3,
                )?;
            }
            Ok((
                agent.to_string(),
                other.to_string(),
                person.to_string(),
                Arc::new(Ed25519Identity::load(&config.event_key_file)?),
            ))
        })
        .await?;
        let cookie = service
            .sign_in(identity_contract::apps::login(ADMINISTRATOR))
            .await?;
        let request = lys_core::ca::create_certificate_request(&key, &agent)?;
        let (status, body) = service
            .post(
                &format!("/agents/{agent}/certificates"),
                Some(&cookie),
                &json!({
                    "operation":OperationId::generate()?.to_string(),
                    "request":base64::engine::general_purpose::STANDARD.encode(request)
                }),
            )
            .await?;
        assert_eq!(status, 200, "{body}");
        Ok(Self {
            service,
            cookie,
            agent,
            other,
            person,
            key,
        })
    }

    async fn signed(&self, cookie: bool) -> TestResult<(u16, Value)> {
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = crate::routes::hex(&Sha256::digest(
            OperationId::generate()?.to_string().as_bytes(),
        ));
        let payload = crate::agent_signature::payload("GET", "/agent/grants", &[], at, &nonce);
        let signature = crate::routes::hex(
            &lys_core::attestation::sign_attestation(&payload, &self.key).to_cose_bytes(),
        );
        let mut request = reqwest::Client::new()
            .get(format!("{}/agent/grants", self.service.base))
            .header(
                crate::agent_signature::HEADER,
                format!("{} {at} {nonce} {signature}", self.agent),
            );
        if cookie {
            request = request.header("cookie", &self.cookie);
        }
        let response = request.send().await?;
        Ok((response.status().as_u16(), response.json().await?))
    }

    async fn grant(&self, recipient: &str, starts: u64) -> TestResult<String> {
        let (status, root) = self.service.post("/grants/roots", Some(&self.cookie), &json!({
            "operation":OperationId::generate()?.to_string(), "route":"api", "holder":self.person,
            "resource":{"kind":"directory","id":"people"}, "relation":"alpha",
            "pass_on":{"kind":"to","actions":["read","write"],"recipients":["agent"]},
            "window":{"starts_at":0,"ends_at":null}
        })).await?;
        assert_eq!(status, 200, "{root}");
        let (status, given) = self.service.post("/grants", Some(&self.cookie), &json!({
            "operation":OperationId::generate()?.to_string(), "route":"api", "source":root["grant"],
            "recipient":recipient, "responsible":self.person,
            "resource":{"kind":"directory","id":"people"}, "relation":"alpha",
            "pass_on":{"kind":"use_only"}, "window":{"starts_at":starts,"ends_at":null}
        })).await?;
        assert_eq!(status, 200, "{given}");
        Ok(given["grant"].as_str().ok_or("no grant id")?.to_owned())
    }
}

#[tokio::test]
async fn an_agent_reads_only_its_own_live_grants_without_recording_a_use() -> TestResult {
    let fixture = Fixture::fresh().await?;
    let own = fixture.grant(&fixture.agent, 0).await?;
    fixture.grant(&fixture.other, 0).await?;
    fixture
        .grant(&fixture.agent, crate::session::now() + 3600)
        .await?;
    let revoked = fixture.grant(&fixture.agent, 0).await?;
    let (status, body) = fixture
        .service
        .post(
            &format!("/grants/{revoked}/revoke"),
            Some(&fixture.cookie),
            &json!({
                "operation":OperationId::generate()?.to_string(), "route":"api", "reason":"ended"
            }),
        )
        .await?;
    assert_eq!(status, 200, "{body}");
    let (_, before) = fixture
        .service
        .get("/grants", Some(&fixture.cookie))
        .await?;
    let (status, body) = fixture.signed(false).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["agent"], fixture.agent);
    let grants = body["grants"].as_array().ok_or("no grants")?;
    assert_eq!(grants.len(), 1, "{body}");
    assert_eq!(grants[0]["grant"], own);
    assert_eq!(
        grants[0]["resource"],
        json!({"kind":"directory","id":"people"})
    );
    assert_eq!(grants[0]["actions"], json!(["read", "write"]));
    assert_eq!(grants[0]["responsible"], fixture.person);
    assert_eq!(grants[0]["window"], json!({"starts_at":0,"ends_at":null}));
    let (_, after) = fixture
        .service
        .get("/grants", Some(&fixture.cookie))
        .await?;
    assert_eq!(before["revision"], after["revision"]);
    assert_eq!(before["grants"], after["grants"]);
    Ok(())
}

#[tokio::test]
async fn an_agent_grant_read_refuses_a_signature_with_an_administrator_cookie() -> TestResult {
    let fixture = Fixture::fresh().await?;
    let (status, body) = fixture.signed(true).await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "AgentSignatureRefused");
    assert!(
        body["reason"]
            .as_str()
            .ok_or("no reason")?
            .contains("cookie")
    );
    let (status, body) = fixture
        .service
        .get("/agent/grants", Some(&fixture.cookie))
        .await?;
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["refusal"], "AgentSignatureRefused");
    Ok(())
}
