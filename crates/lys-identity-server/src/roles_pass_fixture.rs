use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::{Directory, LoginBinding, OperationId};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};

use crate::roles_records::{Holding, Role, Version, Words};

#[path = "roles_pass_template.rs"]
mod template;

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const MODEL: &str = r#"{"version":1,"relations":{"reader":["read"],"creator":["role.create"],"reviser":["role.revise"],"assigner":["role.holder.assign"],"mover":["role.holder.move"],"ender":["role.holder.end"]}}"#;

fn joined<T>(worker: std::thread::ScopedJoinHandle<'_, T>) -> T {
    match worker.join() {
        Ok(result) => result,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

fn prepare_stores(
    log: &std::path::Path,
    key_file: &std::path::Path,
    apps_dir: &std::path::Path,
    runtime_dir: &std::path::Path,
    key: &Arc<Ed25519Identity>,
) -> TestResult {
    std::thread::scope(|scope| -> TestResult {
        let organisation = scope.spawn(|| {
            crate::configuration_store::ConfigurationStore::open(
                &log.with_file_name("organisation"),
                Arc::clone(key),
            )
        });
        let acts = scope.spawn(|| {
            crate::runner_acts::ActStore::open(&log.with_file_name("runner-acts"), Arc::clone(key))
        });
        let launches = scope.spawn(|| -> Result<_, Box<dyn Error + Send + Sync>> {
            Ok(lys_identity::start::LaunchRecords::open(
                &log.with_file_name("launch-records"),
                Ed25519Identity::load(key_file)?,
            )?)
        });
        let apps = scope.spawn(|| crate::apps_store::AppStore::open(apps_dir, Arc::clone(key)));
        let runtime =
            scope.spawn(|| crate::runtime_store::RuntimeStore::open(runtime_dir, Arc::clone(key)));
        joined(runtime)?;
        joined(organisation)?;
        joined(acts)?;
        joined(launches).map_err(|error| error as Box<dyn Error>)?;
        joined(apps)?;
        Ok(())
    })
}

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
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                let (agent, person) = template::restore(
                    &config.log_dir,
                    &config.event_key_file,
                    &config.apps_dir(),
                    config
                        .runtime_dir
                        .as_deref()
                        .ok_or("runtime fixture is disabled")?,
                    &config.log_origin,
                    &key,
                    &config.grant_model()?,
                )?;
                let path = config.log_dir.clone();
                let mut directory = Directory::open(
                    Box::new(move || FileLeafStore::open(&path)),
                    Ed25519Identity::load(&config.event_key_file)?,
                )?;
                directory.bind_login(
                    template::actor()?,
                    OperationId::generate()?,
                    person,
                    LoginBinding::new(&config.issuer, ADMINISTRATOR)?,
                    4,
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
        let (status, issued) = self.give(resource, relation, action).await?;
        assert_eq!(status, 200, "{issued}");
        Ok(())
    }

    /// Pass `action` on a role to the agent through a root of the
    /// person's, answering the status and body of the pass on, or of the
    /// root when it is refused.
    pub(super) async fn give(
        &self,
        resource: &str,
        relation: &str,
        action: &str,
    ) -> TestResult<(u16, Value)> {
        let resource = json!({"kind":"role", "id":resource});
        let (status, root) = self.service.post("/grants/roots", Some(&self.cookie), &json!({
            "operation":operation()?, "route":"api", "holder":self.person, "resource":resource,
            "relation":relation, "pass_on":{"kind":"to", "actions":[action], "recipients":["agent"]},
            "window":{"starts_at":0,"ends_at":null}
        })).await?;
        if status != 200 {
            return Ok((status, root));
        }
        self.service.post("/grants", Some(&self.cookie), &json!({
            "operation":operation()?, "route":"api", "source":root["grant"], "recipient":self.agent,
            "responsible":self.person, "resource":resource, "relation":relation,
            "pass_on":{"kind":"use_only"}, "window":{"starts_at":0,"ends_at":null}
        })).await
    }

    /// The administrator's own call, answering its status and body.
    pub(super) async fn as_administrator(&self, path: &str, body: &Value) -> TestResult<(u16, Value)> {
        self.service.post(path, Some(&self.cookie), body).await
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
