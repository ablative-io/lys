//! An owner change of scope or recipients carries an operation id made once
//! for that change. A resend of the same change under the same id answers
//! the outcome recorded the first time and applies nothing; the same id with
//! another change is refused by name; a change without a well-shaped id is
//! refused. The settings answer the id of the last change applied, and a
//! start answers it as it stood, from a snapshot and from every line alike.

use std::num::NonZeroU64;

use lys_secrets::{
    Broker, BrokerPaths, LocalGrants, OwnerChanged, Recipients, Scope, Secret, SecretsError,
    SnapshotRefusal, Start,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Opened = Result<Broker<LocalGrants>, SecretsError>;

const START_MS: i64 = 1_800_000_000_000;
const OWNER: &str = "person:dana";
const SECRET: &str = "ledger";
const OTHER_SECRET: &str = "payroll";
const SERVICE: &str = "identity";
const FIRST: &str = "scope-change-00001";
const SECOND: &str = "recipients_change-00002";
/// A count of lines no log here reaches, so no snapshot is written by count.
const NEVER: NonZeroU64 = NonZeroU64::MAX;

struct World {
    _dir: TempDir,
    paths: BrokerPaths,
}

fn world() -> Result<World, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let keys = dir.path().join("keys");
    std::fs::create_dir_all(&keys)?;
    let paths = BrokerPaths {
        store_dir: dir.path().join("store"),
        log_dir: dir.path().join("log"),
        store_key: keys.join("store.key"),
        audit_key: keys.join("audit.key"),
        anchor: keys.join("audit.anchor"),
    };
    Ok(World { _dir: dir, paths })
}

fn clock() -> lys_secrets::Clock {
    Box::new(|| START_MS)
}

fn open(world: &World, every: NonZeroU64) -> Opened {
    Broker::open_every(&world.paths, LocalGrants::new(), clock(), every)
}

/// A new broker holding two secrets, both owned by `OWNER`.
fn sealed(world: &World) -> Opened {
    let mut broker = Broker::create(&world.paths, LocalGrants::new(), clock())?;
    broker.seal(SECRET, OWNER, &Secret::from_slice(b"value-one"))?;
    broker.seal(OTHER_SECRET, OWNER, &Secret::from_slice(b"value-two"))?;
    Ok(broker)
}

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

fn scope(
    broker: &mut Broker<LocalGrants>,
    text: &str,
    operation: Option<&str>,
) -> Result<OwnerChanged, SecretsError> {
    broker.set_scope_via(OWNER, SECRET, Scope::parse(text)?, Some(SERVICE), operation)
}

fn recipients(
    broker: &mut Broker<LocalGrants>,
    policy: Recipients,
    operation: Option<&str>,
) -> Result<OwnerChanged, SecretsError> {
    broker.set_recipients_via(OWNER, SECRET, policy, Some(SERVICE), operation)
}

fn last(broker: &Broker<LocalGrants>) -> Result<Option<String>, SecretsError> {
    Ok(broker.settings(OWNER, SECRET)?.last_operation)
}

#[test]
fn a_repeat_answers_the_recorded_outcome_and_applies_nothing() -> TestResult {
    let world = world()?;
    let mut broker = sealed(&world)?;
    let accounts = Scope::parse("team:accounts")?;

    assert_eq!(
        scope(&mut broker, "team:accounts", Some(FIRST))?,
        OwnerChanged::Applied
    );
    broker.set_scope(OWNER, SECRET, Scope::parse("team:payments")?)?;
    let len = broker.audit().len();

    assert_eq!(
        scope(&mut broker, "team:accounts", Some(FIRST))?,
        OwnerChanged::Repeated {
            outcome: format!("scope {} via {SERVICE}", accounts.target())
        }
    );
    assert_eq!(broker.audit().len(), len, "a repeat writes no line");
    let settings = broker.settings(OWNER, SECRET)?;
    assert_eq!(
        settings.scope.map(|scope| scope.target()),
        Some(Scope::parse("team:payments")?.target()),
        "a repeat applies nothing"
    );
    assert_eq!(settings.last_operation, None);
    Ok(())
}

#[test]
fn the_same_id_with_another_change_is_refused_naming_it() -> TestResult {
    let world = world()?;
    let mut broker = sealed(&world)?;
    scope(&mut broker, "team:accounts", Some(FIRST))?;
    let len = broker.audit().len();

    let Err(reused) = scope(&mut broker, "team:payments", Some(FIRST)) else {
        return Err("another scope under the same id was applied".into());
    };
    assert_eq!(reused.name(), "OperationReused");
    assert!(reused.to_string().contains(FIRST), "{reused}");
    assert_eq!(
        refusal(recipients(&mut broker, Recipients::PeopleOnly, Some(FIRST))),
        "OperationReused"
    );
    assert_eq!(broker.audit().len(), len, "a refusal writes no line");
    assert_eq!(broker.store().recipients(SECRET), Recipients::Anyone);

    let other = broker.set_scope_via(
        OWNER,
        OTHER_SECRET,
        Scope::parse("team:payments")?,
        None,
        Some(FIRST),
    )?;
    assert_eq!(other, OwnerChanged::Applied, "an id is counted per secret");
    Ok(())
}

#[test]
fn a_change_without_a_well_shaped_id_is_refused() -> TestResult {
    let world = world()?;
    let mut broker = sealed(&world)?;
    let len = broker.audit().len();
    let long = "a".repeat(65);

    for operation in [
        None,
        Some("short-id"),
        Some(long.as_str()),
        Some("has a space in it"),
    ] {
        assert_eq!(
            refusal(scope(&mut broker, "team:accounts", operation)),
            "OperationMissing",
            "{operation:?}"
        );
        assert_eq!(
            refusal(recipients(&mut broker, Recipients::PeopleOnly, operation)),
            "OperationMissing",
            "{operation:?}"
        );
    }
    assert_eq!(broker.audit().len(), len, "a refusal writes no line");
    assert_eq!(broker.store().scope(SECRET), None);
    assert_eq!(
        refusal(broker.set_scope_via(
            "person:tom",
            SECRET,
            Scope::parse("team:accounts")?,
            None,
            Some(FIRST)
        )),
        "LendingNotPermitted"
    );

    let shortest = "a".repeat(16);
    let longest = "Z_-9".repeat(16);
    assert_eq!(
        scope(&mut broker, "team:accounts", Some(&shortest))?,
        OwnerChanged::Applied
    );
    assert_eq!(
        recipients(&mut broker, Recipients::PeopleOnly, Some(&longest))?,
        OwnerChanged::Applied
    );
    Ok(())
}

#[test]
fn the_settings_answer_the_last_operation_applied() -> TestResult {
    let world = world()?;
    let mut broker = sealed(&world)?;
    assert_eq!(last(&broker)?, None);

    scope(&mut broker, "team:accounts", Some(FIRST))?;
    assert_eq!(last(&broker)?.as_deref(), Some(FIRST));
    recipients(&mut broker, Recipients::PeopleOnly, Some(SECOND))?;
    assert_eq!(last(&broker)?.as_deref(), Some(SECOND));
    scope(&mut broker, "team:accounts", Some(FIRST))?;
    assert_eq!(
        last(&broker)?.as_deref(),
        Some(SECOND),
        "a repeat is not a change applied"
    );
    broker.set_recipients(OWNER, SECRET, Recipients::Anyone)?;
    assert_eq!(last(&broker)?, None, "the last change carried no id");
    assert_eq!(
        broker
            .settings(OWNER, OTHER_SECRET)?
            .last_operation
            .as_deref(),
        None
    );
    Ok(())
}

/// What a start must answer of the changes made under `FIRST` and `SECOND`.
fn holds_both(broker: &mut Broker<LocalGrants>) -> TestResult {
    assert_eq!(last(broker)?.as_deref(), Some(SECOND));
    assert!(matches!(
        scope(broker, "team:accounts", Some(FIRST))?,
        OwnerChanged::Repeated { .. }
    ));
    assert!(matches!(
        recipients(broker, Recipients::PeopleOnly, Some(SECOND))?,
        OwnerChanged::Repeated { .. }
    ));
    assert_eq!(
        refusal(scope(broker, "team:payments", Some(FIRST))),
        "OperationReused"
    );
    Ok(())
}

#[test]
fn the_last_operation_survives_a_start_from_a_snapshot_and_from_every_line() -> TestResult {
    let world = world()?;
    drop(sealed(&world)?);
    let every = NonZeroU64::new(2).ok_or("two is not zero")?;

    let mut first = open(&world, every)?;
    scope(&mut first, "team:accounts", Some(FIRST))?;
    recipients(&mut first, Recipients::PeopleOnly, Some(SECOND))?;
    first.seal("notes", OWNER, &Secret::from_slice(b"value-three"))?;
    assert_eq!(first.snapshot_failure(), None);
    let held = first.folded()?;
    drop(first);

    let mut resumed = open(&world, NEVER)?;
    assert!(
        matches!(resumed.start(), Start::Resumed { replayed, .. } if *replayed < 2),
        "{}",
        resumed.start()
    );
    assert_eq!(resumed.folded()?, held);
    holds_both(&mut resumed)?;
    drop(resumed);

    std::fs::remove_file(world.paths.log_dir.join("snapshot.bin"))?;
    let mut rebuilt = open(&world, NEVER)?;
    assert_eq!(rebuilt.start().refusal(), Some(&SnapshotRefusal::Missing));
    assert_eq!(rebuilt.folded()?, held);
    holds_both(&mut rebuilt)?;
    drop(rebuilt);

    let mut again = open(&world, NEVER)?;
    assert!(
        matches!(again.start(), Start::Resumed { replayed: 0, .. }),
        "{}",
        again.start()
    );
    assert_eq!(again.folded()?, held);
    holds_both(&mut again)?;
    Ok(())
}
