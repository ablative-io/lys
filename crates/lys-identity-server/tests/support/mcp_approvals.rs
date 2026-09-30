use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{Actor, AuthMethod, IdentityId, OperationId, Profile, Provenance, Transition};
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::provisioning_store::{ProvisioningStore, Version};
use lys_identity_server::routes::open_directory;
use lys_identity_server::session::now;
use serde_json::{Value, json};

pub type TestResult = Result<(), Box<dyn Error>>;
const HOLDER: &str = "approval-holder";

pub struct Fixture {
    pub service: Service,
    pub administrator: String,
    pub lead: String,
    signer: Option<Certified>,
    pub target: String,
    profiles: PathBuf,
}

#[derive(Clone, Copy)]
pub enum Scenario {
    Lead,
    LatestReviewed,
    Narrow,
    Outside,
    Person,
    Unsigned,
}

pub fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

pub fn server(url: &str) -> Value {
    json!({"name": "dot", "url": url, "channel": "wake"})
}

pub fn approval(operation: &str) -> Value {
    json!({"operation": operation, "note": "reviewed server access"})
}

pub async fn post(
    service: &Service,
    path: &str,
    cookie: &str,
    body: &Value,
    expected: u16,
) -> Result<Value, Box<dyn Error>> {
    let (status, answer) = service.post(path, Some(cookie), body).await?;
    assert_eq!(status, expected, "POST {path}: {answer}");
    Ok(answer)
}

impl Fixture {
    pub async fn new(scenario: Scenario) -> Result<Self, Box<dyn Error>> {
        let (service, (seeded, approver, profiles)) = Service::start_with(move |config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, HOLDER])?;
            let mut approver = seeded.people[1].agents[0].id;
            if matches!(scenario, Scenario::Narrow) {
                let mut directory = open_directory(config)?;
                let actor = Actor::new(
                    config.administrator_binding()?,
                    Provenance::new(AuthMethod::Oidc, now()),
                );
                (approver, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    seeded.people[1].id,
                    Profile::new("approval approver")?,
                    now(),
                )?;
                directory.transition(
                    actor,
                    OperationId::generate()?,
                    IdentityId::Agent(approver),
                    Transition::Activate,
                    "",
                    now(),
                )?;
            }
            Ok((
                seeded,
                approver,
                config.provisioning_file.clone().ok_or("no profiles path")?,
            ))
        })
        .await?;
        let approver = approver.to_string();
        let administrator = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        let signer = if matches!(scenario, Scenario::Person | Scenario::Unsigned) {
            None
        } else {
            let holder = service
                .sign_in(Login {
                    subject: HOLDER.to_owned(),
                    email: "holder@example.test".to_owned(),
                })
                .await?;
            Some(Certified::enroll(&service, &holder, &approver).await?)
        };
        let fixture = Self {
            target: seeded.people[0].agents[0].id.to_string(),
            lead: seeded.people[1].agents[0].id.to_string(),
            service,
            administrator,
            signer,
            profiles,
        };
        fixture.version(&fixture.target, 0, json!([]), true).await?;
        match scenario {
            Scenario::Lead | Scenario::LatestReviewed | Scenario::Unsigned => {
                fixture
                    .version(
                        &fixture.lead,
                        0,
                        json!([server("https://lead.example.test/mcp")]),
                        true,
                    )
                    .await?;
                if matches!(scenario, Scenario::LatestReviewed) {
                    fixture
                        .version(
                            &fixture.lead,
                            1,
                            json!([server("https://unreviewed.example.test/mcp")]),
                            false,
                        )
                        .await?;
                }
            }
            Scenario::Narrow => {
                fixture
                    .version(
                        &fixture.lead,
                        0,
                        json!([{"name": "waffles", "url": "https://waffles.example.test/mcp"}]),
                        true,
                    )
                    .await?;
                fixture.version(&approver, 0, json!([]), true).await?;
            }
            Scenario::Outside | Scenario::Person => {
                fixture
                    .version(
                        &fixture.lead,
                        0,
                        json!([
                            server("https://estate.example.test/mcp"),
                            {"name": "waffles", "url": "https://waffles.example.test/mcp"}
                        ]),
                        true,
                    )
                    .await?;
                fixture
                    .version(
                        &fixture.lead,
                        1,
                        json!([server("https://estate.example.test/mcp")]),
                        true,
                    )
                    .await?;
            }
        }
        if matches!(
            scenario,
            Scenario::Lead | Scenario::LatestReviewed | Scenario::Narrow
        ) {
            let parent = operation()?;
            post(
                &fixture.service,
                "/teams",
                &fixture.administrator,
                &json!({"operation": parent, "name": "approval team"}),
                200,
            )
            .await?;
            post(
                &fixture.service,
                &format!("/teams/{parent}/members"),
                &fixture.administrator,
                &json!({"operation": operation()?, "member": approver}),
                200,
            )
            .await?;
            post(
                &fixture.service,
                &format!("/teams/{parent}/nesting"),
                &fixture.administrator,
                &json!({"operation": operation()?, "parent": null, "lead": approver}),
                200,
            )
            .await?;
            let team = if matches!(scenario, Scenario::Narrow) {
                parent
            } else {
                let child = operation()?;
                post(
                    &fixture.service,
                    "/teams",
                    &fixture.administrator,
                    &json!({"operation": child, "name": "approval child", "parent": parent}),
                    200,
                )
                .await?;
                child
            };
            post(
                &fixture.service,
                &format!("/teams/{team}/members"),
                &fixture.administrator,
                &json!({"operation": operation()?, "member": fixture.target}),
                200,
            )
            .await?;
        }
        Ok(fixture)
    }

    pub fn signer(&self) -> Result<&Certified, Box<dyn Error>> {
        self.signer
            .as_ref()
            .ok_or_else(|| "scenario has no certified approver".into())
    }

    pub async fn version(
        &self,
        agent: &str,
        from: u32,
        servers: Value,
        review: bool,
    ) -> TestResult {
        let path = format!("/agents/{agent}/provisioning");
        post(
            &self.service,
            &path,
            &self.administrator,
            &json!({
                "operation": operation()?, "from_version": from,
                "model_access": ["primary-model"], "tools": [], "skills": [],
                "mcp_servers": servers, "instructions": "Keep reviewed instructions.", "note": "",
                "harness": crate::harness_description::declared(),
            }),
            200,
        )
        .await?;
        if review {
            post(
                &self.service,
                &format!("{path}/{}/review", from + 1),
                &self.administrator,
                &json!({"operation": operation()?}),
                200,
            )
            .await?;
        }
        Ok(())
    }

    pub fn versions(&self) -> Result<Vec<Version>, Box<dyn Error>> {
        let store = ProvisioningStore::open(&self.profiles)?;
        Ok(store
            .profile(&self.target)
            .ok_or("no target profile")?
            .versions
            .clone())
    }

    pub async fn ask(&self, server: &str) -> Result<Value, Box<dyn Error>> {
        post(
            &self.service,
            &format!("/agents/{}/mcp-requests", self.target),
            &self.administrator,
            &json!({"operation": operation()?, "server": server}),
            200,
        )
        .await
    }

    pub fn approval_path(&self, request: &str) -> String {
        format!("/agents/{}/mcp-requests/{request}/approve", self.target)
    }

    pub async fn approve(
        &self,
        request: &str,
        approver: &Certified,
        body: &Value,
        expected: u16,
    ) -> Result<Value, Box<dyn Error>> {
        let path = self.approval_path(request);
        let bytes = serde_json::to_vec(body)?;
        let signature = approver.signature(&path, &bytes)?;
        let (status, answer) = self
            .service
            .post_signed(&path, (HEADER, &signature), bytes)
            .await?;
        assert_eq!(status, expected, "POST {path}: {answer}");
        Ok(answer)
    }

    pub async fn approve_person(
        &self,
        request: &str,
        body: &Value,
    ) -> Result<Value, Box<dyn Error>> {
        post(
            &self.service,
            &self.approval_path(request),
            &self.administrator,
            body,
            200,
        )
        .await
    }
}

pub struct Certified {
    pub agent: String,
    key: Arc<Ed25519Identity>,
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

impl Certified {
    async fn enroll(service: &Service, holder: &str, agent: &str) -> Result<Self, Box<dyn Error>> {
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join(format!("{agent}.key")),
        )?);
        post(service, &format!("/agents/{agent}/certificates"), holder,
            &json!({"operation": operation()?, "request": STANDARD.encode(create_certificate_request(&key, agent)?)}), 200).await?;
        Ok(Self {
            agent: agent.to_owned(),
            key,
        })
    }

    pub fn signature(&self, path: &str, body: &[u8]) -> Result<String, Box<dyn Error>> {
        let at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = hex(operation()?.as_bytes());
        let cose =
            sign_attestation(&payload("POST", path, body, at, &nonce), &self.key).to_cose_bytes();
        Ok(format!("{} {at} {nonce} {}", self.agent, hex(&cose)))
    }
}
