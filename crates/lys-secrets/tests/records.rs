//! Sealed records: a memory record is read by name, in the piece asked for,
//! by an identity holding the read relation; a key is never read; an
//! unknown name is `NotFound`; a removed relation is named `RelationRemoved`,
//! also after a restart; and every read is one audit line.

use lys_secrets::{
    AuditKind, Broker, BrokerPaths, EntryClass, LocalGrants, Relation, Secret, SecretRelation,
    SecretsError,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn paths(root: &TempDir) -> Result<BrokerPaths, std::io::Error> {
    let keys = root.path().join("keys");
    std::fs::create_dir_all(&keys)?;
    Ok(BrokerPaths {
        store_dir: root.path().join("store"),
        log_dir: root.path().join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    })
}

fn reader() -> LocalGrants {
    let grants = LocalGrants::new();
    for record in ["notes", "signing-key"] {
        grants.grant_as(
            Relation::Read,
            SecretRelation {
                identity: "agent:noor".to_owned(),
                secret: record.to_owned(),
                granted_by: Some("person:tom".to_owned()),
            },
        );
    }
    grants
}

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

fn sealed(paths: &BrokerPaths) -> Result<Broker<LocalGrants>, SecretsError> {
    let mut broker = Broker::create(paths, reader(), Box::new(|| 1))?;
    broker.seal_record(
        "notes",
        EntryClass::Memory,
        "person:tom",
        &Secret::from_slice(b"the meeting is at nine"),
    )?;
    broker.seal_record(
        "signing-key",
        EntryClass::Key,
        "person:tom",
        &Secret::from_slice(b"key-bytes"),
    )?;
    Ok(broker)
}

#[test]
fn a_memory_record_is_read_whole_or_in_the_piece_asked_for() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut broker = sealed(&paths(&root)?)?;
    let whole = broker.read_record("agent:noor", "notes", None)?;
    assert_eq!(whole.expose(), b"the meeting is at nine");
    let piece = broker.read_record("agent:noor", "notes", Some(4..11))?;
    assert_eq!(piece.expose(), b"meeting");
    let past_the_end = broker.read_record("agent:noor", "notes", Some(18..99))?;
    assert_eq!(past_the_end.expose(), b"nine");
    let reads = broker
        .audit()
        .replay()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::SealedRead)
        .count();
    assert_eq!(reads, 3);
    Ok(())
}

#[test]
fn a_key_is_never_read_and_an_unknown_name_is_not_found() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut broker = sealed(&paths(&root)?)?;
    assert_eq!(
        refusal(broker.read_record("agent:noor", "signing-key", None)),
        "KeyNotReadable"
    );
    assert_eq!(
        refusal(broker.read_record("agent:noor", "no-such-record", None)),
        "NotFound"
    );
    assert_eq!(
        refusal(broker.read_record("agent:other", "notes", None)),
        "NoRelation"
    );
    Ok(())
}

#[test]
fn a_removed_relation_is_named_and_stays_named_after_a_restart() -> TestResult {
    let root = tempfile::tempdir()?;
    let paths = paths(&root)?;
    let mut broker = sealed(&paths)?;
    broker.read_record("agent:noor", "notes", None)?;
    broker
        .permissions()
        .revoke_as(Relation::Read, "agent:noor", "notes");
    assert_eq!(
        refusal(broker.read_record("agent:noor", "notes", None)),
        "RelationRemoved"
    );
    drop(broker);
    let grants = reader();
    grants.revoke_as(Relation::Read, "agent:noor", "notes");
    let mut reopened = Broker::open(&paths, grants, Box::new(|| 2))?;
    assert_eq!(
        refusal(reopened.read_record("agent:noor", "notes", None)),
        "RelationRemoved"
    );
    Ok(())
}
