//! DIRECTORY-011 R2: an agent registered before it ever runs appears in the
//! directory under its responsible person, and its record carries no session
//! credential.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use lys_core::Ed25519Identity;
use lys_identity::{
    Actor, AuthMethod, Directory, IdentityId, LoginBinding, OperationId, Profile, Provenance,
};
use lys_log_store::FileLeafStore;

type TestResult = Result<(), Box<dyn Error>>;

/// The contract's list of an agent record's members, the Reading table's
/// `Record::…` row at docs/design/identity/DIRECTORY-CONTRACT.md:36.
const CONTRACT: &str = include_str!("../../../docs/design/identity/DIRECTORY-CONTRACT.md");

fn contract_members() -> Result<Vec<String>, Box<dyn Error>> {
    let row = CONTRACT
        .lines()
        .find(|line| line.starts_with("| `Record::"))
        .ok_or("the contract has no Record row in its Reading table")?;
    let calls = row
        .split('|')
        .nth(1)
        .ok_or("the Record row has no call cell")?;
    Ok(calls
        .split(',')
        .map(|call| {
            call.trim()
                .trim_matches('`')
                .trim_start_matches("Record::")
                .trim_end_matches("()")
                .to_owned()
        })
        .collect())
}

/// The top-level member names of a record's debug rendering, which shows
/// every field the record holds.
fn record_members(rendered: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let body = rendered
        .strip_prefix("Record {")
        .and_then(|rest| rest.strip_suffix('}'))
        .ok_or("the read is not a Record")?;
    let mut members = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (index, ch) in body.char_indices() {
        match ch {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                members.push(body[start..index].to_owned());
                start = index + 1;
            }
            _ => {}
        }
    }
    members.push(body[start..].to_owned());
    members
        .iter()
        .filter(|member| !member.trim().is_empty())
        .map(|member| {
            member
                .split_once(':')
                .map(|(name, _)| name.trim().to_owned())
                .ok_or_else(|| format!("member without a name: {member}").into())
        })
        .collect()
}

struct Fixture {
    _dir: tempfile::TempDir,
    directory: Directory<FileLeafStore>,
}

fn fixture() -> Result<Fixture, Box<dyn Error>> {
    let dir = tempfile::TempDir::new()?;
    let log: PathBuf = dir.path().join("log");
    FileLeafStore::create(&log, "example.test/lys/directory")?;
    let key = dir.path().join("service.key");
    std::fs::write(&key, [11_u8; 32])?;
    std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
    let reopen = Box::new(move || FileLeafStore::open(&log));
    let directory = Directory::open(reopen, Ed25519Identity::load(&key)?)?;
    Ok(Fixture {
        _dir: dir,
        directory,
    })
}

fn administrator() -> Result<Actor, Box<dyn Error>> {
    Ok(Actor::new(
        LoginBinding::new("https://issuer.test", "administrator")?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    ))
}

#[test]
fn d011_r2_ac1() -> TestResult {
    let mut fixture = fixture()?;
    let directory = &mut fixture.directory;
    let (person, _) = directory.register_person(
        administrator()?,
        OperationId::from_bytes([1; 16]),
        Profile::new("Ada")?,
        10,
    )?;
    let (agent, _) = directory.register_agent(
        administrator()?,
        OperationId::from_bytes([2; 16]),
        person,
        Profile::new("Unstarted")?,
        11,
    )?;
    let agents: Vec<_> = directory
        .projection()?
        .records()
        .filter(|(id, _)| matches!(id, IdentityId::Agent(_)))
        .map(|(id, record)| (*id, record.responsible()))
        .collect();
    assert_eq!(agents, vec![(IdentityId::Agent(agent), Some(person))]);
    Ok(())
}

#[test]
fn d011_r2_ac2() -> TestResult {
    let mut fixture = fixture()?;
    let directory = &mut fixture.directory;
    let (person, _) = directory.register_person(
        administrator()?,
        OperationId::from_bytes([1; 16]),
        Profile::new("Ada")?,
        10,
    )?;
    let (agent, _) = directory.register_agent(
        administrator()?,
        OperationId::from_bytes([2; 16]),
        person,
        Profile::new("Unstarted")?,
        11,
    )?;
    let record = directory
        .record(IdentityId::Agent(agent))?
        .ok_or("the registered agent cannot be read")?;
    let expected = contract_members()?;
    assert!(!expected.is_empty());
    let members = record_members(&format!("{record:?}"))?;
    assert_eq!(members, expected);
    assert_eq!(
        members
            .iter()
            .filter(|name| name.contains("session"))
            .count(),
        0
    );
    Ok(())
}
