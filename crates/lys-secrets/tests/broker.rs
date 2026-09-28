//! The broker end to end: issue, use, every named refusal, retries, drops,
//! key rotation, accounts, and the audit log read back after a restart.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use lys_core::Ed25519Identity;
use lys_secrets::{
    AuditKind, Broker, BrokerPaths, Holder, IssuedHandle, LocalGrants, Presentation, Secret,
    SecretRelation, SecretsError, Used, new_operation_id,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct World {
    _dir: TempDir,
    paths: BrokerPaths,
    clock: Arc<AtomicI64>,
    agent: Ed25519Identity,
    holder: Holder,
}

const START_MS: i64 = 1_800_000_000_000;
const CALL: [u8; 32] = [7; 32];
const OTHER_CALL: [u8; 32] = [9; 32];

fn world() -> Result<World, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let keys = dir.path().join("keys");
    std::fs::create_dir_all(&keys)?;
    let agent = Ed25519Identity::load_or_generate(&keys.join("agent.key"))?;
    let holder = Holder {
        identity: "agent:noor".to_owned(),
        key: agent.public_key_bytes(),
    };
    let paths = BrokerPaths {
        store_dir: dir.path().join("store"),
        log_dir: dir.path().join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    Ok(World {
        _dir: dir,
        paths,
        clock: Arc::new(AtomicI64::new(START_MS)),
        agent,
        holder,
    })
}

fn clock(world: &World) -> lys_secrets::Clock {
    let shared = Arc::clone(&world.clock);
    Box::new(move || shared.load(Ordering::SeqCst))
}

fn granted() -> LocalGrants {
    let grants = LocalGrants::new();
    grants.grant(SecretRelation {
        identity: "agent:noor".to_owned(),
        secret: "token".to_owned(),
        granted_by: Some("person:tom".to_owned()),
    });
    grants
}

fn started(world: &World) -> Result<Broker<LocalGrants>, SecretsError> {
    let mut broker = Broker::create(&world.paths, granted(), clock(world))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"value-one"))?;
    Ok(broker)
}

fn present(
    world: &World,
    issued: &IssuedHandle,
    key: &Ed25519Identity,
) -> Result<Presentation, SecretsError> {
    Presentation::sign(
        &issued.id,
        &new_operation_id()?,
        world.clock.load(Ordering::SeqCst),
        CALL,
        key,
    )
}

fn value(
    broker: &mut Broker<LocalGrants>,
    issued: &IssuedHandle,
    presentation: &Presentation,
) -> Result<Vec<u8>, SecretsError> {
    match broker.use_handle(&issued.token, presentation, |credential| {
        credential.expose().to_vec()
    })? {
        Used::Forwarded { answer, .. } => Ok(answer),
        Used::Retried { outcome } => Ok(outcome.into_bytes()),
    }
}

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

#[test]
fn a_granted_holder_uses_the_credential_only_inside_the_forward() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    let issued = broker.issue(&world.holder, "token", 2, START_MS + 60_000)?;
    let presentation = present(&world, &issued, &world.agent)?;
    assert_eq!(value(&mut broker, &issued, &presentation)?, b"value-one");
    Ok(())
}

#[test]
fn issue_needs_a_grant_that_traces_to_a_person() -> TestResult {
    let world = world()?;
    let mut broker = Broker::create(&world.paths, LocalGrants::new(), clock(&world))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"v"))?;
    assert_eq!(
        refusal(broker.issue(&world.holder, "token", 1, START_MS + 1)),
        "PermissionDenied"
    );
    broker.permissions().grant(SecretRelation {
        identity: "agent:noor".to_owned(),
        secret: "token".to_owned(),
        granted_by: None,
    });
    assert_eq!(
        refusal(broker.issue(&world.holder, "token", 1, START_MS + 1)),
        "NoPersonRoot"
    );
    Ok(())
}

#[test]
fn every_refusal_is_named() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    let issued = broker.issue(&world.holder, "token", 1, START_MS + 60_000)?;
    let thief = Ed25519Identity::load_or_generate(
        &world
            .paths
            .store_dir
            .parent()
            .ok_or("no parent")?
            .join("keys/thief.key"),
    )?;
    let stolen = present(&world, &issued, &thief)?;
    assert_eq!(
        refusal(broker.use_handle(&issued.token, &stolen, Secret::len)),
        "PresentationInvalid"
    );
    let unknown = lys_secrets::HandleToken::from_bytes(&[7u8; 32]);
    let fine = present(&world, &issued, &world.agent)?;
    assert_eq!(
        refusal(broker.use_handle(&unknown, &fine, Secret::len)),
        "HandleUnknown"
    );
    let stale = Presentation::sign(
        &issued.id,
        &new_operation_id()?,
        START_MS - 31_000,
        CALL,
        &world.agent,
    )?;
    assert_eq!(
        refusal(broker.use_handle(&issued.token, &stale, Secret::len)),
        "PresentationStale"
    );
    assert_eq!(
        refusal(Presentation::sign(
            &issued.id,
            &[1u8; 8],
            START_MS,
            CALL,
            &world.agent
        )),
        "OperationIdTooShort"
    );
    broker.use_handle(&issued.token, &fine, Secret::len)?;
    let again = present(&world, &issued, &world.agent)?;
    assert_eq!(
        refusal(broker.use_handle(&issued.token, &again, Secret::len)),
        "LeaseExhausted"
    );
    Ok(())
}

#[test]
fn a_retry_is_answered_from_the_log_and_counts_no_use() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    let issued = broker.issue(&world.holder, "token", 1, START_MS + 60_000)?;
    let presentation = present(&world, &issued, &world.agent)?;
    let mut forwarded = 0;
    broker.use_handle(&issued.token, &presentation, |_credential| forwarded += 1)?;
    let retried = broker.use_handle(&issued.token, &presentation, |_credential| forwarded += 1)?;
    assert!(matches!(retried, Used::Retried { .. }));
    assert_eq!(forwarded, 1);
    Ok(())
}

#[test]
fn an_operation_id_signed_for_another_request_is_refused_before_and_after_a_restart() -> TestResult
{
    let world = world()?;
    let mut broker = started(&world)?;
    let issued = broker.issue(&world.holder, "token", 5, START_MS + 60_000)?;
    let first = present(&world, &issued, &world.agent)?;
    broker.use_handle(&issued.token, &first, Secret::len)?;
    let other = Presentation::sign(
        &issued.id,
        &first.operation_id,
        START_MS,
        OTHER_CALL,
        &world.agent,
    )?;
    assert_eq!(
        refusal(broker.use_handle(&issued.token, &other, Secret::len)),
        "OperationIdReused"
    );
    drop(broker);
    let mut reopened = Broker::open(&world.paths, granted(), clock(&world))?;
    assert_eq!(
        refusal(reopened.use_handle(&issued.token, &other, Secret::len)),
        "OperationIdReused"
    );
    let retried = reopened.use_handle(&issued.token, &first, Secret::len)?;
    assert!(matches!(retried, Used::Retried { .. }));
    Ok(())
}

#[test]
fn a_window_closes_and_a_drop_and_a_revocation_bite_at_the_next_use() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    let short = broker.issue(&world.holder, "token", 5, START_MS + 1_000)?;
    world.clock.store(START_MS + 2_000, Ordering::SeqCst);
    let late = present(&world, &short, &world.agent)?;
    assert_eq!(
        refusal(broker.use_handle(&short.token, &late, Secret::len)),
        "LeaseWindowClosed"
    );
    let dropped = broker.issue(&world.holder, "token", 5, START_MS + 60_000)?;
    broker.drop_handle(&dropped.id)?;
    let after_drop = present(&world, &dropped, &world.agent)?;
    assert_eq!(
        refusal(broker.use_handle(&dropped.token, &after_drop, Secret::len)),
        "HandleDropped"
    );
    let revoked = broker.issue(&world.holder, "token", 5, START_MS + 60_000)?;
    broker.permissions().revoke("agent:noor", "token");
    let after_revoke = present(&world, &revoked, &world.agent)?;
    assert_eq!(
        refusal(broker.use_handle(&revoked.token, &after_revoke, Secret::len)),
        "PermissionDenied"
    );
    Ok(())
}

#[test]
fn uses_and_drops_survive_a_restart_because_they_are_read_from_the_log() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    let issued = broker.issue(&world.holder, "token", 1, START_MS + 60_000)?;
    let dropped = broker.issue(&world.holder, "token", 1, START_MS + 60_000)?;
    broker.use_handle(
        &issued.token,
        &present(&world, &issued, &world.agent)?,
        Secret::len,
    )?;
    broker.drop_handle(&dropped.id)?;
    drop(broker);
    let mut reopened = Broker::open(&world.paths, granted(), clock(&world))?;
    let spent = present(&world, &issued, &world.agent)?;
    assert_eq!(
        refusal(reopened.use_handle(&issued.token, &spent, Secret::len)),
        "LeaseExhausted"
    );
    let gone = present(&world, &dropped, &world.agent)?;
    assert_eq!(
        refusal(reopened.use_handle(&dropped.token, &gone, Secret::len)),
        "HandleDropped"
    );
    assert!(reopened.audit().replay()?.len() >= 6);
    Ok(())
}

#[test]
fn a_second_broker_cannot_open_a_held_store() -> TestResult {
    let world = world()?;
    let _broker = started(&world)?;
    assert_eq!(
        refusal(Broker::open(&world.paths, granted(), clock(&world))),
        "StoreLocked"
    );
    Ok(())
}

#[test]
fn a_key_file_inside_the_store_is_refused() -> TestResult {
    let mut world = world()?;
    std::fs::create_dir_all(&world.paths.store_dir)?;
    world.paths.store_key = world.paths.store_dir.join("store.key");
    assert_eq!(
        refusal(Broker::create(&world.paths, granted(), clock(&world))),
        "KeyFileMisplaced"
    );
    Ok(())
}

#[test]
fn a_ciphertext_moved_between_entries_is_refused() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    broker.seal("other", "person:tom", &Secret::from_slice(b"value-two"))?;
    let id_of = |name: &str| {
        broker
            .store()
            .entry(name)
            .map(|entry| entry.id.clone())
            .ok_or("missing entry")
    };
    let entries = world.paths.store_dir.join("entries");
    std::fs::copy(
        entries.join(format!("{}.1.sealed", id_of("other")?)),
        entries.join(format!("{}.1.sealed", id_of("token")?)),
    )?;
    let issued = broker.issue(&world.holder, "token", 1, START_MS + 60_000)?;
    let presentation = present(&world, &issued, &world.agent)?;
    assert_eq!(
        refusal(value(&mut broker, &issued, &presentation)),
        "EntryBindingMismatch"
    );
    Ok(())
}

#[test]
fn rotation_retires_the_old_key_and_every_entry_opens_under_the_new_one() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    let new_key = world.paths.store_key.with_file_name("store-2.key");
    let (old, new) = broker.rotate_store_key(&new_key)?;
    assert_ne!(old, new);
    assert!(!world.paths.store_key.exists());
    let issued = broker.issue(&world.holder, "token", 1, START_MS + 60_000)?;
    let presentation = present(&world, &issued, &world.agent)?;
    assert_eq!(value(&mut broker, &issued, &presentation)?, b"value-one");
    let change = format!("{old} to {new}");
    let rotations: Vec<String> = broker
        .audit()
        .replay()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Rotation)
        .map(|recorded| recorded.line.outcome)
        .collect();
    assert_eq!(
        rotations,
        [format!("rotating {change}"), format!("rotated {change}")]
    );
    let entries = world.paths.store_dir.join("entries");
    assert_eq!(std::fs::read_dir(&entries)?.count(), 1);
    Ok(())
}

#[test]
fn a_sealing_no_entry_names_is_swept_when_the_store_opens() -> TestResult {
    let world = world()?;
    let broker = started(&world)?;
    let stray = world.paths.store_dir.join("entries").join("stray.9.sealed");
    std::fs::write(&stray, b"left by a rotation that never took effect")?;
    drop(broker);
    let reopened = Broker::open(&world.paths, granted(), clock(&world))?;
    assert!(!stray.exists());
    assert!(reopened.store().entry("token").is_some());
    Ok(())
}

#[test]
fn accounts_move_under_one_handle_and_refuse_when_all_rest() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    broker.add_account("token", "second", &Secret::from_slice(b"value-two"))?;
    let issued = broker.issue(&world.holder, "token", 9, START_MS + 60_000)?;
    assert_eq!(
        value(
            &mut broker,
            &issued,
            &present(&world, &issued, &world.agent)?
        )?,
        b"value-one"
    );
    assert_eq!(broker.next_account("token")?, "second");
    assert_eq!(
        value(
            &mut broker,
            &issued,
            &present(&world, &issued, &world.agent)?
        )?,
        b"value-two"
    );
    assert_eq!(refusal(broker.next_account("token")), "NoAccountAvailable");
    assert_eq!(
        value(
            &mut broker,
            &issued,
            &present(&world, &issued, &world.agent)?
        )?,
        b"value-two"
    );
    broker.restore_account("token", "primary")?;
    assert_eq!(broker.next_account("token")?, "primary");
    Ok(())
}

#[test]
fn the_audit_log_holds_no_credential_and_no_handle() -> TestResult {
    let world = world()?;
    let mut broker = started(&world)?;
    let issued = broker.issue(&world.holder, "token", 1, START_MS + 60_000)?;
    broker.use_handle(
        &issued.token,
        &present(&world, &issued, &world.agent)?,
        Secret::len,
    )?;
    let handle_hex = lys_secrets::to_hex(issued.token.expose());
    for dir in [&world.paths.log_dir, &world.paths.store_dir] {
        for entry in walk(dir)? {
            let bytes = std::fs::read(&entry)?;
            assert!(
                !contains(&bytes, b"value-one"),
                "credential in clear in {}",
                entry.display()
            );
            assert!(
                !contains(&bytes, handle_hex.as_bytes()),
                "raw handle in {}",
                entry.display()
            );
            assert!(
                !contains(&bytes, issued.token.expose()),
                "raw handle bytes in {}",
                entry.display()
            );
        }
    }
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn walk(dir: &Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files.extend(walk(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}
