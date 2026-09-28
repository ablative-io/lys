#![cfg(test)]
//! CONFORMANCE 7.6, the revoke: the person acted for under a lease revokes
//! it at any time, issuing stops at once and the lease reads upstream
//! pending until the system behind acknowledges; its holder is told to
//! relinquish; anyone else, the secret's owner included, is answered as for
//! a lease that does not exist; and a lease already ended is refused with
//! how, when and by whom it first ended, recording nothing.

#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/leases.rs"]
mod leases;

use fixture::{A_ORG, AGENT_A, B_ORG, PERSON_A, PERSON_B, TEAM_B, TestResult, asker, list, names};
use leases::{CONFIRMED, Leases, PERSON_C, RELINQUISHED, REVOKED, UNCONFIRMED};
use lys_secrets::{AskerKind, EndAct, HandleId};
use serde_json::Value;

/// The body a revoke of a lease never issued answers with.
fn never_issued(world: &mut Leases) -> Result<Value, Box<dyn std::error::Error>> {
    let refused = world
        .broker
        .revoke_lease(
            PERSON_B,
            &HandleId::from_text("no-such-lease"),
            &mut |_lease: &str, _secret: &str| {},
        )
        .err()
        .ok_or("a lease never issued was revoked")?;
    assert_eq!(refused.status(), 404);
    Ok(refused.body())
}

#[test]
fn conformance_7_6_a_revoke_stops_issuing_at_once_and_reads_upstream_pending() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;
    assert!(world.use_lease(&l1)?, "the lease issues before the revoke");
    let forwarded = world.forwarded;

    let (status, body) = world.revoke(PERSON_A, &l1);
    assert_eq!(status, 200, "{body}");
    assert!(!world.use_lease(&l1)?, "the next use is refused");
    assert_eq!(world.forwarded, forwarded, "nothing reached the upstream");
    assert_eq!(world.lines(&l1, UNCONFIRMED)?, 1, "one line for the move");
    let (status, read) = world.read(PERSON_A, &l1);
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["ended_by"], "revoke", "{read}");
    assert_eq!(read["issuing"], "stopped", "{read}");
    assert_eq!(read["upstream"], "pending", "{read}");
    assert_eq!(world.revoke_requests, vec![l1.id.as_str().to_owned()]);

    assert!(world.ack(&l1)?, "the acknowledgement moves the state");
    assert_eq!(world.upstream(&l1)?, "confirmed");
    assert_eq!(world.lines(&l1, CONFIRMED)?, 1, "one more line, for that move");
    assert_eq!(world.lines(&l1, UNCONFIRMED)?, 1);
    Ok(())
}

#[test]
fn conformance_7_6_a_caller_who_cannot_discover_the_lease_learns_nothing() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;
    let person_b = asker(PERSON_B, AskerKind::Person, &[TEAM_B]);
    let seen = names(&list(&world.broker, &person_b, Some("organisation")).1);
    assert!(seen.contains(&A_ORG.to_owned()), "person_b sees a_org: {seen:?}");
    let never = never_issued(&mut world)?;

    for caller in [PERSON_B, PERSON_C] {
        let (status, body) = world.revoke(caller, &l1);
        assert_eq!(status, 404, "{body}");
        assert_eq!(body["error"], "lease_not_found", "{body}");
        assert_eq!(body.to_string(), never.to_string(), "byte-identical");
        let text = body.to_string();
        for named in [l1.id.as_str(), A_ORG, B_ORG, AGENT_A, PERSON_A, PERSON_B, PERSON_C] {
            assert!(!text.contains(named), "{named} named in {text}");
        }
        let (status, read) = world.read(caller, &l1);
        assert_eq!(status, 404, "{read}");
        assert_eq!(read.to_string(), never.to_string(), "the read answers alike");
    }

    let forwarded = world.forwarded;
    assert!(world.use_lease(&l1)?, "the lease keeps issuing");
    assert_eq!(world.forwarded, forwarded + 1);
    assert_eq!(world.revoke_requests.len(), 0, "the system behind is not asked");
    assert_eq!(world.lines(&l1, REVOKED)?, 0);
    Ok(())
}

#[test]
fn conformance_7_6_the_holder_is_told_to_relinquish() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;

    let (status, body) = world.revoke(AGENT_A, &l1);
    assert_eq!(status, 403, "{body}");
    assert_eq!(body["error"], "holder_relinquishes", "{body}");
    assert_eq!(body["lease"], l1.id.as_str(), "{body}");
    assert_eq!(body["act"], "relinquish", "{body}");

    let (status, read) = world.read(PERSON_A, &l1);
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["ended_by"], Value::Null, "L1 carries no end: {read}");
    let forwarded = world.forwarded;
    assert!(world.use_lease(&l1)?, "the lease keeps issuing");
    assert_eq!(world.forwarded, forwarded + 1);
    assert_eq!(world.revoke_requests.len(), 0);
    assert_eq!(world.lines(&l1, RELINQUISHED)?, 0);
    Ok(())
}

#[test]
fn conformance_7_6_the_secrets_owner_may_not_revoke_as_owner() -> TestResult {
    let mut world = Leases::new()?;
    let l2 = world.issue(AGENT_A, B_ORG)?;
    let never = never_issued(&mut world)?;

    let (status, body) = world.revoke(PERSON_B, &l2);
    assert_eq!(status, 404, "{body}");
    assert_eq!(body["error"], "lease_not_found", "{body}");
    assert_eq!(body.to_string(), never.to_string(), "naming nothing");
    assert!(world.use_lease(&l2)?, "L2 keeps issuing");
    assert_eq!(world.revoke_requests.len(), 0);
    Ok(())
}

#[test]
fn conformance_7_6_a_holder_who_is_the_person_acted_for_revokes() -> TestResult {
    let mut world = Leases::new()?;
    let l4 = world.issue(PERSON_A, A_ORG)?;

    let (status, body) = world.revoke(PERSON_A, &l4);
    assert_eq!(status, 200, "{body}");
    let (status, read) = world.read(PERSON_A, &l4);
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["ended_by"], "revoke", "{read}");
    assert_eq!(world.lines(&l4, REVOKED)?, 1);
    assert_eq!(world.lines(&l4, RELINQUISHED)?, 0);
    Ok(())
}

#[test]
fn conformance_7_6_a_second_revoke_is_refused_and_records_nothing() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;
    let revoked_at = world.now();
    let (status, body) = world.revoke(PERSON_A, &l1);
    assert_eq!(status, 200, "{body}");
    world.advance(5_000);
    let before = world.broker.audit().len();

    let (status, body) = world.revoke(PERSON_A, &l1);
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], "lease_already_ended", "{body}");
    assert_eq!(body["ended_by"], "revoke", "{body}");
    assert_eq!(body["ended_at_ms"], revoked_at, "{body}");
    assert_eq!(body["ended_by_identity"], PERSON_A, "{body}");
    assert_eq!(world.broker.audit().len(), before, "nothing recorded");
    assert_eq!(world.lines(&l1, REVOKED)?, 1, "exactly one end record");
    assert_eq!(world.revoke_requests.len(), 1, "one revoke request in all");
    Ok(())
}

#[test]
fn conformance_7_6_a_lease_past_its_window_is_refused_as_expired() -> TestResult {
    let mut world = Leases::new()?;
    let l5 = world.issue(AGENT_A, A_ORG)?;
    let closed = world.pass_window(&l5)?;

    let (status, body) = world.revoke(PERSON_A, &l5);
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["error"], "lease_already_ended", "{body}");
    assert_eq!(body["ended_by"], "expired", "{body}");
    assert_eq!(body["ended_at_ms"], closed, "{body}");
    assert_eq!(world.lines(&l5, REVOKED)?, 0);
    assert_eq!(world.lines(&l5, RELINQUISHED)?, 0);
    assert_eq!(world.revoke_requests.len(), 0);
    Ok(())
}

#[test]
fn conformance_7_6_nothing_but_the_acknowledgement_confirms_a_revoked_lease() -> TestResult {
    let mut world = Leases::new()?;
    let l1 = world.issue(AGENT_A, A_ORG)?;
    let (status, body) = world.revoke(PERSON_A, &l1);
    assert_eq!(status, 200, "{body}");

    let readings = world.nine_steps(&l1, EndAct::Revoke)?;
    assert_eq!(readings.len(), 9, "one reading for each step");
    assert!(
        readings.iter().all(|reading| reading == "pending"),
        "{readings:?}"
    );

    assert!(world.ack(&l1)?);
    assert_eq!(world.upstream(&l1)?, "confirmed");
    assert_eq!(world.lines(&l1, CONFIRMED)?, 1);
    Ok(())
}
