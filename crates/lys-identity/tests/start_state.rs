#![cfg(test)]
//! DIRECTORY-029 R10: a launch record reads running only on a verified signed
//! report naming it and its agent (CONFORMANCE 5.5), unconfirmed without one
//! (CONFORMANCE 5.6), and withdrawn by a signed withdrawal that never says
//! the agent did not start; the answer for an agent says which record
//! stands, and no derivation writes a session record or an agent record.

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
use lys_identity::start::state::{LaunchState, Reading, standing};
use lys_identity::start::{Grammars, Owners};
use lys_identity::start::{StartError, give, give_again, state_of, withdraw};

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
        self.profile_versions
            .iter()
            .any(|held| held == profile_version)
    }
}

impl AgentRecords for World {
    fn agent(&self, agent: &str) -> Option<AgentRecord> {
        self.agents
            .iter()
            .find(|record| record.id == agent)
            .cloned()
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
        self.needs
            .clone()
            .filter(|_| self.knows_version(profile_version))
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

fn report(session: &str, agent: &str, launch_record: &str, verified: bool) -> SessionReport {
    SessionReport {
        session: session.to_owned(),
        agent: agent.to_owned(),
        launch_record: launch_record.to_owned(),
        verified,
    }
}

/// A world, its store, and launch record L1 given for agent-fixture-1.
fn given_l1(dir: &Path) -> Result<(World, LaunchRecords, String), Box<dyn Error>> {
    let world = World::passing();
    let mut records = store(dir)?;
    let members = request("agent-fixture-1", "pv-fixture-1", "machine-fixture-1");
    let given = give(&mut records, &world.owners(), "admin-fixture", members, 10)?;
    Ok((world, records, given.record.id))
}

fn state(world: &World, records: &LaunchRecords, id: &str) -> Result<LaunchState, StartError> {
    state_of(records, id, world.owners().sessions)
}

#[test]
fn with_no_report_a_record_reads_unconfirmed() -> TestResult {
    // CONFORMANCE 5.6.
    let dir = tempfile::tempdir()?;
    let (world, records, l1) = given_l1(dir.path())?;
    let read = state(&world, &records, &l1)?;
    assert_eq!(read.reading, Reading::Unconfirmed);
    assert_eq!(read.name(), "unconfirmed");
    Ok(())
}

#[test]
fn a_verified_report_naming_the_agent_and_the_record_reads_running() -> TestResult {
    // CONFORMANCE 5.5.
    let dir = tempfile::tempdir()?;
    let (world, records, l1) = given_l1(dir.path())?;
    world
        .reports
        .borrow_mut()
        .push(report("sess-fixture-1", "agent-fixture-1", &l1, true));
    let read = state(&world, &records, &l1)?;
    assert_eq!(
        read.reading,
        Reading::Running {
            session: "sess-fixture-1".to_owned()
        }
    );
    Ok(())
}

#[test]
fn a_report_naming_another_agent_or_not_verified_leaves_it_unconfirmed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, records, l1) = given_l1(dir.path())?;
    let mut read = 0;
    for (agent, verified) in [("agent-fixture-2", true), ("agent-fixture-1", false)] {
        world.reports.borrow_mut().clear();
        world
            .reports
            .borrow_mut()
            .push(report("sess-fixture-1", agent, &l1, verified));
        assert_eq!(state(&world, &records, &l1)?.reading, Reading::Unconfirmed);
        read += 1;
    }
    assert_eq!(read, 2);
    Ok(())
}

#[test]
fn a_report_naming_l2_only_leaves_l1_unconfirmed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, mut records, l1) = given_l1(dir.path())?;
    // L1 reads running while L2 is given, so L2 is not refused; then the
    // sessions record holds a report naming L2 only.
    world
        .reports
        .borrow_mut()
        .push(report("sess-fixture-1", "agent-fixture-1", &l1, true));
    let members = request("agent-fixture-1", "pv-fixture-1", "machine-fixture-1");
    let l2 = give(&mut records, &world.owners(), "admin-fixture", members, 11)?
        .record
        .id;
    world.reports.borrow_mut().clear();
    world
        .reports
        .borrow_mut()
        .push(report("sess-fixture-2", "agent-fixture-1", &l2, true));
    assert_eq!(state(&world, &records, &l1)?.reading, Reading::Unconfirmed);
    assert_eq!(state(&world, &records, &l2)?.name(), "running");
    Ok(())
}

#[test]
fn the_answer_for_the_agent_names_the_record_that_stands() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, records, l1) = given_l1(dir.path())?;
    assert_eq!(
        standing(&records, "agent-fixture-1", world.owners().sessions),
        Some(l1)
    );
    Ok(())
}

#[test]
fn a_withdrawal_names_who_and_when_and_nothing_stands_after() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, mut records, l1) = given_l1(dir.path())?;
    let after = withdraw(&mut records, &world.owners(), "admin-fixture", &l1, 42)?;
    assert_eq!(after.reading, Reading::Withdrawn);
    assert_eq!(state(&world, &records, &l1)?.name(), "withdrawn");
    let withdrawal = records.withdrawal(&l1).ok_or("no withdrawal is kept")?;
    assert_eq!(withdrawal.by, "admin-fixture");
    assert_eq!(withdrawal.at, 42);
    assert_eq!(records.withdrawal_count(), 1);
    assert_eq!(
        standing(&records, "agent-fixture-1", world.owners().sessions),
        None
    );
    Ok(())
}

#[test]
fn anyone_else_is_refused_a_withdrawal_and_nothing_is_kept() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, mut records, l1) = given_l1(dir.path())?;
    let error = withdraw(&mut records, &world.owners(), "person-fixture-2", &l1, 42)
        .expect_err("person-fixture-2 holds no right");
    let refused = error.refused().ok_or("expected a refusal")?;
    assert!(refused.names("start_right_missing"), "{refused}");
    assert_eq!(state(&world, &records, &l1)?.reading, Reading::Unconfirmed);
    assert_eq!(records.withdrawal_count(), 0);
    Ok(())
}

#[test]
fn a_report_after_a_withdrawal_reads_running_with_the_withdrawal_beside_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, mut records, l1) = given_l1(dir.path())?;
    withdraw(&mut records, &world.owners(), "admin-fixture", &l1, 42)?;
    world
        .reports
        .borrow_mut()
        .push(report("sess-fixture-1", "agent-fixture-1", &l1, true));
    let read = state(&world, &records, &l1)?;
    assert_eq!(read.name(), "running");
    let beside = read
        .withdrawal
        .as_ref()
        .ok_or("the withdrawal is not beside it")?;
    assert_eq!(beside.by, "admin-fixture");
    assert!(
        read.to_json()
            .contains("\"withdrawal\":{\"by\":\"admin-fixture\"")
    );
    Ok(())
}

#[test]
fn a_record_reading_running_does_not_stand() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, records, l1) = given_l1(dir.path())?;
    world
        .reports
        .borrow_mut()
        .push(report("sess-fixture-1", "agent-fixture-1", &l1, true));
    assert_eq!(
        standing(&records, "agent-fixture-1", world.owners().sessions),
        None
    );
    Ok(())
}

#[test]
fn no_state_says_the_agent_did_not_start_and_nothing_else_is_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (world, mut records, l1) = given_l1(dir.path())?;
    let sessions_before = format!("{:?}", world.reports.borrow());
    let agent_before = format!("{:?}", world.agents);
    let mut reached = vec![state(&world, &records, &l1)?];
    let again = withdraw(&mut records, &world.owners(), "admin-fixture", &l1, 42)?;
    reached.push(again);
    let l2 = give_again(&mut records, &world.owners(), "admin-fixture", &l1, 43)?;
    reached.push(l2.state);
    assert_eq!(format!("{:?}", world.reports.borrow()), sessions_before);
    assert_eq!(format!("{:?}", world.agents), agent_before);
    world
        .reports
        .borrow_mut()
        .push(report("sess-fixture-1", "agent-fixture-1", &l1, true));
    reached.push(state(&world, &records, &l1)?);
    let names: Vec<&str> = reached.iter().map(LaunchState::name).collect();
    assert_eq!(
        names,
        ["unconfirmed", "withdrawn", "unconfirmed", "running"]
    );
    for read in &reached {
        let shown = format!("{} {} {}", read.name(), read.words(), read.to_json()).to_lowercase();
        assert!(!shown.contains("not started"), "{shown}");
        assert!(!shown.contains("not_started"), "{shown}");
    }
    assert_eq!(world.agents.len(), 1);
    Ok(())
}
