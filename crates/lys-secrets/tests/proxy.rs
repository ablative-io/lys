//! The proxy's refusals, driven at the library's admission: the checks a
//! presented handle passes before any credential is opened, each refused by
//! name with the act that answers it, and no use counted for a refusal.

#[path = "support/signer.rs"]
mod signer;
#[path = "support/world.rs"]
mod world;

use lys_secrets::new_operation_id;
use lys_secrets::{Admitted, AuditKind, Broker, HandleToken, LocalGrants, Presentation, Secret};
use signer::Signer;
use world::{NOW_MS, TestResult, World, granted, refusal};

const ACT_ACCOUNT: &str = "(act: bring an account back)";
const ACT_SIGN: &str =
    "(act: sign the presentation with the key registered for the handle's identity)";

/// How many uses the log shows admitted.
fn admitted_uses(broker: &Broker<LocalGrants>) -> Result<usize, lys_secrets::SecretsError> {
    Ok(broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| {
            recorded.line.kind == AuditKind::Use && recorded.line.outcome == "admitted"
        })
        .count())
}

#[test]
fn sec3_proxy_refusals_accounts_rested() -> TestResult {
    let world = World::new()?;
    let noor = Signer::new(&world.keys(), "agent:noor")?;
    let mut broker = world.broker(granted("agent:noor", &["token"]))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"value-one"))?;
    broker.add_account("token", "second", &Secret::from_slice(b"value-two"))?;
    let issued = broker.issue(&noor.holder, "token", 5, NOW_MS + 60_000)?;
    broker.rest_account("token", "primary")?;
    broker.rest_account("token", "second")?;

    let refused = refusal(broker.admit_use(&issued.token, &noor.present(&issued.id)?, 0));
    assert!(refused.starts_with("AccountsRested: "), "{refused}");
    assert!(refused.contains(ACT_ACCOUNT), "{refused}");
    assert_eq!(admitted_uses(&broker)?, 0);

    broker.restore_account("token", "second")?;
    let Admitted::Fresh(ticket) = broker.admit_use(&issued.token, &noor.present(&issued.id)?, 0)?
    else {
        return Err("retried".into());
    };
    assert_eq!(ticket.credential().expose(), b"value-two");
    broker.settle(ticket, 0)?;
    assert_eq!(admitted_uses(&broker)?, 1);
    Ok(())
}

#[test]
fn sec3_proxy_refusals_presentation_unsigned() -> TestResult {
    let world = World::new()?;
    let noor = Signer::new(&world.keys(), "agent:noor")?;
    let mut broker = world.broker(granted("agent:noor", &["token"]))?;
    broker.seal("token", "person:tom", &Secret::from_slice(b"value-one"))?;
    let issued = broker.issue(&noor.holder, "token", 5, NOW_MS + 60_000)?;
    let unsigned = Presentation::unsigned(&issued.id, &new_operation_id()?, NOW_MS, [7; 32]);

    let refused = refusal(broker.admit_use(&issued.token, &unsigned, 0));
    assert!(refused.starts_with("PresentationUnsigned: "), "{refused}");
    assert!(refused.contains(ACT_SIGN), "{refused}");
    assert_eq!(admitted_uses(&broker)?, 0);

    let unknown = HandleToken::from_bytes(&[9; 32]);
    let refused = refusal(broker.admit_use(&unknown, &unsigned, 0));
    assert!(refused.starts_with("HandleUnknown: "), "{refused}");

    let Admitted::Fresh(ticket) = broker.admit_use(&issued.token, &noor.present(&issued.id)?, 0)?
    else {
        return Err("retried".into());
    };
    broker.settle(ticket, 0)?;
    assert_eq!(admitted_uses(&broker)?, 1);
    Ok(())
}
