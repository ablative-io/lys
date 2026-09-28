#![cfg(test)]
//! DIRECTORY-029 R12: the start route gives, gives again, withdraws and reads
//! a start through the library and answers the library's own bytes; no start
//! path runs the command (CONFORMANCE 5.1, a marker-writing executable is
//! given and the marker never appears); no credential value is in an answer
//! (CONFORMANCE 5.3); a start given again is a new record naming its source
//! (CONFORMANCE 5.4); a record reads running only on its verified report
//! (CONFORMANCE 5.5) and unconfirmed without one (CONFORMANCE 5.6).

use std::error::Error;
use std::path::PathBuf;
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
use lys_identity::start::{Grammars, LaunchRecords, give};
use lys_identity_server::routes::start::{Callers, StartOwners, StartService, routes};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

/// A value the door keeps beside vc-fixture-1; no answer ever holds it.
const VALUE_ONE: &str = "fixture-credential-value-1f3a";

/// The header the fixture callers are named in.
const CALLER: &str = "x-fixture-caller";

/// Every owner's record, shared by the route's seams and changed by the test.
struct World {
    state: Mutex<LifecycleState>,
    reports: Mutex<Vec<SessionReport>>,
    executable: String,
    arguments: Vec<String>,
}

/// One seam over the shared world.
struct Seam(Arc<World>);

fn lock<T>(held: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

fn is_fixture_agent(agent: &str) -> bool {
    agent == "agent-fixture-1"
}

impl AgentRecords for Seam {
    fn agent(&self, agent: &str) -> Option<AgentRecord> {
        is_fixture_agent(agent).then(|| AgentRecord {
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
        is_fixture_agent(agent).then(|| *lock(&self.0.state))
    }
}

impl ProfileReviews for Seam {
    fn review(&self, profile_version: &str) -> Option<Review> {
        Some(if profile_version == "pv-fixture-1" {
            Review::Reviewed
        } else {
            Review::NotReviewed
        })
    }
}

impl RoleMachines for Seam {
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>> {
        is_fixture_agent(agent).then(|| {
            vec![HeldRole {
                role: "role-fixture-builder".to_owned(),
                machines: vec!["machine-fixture-1".to_owned()],
            }]
        })
    }
}

impl HandleRecords for Seam {
    fn handles(&self, agent: &str) -> HandleAnswer {
        // The door keeps VALUE_ONE beside vc-fixture-1; its seam answers the
        // id and validity only.
        if is_fixture_agent(agent) {
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
        (profile_version == "pv-fixture-1").then(|| vec!["model providers".to_owned()])
    }
}

impl EgressLists for Seam {
    fn egress(&self, machine: &str) -> Option<Vec<String>> {
        (machine == "machine-fixture-1").then(|| vec!["model providers".to_owned()])
    }
}

impl ProfileVersionRecords for Seam {
    fn executable(&self, profile_version: &str) -> Option<String> {
        (profile_version == "pv-fixture-1").then(|| self.0.executable.clone())
    }

    fn arguments(&self, profile_version: &str) -> Option<Vec<String>> {
        (profile_version == "pv-fixture-1").then(|| self.0.arguments.clone())
    }

    fn working_directory(&self, profile_version: &str) -> Option<String> {
        (profile_version == "pv-fixture-1").then(|| "fixture-cwd".to_owned())
    }
}

impl SessionReports for Seam {
    fn reports(&self, launch_record: &str) -> Vec<SessionReport> {
        lock(&self.0.reports)
            .iter()
            .filter(|report| report.launch_record == launch_record)
            .cloned()
            .collect()
    }
}

/// Callers named in a fixture header.
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

/// The route served over the world, with the marker its executable would
/// write if anything ran it.
struct Served {
    dir: tempfile::TempDir,
    base: String,
    world: Arc<World>,
    service: Arc<StartService>,
    client: reqwest::Client,
}

/// Where the fixture executable writes its marker, in `dir`.
fn marker(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("fixture-exec-ran")
}

impl Served {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let script = dir.path().join("fixture-exec");
        std::fs::write(&script, format!("touch '{}'\n", marker(&dir).display()))?;
        let world = Arc::new(World {
            state: Mutex::new(LifecycleState::Active),
            reports: Mutex::new(Vec::new()),
            executable: "/bin/sh".to_owned(),
            arguments: vec![script.display().to_string()],
        });
        let seam = || Seam(Arc::clone(&world));
        let owners = StartOwners {
            agents: Box::new(seam()),
            admission: Box::new(seam()),
            lifecycles: Box::new(seam()),
            reviews: Box::new(seam()),
            role_machines: Box::new(seam()),
            handles: Box::new(seam()),
            needs: Box::new(seam()),
            egress: Box::new(seam()),
            profiles: Box::new(seam()),
            sessions: Box::new(seam()),
            grammars: Grammars {
                agent_id: fixture_id,
                credential_id: fixture_id,
            },
        };
        let key = Ed25519Identity::load_or_generate(&dir.path().join("service.key"))?;
        let launches = LaunchRecords::open(&dir.path().join("launch-records"), key)?;
        let service = Arc::new(StartService::new(
            owners,
            Box::new(HeaderCallers),
            launches,
            clock,
        ));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let base = format!("http://{}", listener.local_addr()?);
        let app = routes(Arc::clone(&service));
        tokio::spawn(async move { axum::serve(listener, app).await });
        Ok(Self {
            dir,
            base,
            world,
            service,
            client: reqwest::Client::new(),
        })
    }

    async fn send(
        &self,
        post: bool,
        path: &str,
        caller: &str,
        body: &str,
    ) -> Result<(u16, String), Box<dyn Error>> {
        let url = format!("{}{path}", self.base);
        let request = if post {
            self.client.post(url).body(body.to_owned())
        } else {
            self.client.get(url)
        };
        let answer = request.header(CALLER, caller).send().await?;
        let status = answer.status().as_u16();
        Ok((status, answer.text().await?))
    }

    async fn start_agent(&self, caller: &str) -> Result<(u16, String), Box<dyn Error>> {
        self.send(
            true,
            "/agents/agent-fixture-1/start",
            caller,
            r#"{"profile_version":"pv-fixture-1","machine":"machine-fixture-1"}"#,
        )
        .await
    }

    fn report(&self, launch_record: &str) {
        lock(&self.world.reports).push(SessionReport {
            session: "sess-fixture-1".to_owned(),
            agent: "agent-fixture-1".to_owned(),
            launch_record: launch_record.to_owned(),
            verified: true,
        });
    }

    fn ran(&self) -> bool {
        marker(&self.dir).exists()
    }
}

fn parsed(body: &str) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(body)?)
}

fn launch_record(body: &str) -> Result<String, Box<dyn Error>> {
    parsed(body)?["launch_record"]["id"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("no launch record in {body}").into())
}

#[tokio::test]
async fn a_passing_start_answers_a_command_and_runs_nothing() -> TestResult {
    let served = Served::start().await?;
    let (status, body) = served.start_agent("admin-fixture").await?;
    assert_eq!(status, 200, "{body}");
    let answer = parsed(&body)?;
    assert!(answer["command"].as_str().is_some_and(|line| line.starts_with("env ")));
    assert_eq!(answer["working_directory"], "fixture-cwd");
    assert!(launch_record(&body)?.starts_with("launch-"));
    assert!(!body.contains(VALUE_ONE));
    assert!(!served.ran(), "the marker exists: something ran the command");
    Ok(())
}

#[tokio::test]
async fn a_suspended_agent_is_refused_by_name_with_no_command() -> TestResult {
    let served = Served::start().await?;
    *lock(&served.world.state) = LifecycleState::Suspended;
    let (status, body) = served.start_agent("admin-fixture").await?;
    assert_eq!(status, 409, "{body}");
    assert!(body.contains("agent_not_active"), "{body}");
    assert!(body.contains("the agent is active"), "{body}");
    assert!(parsed(&body)?.get("command").is_none());
    Ok(())
}

#[tokio::test]
async fn a_start_is_withdrawn_given_again_and_read() -> TestResult {
    let served = Served::start().await?;
    let (_, body) = served.start_agent("admin-fixture").await?;
    let l1 = launch_record(&body)?;
    let state = format!("/launch-records/{l1}/state");
    let (_, read) = served.send(false, &state, "admin-fixture", "").await?;
    assert_eq!(parsed(&read)?["state"], "unconfirmed");

    let withdraw = format!("/launch-records/{l1}/withdraw");
    let (status, withdrawn) = served.send(true, &withdraw, "admin-fixture", "").await?;
    assert_eq!(status, 200, "{withdrawn}");
    let withdrawn = parsed(&withdrawn)?;
    assert_eq!(withdrawn["state"], "withdrawn");
    assert_eq!(withdrawn["withdrawal"]["by"], "admin-fixture");
    let (_, read) = served.send(false, &state, "admin-fixture", "").await?;
    assert_eq!(parsed(&read)?["state"], "withdrawn");

    let again = format!("/launch-records/{l1}/start-again");
    let (status, body) = served.send(true, &again, "admin-fixture", "").await?;
    assert_eq!(status, 200, "{body}");
    let l2 = launch_record(&body)?;
    assert_ne!(l1, l2);
    assert_eq!(parsed(&body)?["launch_record"]["copied_from"], l1.as_str());
    assert!(parsed(&body)?["command"].is_string());

    served.report(&l1);
    let (_, read) = served.send(false, &state, "admin-fixture", "").await?;
    let read = parsed(&read)?;
    assert_eq!(read["state"], "running");
    assert_eq!(read["withdrawal"]["by"], "admin-fixture");
    assert!(!served.ran(), "the marker exists: something ran the command");
    Ok(())
}

#[tokio::test]
async fn each_refusal_is_the_librarys_own_bytes() -> TestResult {
    let served = Served::start().await?;
    let library = |agent: &str, caller: &str| -> Result<String, Box<dyn Error>> {
        let members = vec![
            ("profile_version".to_owned(), "pv-fixture-1".to_owned()),
            ("machine".to_owned(), "machine-fixture-1".to_owned()),
            ("agent".to_owned(), agent.to_owned()),
        ];
        let mut launches = served.service.launches();
        let error = give(&mut launches, &served.service.owners(), caller, members, 1_000)
            .expect_err("the library refuses the same inputs");
        Ok(error.to_json())
    };
    let mut compared = 0;

    let (_, body) = served
        .send(
            true,
            "/agents/agent-fixture-missing/start",
            "admin-fixture",
            r#"{"profile_version":"pv-fixture-1","machine":"machine-fixture-1"}"#,
        )
        .await?;
    assert!(body.contains("agent_unknown"), "{body}");
    assert_eq!(body, library("agent-fixture-missing", "admin-fixture")?);
    compared += 1;

    let (_, body) = served.start_agent("person-fixture-2").await?;
    assert!(body.contains("start_right_missing"), "{body}");
    assert_eq!(body, library("agent-fixture-1", "person-fixture-2")?);
    compared += 1;

    *lock(&served.world.state) = LifecycleState::Suspended;
    let (_, body) = served.start_agent("admin-fixture").await?;
    assert!(body.contains("agent_not_active"), "{body}");
    assert_eq!(body, library("agent-fixture-1", "admin-fixture")?);
    *lock(&served.world.state) = LifecycleState::Active;
    compared += 1;

    served.start_agent("admin-fixture").await?;
    let (_, body) = served.start_agent("admin-fixture").await?;
    assert!(body.contains("start_unconfirmed"), "{body}");
    assert_eq!(body, library("agent-fixture-1", "admin-fixture")?);
    compared += 1;

    assert_eq!(compared, 4);
    Ok(())
}

const ROUTE: &str = include_str!("../src/start.rs");
const ROUTES: &str = include_str!("../src/routes.rs");

#[test]
fn the_route_holds_no_start_logic_and_reads_the_door_through_the_seam() {
    for forbidden in [
        "start_unconfirmed",
        "check_record_missing",
        "agent_not_active",
        "LYS_AGENT_ID",
        "LYS_LAUNCH_RECORD",
        "LYS_CREDENTIAL_IDS",
    ] {
        assert!(!ROUTE.contains(forbidden), "the route names {forbidden}");
    }
    assert!(ROUTE.contains("pub handles: Box<dyn HandleRecords + Send + Sync>"));
    assert!(!ROUTE.contains("DoorHandles") && !ROUTE.contains("door_handles"));
    assert!(ROUTES.contains("let handles = door_handles::DoorHandles::unconfigured();"));
    assert!(ROUTES.contains("Box::new(handles),\n        door_handles::credential_id,"));
    assert!(ROUTES.contains("let starts = start::routes(start_service(config, &state)?);"));
}
