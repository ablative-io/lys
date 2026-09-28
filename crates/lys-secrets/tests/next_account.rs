//! The revolver's ask for its next account, made with its own handle:
//! refused by name when every account is resting, and when every account
//! but the one in use is, which then stays in use; no account name is
//! answered to a refused ask.

#[path = "support/signer.rs"]
mod signer;
#[path = "support/world.rs"]
mod world;

use lys_secrets::{AuditKind, Secret, Used};
use signer::Signer;
use world::{GRANTOR, NOW_MS, TestResult, World, granted, refusal};

const ACT_ACCOUNT: &str = "(act: bring an account back)";

#[test]
fn sec3_next_refusals_accounts_rested() -> TestResult {
    let world = World::new()?;
    let noor = Signer::new(&world.keys(), "agent:noor")?;
    let mut broker = world.broker(granted("agent:noor", &["token"]))?;
    broker.seal("token", GRANTOR, &Secret::from_slice(b"value-one"))?;
    broker.add_account("token", "second", &Secret::from_slice(b"value-two"))?;
    let issued = broker.issue(&noor.holder, "token", 5, NOW_MS + 60_000)?;
    broker.rest_account("token", "primary")?;
    broker.rest_account("token", "second")?;

    let refused = refusal(broker.next_account_for(&issued.token, &noor.present(&issued.id)?));
    assert!(refused.starts_with("AccountsRested: "), "{refused}");
    assert!(refused.contains(ACT_ACCOUNT), "{refused}");

    broker.restore_account("token", "second")?;
    let refused = refusal(broker.next_account_for(&issued.token, &noor.present(&issued.id)?));
    assert!(refused.starts_with("AccountsRested: "), "{refused}");
    assert!(refused.contains("none to move to"), "{refused}");
    let answered = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| {
            recorded.line.kind == AuditKind::NextAccount
                && recorded.line.outcome.starts_with("now ")
        })
        .count();
    assert_eq!(answered, 0);

    let used = broker.use_handle(&issued.token, &noor.present(&issued.id)?, |value| {
        value.expose().to_vec()
    })?;
    assert!(matches!(used, Used::Forwarded { ref answer, .. } if answer == b"value-two"));
    broker.restore_account("token", "primary")?;
    assert_eq!(
        broker.next_account_for(&issued.token, &noor.present(&issued.id)?)?,
        "primary"
    );
    Ok(())
}
