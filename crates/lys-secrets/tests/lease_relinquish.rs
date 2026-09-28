#![cfg(test)]
//! CONFORMANCE 7.6, the relinquish: a lease's holder gives it back, recorded
//! as its own act and never as a revoke; issuing stops at once and the lease
//! reads upstream pending until the system behind acknowledges, exactly as
//! after a revoke; the person acted for may not relinquish, anyone else is
//! answered as for a lease that does not exist, and a lease already ended is
//! refused with how, when and by whom it first ended, recording nothing.

#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/leases.rs"]
mod leases;

use fixture::{A_ORG, A_PERSONAL, AGENT_A, PERSON_A, PERSON_B, TestResult, list, names, person_a};
use leases::{CONFIRMED, Leases, PERSON_C, RELINQUISHED, REVOKED, UNCONFIRMED};
use lys_secrets::EndAct;

#[test]
fn conformance_7_6_a_relinquish_stops_issuing_at_once_and_reads_upstream_pending() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;
    assert!(world.use_lease(&l1)?, "the lease issues before the relinquish");
    let forwarded = world.forwarded;

    let (status, body) = world.relinquish(AGENT_A, &l1);
    assert_eq!(status, 200, "{body}");
    assert!(!world.use_lease(&l1)?, "the next use is refused");
    assert_eq!(world.forwarded, forwarded, "nothing reached the upstream");
    let (status, read) = world.read(PERSON_A, &l1);
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["ended_by"], "relinquish", "{read}");
    assert_eq!(read["issuing"], "stopped", "{read}");
    assert_eq!(read["upstream"], "pending", "{read}");
    assert_eq!(world.lines(&l1, UNCONFIRMED)?, 1, "one line for the move");
    assert_eq!(world.revoke_requests, vec![l1.id.as_str().to_owned()]);

    assert!(world.ack(&l1)?, "the double delivers its acknowledgement once");
    assert_eq!(world.upstream(&l1)?, "confirmed");
    assert_eq!(world.lines(&l1, CONFIRMED)?, 1);
    Ok(())
}

#[test]
fn conformance_7_6_a_relinquish_is_recorded_as_the_holders_own_act() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;
    let relinquished_at = world.now();

    let (status, body) = world.relinquish(AGENT_A, &l1);
    assert_eq!(status, 200, "{body}");
    let (status, read) = world.read(AGENT_A, &l1);
    assert_eq!(status, 200, "the holder reads its own lease: {read}");
    assert_eq!(read["ended_by"], "relinquish", "{read}");
    assert_eq!(read["ended_by_identity"], AGENT_A, "{read}");
    assert_eq!(read["ended_at_ms"], relinquished_at, "{read}");
    assert_eq!(world.lines(&l1, RELINQUISHED)?, 1);
    assert_eq!(world.lines(&l1, REVOKED)?, 0, "L1 carries no revoke record");
    Ok(())
}

#[test]
fn conformance_7_6_the_person_acted_for_may_not_relinquish() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;

    let (status, body) = world.relinquish(PERSON_A, &l1);
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["error"], "relinquish_not_permitted", "{body}");
    assert_eq!(body["lease"], l1.id.as_str(), "{body}");
    assert!(world.use_lease(&l1)?, "L1 keeps issuing");
    assert_eq!(world.lines(&l1, RELINQUISHED)?, 0);
    assert_eq!(world.revoke_requests.len(), 0);
    Ok(())
}

#[test]
fn conformance_7_6_a_caller_who_cannot_discover_the_lease_may_not_relinquish() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;

    for caller in [PERSON_B, PERSON_C] {
        let (status, body) = world.relinquish(caller, &l1);
        assert_eq!(status, 404, "{body}");
        assert_eq!(body["error"], "lease_not_found", "{body}");
        assert!(!body.to_string().contains(l1.id.as_str()), "{body}");
    }
    let forwarded = world.forwarded;
    assert!(world.use_lease(&l1)?, "L1 keeps issuing");
    assert_eq!(world.forwarded, forwarded + 1);
    assert_eq!(world.revoke_requests.len(), 0);
    Ok(())
}

#[test]
fn conformance_7_6_a_relinquish_after_a_revoke_is_refused() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;
    let revoked_at = world.now();
    let (status, body) = world.revoke(PERSON_A, &l1);
    assert_eq!(status, 200, "{body}");
    world.advance(5_000);
    let before = world.broker.audit().len();

    let (status, body) = world.relinquish(AGENT_A, &l1);
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], "lease_already_ended", "{body}");
    assert_eq!(body["ended_by"], "revoke", "{body}");
    assert_eq!(body["ended_at_ms"], revoked_at, "{body}");
    assert_eq!(body["ended_by_identity"], PERSON_A, "{body}");
    assert_eq!(world.broker.audit().len(), before, "nothing recorded");
    assert_eq!(world.lines(&l1, REVOKED)?, 1, "exactly one end record");
    assert_eq!(world.lines(&l1, RELINQUISHED)?, 0);
    Ok(())
}

#[test]
fn conformance_7_6_a_second_relinquish_is_refused() -> TestResult {
    let mut world = Leases::new()?;
    let l3 = world.issue(AGENT_A, A_ORG)?;
    let relinquished_at = world.now();
    let (status, body) = world.relinquish(AGENT_A, &l3);
    assert_eq!(status, 200, "{body}");
    world.advance(5_000);

    let (status, body) = world.relinquish(AGENT_A, &l3);
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], "lease_already_ended", "{body}");
    assert_eq!(body["ended_by"], "relinquish", "{body}");
    assert_eq!(body["ended_at_ms"], relinquished_at, "{body}");
    assert_eq!(body["ended_by_identity"], AGENT_A, "{body}");
    assert_eq!(world.lines(&l3, RELINQUISHED)?, 1, "exactly one end record");
    assert_eq!(world.revoke_requests.len(), 1);
    Ok(())
}

#[test]
fn conformance_7_6_a_revoke_after_a_relinquish_is_refused() -> TestResult {
    let mut world = Leases::new()?;
    let l3 = world.issue(AGENT_A, A_ORG)?;
    let relinquished_at = world.now();
    let (status, body) = world.relinquish(AGENT_A, &l3);
    assert_eq!(status, 200, "{body}");
    world.advance(5_000);
    let before = world.broker.audit().len();

    let (status, body) = world.revoke(PERSON_A, &l3);
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], "lease_already_ended", "{body}");
    assert_eq!(body["ended_by"], "relinquish", "{body}");
    assert_eq!(body["ended_at_ms"], relinquished_at, "{body}");
    assert_eq!(body["ended_by_identity"], AGENT_A, "{body}");
    assert_eq!(world.broker.audit().len(), before, "nothing recorded");
    assert_eq!(world.lines(&l3, RELINQUISHED)?, 1, "exactly one end record");
    assert_eq!(world.lines(&l3, REVOKED)?, 0);
    Ok(())
}

#[test]
fn conformance_7_6_nothing_but_the_acknowledgement_confirms_a_relinquished_lease() -> TestResult {
    let mut world = Leases::new()?;
    let l3 = world.issue(AGENT_A, A_ORG)?;
    let (status, body) = world.relinquish(AGENT_A, &l3);
    assert_eq!(status, 200, "{body}");

    let readings = world.nine_steps(&l3, EndAct::Relinquish)?;
    assert_eq!(readings.len(), 9, "one reading for each step");
    assert!(
        readings.iter().all(|reading| reading == "pending"),
        "{readings:?}"
    );
    assert_eq!(
        names(&list(&world.broker, &person_a(), Some("mine")).1),
        vec![A_PERSONAL.to_owned()]
    );

    assert!(world.ack(&l3)?);
    assert_eq!(world.upstream(&l3)?, "confirmed");
    assert_eq!(world.lines(&l3, CONFIRMED)?, 1);
    Ok(())
}
