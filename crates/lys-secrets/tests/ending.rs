//! A person ending a handle held for them: only the person the holder acts
//! for may end it, a handle lent on from it ends with it and the audit log
//! names the line, the ending is made once per operation id, a restart from
//! the snapshot keeps it ended, and the provider's part of the revocation
//! stays apart and is never taken as confirmed.

use std::num::NonZeroU64;
use std::path::Path;

use lys_core::Ed25519Identity;
use lys_log_store::{seal, unseal};
use lys_secrets::{
    AuditKind, Broker, BrokerPaths, Ended, HandleEnded, Holder, IssuedHandle, LocalGrants,
    Presentation, Relation, STATE_DOMAIN, Secret, SecretRelation, SecretsError, Start,
    UpstreamRevocation, new_operation_id,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NOW_MS: i64 = 1_000;
const DANA: &str = "person:dana";
const TOM: &str = "person:tom";
const BOT: &str = "agent:dana-bot";
const HELPER: &str = "agent:helper";
const ENDING: &str = "end-dana-bot-token-01";
const ORIGIN: &str = "lys.local/secrets-audit";
const NEVER: NonZeroU64 = NonZeroU64::MAX;

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

fn relation(identity: &str, secret: &str) -> SecretRelation {
    SecretRelation {
        identity: identity.to_owned(),
        secret: secret.to_owned(),
        granted_by: Some(DANA.to_owned()),
    }
}

/// Dana owns the token; her agent acts for her and may lend it on; Tom may
/// use it but nothing acts for him here; the helper may use it.
fn granted() -> LocalGrants {
    let grants = LocalGrants::new();
    for identity in [BOT, TOM, HELPER] {
        grants.grant(relation(identity, "token"));
    }
    grants.grant_as(Relation::Lend, relation(BOT, "token"));
    grants.grant_as(Relation::Member, relation(BOT, "person/person:dana"));
    grants
}

struct World {
    dir: TempDir,
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
    Ok(World { dir, paths })
}

fn party(
    dir: &Path,
    identity: &str,
) -> Result<(Ed25519Identity, Holder), Box<dyn std::error::Error>> {
    let key = Ed25519Identity::load_or_generate(&dir.join(format!("{identity}.key")))?;
    let holder = Holder {
        identity: identity.to_owned(),
        key: key.public_key_bytes(),
    };
    Ok((key, holder))
}

fn sign(issued: &IssuedHandle, key: &Ed25519Identity) -> Result<Presentation, SecretsError> {
    Presentation::sign(&issued.id, &new_operation_id()?, NOW_MS, [1; 32], key)
}

/// The handles a world is seeded with.
struct Seeded {
    broker: Broker<LocalGrants>,
    bot: IssuedHandle,
    bot_key: Ed25519Identity,
    helper: IssuedHandle,
    helper_key: Ed25519Identity,
    tom: IssuedHandle,
    tom_key: Ed25519Identity,
}

fn seeded(world: &World) -> Result<Seeded, Box<dyn std::error::Error>> {
    let mut broker = Broker::create(&world.paths, granted(), Box::new(|| NOW_MS))?;
    broker.seal("token", DANA, &Secret::from_slice(b"value"))?;
    let holders = world.dir.path().join("holders");
    std::fs::create_dir_all(&holders)?;
    let (bot_key, bot_holder) = party(&holders, BOT)?;
    let (helper_key, helper_holder) = party(&holders, HELPER)?;
    let (tom_key, tom_holder) = party(&holders, TOM)?;
    let bot = broker.issue(&bot_holder, "token", 5, 50_000)?;
    let helper = broker.derive(
        &bot.token,
        &sign(&bot, &bot_key)?,
        &helper_holder,
        (2, 40_000),
        None,
    )?;
    let tom = broker.issue(&tom_holder, "token", 5, 50_000)?;
    Ok(Seeded {
        broker,
        bot,
        bot_key,
        helper,
        helper_key,
        tom,
        tom_key,
    })
}

fn ended_by_dana(root: &IssuedHandle) -> Ended {
    Ended {
        by: DANA.to_owned(),
        operation: ENDING.to_owned(),
        root: root.id.as_str().to_owned(),
    }
}

/// `bytes` with the first `from` replaced by `to`, of the same length.
fn replaced(bytes: &[u8], from: &[u8], to: &[u8]) -> Option<Vec<u8>> {
    let at = bytes
        .windows(from.len())
        .position(|window| window == from)?;
    let mut out = bytes.to_vec();
    out.get_mut(at..at + to.len())?.copy_from_slice(to);
    Some(out)
}

#[test]
fn the_person_acted_for_ends_a_handle_and_what_was_lent_on_ends_with_it() -> TestResult {
    let world = world()?;
    let mut seeded = seeded(&world)?;
    let (bot, helper) = (seeded.bot.id.as_str(), seeded.helper.id.as_str());

    let answered =
        seeded
            .broker
            .end_handle(DANA, &seeded.bot.id, Some("identity"), Some(ENDING))?;
    assert_eq!(
        answered,
        HandleEnded::Ended {
            ended: vec![bot.to_owned(), helper.to_owned()]
        }
    );

    let broker = &mut seeded.broker;
    let on_bot = sign(&seeded.bot, &seeded.bot_key)?;
    assert_eq!(
        refusal(broker.use_handle(&seeded.bot.token, &on_bot, Secret::len)),
        "HandleDropped"
    );
    let on_helper = sign(&seeded.helper, &seeded.helper_key)?;
    assert_eq!(
        refusal(broker.use_handle(&seeded.helper.token, &on_helper, Secret::len)),
        "HandleDropped"
    );
    let on_tom = sign(&seeded.tom, &seeded.tom_key)?;
    assert!(
        broker
            .use_handle(&seeded.tom.token, &on_tom, Secret::len)
            .is_ok(),
        "a handle issued on its own is untouched"
    );

    let held = broker.held_by(DANA, BOT);
    assert_eq!(held.len(), 1);
    assert!(held[0].dropped);
    assert_eq!(held[0].ended, Some(ended_by_dana(&seeded.bot)));
    let lent = broker.held_by(DANA, HELPER);
    assert_eq!(lent[0].ended, Some(ended_by_dana(&seeded.bot)));

    let drops: Vec<String> = broker
        .audit()
        .replay()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Drop)
        .map(|recorded| recorded.line.outcome)
        .collect();
    assert_eq!(
        drops,
        vec![
            format!("ended by {DANA} along {bot} via identity"),
            format!("ended by {DANA} along {bot} > {helper} via identity"),
        ],
        "each line names the person and the line of handles"
    );
    Ok(())
}

#[test]
fn anyone_but_the_person_acted_for_is_refused_and_nothing_is_written() -> TestResult {
    let world = world()?;
    let mut seeded = seeded(&world)?;
    let before = seeded.broker.audit().len();

    assert_eq!(
        refusal(
            seeded
                .broker
                .end_handle(TOM, &seeded.bot.id, None, Some(ENDING))
        ),
        "LendingNotPermitted",
        "a person who may use the secret but is not acted for"
    );
    assert_eq!(
        refusal(
            seeded
                .broker
                .end_handle("person:stranger", &seeded.bot.id, None, Some(ENDING))
        ),
        "HandleUnknown",
        "a person who may not discover the secret learns nothing of the handle"
    );
    assert_eq!(
        refusal(
            seeded
                .broker
                .end_handle(DANA, &seeded.tom.id, None, Some(ENDING))
        ),
        "LendingNotPermitted",
        "owning the secret confers nothing over a handle not held for her"
    );
    assert_eq!(seeded.broker.audit().len(), before);
    Ok(())
}

#[test]
fn an_ending_is_made_once_per_operation_id() -> TestResult {
    let world = world()?;
    let mut seeded = seeded(&world)?;
    let broker = &mut seeded.broker;

    assert_eq!(
        refusal(broker.end_handle(DANA, &seeded.bot.id, None, None)),
        "OperationMissing"
    );
    assert_eq!(
        refusal(broker.end_handle(DANA, &seeded.bot.id, None, Some("short"))),
        "OperationMissing"
    );

    let first = broker.end_handle(DANA, &seeded.bot.id, None, Some(ENDING))?;
    let len = broker.audit().len();
    let HandleEnded::Ended { ended } = first else {
        return Err(format!("the first ending answered {first:?}").into());
    };
    assert_eq!(
        broker.end_handle(DANA, &seeded.bot.id, None, Some(ENDING))?,
        HandleEnded::Repeated { ended },
        "a resend answers what was recorded"
    );
    assert_eq!(broker.audit().len(), len, "and appends nothing");

    assert_eq!(
        refusal(broker.end_handle(DANA, &seeded.helper.id, None, Some(ENDING))),
        "OperationReused"
    );
    assert_eq!(
        broker.end_handle(DANA, &seeded.helper.id, None, Some("end-helper-again-0001"))?,
        HandleEnded::AlreadyEnded
    );
    assert_eq!(broker.audit().len(), len);
    Ok(())
}

#[test]
fn a_restart_from_the_snapshot_keeps_the_handle_ended() -> TestResult {
    let world = world()?;
    let mut seeded = seeded(&world)?;
    seeded
        .broker
        .end_handle(DANA, &seeded.bot.id, None, Some(ENDING))?;
    let (bot, helper) = (seeded.bot, seeded.helper);
    drop(seeded.broker);

    let every = NonZeroU64::new(2).ok_or("two is not zero")?;
    let first = Broker::open_every(&world.paths, granted(), Box::new(|| NOW_MS), every)?;
    let held = first.folded()?;
    drop(first);

    let mut resumed = Broker::open_every(&world.paths, granted(), Box::new(|| NOW_MS), NEVER)?;
    assert!(
        matches!(resumed.start(), Start::Resumed { replayed: 0, .. }),
        "{}",
        resumed.start()
    );
    assert_eq!(resumed.folded()?, held);
    assert_eq!(
        resumed.held_by(DANA, BOT)[0].ended,
        Some(ended_by_dana(&bot))
    );
    assert_eq!(
        resumed.held_by(DANA, HELPER)[0].ended,
        Some(ended_by_dana(&bot))
    );
    let on_helper = sign(&helper, &seeded.helper_key)?;
    assert_eq!(
        refusal(resumed.use_handle(&helper.token, &on_helper, Secret::len)),
        "HandleDropped"
    );
    assert!(matches!(
        resumed.end_handle(DANA, &bot.id, None, Some(ENDING))?,
        HandleEnded::Repeated { .. }
    ));
    drop(resumed);

    let snapshot = world.paths.log_dir.join("snapshot.bin");
    let key = Ed25519Identity::load(&world.paths.audit_key)?;
    let written = std::fs::read(&snapshot)?;
    let honest = unseal(&written, STATE_DOMAIN, ORIGIN, &key.public_key_bytes())?;
    let older = replaced(honest.state(), b"broker-folded/v3", b"broker-folded/v2")
        .ok_or("the state does not name its format")?;
    std::fs::write(
        &snapshot,
        seal(STATE_DOMAIN, ORIGIN, honest.frontier(), &older, &key),
    )?;
    let rebuilt = Broker::open_every(&world.paths, granted(), Box::new(|| NOW_MS), NEVER)?;
    let refused = rebuilt
        .start()
        .refusal()
        .ok_or("a snapshot of the older format was used")?;
    assert!(
        refused.to_string().starts_with("SnapshotStateUnreadable"),
        "{refused}"
    );
    assert_eq!(rebuilt.folded()?, held, "every line is read instead");
    assert_eq!(
        rebuilt.held_by(DANA, BOT)[0].ended,
        Some(ended_by_dana(&bot))
    );
    Ok(())
}

#[test]
fn the_providers_part_stays_unconfirmed_until_the_provider_confirms() -> TestResult {
    let world = world()?;
    let mut seeded = seeded(&world)?;
    let bot = seeded.bot.id.clone();
    let broker = &mut seeded.broker;

    broker.end_handle(DANA, &bot, None, Some(ENDING))?;
    let state = broker.revocation_state(&bot)?;
    assert!(state.stopped_here);
    assert_eq!(
        state.upstream,
        UpstreamRevocation::NotAsked,
        "ending here asks nothing of the provider and confirms nothing"
    );

    broker.record_upstream_revocation(&bot, Err("the provider did not answer".to_owned()))?;
    let unconfirmed = UpstreamRevocation::Unconfirmed("the provider did not answer".to_owned());
    assert_eq!(broker.revocation_state(&bot)?.upstream, unconfirmed);
    assert_eq!(broker.held_by(DANA, BOT)[0].upstream, unconfirmed);
    drop(seeded.broker);

    let reopened = Broker::open(&world.paths, granted(), Box::new(|| NOW_MS))?;
    assert_eq!(reopened.revocation_state(&bot)?.upstream, unconfirmed);
    assert_eq!(
        reopened.held_by(DANA, HELPER)[0].upstream,
        UpstreamRevocation::NotAsked,
        "a handle ended with it has its own provider state"
    );
    Ok(())
}
