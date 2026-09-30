use std::error::Error;
use std::path::PathBuf;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::provisioning_store::{ProvisioningStore, Version};
use lys_identity_server::routes::open_directory;
use lys_identity_server::session::{Sessions, now};
use serde_json::{Value, json};

pub type TestResult = Result<(), Box<dyn Error>>;
const HOLDER: &str = "approval-holder";

pub struct Fixture {
    pub service: Service,
    pub administrator: String,
    pub lead: String,
    pub lead_cookie: String,
    pub narrow: String,
    pub narrow_cookie: String,
    pub outside: String,
    pub outside_cookie: String,
    pub target: String,
    profiles: PathBuf,
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
    let response = reqwest::Client::new()
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
        let (service, (seeded, narrow, outside, cookies, profiles)) =
            Service::start_with(|config| {
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
                let sessions = Sessions::open(
                    config.sessions_file.clone().ok_or("no sessions path")?,
                    config.session_seconds,
                    config.secure_cookie,
                )?;
                let agent_cookie = |agent: AgentId| {
                    sessions.begin(Actor::new(
                        LoginBinding::new(&config.issuer, HOLDER)?,
                        Provenance::by_agent(agent, now()),
                    ))
                };
                let cookies = [
                    agent_cookie(seeded.people[1].agents[0].id)?,
                    agent_cookie(agents[0])?,
                    agent_cookie(agents[1])?,
                ];
                Ok((
                    seeded,
                    agents[0],
                    agents[1],
                    cookies,
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
        let [lead_cookie, narrow_cookie, outside_cookie] = cookies;
        let fixture = Self {
            target: seeded.people[0].agents[0].id.to_string(),
            lead: seeded.people[1].agents[0].id.to_string(),
            narrow: narrow.to_string(),
            outside: outside.to_string(),
            service,
            administrator,
            lead_cookie,
            narrow_cookie,
            outside_cookie,
            profiles,
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
                &fixture.service,
                "/teams",
                &fixture.administrator,
                &json!({"operation": team, "name": "approval team", "parent": under}),
                200,
            )
            .await?;
            post(
                &fixture.service,
                &format!("/teams/{team}/members"),
                &fixture.administrator,
                &json!({"operation": operation()?, "member": lead}),
                200,
            )
            .await?;
            post(
                &fixture.service,
                &format!("/teams/{team}/nesting"),
                &fixture.administrator,
                &json!({"operation": operation()?, "parent": under, "lead": lead}),
                200,
            )
            .await?;
        }
        post(
            &fixture.service,
            &format!("/teams/{child}/members"),
            &fixture.administrator,
            &json!({"operation": operation()?, "member": fixture.target}),
            200,
        )
        .await?;
        Ok(fixture)
    }

    async fn version(&self, agent: &str, from: u32, servers: Value, review: bool) -> TestResult {
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

    pub async fn approve(
        &self,
        request: &str,
        cookie: &str,
        body: &Value,
        expected: u16,
    ) -> Result<Value, Box<dyn Error>> {
        post(
            &self.service,
            &format!("/agents/{}/mcp-requests/{request}/approve", self.target),
            cookie,
            body,
            expected,
        )
        .await
    }
}
