#![cfg(test)]
//! DIRECTORY-029 R1: a start request names the agent, the profile version
//! and the machine and nothing else, it resolves the agent to its enduring
//! record before anything runs, and every refusal is named without ever
//! carrying a credential value. CONFORMANCE 5.2 rides on the refusals here,
//! and CONFORMANCE 5.3 on the refusal redaction test.

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::path::Path;

use lys_core::Ed25519Identity;
use lys_identity::LifecycleState;
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::checks::Check;
use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential};
use lys_identity::start::egress::{EgressLists, ProfileNeeds};
use lys_identity::start::error::{REFUSAL_NAMES, Refusal};
use lys_identity::start::machine_role::{HeldRole, RoleMachines};
use lys_identity::start::profile_command::ProfileVersionRecords;
use lys_identity::start::profile_review::{ProfileReviews, Review};
use lys_identity::start::request::{AgentRecord, AgentRecords, StartRequest};
use lys_identity::start::state::{LaunchRecords, SessionReport, SessionReports};
use lys_identity::start::{Grammars, Owners, StartError, give};

type TestResult = Result<(), Box<dyn Error>>;

/// A value a door keeps beside a credential id; no refusal ever holds it.
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

fn owned(members: &[(&str, &str)]) -> Vec<(String, String)> {
    members
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn refusal(error: StartError) -> Result<Refusal, Box<dyn Error>> {
    let StartError::Refused(refused) = error else {
        return Err(format!("expected a refusal, got {error}").into());
    };
    assert_eq!(refused.refusals.len(), 1, "{refused}");
    Ok(refused.refusals[0].clone())
}

#[test]
fn three_members_parse_into_a_start_request() -> TestResult {
    let request = StartRequest::from_members(request(
        "agent-fixture-1",
        "pv-fixture-1",
        "machine-fixture-1",
    ))?;
    assert_eq!(
        request,
        StartRequest::new("agent-fixture-1", "pv-fixture-1", "machine-fixture-1")
    );
    assert_eq!(request.agent(), "agent-fixture-1");
    assert_eq!(request.profile_version(), "pv-fixture-1");
    assert_eq!(request.machine(), "machine-fixture-1");
    Ok(())
}

#[test]
fn an_executable_or_a_working_directory_in_the_request_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    for (member, value) in [("executable", "/bin/sh"), ("working_directory", "/tmp")] {
        let mut members = request("agent-fixture-1", "pv-fixture-1", "machine-fixture-1");
        members.push((member.to_owned(), value.to_owned()));
        let error = give(&mut records, &world.owners(), "admin-fixture", members, 1)
            .expect_err("a fourth member is refused");
        let refused = refusal(error)?;
        assert_eq!(refused.name(), "start_field_not_allowed");
        assert!(refused.to_string().contains(member), "{refused}");
        assert_eq!(records.record_count(), 0);
    }
    assert_eq!(world.checks_run.get(), 0);
    Ok(())
}

#[test]
fn a_request_without_its_machine_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    let members = owned(&[
        ("agent", "agent-fixture-1"),
        ("profile_version", "pv-fixture-1"),
    ]);
    let error = give(&mut records, &world.owners(), "admin-fixture", members, 1)
        .expect_err("a missing member is refused");
    let refused = refusal(error)?;
    assert_eq!(refused.name(), "start_member_missing");
    assert!(refused.to_string().contains("machine"), "{refused}");
    assert_eq!(records.record_count(), 0);
    Ok(())
}

#[test]
fn an_agent_with_no_record_is_refused_and_nothing_is_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    let before = format!("{:?}", world.agents);
    let members = request("agent-fixture-missing", "pv-fixture-1", "machine-fixture-1");
    let error = give(&mut records, &world.owners(), "admin-fixture", members, 1)
        .expect_err("an unknown agent is refused");
    let refused = refusal(error)?;
    assert_eq!(refused.name(), "agent_unknown");
    assert!(
        refused.to_string().contains("agent-fixture-missing"),
        "{refused}"
    );
    assert_eq!(world.checks_run.get(), 0);
    assert_eq!(records.record_count(), 0);
    assert_eq!(records.signed_events().len(), 0);
    assert_eq!(world.agents.len(), 1);
    assert_eq!(format!("{:?}", world.agents), before);
    assert!(world.reports.borrow().is_empty());
    Ok(())
}

/// One of each of the fourteen refusals, built with the fixture value
/// `VALUE_ONE` in scope, as it is when a door's answer carrying it is refused:
/// no refusal has a field that could hold it.
fn every_refusal() -> Vec<Refusal> {
    vec![
        Refusal::StartFieldNotAllowed {
            member: "executable".to_owned(),
        },
        Refusal::StartMemberMissing { member: "machine" },
        Refusal::AgentUnknown {
            agent: "agent-fixture-missing".to_owned(),
        },
        Refusal::StartRightMissing {
            agent: "agent-fixture-1".to_owned(),
        },
        Refusal::AgentNotActive {
            state: "suspended".to_owned(),
        },
        Refusal::ProfileVersionNotReviewed {
            profile_version: "pv-fixture-2".to_owned(),
        },
        Refusal::MachineNotAllowedForRole {
            machine: "machine-fixture-2".to_owned(),
            roles: texts(&["role-fixture-builder"]),
        },
        Refusal::VirtualCredentialsNotValid {
            agent: "agent-fixture-1".to_owned(),
        },
        Refusal::EgressNotReachable {
            machine: "machine-fixture-1".to_owned(),
            destinations: texts(&["mcp-fixture"]),
        },
        Check::VirtualCredentialsAreValid.record_missing(),
        Refusal::ProfileVersionFieldMissing {
            profile_version: "pv-fixture-1".to_owned(),
            field: "working directory",
        },
        Refusal::CommandValueOutsideGrammar {
            assignment: "LYS_CREDENTIAL_IDS",
        },
        Refusal::StartUnconfirmed {
            launch_record: "launch-00000000000000000000000000000001".to_owned(),
        },
        Refusal::CredentialValueInAnswer {
            record: "vc-fixture-1".to_owned(),
            field: "value".to_owned(),
        },
    ]
}

#[test]
fn every_refusal_is_named_and_none_carries_a_credential_value() {
    let refusals = every_refusal();
    assert_eq!(refusals.len(), 14);
    let names: Vec<&str> = refusals.iter().map(Refusal::name).collect();
    assert_eq!(names, REFUSAL_NAMES);
    let mut checked = 0;
    for refusal in &refusals {
        let shown = format!("{refusal} {refusal:?} {}", refusal.to_json());
        assert!(shown.starts_with(refusal.name()), "{shown}");
        assert!(!shown.contains(VALUE_ONE), "{shown}");
        checked += 1;
    }
    assert_eq!(checked, 14);
}
