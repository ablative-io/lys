#![cfg(test)]
//! DIRECTORY-029 R3, CONFORMANCE 5.2: the five named checks run before any
//! command, in their order, each reading its owner's record; the agent is
//! active reads the lifecycle state live and passes only on active; and a
//! missing owner's record is refused by name, naming the check and its card,
//! with nothing faked to let the start through.

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::path::Path;

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
use lys_identity::start::state::{LaunchRecords, SessionReport, SessionReports};
use lys_identity::start::{Grammars, Owners};
use lys_identity::start::checks::Check;
use lys_identity::start::{StartError, active, give};

type TestResult = Result<(), Box<dyn Error>>;

/// A value a door keeps beside a credential id; nothing a start answers holds it.
const VALUE_ONE: &str = "fixture-credential-value-1f3a";

/// Every owner's record a start reads, held by the test: agent records,
/// admission, lifecycle, reviews, role machines, handles, needs, egress,
/// profile versions and session reports. Each seam counts the checks run.
struct World {
    agents: Vec<AgentRecord>,
    profile_versions: Vec<String>,
    machines: Vec<String>,
    administrators: Vec<String>,
    admitted: Vec<String>,
    state: Option<LifecycleState>,
    reviewed: Option<Vec<String>>,
    roles: Option<Vec<HeldRole>>,
    /// Each credential's id, whether it is valid, and the value the door
    /// keeps beside it, which the seam never answers.
    handles: Option<Vec<(String, bool, String)>>,
    needs: Option<Vec<String>>,
    egress: Option<Vec<String>>,
    executable: Option<String>,
    arguments: Option<Vec<String>>,
    working_directory: Option<String>,
    reports: RefCell<Vec<SessionReport>>,
    checks_run: Cell<usize>,
}

fn texts(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// The fixtures' id grammar: lowercase letters, digits and '-'.
fn fixture_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl World {
    /// Every owner's record present, and every check passing for
    /// agent-fixture-1, pv-fixture-1 and machine-fixture-1.
    fn passing() -> Self {
        Self {
            agents: vec![AgentRecord {
                id: "agent-fixture-1".to_owned(),
                responsible: Some("person-fixture-1".to_owned()),
            }],
            profile_versions: texts(&["pv-fixture-1"]),
            machines: texts(&["machine-fixture-1", "machine-fixture-2"]),
            administrators: texts(&["admin-fixture"]),
            admitted: texts(&["admin-fixture"]),
            state: Some(LifecycleState::Active),
            reviewed: Some(texts(&["pv-fixture-1"])),
            roles: Some(vec![HeldRole {
                role: "role-fixture-builder".to_owned(),
                machines: texts(&["machine-fixture-1", "machine-fixture-2"]),
            }]),
            handles: Some(vec![(
                "vc-fixture-1".to_owned(),
                true,
                VALUE_ONE.to_owned(),
            )]),
            needs: Some(texts(&["model providers", "mcp-fixture"])),
            egress: Some(texts(&["model providers", "mcp-fixture"])),
            executable: Some("fixture-exec".to_owned()),
            arguments: Some(texts(&["--fixture-arg", "two words"])),
            working_directory: Some("fixture-cwd".to_owned()),
            reports: RefCell::new(Vec::new()),
            checks_run: Cell::new(0),
        }
    }

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
                agent_id: fixture_id,
                credential_id: fixture_id,
            },
        }
    }

    fn ran(&self) {
        self.checks_run.set(self.checks_run.get() + 1);
    }

    fn knows_agent(&self, agent: &str) -> bool {
        self.agents.iter().any(|record| record.id == agent)
    }

    fn knows_version(&self, profile_version: &str) -> bool {
        self.profile_versions.iter().any(|held| held == profile_version)
    }
}

impl AgentRecords for World {
    fn agent(&self, agent: &str) -> Option<AgentRecord> {
        self.agents.iter().find(|record| record.id == agent).cloned()
    }
}

impl Admission for World {
    fn is_administrator(&self, caller: &str) -> bool {
        self.administrators.iter().any(|admin| admin == caller)
    }

    fn admits(&self, caller: &str) -> bool {
        self.admitted.iter().any(|admitted| admitted == caller)
    }
}

impl Lifecycles for World {
    fn state(&self, agent: &str) -> Option<LifecycleState> {
        self.ran();
        self.state.filter(|_| self.knows_agent(agent))
    }
}

impl ProfileReviews for World {
    fn review(&self, profile_version: &str) -> Option<Review> {
        self.ran();
        let reviewed = self.reviewed.as_ref()?;
        Some(if reviewed.iter().any(|held| held == profile_version) {
            Review::Reviewed
        } else {
            Review::NotReviewed
        })
    }
}

impl RoleMachines for World {
    fn roles(&self, agent: &str) -> Option<Vec<HeldRole>> {
        self.ran();
        self.roles.clone().filter(|_| self.knows_agent(agent))
    }
}

impl HandleRecords for World {
    fn handles(&self, agent: &str) -> HandleAnswer {
        self.ran();
        match &self.handles {
            Some(held) if self.knows_agent(agent) => HandleAnswer::Held(
                held.iter()
                    .map(|(id, valid, _)| HeldCredential {
                        id: id.clone(),
                        valid: *valid,
                    })
                    .collect(),
            ),
            Some(_) | None => HandleAnswer::RecordMissing,
        }
    }
}

impl ProfileNeeds for World {
    fn needs(&self, profile_version: &str) -> Option<Vec<String>> {
        self.needs.clone().filter(|_| self.knows_version(profile_version))
    }
}

impl EgressLists for World {
    fn egress(&self, machine: &str) -> Option<Vec<String>> {
        self.ran();
        let known = self.machines.iter().any(|held| held == machine);
        self.egress.clone().filter(|_| known)
    }
}

impl ProfileVersionRecords for World {
    fn executable(&self, profile_version: &str) -> Option<String> {
        self.executable
            .clone()
            .filter(|_| self.knows_version(profile_version))
    }

    fn arguments(&self, profile_version: &str) -> Option<Vec<String>> {
        self.arguments
            .clone()
            .filter(|_| self.knows_version(profile_version))
    }

    fn working_directory(&self, profile_version: &str) -> Option<String> {
        self.working_directory
            .clone()
            .filter(|_| self.knows_version(profile_version))
    }
}

impl SessionReports for World {
    fn reports(&self, launch_record: &str) -> Vec<SessionReport> {
        self.reports
            .borrow()
            .iter()
            .filter(|report| report.launch_record == launch_record)
            .cloned()
            .collect()
    }
}

/// A launch-record store in `dir`, signed by the key kept there.
fn store(dir: &Path) -> Result<LaunchRecords, Box<dyn Error>> {
    let key = Ed25519Identity::load_or_generate(&dir.join("service.key"))?;
    Ok(LaunchRecords::open(&dir.join("launch-records"), key)?)
}

/// The members of a request naming these three.
fn request(agent: &str, profile_version: &str, machine: &str) -> Vec<(String, String)> {
    vec![
        ("agent".to_owned(), agent.to_owned()),
        ("profile_version".to_owned(), profile_version.to_owned()),
        ("machine".to_owned(), machine.to_owned()),
    ]
}

fn refused(error: StartError) -> Result<lys_identity::start::Refused, Box<dyn Error>> {
    match error {
        StartError::Refused(refused) => Ok(refused),
        other => Err(format!("expected a refusal, got {other}").into()),
    }
}

#[test]
fn the_agent_is_active_passes_on_active() {
    let world = World::passing();
    assert_eq!(active::check(&world, "agent-fixture-1"), Ok(()));
}

#[test]
fn a_suspended_agent_is_refused_and_no_command_is_given() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut world = World::passing();
    world.state = Some(LifecycleState::Suspended);
    let mut records = store(dir.path())?;
    let members = request("agent-fixture-1", "pv-fixture-1", "machine-fixture-1");
    let error = give(&mut records, &world.owners(), "admin-fixture", members, 1)
        .expect_err("a suspended agent is refused");
    let refused = refused(error)?;
    assert_eq!(refused.refusals.len(), 1);
    let refusal = &refused.refusals[0];
    assert_eq!(refusal.name(), "agent_not_active");
    let words = refusal.to_string();
    assert!(words.contains("the agent is active"), "{words}");
    assert!(words.contains("suspended"), "{words}");
    assert!(!refused.to_json().contains("\"command\""));
    assert_eq!(records.record_count(), 0);
    Ok(())
}

#[test]
fn a_registered_agent_fails_the_agent_is_active() {
    let mut world = World::passing();
    world.state = Some(LifecycleState::Registered);
    let refusal = active::check(&world, "agent-fixture-1").expect_err("registered is not active");
    assert_eq!(refusal.name(), "agent_not_active");
    assert!(refusal.to_string().contains("registered"), "{refusal}");
}

#[test]
fn four_missing_owner_records_are_four_named_refusals() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut world = World::passing();
    world.reviewed = None;
    world.roles = None;
    world.handles = None;
    world.egress = None;
    let mut records = store(dir.path())?;
    let members = request("agent-fixture-1", "pv-fixture-1", "machine-fixture-1");
    let error = give(&mut records, &world.owners(), "admin-fixture", members, 1)
        .expect_err("missing records are refused");
    let refused = refused(error)?;
    let missing: Vec<String> = refused
        .refusals
        .iter()
        .filter(|refusal| refusal.name() == "check_record_missing")
        .map(ToString::to_string)
        .collect();
    assert_eq!(missing.len(), 4, "{refused}");
    assert_eq!(refused.refusals.len(), 4, "{refused}");
    for (words, card) in missing
        .iter()
        .zip(["Ink1H1Os", "Ink1H1Os", "SECRETS-002", "row 8.5"])
    {
        assert!(words.contains(card), "{words}");
    }
    assert!(refused.checks[0].passed());
    assert_eq!(records.record_count(), 0);
    Ok(())
}

#[test]
fn the_start_runs_exactly_five_checks_in_order() -> TestResult {
    let names: Vec<&str> = Check::ALL.into_iter().map(Check::name).collect();
    let order = [
        "the agent is active",
        "its profile version is reviewed",
        "the machine is allowed for the role",
        "its virtual credentials are valid",
        "the machine may reach what the profile needs",
    ];
    assert_eq!(names, order);
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    let members = request("agent-fixture-1", "pv-fixture-1", "machine-fixture-1");
    let given = give(&mut records, &world.owners(), "admin-fixture", members, 1)?;
    let ran: Vec<&str> = given.checks.iter().map(|report| report.check.name()).collect();
    assert_eq!(ran, order);
    assert!(given.checks.iter().all(|report| report.result() == "passed"));
    assert_eq!(world.checks_run.get(), 5);
    Ok(())
}
