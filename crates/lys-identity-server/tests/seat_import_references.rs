#![cfg(test)]
//! AGENTS-003 R3: a seat's credentials are kept as references. The declared
//! seat-identity file is recorded by path and judged by its metadata alone,
//! never opened; a declared broker handle is accepted only when the approved
//! broker's handle record holds it active for the seat's agent; the person
//! answering for the seat comes from the directory, never from a path. Every
//! fixture is fabricated in a temporary folder and every value is fake.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use lys_identity::start::credentials::{HandleAnswer, HandleRecords, HeldCredential};
use lys_identity_server::seat_import_plan::{CredentialReference, Fragment, Harness, Manifest};
use lys_identity_server::seat_import_references::{SeatFacts, read_references_from};

type TestResult = Result<(), Box<dyn Error>>;

/// The fabricated private key every seat-identity fixture holds; it must
/// never appear in any answer.
const CANARY: &str = "fixture-private-key-canary-41c2";

const AGENT: &str = "agent-fixture";
const PERSON: &str = "person-fixture";

/// A handle record that answers one fixed answer and counts its reads.
struct Broker {
    answer: HandleAnswer,
    reads: Cell<usize>,
}

impl Broker {
    fn answering(answer: HandleAnswer) -> Self {
        Self {
            answer,
            reads: Cell::new(0),
        }
    }
}

impl HandleRecords for Broker {
    fn handles(&self, agent: &str) -> HandleAnswer {
        assert_eq!(agent, AGENT);
        self.reads.set(self.reads.get() + 1);
        self.answer.clone()
    }
}

fn held(id: &str, valid: bool) -> HeldCredential {
    HeldCredential {
        id: id.to_owned(),
        valid,
    }
}

fn facts() -> SeatFacts {
    SeatFacts {
        agent: AGENT.to_owned(),
        responsible: Some(PERSON.to_owned()),
        recorded_identity: Some(AGENT.to_owned()),
        door_identity: None,
    }
}

fn manifest(seat_file: Option<PathBuf>, handles: &[&str]) -> Manifest {
    Manifest {
        seat: "fixture-seat".to_owned(),
        harness: Harness::Claude,
        codex_profile: None,
        claude_folder: None,
        seat_identity_file: seat_file,
        monitor: None,
        replaced_env_prefixes: Vec::new(),
        secret_handles: handles.iter().map(|handle| (*handle).to_owned()).collect(),
        recipient_map: BTreeMap::new(),
    }
}

/// A seat-identity file holding the canary at `mode`, and the user that
/// owns it.
fn seat_file(dir: &Path, mode: u32) -> Result<(PathBuf, u32), Box<dyn Error>> {
    let path = dir.join("fixture.seat.json");
    fs::write(&path, format!(r#"{{ "privateKeyPem": "{CANARY}" }}"#))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(mode))?;
    let owner = fs::symlink_metadata(&path)?.uid();
    Ok((path, owner))
}

fn reference<'a>(fragment: &'a Fragment, kind: &str) -> Result<&'a CredentialReference, String> {
    fragment
        .references
        .iter()
        .find(|one| one.kind == kind)
        .ok_or(format!("no {kind} reference"))
}

fn refused(fragment: &Fragment, name: &str, member: &str) -> bool {
    fragment
        .refusals
        .iter()
        .any(|one| one.name == name && one.member == member)
}

fn quiet(fragment: &Fragment) -> TestResult {
    let answer = serde_json::to_string(fragment)?;
    assert!(!answer.contains(CANARY), "the canary reached the answer");
    Ok(())
}

#[test]
fn seat_import_keys_reference_only() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, owner) = seat_file(dir.path(), 0o600)?;
    let broker = Broker::answering(HandleAnswer::Held(vec![held("handle-1", true)]));
    let given = manifest(Some(path.clone()), &["handle-1"]);
    let fragment = read_references_from(&given, &facts(), Some(&broker), owner);

    assert!(fragment.refusals.is_empty(), "{:?}", fragment.refusals);
    let file = reference(&fragment, "seat_identity_file")?;
    assert_eq!(file.locator, path.display().to_string());
    assert_eq!(file.owner, PERSON);
    assert_eq!(file.public_identity.as_deref(), Some(AGENT));
    assert!(file.usable);
    let handle = reference(&fragment, "broker_handle")?;
    assert_eq!(handle.locator, "handle-1");
    assert_eq!(handle.owner, PERSON);
    assert!(handle.usable);
    assert_eq!(broker.reads.get(), 1);
    assert!(
        fragment
            .prerequisites
            .iter()
            .any(|one| one.contains("not confirmed against the door"))
    );
    quiet(&fragment)
}

#[test]
fn seat_import_references_judge_the_file_by_metadata_alone() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, owner) = seat_file(dir.path(), 0o000)?;
    let given = manifest(Some(path.clone()), &[]);
    let fragment = read_references_from(&given, &facts(), None, owner);

    let file = reference(&fragment, "seat_identity_file")?;
    assert!(!file.usable);
    // The refusal names the mode read from the metadata: had the file been
    // opened, a mode-000 file would have answered permission denied instead.
    let reason = file.reason.as_deref().ok_or("an unusable file has a reason")?;
    assert!(reason.contains("0000"), "{reason}");
    let locator = path.display().to_string();
    assert!(refused(&fragment, "import_reference_unusable", &locator));
    quiet(&fragment)
}

#[test]
fn seat_import_keys_refuses_wrong_owner() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (path, owner) = seat_file(dir.path(), 0o600)?;
    let locator = path.display().to_string();
    let given = manifest(Some(path.clone()), &[]);

    let fragment = read_references_from(&given, &facts(), None, owner + 1);
    assert!(refused(&fragment, "import_reference_unusable", &locator));
    let reason = reference(&fragment, "seat_identity_file")?.reason.clone();
    assert!(reason.is_some_and(|reason| reason.contains("owned by user")));

    let mut bound_elsewhere = facts();
    bound_elsewhere.door_identity = Some("agent-elsewhere".to_owned());
    let fragment = read_references_from(&given, &bound_elsewhere, None, owner);
    assert!(refused(&fragment, "import_reference_identity_mismatch", &locator));
    let file = reference(&fragment, "seat_identity_file")?;
    assert!(!file.usable);
    assert_eq!(file.public_identity.as_deref(), Some(AGENT));

    let mut unanswered = facts();
    unanswered.responsible = None;
    let fragment = read_references_from(&given, &unanswered, None, owner);
    assert!(refused(&fragment, "import_responsible_missing", "fixture-seat"));

    let missing = dir.path().join("absent.seat.json");
    let absent = manifest(Some(missing.clone()), &[]);
    let fragment = read_references_from(&absent, &facts(), None, owner);
    let locator = missing.display().to_string();
    assert!(refused(&fragment, "import_reference_unusable", &locator));

    let linked = dir.path().join("linked.seat.json");
    std::os::unix::fs::symlink(&path, &linked)?;
    let through_link = manifest(Some(linked), &[]);
    let fragment = read_references_from(&through_link, &facts(), None, owner);
    let reason = reference(&fragment, "seat_identity_file")?.reason.clone();
    assert!(reason.is_some_and(|reason| reason.contains("symbolic link")));
    quiet(&fragment)
}

#[test]
fn seat_import_references_resolve_handles_only_through_the_broker() -> TestResult {
    let cases = [
        (
            HandleAnswer::Held(vec![held("handle-1", false)]),
            "import_reference_unresolved",
        ),
        (
            HandleAnswer::Held(vec![held("handle-2", true)]),
            "import_reference_unresolved",
        ),
        (HandleAnswer::RecordMissing, "import_reference_unresolved"),
        (
            HandleAnswer::ValueInAnswer {
                record: "handle-1".to_owned(),
                field: "value".to_owned(),
            },
            "credential_value_in_answer",
        ),
        (
            HandleAnswer::Unreadable {
                reason: "fixture refusal".to_owned(),
            },
            "import_reference_unreadable",
        ),
    ];
    for (answer, name) in cases {
        let broker = Broker::answering(answer);
        let given = manifest(None, &["handle-1", "bad handle"]);
        let fragment = read_references_from(&given, &facts(), Some(&broker), 0);
        assert!(refused(&fragment, name, "handle-1"), "{name}: {:?}", fragment.refusals);
        assert!(refused(&fragment, "import_reference_unresolved", "bad handle"));
        assert_eq!(broker.reads.get(), 1);
        assert_eq!(fragment.references.len(), 2);
        assert!(fragment.references.iter().all(|one| !one.usable));
    }

    let fragment = read_references_from(&manifest(None, &["handle-1"]), &facts(), None, 0);
    assert!(refused(&fragment, "import_reference_unresolved", "handle-1"));
    let reason = reference(&fragment, "broker_handle")?.reason.clone();
    assert!(reason.is_some_and(|reason| reason.contains("no door address")));
    Ok(())
}
