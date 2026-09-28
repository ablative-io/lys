#![cfg(test)]
//! DIRECTORY-029 R11: a launch record is kept for every command given and
//! reads back after a restart (CONFORMANCE 5.4); the command is rendered from
//! it as `env` and three id assignments before the profile version's own
//! executable and its recorded arguments, with no credential value on it
//! (CONFORMANCE 5.3); a second start is refused while one stands unconfirmed
//! (CONFORMANCE 5.6); and no start writes a session record or changes the
//! agent record.

use std::cell::{Cell, RefCell};
use std::error::Error;
use std::num::NonZeroU64;
use std::path::Path;

use lys_core::Ed25519Identity;
use lys_identity::LifecycleState;
use lys_identity::start::active::Lifecycles;
use lys_identity::start::authority::Admission;
use lys_identity::start::command::render;
use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential};
use lys_identity::start::egress::{EgressLists, ProfileNeeds};
use lys_identity::start::machine_role::{HeldRole, RoleMachines};
use lys_identity::start::profile_command::ProfileVersionRecords;
use lys_identity::start::profile_review::{ProfileReviews, Review};
use lys_identity::start::request::{AgentRecord, AgentRecords};
use lys_identity::start::state::{LaunchRecords, SessionReport, SessionReports};
use lys_identity::start::state::{Reading, state_of};
use lys_identity::start::{Given, StartError, give, give_again, withdraw};
use lys_identity::start::{Grammars, Owners};
use lys_log_store::{FileLeafStore, Start};

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

const VALUE_TWO: &str = "fixture-credential-value-2b7c";

/// `line` split into words as a POSIX shell splits it: blanks separate words,
/// single quotes keep everything to the next single quote, double quotes
/// keep everything but a backslash-escaped `"`, `\` or `$`, and a backslash
/// outside quotes keeps the next character.
fn shell_words(line: &str) -> Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' | '\n' => {
                if let Some(done) = word.take() {
                    words.push(done);
                }
            }
            '\'' => {
                let held = word.get_or_insert_with(String::new);
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(inner) => held.push(inner),
                        None => return Err("an unclosed single quote".to_owned()),
                    }
                }
            }
            '"' => {
                let held = word.get_or_insert_with(String::new);
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(escaped @ ('"' | '\\' | '$')) => held.push(escaped),
                            Some(other) => {
                                held.push('\\');
                                held.push(other);
                            }
                            None => return Err("an unclosed double quote".to_owned()),
                        },
                        Some(inner) => held.push(inner),
                        None => return Err("an unclosed double quote".to_owned()),
                    }
                }
            }
            '\\' => {
                let next = chars.next().ok_or("a trailing backslash")?;
                word.get_or_insert_with(String::new).push(next);
            }
            other => word.get_or_insert_with(String::new).push(other),
        }
    }
    words.extend(word);
    Ok(words)
}

fn given(world: &World, records: &mut LaunchRecords, machine: &str) -> Result<Given, StartError> {
    let members = request("agent-fixture-1", "pv-fixture-1", machine);
    give(records, &world.owners(), "admin-fixture", members, 20)
}

fn running(session: &str, launch_record: &str) -> SessionReport {
    SessionReport {
        session: session.to_owned(),
        agent: "agent-fixture-1".to_owned(),
        launch_record: launch_record.to_owned(),
        verified: true,
    }
}

#[test]
fn a_start_keeps_exactly_one_launch_record_naming_what_it_gave() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let agent_before = format!("{:?}", world.agents);
    let mut records = store(dir.path())?;
    let given = given(&world, &mut records, "machine-fixture-1")?;
    assert_eq!(records.record_count(), 1);
    let kept = records
        .record(&given.record.id)
        .ok_or("the record is not kept")?;
    assert_eq!(kept.agent, "agent-fixture-1");
    assert_eq!(kept.machine, "machine-fixture-1");
    assert_eq!(kept.executable, "fixture-exec");
    assert_eq!(kept.working_directory, "fixture-cwd");
    assert_eq!(kept.profile_version, "pv-fixture-1");
    assert_eq!(kept.credential_ids, ["vc-fixture-1"]);
    assert_eq!(kept.given_by, "admin-fixture");
    assert_eq!(kept.copied_from, None);
    assert_eq!(format!("{:?}", world.agents), agent_before);
    assert_eq!(world.agents.len(), 1);
    Ok(())
}

#[test]
fn the_command_splits_into_env_three_ids_and_the_recorded_command() -> TestResult {
    // CONFORMANCE 5.3: no credential value is on the command line.
    let dir = tempfile::tempdir()?;
    let mut world = World::passing();
    world.handles = Some(vec![
        ("vc-fixture-1".to_owned(), true, VALUE_ONE.to_owned()),
        ("vc-fixture-2".to_owned(), true, VALUE_TWO.to_owned()),
    ]);
    let mut records = store(dir.path())?;
    let given = given(&world, &mut records, "machine-fixture-1")?;
    let words = shell_words(&given.command)?;
    let launch = format!("LYS_LAUNCH_RECORD={}", given.record.id);
    assert_eq!(
        words,
        [
            "env",
            "LYS_AGENT_ID=agent-fixture-1",
            launch.as_str(),
            "LYS_CREDENTIAL_IDS=vc-fixture-1,vc-fixture-2",
            "fixture-exec",
            "--fixture-arg",
            "two words",
        ]
    );
    assert_eq!(given.working_directory(), "fixture-cwd");
    let answer = given.to_json();
    for value in [VALUE_ONE, VALUE_TWO] {
        assert!(!given.command.contains(value));
        assert!(!answer.contains(value));
    }
    assert_eq!(given.record.executable, "fixture-exec");
    assert_eq!(given.record.arguments, ["--fixture-arg", "two words"]);
    assert_eq!(given.record.working_directory, "fixture-cwd");
    assert_eq!(words[0], "env");
    assert_ne!(words[0], "lys");
    Ok(())
}

#[test]
fn a_credential_id_outside_its_grammar_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut world = World::passing();
    world.handles = Some(vec![(
        "vc-fixture;1".to_owned(),
        true,
        VALUE_ONE.to_owned(),
    )]);
    let mut records = store(dir.path())?;
    let error = given(&world, &mut records, "machine-fixture-1").expect_err("outside its grammar");
    let refused = error.refused().ok_or("expected a refusal")?;
    assert!(refused.names("command_value_outside_grammar"), "{refused}");
    assert!(
        refused.to_string().contains("LYS_CREDENTIAL_IDS"),
        "{refused}"
    );
    assert!(!refused.to_json().contains("\"command\""));
    assert_eq!(records.record_count(), 0);
    Ok(())
}

#[test]
fn a_record_renders_the_same_bytes_every_time_and_reads_back_after_a_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let kept = {
        let mut records = store(dir.path())?;
        let given = given(&world, &mut records, "machine-fixture-1")?;
        assert_eq!(render(&given.record), render(&given.record));
        assert_eq!(render(&given.record), given.command);
        given.record
    };
    let reopened = store(dir.path())?;
    let read = reopened
        .record(&kept.id)
        .ok_or("the record does not read back")?;
    assert_eq!(read, &kept);
    Ok(())
}

#[test]
fn a_restart_reads_only_the_entries_after_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let log = dir.path().join("launch-records");
    // The first open creates the log and seals its empty state.
    store(dir.path())?;
    let every = NonZeroU64::new(2).ok_or("two is not zero")?;
    let open = || -> Result<LaunchRecords, Box<dyn Error>> {
        let key = Ed25519Identity::load_or_generate(&dir.path().join("service.key"))?;
        let at = log.clone();
        Ok(LaunchRecords::over(
            Box::new(move || FileLeafStore::open(&at)),
            key,
            every,
        )?)
    };
    let mut records = open()?;
    let l1 = given(&world, &mut records, "machine-fixture-1")?.record.id;
    withdraw(&mut records, &world.owners(), "admin-fixture", &l1, 21)?;
    let l2 = given(&world, &mut records, "machine-fixture-1")?.record.id;
    drop(records);
    let reopened = open()?;
    assert_eq!(
        reopened.start(),
        &Start::Resumed {
            size: 2,
            replayed: 1
        }
    );
    assert_eq!(reopened.record_count(), 2);
    assert!(reopened.record(&l2).is_some());
    assert!(reopened.withdrawal(&l1).is_some());
    Ok(())
}

#[test]
fn a_start_given_again_is_a_new_record_naming_its_source() -> TestResult {
    // CONFORMANCE 5.4.
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    let l1 = given(&world, &mut records, "machine-fixture-1")?.record;
    withdraw(&mut records, &world.owners(), "admin-fixture", &l1.id, 21)?;
    let l2 = give_again(&mut records, &world.owners(), "admin-fixture", &l1.id, 22)?.record;
    assert_ne!(l1.id, l2.id);
    assert_eq!(l2.copied_from.as_deref(), Some(l1.id.as_str()));
    assert_eq!(
        (&l1.machine, &l1.executable, &l1.working_directory),
        (&l2.machine, &l2.executable, &l2.working_directory)
    );
    assert_eq!(
        (&l1.profile_version, &l1.credential_ids),
        (&l2.profile_version, &l2.credential_ids)
    );
    assert_eq!(records.record_count(), 2);
    Ok(())
}

#[test]
fn a_second_start_is_refused_while_one_stands_unconfirmed() -> TestResult {
    // CONFORMANCE 5.6.
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    let l1 = given(&world, &mut records, "machine-fixture-1")?.record.id;
    let error = given(&world, &mut records, "machine-fixture-2").expect_err("L1 stands");
    let refused = error.refused().ok_or("expected a refusal")?;
    assert!(refused.names("start_unconfirmed"), "{refused}");
    let words = refused.to_string();
    for part in [l1.as_str(), "wait for its report", "withdraw"] {
        assert!(words.contains(part), "{words}");
    }
    assert_eq!(records.record_count(), 1);
    Ok(())
}

#[test]
fn after_a_withdrawal_a_new_start_is_given() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    let l1 = given(&world, &mut records, "machine-fixture-1")?.record.id;
    withdraw(&mut records, &world.owners(), "admin-fixture", &l1, 21)?;
    given(&world, &mut records, "machine-fixture-2")?;
    assert_eq!(records.record_count(), 2);
    Ok(())
}

#[test]
fn a_start_of_a_running_agent_keeps_a_new_record_and_writes_no_session() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let agent_before = format!("{:?}", world.agents);
    let mut records = store(dir.path())?;
    let l1 = given(&world, &mut records, "machine-fixture-1")?.record.id;
    world
        .reports
        .borrow_mut()
        .push(running("sess-fixture-1", &l1));
    let sessions_before = format!("{:?}", world.reports.borrow());
    let l2 = given(&world, &mut records, "machine-fixture-1")?.record;
    assert_ne!(l2.id, l1);
    assert_eq!(l2.agent, "agent-fixture-1");
    let sessions = world.owners().sessions;
    let first = state_of(&records, &l1, sessions)?;
    assert_eq!(
        first.reading,
        Reading::Running {
            session: "sess-fixture-1".to_owned()
        }
    );
    assert_eq!(format!("{:?}", world.reports.borrow()), sessions_before);
    assert_eq!(world.agents.len(), 1);
    assert_eq!(format!("{:?}", world.agents), agent_before);
    world
        .reports
        .borrow_mut()
        .push(running("sess-fixture-2", &l2.id));
    for (id, session) in [(&l1, "sess-fixture-1"), (&l2.id, "sess-fixture-2")] {
        assert_eq!(
            state_of(&records, id, sessions)?.reading,
            Reading::Running {
                session: session.to_owned()
            }
        );
    }
    Ok(())
}

#[test]
fn no_signed_launch_event_holds_a_credential_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let world = World::passing();
    let mut records = store(dir.path())?;
    let l1 = given(&world, &mut records, "machine-fixture-1")?.record.id;
    withdraw(&mut records, &world.owners(), "admin-fixture", &l1, 21)?;
    give_again(&mut records, &world.owners(), "admin-fixture", &l1, 22)?;
    let events = records.signed_events();
    assert_eq!(events.len(), 3);
    for event in events {
        let found = event
            .windows(VALUE_ONE.len())
            .any(|window| window == VALUE_ONE.as_bytes());
        assert!(!found);
    }
    Ok(())
}
