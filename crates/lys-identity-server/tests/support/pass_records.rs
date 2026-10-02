#![cfg(test)]
//! Real signed records for ending routes, without an unrelated runner or profile API.
use identity_contract::harness::{GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential};
use lys_identity::start::egress::{EgressLists, ProfileNeeds};
use lys_identity::start::machine_role::{HeldRole, RoleMachines};
use lys_identity::start::profile_command::ProfileVersionRecords;
use lys_identity::start::profile_review::{ProfileReviews, Review};
use lys_identity::start::request::{AgentRecord, AgentRecords};
use lys_identity::start::state::{SessionReport, SessionReports};
use lys_identity::start::{Grammars, LaunchRecords, Owners, give};
use lys_identity::{
    Actor, AgentId, AuthMethod, IdentityId, LifecycleState, OperationId, Profile, Provenance,
    Transition,
};
use lys_identity_server::agent_pass_store::Passes;
use lys_identity_server::routes::open_directory;
use lys_identity_server::runtime_state::{Report, Reported};
use lys_identity_server::runtime_store::RuntimeStore;
use lys_identity_server::session::{Sessions, now};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zeroize::Zeroizing;

pub type TestResult = Result<(), Box<dyn Error>>;
const PROFILE: &str = "profile-fixture-1";
struct OwnersFixture {
    agent: String,
    person: String,
    machine: String,
}
impl AgentRecords for OwnersFixture {
    fn agent(&self, agent: &str) -> Option<AgentRecord> {
        (agent == self.agent).then(|| AgentRecord {
            id: self.agent.clone(),
            responsible: Some(self.person.clone()),
        })
    }
}
impl Admission for OwnersFixture {
    fn is_administrator(&self, caller: &str) -> bool {
        caller == self.person
    }
    fn admits(&self, caller: &str) -> bool {
        caller == self.person
    }
}
impl Lifecycles for OwnersFixture {
    fn state(&self, agent: &str) -> Option<LifecycleState> {
        (agent == self.agent).then_some(LifecycleState::Active)
    }
}
impl ProfileReviews for OwnersFixture {
    fn review(&self, version: &str) -> Option<Review> {
        (version == PROFILE).then_some(Review::Reviewed)
    }
}
impl RoleMachines for OwnersFixture {
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>> {
        (agent == self.agent).then(|| {
            vec![HeldRole {
                role: "role-fixture-1".to_owned(),
                machines: vec![self.machine.clone()],
            }]
        })
    }
}
impl HandleRecords for OwnersFixture {
    fn handles(&self, agent: &str) -> HandleAnswer {
        if agent == self.agent {
            HandleAnswer::Held(vec![HeldCredential {
                id: "credential-fixture-1".to_owned(),
                valid: true,
            }])
        } else {
            HandleAnswer::RecordMissing
        }
    }
}
impl ProfileNeeds for OwnersFixture {
    fn needs(&self, version: &str) -> Option<Vec<String>> {
        (version == PROFILE).then(Vec::new)
    }
}
impl EgressLists for OwnersFixture {
    fn egress(&self, machine: &str) -> Option<Vec<String>> {
        (machine == self.machine).then(Vec::new)
    }
}
impl ProfileVersionRecords for OwnersFixture {
    fn executable(&self, version: &str) -> Option<String> {
        (version == PROFILE).then(|| "/bin/sh".to_owned())
    }
    fn arguments(&self, version: &str) -> Option<Vec<String>> {
        (version == PROFILE).then(Vec::new)
    }
    fn working_directory(&self, version: &str) -> Option<String> {
        (version == PROFILE).then(|| "/".to_owned())
    }
}
impl SessionReports for OwnersFixture {
    fn reports(&self, launch: &str) -> Vec<SessionReport> {
        assert!(lys_identity::start::launch_record::is_launch_record_id(
            launch
        ));
        Vec::new()
    }
}
impl OwnersFixture {
    fn owners(&self) -> Owners<'_> {
        Owners {
            agents: self,
            admission: self,
            lifecycles: self,
            reviews: self,
            role_machines: self,
            handles: self,
            needs: self,
            egress: self,
            profiles: self,
            sessions: self,
            grammars: Grammars {
                agent_id: valid_id,
                credential_id: valid_id,
            },
        }
    }
}
fn valid_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}
pub fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

pub struct Fixture {
    pub service: Service,
    pub cookie: String,
    pub agent: AgentId,
    pub machine: String,
    pub session: String,
    pub launch: String,
    pub pass: Zeroizing<String>,
}
impl Fixture {
    pub async fn open() -> Result<Self, Box<dyn Error>> {
        let (service, (agent, machine, session, launch, pass, cookie)) = Service::start_adjusted(
            GRANT_MODEL,
            None,
            None,
            None,
            |config| {
                config.requests_dir = None;
                config.certificates_dir = None;
                config.service_accounts_dir = None;
                config.teams_dir = None;
                config.budgets_dir = None;
                config.policies_dir = None;
                config.goals_dir = None;
                config.reviews_dir = None;
            },
            |config| {
                let mut directory = open_directory(config)?;
                let actor = Actor::new(
                    config.administrator_binding()?,
                    Provenance::new(AuthMethod::Oidc, 1),
                );
                let (person, _) = directory.register_person(
                    actor.clone(),
                    OperationId::generate()?,
                    Profile::new("Fixture person")?,
                    1,
                )?;
                directory.bind_login(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    config.administrator_binding()?,
                    1,
                )?;
                directory.transition(
                    actor.clone(),
                    OperationId::generate()?,
                    IdentityId::Person(person),
                    Transition::Activate,
                    "",
                    1,
                )?;
                let (agent, _) = directory.register_agent(
                    actor.clone(),
                    OperationId::generate()?,
                    person,
                    Profile::new("Fixture agent")?,
                    1,
                )?;
                directory.transition(
                    actor,
                    OperationId::generate()?,
                    IdentityId::Agent(agent),
                    Transition::Activate,
                    "",
                    1,
                )?;
                let machine = operation()?;
                let owner = OwnersFixture {
                    agent: agent.to_string(),
                    person: person.to_string(),
                    machine: machine.clone(),
                };
                let mut launches = LaunchRecords::open(
                    &config.log_dir.with_file_name("launch-records"),
                    Ed25519Identity::load(&config.event_key_file)?,
                )?;
                let given = give(
                    &mut launches,
                    &owner.owners(),
                    &owner.person,
                    [
                        ("agent".to_owned(), agent.to_string()),
                        ("profile_version".to_owned(), PROFILE.to_owned()),
                        ("machine".to_owned(), machine.clone()),
                    ],
                    1,
                )?;
                let session = operation()?;
                let mut runtime = RuntimeStore::open(
                    config
                        .runtime_dir
                        .as_ref()
                        .ok_or("fixture needs runtime records")?,
                    Arc::new(Ed25519Identity::load(&config.event_key_file)?),
                )?;
                runtime.report(Report {
                    operation: session.clone(),
                    session: session.clone(),
                    agent: Some(agent.to_string()),
                    machine: machine.clone(),
                    state: Reported::Starting,
                    what: "fixture process starting".to_owned(),
                    confirmation: String::new(),
                    reported_by: person.to_string(),
                    at: 1,
                    launch: None,
                })?;
                runtime.report(Report {
                    operation: operation()?,
                    session: session.clone(),
                    agent: Some(agent.to_string()),
                    machine: machine.clone(),
                    state: Reported::Running,
                    what: "fixture process running".to_owned(),
                    confirmation: String::new(),
                    reported_by: person.to_string(),
                    at: 2,
                    launch: None,
                })?;
                let launch = given.record.id;
                let mut passes = Passes::open(config.log_dir.with_file_name("agent-passes.json"))?;
                let pass = passes.issue(agent, &launch, &session)?;
                let signed_in = Sessions::open(
                    config
                        .sessions_file
                        .clone()
                        .ok_or("fixture needs persisted sessions")?,
                    config.session_seconds,
                    config.secure_cookie,
                )?;
                let cookie = signed_in.begin(Actor::new(
                    config.administrator_binding()?,
                    Provenance::new(AuthMethod::Oidc, now()),
                ))?;
                let cookie = cookie
                    .split(';')
                    .next()
                    .ok_or("fixture session cookie is absent")?
                    .to_owned();
                Ok((agent, machine, session, launch, pass, cookie))
            },
        )
        .await?;
        Ok(Self {
            service,
            cookie,
            agent,
            machine,
            session,
            launch,
            pass,
        })
    }
    pub fn file(&self) -> PathBuf {
        self.service.dir.path().join("agent-passes.json")
    }
    pub fn key(&self) -> String {
        format!("{:x}", Sha256::digest(self.pass.as_bytes()))
    }
    pub fn contains(&self) -> Result<bool, Box<dyn Error>> {
        let table: Value = serde_json::from_slice(&std::fs::read(self.file())?)?;
        Ok(table["passes"].get(self.key()).is_some())
    }
}
pub fn fail_save(file: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let earlier = std::fs::read(file)?;
    std::fs::remove_file(file)?;
    std::fs::create_dir(file)?;
    Ok(earlier)
}
pub fn restore(file: &Path, earlier: &[u8]) -> TestResult {
    std::fs::remove_dir(file)?;
    std::fs::write(file, earlier)?;
    Ok(())
}
