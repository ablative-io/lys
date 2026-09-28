#![cfg(test)]
//! DIRECTORY-050 R3: `POST /agents/{id}/start` through the launcher seam
//! (`StartService::with_launcher`). A start the launcher runs is answered
//! with its `runner` member beside the given start; a start the launcher's
//! runner refuses is answered with that refusal by name, its status, and
//! the start that was given beside it; a machine that names no runner is
//! answered the given start alone, as before. The launcher is a fixture
//! that answers what the test plans and records what it was asked, so each
//! answer is keyed on what the test supplied.

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};

use axum::http::HeaderMap;
use lys_core::Ed25519Identity;
use lys_identity::LifecycleState;
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential};
use lys_identity::start::egress::{EgressLists, ProfileNeeds};
use lys_identity::start::machine_role::{HeldRole, RoleMachines};
use lys_identity::start::profile_command::ProfileVersionRecords;
use lys_identity::start::profile_review::{ProfileReviews, Review};
use lys_identity::start::request::{AgentRecord, AgentRecords};
use lys_identity::start::state::{SessionReport, SessionReports};
use lys_identity::start::{Given, Grammars, LaunchRecords};
use lys_identity_server::error::ServerError;
use lys_identity_server::routes::start::{
    Callers, LaunchFuture, Launcher, StartOwners, StartService, routes,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const AGENT: &str = "agent-fixture-1";
const MACHINE: &str = "machine-fixture-1";
const PROFILE: &str = "pv-fixture-1";
const CALLER: &str = "x-fixture-caller";

/// The fixture's only records: one agent, reviewed, on one machine.
struct Seam;

impl AgentRecords for Seam {
    fn agent(&self, agent: &str) -> Option<AgentRecord> {
        (agent == AGENT).then(|| AgentRecord {
            id: agent.to_owned(),
            responsible: Some("person-fixture-1".to_owned()),
        })
    }
}

impl Admission for Seam {
    fn is_administrator(&self, caller: &str) -> bool {
        caller == "admin-fixture"
    }

    fn admits(&self, caller: &str) -> bool {
        caller == "admin-fixture"
    }
}

impl Lifecycles for Seam {
    fn state(&self, agent: &str) -> Option<LifecycleState> {
        (agent == AGENT).then_some(LifecycleState::Active)
    }
}

impl ProfileReviews for Seam {
    fn review(&self, profile_version: &str) -> Option<Review> {
        Some(if profile_version == PROFILE {
            Review::Reviewed
        } else {
            Review::NotReviewed
        })
    }
}

impl RoleMachines for Seam {
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>> {
        (agent == AGENT).then(|| {
            vec![HeldRole {
                role: "role-fixture-builder".to_owned(),
                machines: vec![MACHINE.to_owned()],
            }]
        })
    }
}

impl HandleRecords for Seam {
    fn handles(&self, agent: &str) -> HandleAnswer {
        if agent == AGENT {
            HandleAnswer::Held(vec![HeldCredential {
                id: "vc-fixture-1".to_owned(),
                valid: true,
            }])
        } else {
            HandleAnswer::RecordMissing
        }
    }
}

impl ProfileNeeds for Seam {
    fn needs(&self, profile_version: &str) -> Option<Vec<String>> {
        (profile_version == PROFILE).then(Vec::new)
    }
}

impl EgressLists for Seam {
    fn egress(&self, machine: &str) -> Option<Vec<String>> {
        (machine == MACHINE).then(Vec::new)
    }
}

impl ProfileVersionRecords for Seam {
    fn executable(&self, profile_version: &str) -> Option<String> {
        (profile_version == PROFILE).then(|| "/bin/sh".to_owned())
    }

    fn arguments(&self, profile_version: &str) -> Option<Vec<String>> {
        (profile_version == PROFILE).then(Vec::new)
    }

    fn working_directory(&self, profile_version: &str) -> Option<String> {
        (profile_version == PROFILE).then(|| "/".to_owned())
    }
}

impl SessionReports for Seam {
    fn reports(&self, launch_record: &str) -> Vec<SessionReport> {
        assert!(!launch_record.is_empty(), "a report is read for a record");
        Vec::new()
    }
}

struct HeaderCallers;

impl Callers for HeaderCallers {
    fn caller(&self, headers: &HeaderMap) -> Option<String> {
        headers
            .get(CALLER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }
}

fn fixture_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn clock() -> u64 {
    1_000
}

/// What the launcher answers next.
#[derive(Clone)]
enum Plan {
    /// The machine names no runner.
    NoRunner,
    /// The runner ran it, answering this member.
    Ran(Value),
    /// The runner refused, by this name.
    Refused(&'static str),
}

/// The launcher the route is given: it answers the plan and keeps each
/// launch record id and caller it was asked for.
struct Planned {
    plan: Mutex<Plan>,
    asked: Mutex<Vec<(String, String, String)>>,
}

/// The planned launcher as the route holds it.
struct Launches(Arc<Planned>);

impl Launcher for Launches {
    fn launch<'a>(&'a self, given: &'a Given, caller: &'a str) -> LaunchFuture<'a> {
        Box::pin(async move {
            self.0
                .asked
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push((
                    given.record.id.clone(),
                    given.record.machine.clone(),
                    caller.to_owned(),
                ));
            let plan = self
                .0
                .plan
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone();
            match plan {
                Plan::NoRunner => None,
                Plan::Ran(member) => Some(Ok(member)),
                Plan::Refused(refusal) => Some(Err(ServerError::Runner {
                    refusal: refusal.to_owned(),
                    words: "the planned refusal".to_owned(),
                })),
            }
        })
    }
}

struct Served {
    base: String,
    planned: Arc<Planned>,
    client: reqwest::Client,
    dir: tempfile::TempDir,
}

impl Served {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let owners = StartOwners {
            agents: Box::new(Seam),
            admission: Box::new(Seam),
            lifecycles: Box::new(Seam),
            reviews: Box::new(Seam),
            role_machines: Box::new(Seam),
            handles: Box::new(Seam),
            needs: Box::new(Seam),
            egress: Box::new(Seam),
            profiles: Box::new(Seam),
            sessions: Box::new(Seam),
            grammars: Grammars {
                agent_id: fixture_id,
                credential_id: fixture_id,
            },
        };
        let key = Ed25519Identity::load_or_generate(&dir.path().join("service.key"))?;
        let launches = LaunchRecords::open(&dir.path().join("launch-records"), key)?;
        let planned = Arc::new(Planned {
            plan: Mutex::new(Plan::NoRunner),
            asked: Mutex::new(Vec::new()),
        });
        let service = StartService::new(owners, Box::new(HeaderCallers), launches, clock)
            .with_launcher(Box::new(Launches(Arc::clone(&planned))));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let app = routes(Arc::new(service));
        tokio::spawn(async move { axum::serve(listener, app).await });
        Ok(Self {
            base,
            planned,
            client: reqwest::Client::new(),
            dir,
        })
    }

    /// Start the agent under `plan`, answering the status and the answer.
    async fn start_under(&self, plan: Plan) -> Result<(u16, Value), Box<dyn Error>> {
        *self
            .planned
            .plan
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = plan;
        let answer = self
            .client
            .post(format!("{}/agents/{AGENT}/start", self.base))
            .header(CALLER, "admin-fixture")
            .body(json!({ "profile_version": PROFILE, "machine": MACHINE }).to_string())
            .send()
            .await?;
        let status = answer.status().as_u16();
        Ok((status, serde_json::from_str(&answer.text().await?)?))
    }

    fn close(self) -> TestResult {
        Ok(self.dir.close()?)
    }

    fn asked(&self) -> Vec<(String, String, String)> {
        self.planned
            .asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

#[tokio::test]
async fn a_start_the_runner_ran_answers_its_runner_member() -> TestResult {
    let served = Served::start().await?;
    let member = json!({ "session": "op-planned", "state": "running", "pid": 4242 });
    let (status, answer) = served.start_under(Plan::Ran(member.clone())).await?;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(answer["runner"], member, "{answer}");
    let record = answer["launch_record"]["id"]
        .as_str()
        .ok_or("no launch record")?;
    assert_eq!(
        served.asked(),
        vec![(
            record.to_owned(),
            MACHINE.to_owned(),
            "admin-fixture".to_owned()
        )],
        "the launcher was asked for the start the route gave"
    );
    served.close()
}

#[tokio::test]
async fn a_start_the_runner_refused_answers_the_refusal_by_name_beside_the_start() -> TestResult {
    let served = Served::start().await?;
    let (status, answer) = served
        .start_under(Plan::Refused("runner_protocol_mismatch"))
        .await?;
    assert_eq!(status, 502, "{answer}");
    assert_eq!(answer["error"], "runner_protocol_mismatch", "{answer}");
    assert!(
        answer["words"]
            .as_str()
            .is_some_and(|words| words.contains("the planned refusal")),
        "{answer}"
    );
    let given = &answer["given"];
    assert!(
        given["launch_record"]["id"].as_str().is_some(),
        "the start that was given is beside the refusal: {answer}"
    );
    assert!(given.get("runner").is_none(), "{answer}");
    assert_eq!(served.asked().len(), 1);
    served.close()
}

#[tokio::test]
async fn a_machine_with_no_runner_is_answered_the_start_alone() -> TestResult {
    let served = Served::start().await?;
    let (status, answer) = served.start_under(Plan::NoRunner).await?;
    assert_eq!(status, 200, "{answer}");
    assert!(answer.get("runner").is_none(), "{answer}");
    assert!(
        answer["command"]
            .as_str()
            .is_some_and(|command| command.starts_with("env ")),
        "{answer}"
    );
    assert_eq!(
        served.asked().len(),
        1,
        "the launcher was asked, and ran nothing"
    );
    served.close()
}
