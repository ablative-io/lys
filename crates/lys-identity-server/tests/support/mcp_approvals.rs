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
    pub lead_auth: Certified,
    pub narrow: String,
    pub narrow_auth: Certified,
    pub outside: String,
    pub outside_auth: Certified,
    pub target: String,
    profiles: PathBuf,
    client: reqwest::Client,
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
    client: &reqwest::Client,
    service: &Service,
    path: &str,
    cookie: &str,
    body: &Value,
    expected: u16,
) -> Result<Value, Box<dyn Error>> {
    let response = client
        .post(format!("{}{path}", service.base))
        .header(reqwest::header::COOKIE, cookie)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await?;
    let status = response.status().as_u16();
    let text = response.text().await?;
    assert_eq!(status, expected, "POST {path}: {text}");
    assert!(!text.is_empty(), "POST {path} must answer a JSON record");
    Ok(serde_json::from_str(&text)?)
}

impl Fixture {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let client = reqwest::Client::new();
        let (service, (seeded, narrow, outside, profiles)) = Service::start_with(|config| {
            let seeded = seed_configured(config, [ADMINISTRATOR, HOLDER])?;
            let mut directory = open_directory(config)?;
            let actor = Actor::new(
                config.administrator_binding()?,
                Provenance::new(AuthMethod::Oidc, now()),
            );
            let mut agents = Vec::new();
            for name in ["narrow approver", "outside approver"] {
                let (agent, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    seeded.people[1].id,
                    Profile::new(name)?,
                    now(),
                )?;
                directory.transition(
                    actor.clone(),
                    OperationId::generate()?,
                    IdentityId::Agent(agent),
                    Transition::Activate,
                    "",
                    now(),
                )?;
                agents.push(agent);
            }
            Ok((
                seeded,
                agents[0],
                agents[1],
                config.provisioning_file.clone().ok_or("no profiles path")?,
            ))
        })
        .await?;
        let administrator = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        let holder = service
            .sign_in(Login {
                subject: HOLDER.to_owned(),
                email: "holder@example.test".to_owned(),
            })
            .await?;
        let lead_auth = Certified::enroll(
            &client,
            &service,
            &holder,
            &seeded.people[1].agents[0].id.to_string(),
        )
        .await?;
        let narrow_auth =
            Certified::enroll(&client, &service, &holder, &narrow.to_string()).await?;
        let outside_auth =
            Certified::enroll(&client, &service, &holder, &outside.to_string()).await?;
        let fixture = Self {
            target: seeded.people[0].agents[0].id.to_string(),
            lead: seeded.people[1].agents[0].id.to_string(),
            narrow: narrow.to_string(),
            outside: outside.to_string(),
            service,
            administrator,
            lead_auth,
            narrow_auth,
            outside_auth,
            profiles,
            client,
        };
        fixture.version(&fixture.target, 0, json!([]), true).await?;
        fixture
            .version(
                &fixture.lead,
                0,
                json!([server("https://lead.example.test/mcp")]),
                true,
            )
            .await?;
        fixture
            .version(
                &fixture.lead,
                1,
                json!([server("https://unreviewed.example.test/mcp")]),
                false,
            )
            .await?;
        fixture.version(&fixture.narrow, 0, json!([]), true).await?;
        fixture
            .version(
                &fixture.outside,
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
                &fixture.outside,
                1,
                json!([server("https://estate.example.test/mcp")]),
                true,
            )
            .await?;
        let parent = operation()?;
        let child = operation()?;
        for (team, under, lead) in [
            (&parent, None, &fixture.lead),
            (&child, Some(parent.as_str()), &fixture.narrow),
        ] {
            post(
                &fixture.client,
                &fixture.service,
                "/teams",
                &fixture.administrator,
                &json!({"operation": team, "name": "approval team", "parent": under}),
                200,
            )
            .await?;
            post(
                &fixture.client,
                &fixture.service,
                &format!("/teams/{team}/members"),
                &fixture.administrator,
                &json!({"operation": operation()?, "member": lead}),
                200,
            )
            .await?;
            post(
                &fixture.client,
                &fixture.service,
                &format!("/teams/{team}/nesting"),
                &fixture.administrator,
                &json!({"operation": operation()?, "parent": under, "lead": lead}),
                200,
            )
            .await?;
        }
        post(
            &fixture.client,
            &fixture.service,
            &format!("/teams/{child}/members"),
            &fixture.administrator,
            &json!({"operation": operation()?, "member": fixture.target}),
            200,
        )
        .await?;
        Ok(fixture)
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
            &self.client,
            &self.service,
            &path,
            &self.administrator,
            &json!({
                "operation": operation()?, "from_version": from,
                "model_access": ["primary-model"], "tools": [], "skills": [],
                "mcp_servers": servers, "instructions": "Keep reviewed instructions.", "note": "",
                "harness": crate::harness_description::declared(),
                "permissions": {"default_mode": "plan"},
            }),
            200,
        )
        .await?;
        if review {
            post(
                &self.client,
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
            &self.client,
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
        let response = self
            .client
            .post(format!("{}{path}", self.service.base))
            .header(HEADER, signature)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(bytes)
            .send()
            .await?;
        let status = response.status().as_u16();
        let text = response.text().await?;
        assert_eq!(status, expected, "POST {path}: {text}");
        assert!(!text.is_empty(), "POST {path} must answer a JSON record");
        Ok(serde_json::from_str(&text)?)
    }

    pub async fn approve_person(
        &self,
        request: &str,
        body: &Value,
    ) -> Result<Value, Box<dyn Error>> {
        post(
            &self.client,
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
    agent: String,
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
    async fn enroll(
        client: &reqwest::Client,
        service: &Service,
        holder: &str,
        agent: &str,
    ) -> Result<Self, Box<dyn Error>> {
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join(format!("{agent}.key")),
        )?);
        post(client, service, &format!("/agents/{agent}/certificates"), holder,
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
