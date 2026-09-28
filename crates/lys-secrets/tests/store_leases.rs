//! Leases: the bounds a handle is cut with and presented within. A lease
//! ends no later than the grant it counts against, and a handle is admitted
//! only for the secret its lease names.

#[path = "support/signer.rs"]
mod signer;
#[path = "support/world.rs"]
mod world;

use lys_secrets::{Admitted, AuditKind, Relation, Secret, SecretRelation};
use signer::Signer;
use world::{GRANTOR, NOW_MS, TestResult, World, granted, refusal};

const ACT_WINDOW: &str = "(act: set the lease's not_after within the grant's window)";
const ACT_SCOPE: &str = "(act: ask the grant's owner for a grant that covers this scope)";

fn relation(identity: &str, secret: &str) -> SecretRelation {
    SecretRelation {
        identity: identity.to_owned(),
        secret: secret.to_owned(),
        granted_by: Some(GRANTOR.to_owned()),
    }
}

#[test]
fn sec3_store_refusals_lease_beyond_grant() -> TestResult {
    let world = World::new()?;
    let noor = Signer::new(&world.keys(), "agent:noor")?;
    let kit = Signer::new(&world.keys(), "agent:kit")?;
    let grants = granted("agent:noor", &[]);
    grants.grant_until(
        Relation::Use,
        relation("agent:noor", "token"),
        Some(NOW_MS + 10_000),
    );
    grants.grant_until(
        Relation::Use,
        relation("agent:kit", "token"),
        Some(NOW_MS + 2_000),
    );
    grants.grant_as(Relation::Lend, relation("agent:noor", "token"));
    let mut broker = world.broker(grants)?;
    broker.seal("token", GRANTOR, &Secret::from_slice(b"value-one"))?;

    let refused = refusal(broker.issue(&noor.holder, "token", 5, NOW_MS + 20_000));
    assert!(refused.starts_with("LeaseBeyondGrant: "), "{refused}");
    assert!(refused.contains(ACT_WINDOW), "{refused}");
    let issued = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Issue)
        .count();
    assert_eq!(issued, 0);

    let within = broker.issue(&noor.holder, "token", 5, NOW_MS + 5_000)?;
    let refused = refusal(broker.derive(
        &within.token,
        &noor.present(&within.id)?,
        &kit.holder,
        (1, NOW_MS + 5_000),
        None,
    ));
    assert!(refused.starts_with("LeaseBeyondGrant: "), "{refused}");
    broker.derive(
        &within.token,
        &noor.present(&within.id)?,
        &kit.holder,
        (1, NOW_MS + 2_000),
        None,
    )?;
    Ok(())
}

#[test]
fn sec3_store_refusals_outside_scope() -> TestResult {
    let world = World::new()?;
    let noor = Signer::new(&world.keys(), "agent:noor")?;
    let mut broker = world.broker(granted("agent:noor", &["token", "other"]))?;
    broker.seal("token", GRANTOR, &Secret::from_slice(b"value-one"))?;
    broker.add_account("token", "second", &Secret::from_slice(b"value-two"))?;
    broker.seal("other", GRANTOR, &Secret::from_slice(b"value-other"))?;
    let issued = broker.issue(&noor.holder, "token", 5, NOW_MS + 60_000)?;

    let refused =
        refusal(broker.admit_use_for(&issued.token, &noor.present(&issued.id)?, "other", 0));
    assert!(refused.starts_with("OutsideScope: "), "{refused}");
    assert!(refused.contains(ACT_SCOPE), "{refused}");
    let uses: Vec<String> = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Use)
        .map(|recorded| recorded.line.outcome)
        .collect();
    assert_eq!(uses, ["OutsideScope"]);

    for asked in ["token", "token@second"] {
        let Admitted::Fresh(ticket) =
            broker.admit_use_for(&issued.token, &noor.present(&issued.id)?, asked, 0)?
        else {
            return Err("retried".into());
        };
        assert_eq!(ticket.credential().expose(), b"value-one");
        broker.settle(ticket, 0)?;
    }
    Ok(())
}
