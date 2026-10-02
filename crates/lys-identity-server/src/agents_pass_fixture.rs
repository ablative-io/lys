use std::error::Error;
use std::sync::Arc;

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
    Transition,
};
use lys_log_store::FileLeafStore;
use serde_json::{Value, json};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;
const MODEL: &str =
    r#"{"version":1,"relations":{"starter":["agent.start-command"],"stopper":["agent.stop"]}}"#;

pub(super) fn operation() -> TestResult<String> {
    Ok(OperationId::generate()?.to_string())
}

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
    stops_dir: &std::path::Path,
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
        let stops = scope.spawn(|| crate::stops_store::StopStore::open(stops_dir, Arc::clone(key)));
        let runtime =
            scope.spawn(|| crate::runtime_store::RuntimeStore::open(runtime_dir, Arc::clone(key)));
        joined(runtime)?;
        joined(organisation)?;
        joined(acts)?;
        joined(launches).map_err(|error| error as Box<dyn Error>)?;
        joined(apps)?;
        joined(stops)?;
        Ok(())
    })
}

pub(super) struct Table {
    pub(super) service: Service,
    pub(super) agent: String,
    pub(super) target: String,
    person: String,
    pass: String,
    pub(super) cookie: String,
    client: reqwest::Client,
}

impl Table {
    pub(super) async fn fresh() -> TestResult<Self> {
        let (service, (agent, target, person, pass)) = Service::start_adjusted(
            MODEL,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.certificates_dir = None;
                config.network_file = None;
                config.roles_file = None;
                config.service_accounts_dir = None;
                config.teams_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
                config.homes_dir = None;
            },
            |config| {
                let key = Arc::new(Ed25519Identity::load(&config.event_key_file)?);
                prepare_stores(
                    &config.log_dir,
                    &config.event_key_file,
                    &config.apps_dir(),
                    config
                        .stops_dir
                        .as_deref()
                        .ok_or("stop fixture is disabled")?,
                    config
                        .runtime_dir
                        .as_deref()
                        .ok_or("runtime fixture is disabled")?,
                    &key,
                )?;
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
                    Profile::new("Operator")?,
                    2,
                )?;
                let (target, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Target")?,
                    2,
                )?;
                for id in [agent, target] {
                    directory.transition(
                        actor.clone(),
                        OperationId::generate()?,
                        IdentityId::Agent(id),
                        Transition::Activate,
                        "",
                        3,
                    )?;
                }
                let mut runtime = crate::runtime_store::RuntimeStore::open(
                    config
                        .runtime_dir
                        .as_deref()
                        .ok_or("runtime fixture is disabled")?,
                    Arc::clone(&key),
                )?;
                runtime.report(crate::runtime_state::Report {
                    operation: "agent-session".to_owned(),
                    session: "agent-session".to_owned(),
                    agent: Some(agent.to_string()),
                    machine: "fixture".to_owned(),
                    state: crate::runtime_state::Reported::Starting,
                    what: String::new(),
                    confirmation: String::new(),
                    reported_by: person.to_string(),
                    at: 3,
                    launch: None,
                })?;
                let pass = crate::agent_pass_store::Passes::open(
                    config.log_dir.with_file_name("agent-passes.json"),
                )?
                .issue(agent, "agent-launch", "agent-session")?
                .to_string();
                Ok((
                    agent.to_string(),
                    target.to_string(),
                    person.to_string(),
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
            target,
            person,
            pass,
            cookie,
            client: reqwest::Client::new(),
        })
    }

    pub(super) async fn call(&self, path: &str, body: &Value) -> TestResult<(u16, Value)> {
        let response = self
            .client
            .post(format!("{}{path}", self.service.base))
            .header("lys-agent-pass", &self.pass)
            .json(body)
            .send()
            .await?;
        Ok((response.status().as_u16(), response.json().await?))
    }

    pub(super) async fn grant(&self, relation: &str, action: &str) -> TestResult {
        let resource = json!({"kind":"agent", "id":self.target});
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

    pub(super) async fn stopped(&self) -> TestResult<Value> {
        let (status, answer) = self
            .service
            .get(
                &format!("/agents/{}/stops", self.target),
                Some(&self.cookie),
            )
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }
}
