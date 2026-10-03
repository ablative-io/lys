#![cfg(test)]
//! Batch admission, receipt roots, retries, and restart.

use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::receipt::verify_receipt;
use lys_identity::{
    Actor, AgentId, AuthMethod, Change, Directory, IdentityError, IdentityEvent, IdentityId,
    LifecycleState, LoginBinding, OperationId, PersonId, Profile, Provenance, Transition,
};
use lys_log_store::{FileLeafStore, Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};

type Outcome = Result<(), Box<dyn std::error::Error>>;

fn open(
    home: &Arc<tempfile::TempDir>,
) -> Result<Directory<FileLeafStore>, Box<dyn std::error::Error>> {
    let home = Arc::clone(home);
    let key = Ed25519Identity::load(&home.path().join("key"))?;
    Ok(Directory::open(
        Box::new(move || FileLeafStore::open(&home.path().join("log"))),
        key,
    )?)
}

fn world() -> Result<(Arc<tempfile::TempDir>, Directory<FileLeafStore>), Box<dyn std::error::Error>>
{
    let home = Arc::new(tempfile::tempdir()?);
    std::fs::write(home.path().join("key"), [7; 32])?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            home.path().join("key"),
            std::fs::Permissions::from_mode(0o600),
        )?;
    }
    FileLeafStore::create(&home.path().join("log"), "example.com/lys/batch-directory")?;
    let directory = open(&home)?;
    Ok((home, directory))
}

fn event(identity: IdentityId, change: Change) -> Result<IdentityEvent, IdentityError> {
    IdentityEvent::new(
        OperationId::generate()?,
        Actor::new(
            LoginBinding::new("https://example.com", "administrator")?,
            Provenance::new(AuthMethod::Oidc, 1),
        ),
        identity,
        1,
        change,
    )
}

fn events() -> Result<Vec<IdentityEvent>, IdentityError> {
    let person = PersonId::generate()?;
    Ok(vec![
        event(
            IdentityId::Person(person),
            Change::RegisterPerson {
                profile: Profile::new("Person")?,
            },
        )?,
        event(
            IdentityId::Person(person),
            Change::Transition {
                transition: Transition::Activate,
                from: LifecycleState::Registered,
                to: LifecycleState::Active,
                reason: String::new(),
            },
        )?,
        event(
            IdentityId::Agent(AgentId::generate()?),
            Change::RegisterAgent {
                responsible: person,
                profile: Profile::new("Agent")?,
            },
        )?,
    ])
}

#[test]
fn batch_receipts_verify_at_each_prefix_and_survive_retry_and_reopen() -> Outcome {
    let (home, mut directory) = world()?;
    let changes = events()?;
    let mut duplicate = changes.clone();
    duplicate.push(changes[0].clone());
    let receipts = directory.commit_batch(&duplicate)?;
    assert_eq!(receipts.len(), 4);
    assert_eq!(receipts[0], receipts[3]);
    assert_eq!(directory.log()?.len()?, 3);
    let key = directory.service_key();
    let mut prefix = Frontier::new();
    for (index, receipt) in receipts[..3].iter().enumerate() {
        let coordinate = receipt.coordinate();
        assert_eq!(coordinate.index, u64::try_from(index)?);
        assert_eq!(coordinate.tree_size, u64::try_from(index + 1)?);
        let log = directory.log()?;
        let signed = log
            .leaf(coordinate.index)?
            .ok_or("receipt leaf is missing")?;
        prefix.push(&signed);
        assert_eq!(coordinate.root, prefix.root());
        verify_receipt(
            receipt,
            &signed,
            &key,
            log.head()?,
            &log.inclusion_proof(coordinate.index)?,
        )?;
    }
    assert_eq!(directory.commit_batch(&changes)?, receipts[..3]);
    assert_eq!(directory.log()?.len()?, 3);
    let projection = directory.projection()?.clone();
    drop(directory);
    let mut reopened = open(&home)?;
    assert_eq!(reopened.projection()?, &projection);
    assert_eq!(reopened.commit_batch(&changes)?, receipts[..3]);
    assert_eq!(reopened.log()?.len()?, 3);
    Ok(())
}

#[test]
fn invalid_later_event_refuses_the_whole_batch_without_state_or_log_changes() -> Outcome {
    let (home, mut directory) = world()?;
    let mut changes = events()?;
    changes[2] = event(
        IdentityId::Agent(AgentId::generate()?),
        Change::RegisterAgent {
            responsible: PersonId::generate()?,
            profile: Profile::new("Unknown owner")?,
        },
    )?;
    assert!(matches!(
        directory.commit_batch(&changes),
        Err(IdentityError::IdentityUnknown { .. })
    ));
    assert_eq!(directory.log()?.len()?, 0);
    assert!(directory.record(changes[0].identity())?.is_none());
    assert_eq!(FileLeafStore::open(&home.path().join("log"))?.extent(), 0);
    Ok(())
}

#[test]
fn operation_reuse_inside_a_batch_refuses_before_writing_and_empty_batch_is_a_noop() -> Outcome {
    let (home, mut directory) = world()?;
    let original = events()?.remove(0);
    let different = IdentityEvent::new(
        original.operation(),
        original.actor().clone(),
        original.identity(),
        1,
        Change::RegisterPerson {
            profile: Profile::new("Changed")?,
        },
    )?;
    assert!(matches!(
        directory.commit_batch(&[original, different]),
        Err(IdentityError::OperationReused { .. })
    ));
    assert!(directory.commit_batch(&[])?.is_empty());
    assert_eq!(directory.log()?.len()?, 0);
    assert_eq!(FileLeafStore::open(&home.path().join("log"))?.extent(), 0);
    Ok(())
}

struct Partial {
    store: FileLeafStore,
    fail: Arc<std::sync::atomic::AtomicBool>,
}

impl LeafStore for Partial {
    fn origin(&self) -> &str {
        self.store.origin()
    }
    fn extent(&self) -> u64 {
        self.store.extent()
    }
    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        self.store.leaf(index)
    }
    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.store.put_leaf(index, bytes)
    }
    fn append(&mut self, index: u64, leaves: &[&[u8]], pin: PinnedRoot) -> StoreResult<()> {
        if self.fail.swap(false, std::sync::atomic::Ordering::SeqCst) {
            // The one flush that would have covered the batch fails: nothing
            // of it is acknowledged (LYSLOGSTORE-008 R3).
            return Err(StoreError::Io {
                context: "injected batch flush failure".to_owned(),
                source: std::io::Error::other("the batch was refused"),
            });
        }
        self.store.append(index, leaves, pin)
    }
    fn pinned(&self) -> PinnedRoot {
        self.store.pinned()
    }
    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.store.pin(pin)
    }
    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        self.store.snapshot()
    }
    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.store.put_snapshot(bytes)
    }
}

#[test]
fn a_failed_batch_records_nothing_and_the_retry_records_every_event() -> Outcome {
    let (home, original) = world()?;
    drop(original);
    let key = Ed25519Identity::load(&home.path().join("key"))?;
    let path = home.path().join("log");
    let fail = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut directory = Directory::open(
        Box::new(move || {
            Ok(Partial {
                store: FileLeafStore::open(&path)?,
                fail: Arc::clone(&fail),
            })
        }),
        key,
    )?;
    let changes = events()?;
    assert!(matches!(
        directory.commit_batch(&changes),
        Err(IdentityError::LogUnavailable { .. })
    ));
    assert_eq!(
        directory.log()?.len()?,
        0,
        "nothing of the batch was acknowledged"
    );
    assert!(directory.record(changes[0].identity())?.is_none());
    assert!(directory.record(changes[2].identity())?.is_none());
    assert_eq!(FileLeafStore::open(&home.path().join("log"))?.extent(), 0);
    let receipts = directory.commit_batch(&changes)?;
    assert_eq!(receipts.len(), 3);
    assert_eq!(directory.log()?.len()?, 3);
    assert_eq!(
        directory
            .record(changes[0].identity())?
            .ok_or("person missing")?
            .state(),
        LifecycleState::Active
    );
    assert!(directory.record(changes[2].identity())?.is_some());
    let mut reopened = open(&home)?;
    assert_eq!(reopened.commit_batch(&changes)?, receipts);
    assert_eq!(reopened.log()?.len()?, 3);
    Ok(())
}
