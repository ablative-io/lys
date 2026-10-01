use std::error::Error;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};

use crate::roles_records::{Holding, Role, Version, Words};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MODEL: &str = r#"{"version":1,"relations":{"reader":["read"],"creator":["role.create"],"reviser":["role.revise"],"assigner":["role.holder.assign"],"mover":["role.holder.move"],"ender":["role.holder.end"]}}"#;

pub(super) fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

pub(super) fn version_body() -> TestResult<Value> {
    Ok(json!({
        "operation":operation()?, "responsibilities":"Record decisions", "goals":"Accurate records",
        "practice":"Check the record", "profile":"", "grant_templates":[], "note":"Revision"
    }))
}

pub(super) struct Table {
    pub(super) service: Service,
    pub(super) agent: String,
    pub(super) role: String,
    pub(super) assignment: String,
    person: String,
    pass: String,
    cookie: String,
    client: reqwest::Client,
}

impl Table {
    pub(super) async fn fresh(holding: bool) -> TestResult<Self> {
        let (service, (agent, person, role, assignment, pass)) = Service::start_adjusted(
            MODEL,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.certificates_dir = None;
                config.network_file = None;
                config.provisioning_file = None;
                config.runtime_dir = None;
                config.service_accounts_dir = None;
                config.teams_dir = None;
                config.stops_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
                config.homes_dir = None;
            },
            |config| {
                FileLeafStore::create(&config.log_dir, &config.log_origin)?;
                let path = config.log_dir.clone();
                let mut directory = Directory::open(
                    Box::new(move || FileLeafStore::open(&path)),
                    Ed25519Identity::load(&config.event_key_file)?,
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
                    Profile::new("Recorder")?,
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
                let role = OperationId::from_bytes([8; 16]).to_string();
                let assignment = OperationId::from_bytes([9; 16]).to_string();
                let words = Words {
                    responsibilities: "Record decisions".to_owned(),
                    goals: "Accurate records".to_owned(),
                    practice: "Check the record".to_owned(),
                    profile: String::new(),
                    grant_templates: Vec::new(),
                    note: "Initial version".to_owned(),
                };
                let held = if holding {
                    vec![Holding {
                        operation: assignment.clone(),
                        holder: agent.to_string(),
                        version: 1,
                        assigned_by: person.to_string(),
                        assigned_at: 2,
                        ends_at: None,
                        moves: Vec::new(),
                        ended: None,
                    }]
                } else {
                    Vec::new()
                };
                let kept = Role {
                    id: role.clone(),
                    name: "Recorder".to_owned(),
                    versions: vec![
                        Version {
                            number: 1,
                            operation: role.clone(),
                            words: words.clone(),
                            made_by: person.to_string(),
                            made_at: 1,
                        },
                        Version {
                            number: 2,
                            operation: OperationId::from_bytes([10; 16]).to_string(),
                            words,
                            made_by: person.to_string(),
                            made_at: 3,
                        },
                    ],
                    holdings: held,
                };
                std::fs::write(
                    config.roles_file.as_deref().ok_or("roles disabled")?,
                    serde_json::to_vec(&json!({"roles":[kept]}))?,
                )?;
                let pass = crate::agent_pass_store::Passes::open(
                    config.log_dir.with_file_name("agent-passes.json"),
                )?
                .issue(agent, "role-launch", "role-session")?
                .to_string();
                Ok((
                    agent.to_string(),
                    person.to_string(),
                    role,
                    assignment,
                    pass,
                ))
            },
        )
        .await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "owner@example.test".to_owned(),
            })
            .await?;
        Ok(Self {
            service,
            agent,
            role,
            assignment,
            person,
            pass,
            cookie,
            client: reqwest::Client::new(),
        })
    }

    pub(super) async fn call(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&Value>,
    ) -> TestResult<(u16, Value)> {
        let request = self
            .client
            .request(method, format!("{}{path}", self.service.base))
            .header("lys-agent-pass", &self.pass);
        let request = match body {
            Some(body) => request.json(body),
            None => request,
        };
        let response = request.send().await?;
        Ok((response.status().as_u16(), response.json().await?))
    }

    pub(super) async fn grant(&self, resource: &str, relation: &str, action: &str) -> TestResult {
        let resource = json!({"kind":"role", "id":resource});
        let (status, root) = self.service.post("/grants/roots", Some(&self.cookie), &json!({
            "operation":operation()?, "route":"api", "holder":self.person, "resource":resource,
            "relation":relation, "pass_on":{"kind":"to", "actions":[action], "recipients":["agent"]},
            "window":{"starts_at":0,"ends_at":null}
        })).await?;
        assert_eq!(status, 200, "{root}");
        let (status, issued) = self.service.post("/grants", Some(&self.cookie), &json!({
            "operation":operation()?, "route":"api", "source":root["grant"], "recipient":self.agent,
            "responsible":self.person, "resource":resource, "relation":relation,
            "pass_on":{"kind":"use_only"}, "window":{"starts_at":0,"ends_at":null}
        })).await?;
        assert_eq!(status, 200, "{issued}");
        Ok(())
    }

    pub(super) async fn role_view(&self, id: &str) -> TestResult<Value> {
        let (status, body) = self
            .service
            .get(&format!("/roles/{id}"), Some(&self.cookie))
            .await?;
        assert_eq!(status, 200, "{body}");
        Ok(body)
    }
}
