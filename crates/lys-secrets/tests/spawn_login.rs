//! A seat's login at spawn: refused by name, with no login answered, when
//! every account of the login set is resting, and answered again once an
//! account is brought back.

#[path = "support/world.rs"]
mod world;

use lys_secrets::{AuditKind, Secret};
use world::{GRANTOR, TestResult, World, granted, refusal};

const ACT_ACCOUNT: &str = "(act: bring an account back)";

#[test]
fn sec3_spawn_refusals_accounts_rested() -> TestResult {
    let world = World::new()?;
    let mut broker = world.broker(granted("seat:one", &["logins"]))?;
    broker.seal("logins", GRANTOR, &Secret::from_slice(b"login-a"))?;
    broker.add_account("logins", "b", &Secret::from_slice(b"login-b"))?;
    broker.rest_account("logins", "primary")?;
    broker.rest_account("logins", "b")?;

    let refused = refusal(broker.spawn_login("seat:one", "logins"));
    assert!(refused.starts_with("AccountsRested: "), "{refused}");
    assert!(refused.contains(ACT_ACCOUNT), "{refused}");
    let handed = |broker: &lys_secrets::Broker<lys_secrets::LocalGrants>| {
        broker.audit().replay().map(|lines| {
            lines
                .into_iter()
                .filter(|recorded| {
                    recorded.line.kind == AuditKind::SpawnLogin && recorded.line.outcome == "handed"
                })
                .count()
        })
    };
    assert_eq!(handed(&broker)?, 0);

    broker.restore_account("logins", "b")?;
    let (account, login) = broker.spawn_login("seat:one", "logins")?;
    assert_eq!(account, "b");
    assert_eq!(login.expose(), b"login-b");
    assert_eq!(handed(&broker)?, 1);
    Ok(())
}
