#![cfg(test)]

//! Spend caps: a capped lease reserves before a call is forwarded, refuses a
//! reservation past its cap, settles what the call spent, and settles a call
//! the broker never saw finish as `outcome_unknown` at its full reservation.
//! A drop lands on calls in flight: cancelled before forwarding, recorded as
//! `completed_after_drop` after.

use lys_core::Ed25519Identity;
use lys_secrets::{
    Admitted, AuditKind, Broker, BrokerPaths, Holder, LocalGrants, Presentation, Secret,
    SecretRelation, SecretsError, Ticket, new_operation_id,
};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const NOW_MS: i64 = 1_800_000_000_000;
const CALL: [u8; 32] = [7; 32];

struct World {
    root: TempDir,
    agent: Ed25519Identity,
    holder: Holder,
}

impl World {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        std::fs::create_dir_all(root.path().join("keys"))?;
        let agent = Ed25519Identity::load_or_generate(&root.path().join("keys").join("agent.key"))?;
        let holder = Holder {
            identity: "agent:noor".to_owned(),
            key: agent.public_key_bytes(),
        };
        Ok(Self {
            root,
            agent,
            holder,
        })
    }

    fn paths(&self) -> BrokerPaths {
        let keys = self.root.path().join("keys");
        BrokerPaths {
            store_dir: self.root.path().join("store"),
            log_dir: self.root.path().join("log"),
            store_key: keys.join("store.key"),
            audit_key: keys.join("audit.key"),
            anchor: keys.join("audit.anchor"),
        }
    }

    fn present(&self, id: &lys_secrets::HandleId) -> Result<Presentation, SecretsError> {
        Presentation::sign(id, &new_operation_id()?, NOW_MS, CALL, &self.agent)
    }
}

fn grants() -> LocalGrants {
    let grants = LocalGrants::new();
    grants
        .grant(SecretRelation {
            identity: "agent:noor".to_owned(),
            secret: "model-key".to_owned(),
            granted_by: Some("person:tom".to_owned()),
        })
        .expect("local grants lock must be healthy");
    grants
}

fn fresh(admitted: Admitted) -> Result<Ticket, Box<dyn std::error::Error>> {
    match admitted {
        Admitted::Fresh(ticket) => Ok(ticket),
        Admitted::Retried { outcome } => Err(format!("retried: {outcome}").into()),
    }
}

fn refusal<T>(result: Result<T, SecretsError>) -> &'static str {
    match result {
        Ok(_) => "admitted",
        Err(error) => error.name(),
    }
}

#[test]
fn a_capped_lease_reserves_before_work_and_refuses_past_its_cap() -> TestResult {
    let world = World::new()?;
    let mut broker = Broker::create(&world.paths(), grants(), Box::new(|| NOW_MS))?;
    broker.seal("model-key", "person:tom", &Secret::from_slice(b"sk-demo"))?;
    let issued = broker.issue_capped(&world.holder, "model-key", 10, NOW_MS + 60_000, Some(100))?;
    let token = &issued.token;
    assert_eq!(
        refusal(broker.admit_use(token, &world.present(&issued.id)?, 0)),
        "ReservationMissing"
    );
    let first = fresh(broker.admit_use(token, &world.present(&issued.id)?, 60)?)?;
    assert_eq!(first.reserved(), Some(60));
    assert_eq!(first.credential().expose(), b"sk-demo");
    assert_eq!(
        refusal(broker.admit_use(token, &world.present(&issued.id)?, 50)),
        "SpendCapReached"
    );
    broker.settle(first, 30)?;
    let second = fresh(broker.admit_use(token, &world.present(&issued.id)?, 70)?)?;
    broker.settle(second, 70)?;
    assert_eq!(
        refusal(broker.admit_use(token, &world.present(&issued.id)?, 1)),
        "SpendCapReached"
    );
    let settled: Vec<(String, Option<u64>)> = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .filter(|recorded| recorded.line.kind == AuditKind::Settlement)
        .map(|recorded| (recorded.line.outcome, recorded.line.spend))
        .collect();
    assert_eq!(
        settled,
        [
            ("completed".to_owned(), Some(30)),
            ("completed".to_owned(), Some(70))
        ]
    );
    Ok(())
}

#[test]
fn an_unsettled_call_counts_in_full_after_a_restart_and_is_never_forwarded_again() -> TestResult {
    let world = World::new()?;
    let mut broker = Broker::create(&world.paths(), grants(), Box::new(|| NOW_MS))?;
    broker.seal("model-key", "person:tom", &Secret::from_slice(b"sk-demo"))?;
    let issued = broker.issue_capped(&world.holder, "model-key", 10, NOW_MS + 60_000, Some(100))?;
    let presented = world.present(&issued.id)?;
    let in_flight = fresh(broker.admit_use(&issued.token, &presented, 80)?)?;
    drop(in_flight);
    drop(broker);
    let mut reopened = Broker::open(&world.paths(), grants(), Box::new(|| NOW_MS))?;
    match reopened.admit_use(&issued.token, &presented, 80)? {
        Admitted::Retried { outcome } => assert_eq!(outcome, "outcome_unknown"),
        Admitted::Fresh(_ticket) => return Err("forwarded again".into()),
    }
    assert_eq!(
        refusal(reopened.admit_use(&issued.token, &world.present(&issued.id)?, 21)),
        "SpendCapReached"
    );
    let last = reopened
        .audit()
        .audit_every_line()?
        .into_iter()
        .rfind(|recorded| recorded.line.kind == AuditKind::Settlement)
        .map(|recorded| (recorded.line.outcome, recorded.line.spend));
    assert_eq!(last, Some(("outcome_unknown".to_owned(), Some(80))));
    Ok(())
}

#[test]
fn an_uncapped_lease_needs_no_reservation_and_records_no_spend() -> TestResult {
    let world = World::new()?;
    let mut broker = Broker::create(&world.paths(), grants(), Box::new(|| NOW_MS))?;
    broker.seal("model-key", "person:tom", &Secret::from_slice(b"sk-demo"))?;
    let issued = broker.issue(&world.holder, "model-key", 1, NOW_MS + 60_000)?;
    let ticket = fresh(broker.admit_use(&issued.token, &world.present(&issued.id)?, 0)?)?;
    assert_eq!(ticket.reserved(), None);
    broker.settle(ticket, 5)?;
    let spends: Vec<Option<u64>> = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .map(|recorded| recorded.line.spend)
        .collect();
    assert!(spends.iter().all(Option::is_none));
    Ok(())
}

#[test]
fn a_drop_between_admission_and_forwarding_cancels_the_call_and_releases_its_use() -> TestResult {
    let world = World::new()?;
    let mut broker = Broker::create(&world.paths(), grants(), Box::new(|| NOW_MS))?;
    broker.seal("model-key", "person:tom", &Secret::from_slice(b"sk-demo"))?;
    let issued = broker.issue_capped(&world.holder, "model-key", 1, NOW_MS + 60_000, Some(100))?;
    let presented = world.present(&issued.id)?;
    let admitted = fresh(broker.admit_use(&issued.token, &presented, 40)?)?;
    broker.drop_handle(&issued.id)?;
    assert_eq!(
        refusal(broker.at_forward_boundary(admitted)),
        "HandleDropped"
    );
    match broker.admit_use(&issued.token, &presented, 40)? {
        Admitted::Retried { outcome } => assert_eq!(outcome, "cancelled_at_boundary"),
        Admitted::Fresh(_ticket) => return Err("forwarded after a drop".into()),
    }
    Ok(())
}

#[test]
fn a_call_forwarded_before_a_revocation_finishes_as_completed_after_drop() -> TestResult {
    let world = World::new()?;
    let grants = grants();
    let mut broker = Broker::create(&world.paths(), grants, Box::new(|| NOW_MS))?;
    broker.seal("model-key", "person:tom", &Secret::from_slice(b"sk-demo"))?;
    let issued = broker.issue_capped(&world.holder, "model-key", 5, NOW_MS + 60_000, Some(100))?;
    let admitted = fresh(broker.admit_use(&issued.token, &world.present(&issued.id)?, 40)?)?;
    let forwarded = broker.at_forward_boundary(admitted)?;
    broker
        .permissions()
        .revoke("agent:noor", "model-key")
        .expect("local grants lock must be healthy");
    broker.settle(forwarded, 25)?;
    let last = broker
        .audit()
        .audit_every_line()?
        .into_iter()
        .rfind(|recorded| recorded.line.kind == AuditKind::Settlement)
        .map(|recorded| (recorded.line.outcome, recorded.line.spend));
    assert_eq!(last, Some(("completed_after_drop".to_owned(), Some(25))));
    Ok(())
}
