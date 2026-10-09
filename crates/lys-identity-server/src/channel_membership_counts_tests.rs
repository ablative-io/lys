//! ACCESS-006 R6: each call's outcome is counted apart. An answer allowed,
//! held, refused at a revision or unanswered, a page answered or refused,
//! and a call failed each move their own count and no other, so a failure
//! is never counted as a success and the calls are the sum of the outcomes.

use std::sync::atomic::{AtomicU64, Ordering};

use lys_pass::membership::Verdict;
use lys_pass::rights::Resource;

use super::Counts;
use crate::error::ServerError;

fn scope() -> Resource {
    Resource {
        kind: "rooms.channel".to_owned(),
        id: "ward-a".to_owned(),
    }
}

fn allowed() -> Verdict {
    Verdict::Allowed {
        grant: "grant-a".to_owned(),
        path: vec!["grant-a".to_owned()],
        scope: scope(),
    }
}

fn held() -> Verdict {
    Verdict::Held {
        grant: "grant-a".to_owned(),
        scope: scope(),
        mode: lys_pass::Mode::ByDraft,
    }
}

fn refused() -> Verdict {
    Verdict::Refused {
        refusal: "NotHeld".to_owned(),
        reason: "no grant reaches it".to_owned(),
    }
}

/// Every outcome count, by name, in a fixed order.
fn outcomes(counts: &Counts) -> [(&'static str, u64); 7] {
    let read = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
    [
        ("allowed", read(&counts.allowed)),
        ("held", read(&counts.held)),
        ("refused", read(&counts.refused)),
        ("unanswered", read(&counts.unanswered)),
        ("answered_pages", read(&counts.answered_pages)),
        ("refused_pages", read(&counts.refused_pages)),
        ("failed", read(&counts.failed)),
    ]
}

/// One decision's outcome, as a route hands it to the counts.
type Outcome<'a> = Result<(Option<u64>, Verdict), &'a ServerError>;

/// Count `outcome` as the decision and admission routes do.
fn decide(counts: &Counts, outcome: &Outcome<'_>) {
    counts.decided(
        outcome
            .as_ref()
            .map(|(revision, verdict)| (*revision, verdict))
            .map_err(|error| *error),
    );
}

/// Only `name` reads one; every other outcome reads zero.
fn only(counts: &Counts, name: &str) {
    for (outcome, n) in outcomes(counts) {
        assert_eq!(n, u64::from(outcome == name), "{outcome} after {name}");
    }
}

#[test]
fn each_decision_outcome_is_counted_apart() {
    let unavailable = ServerError::MembershipUnavailable {
        reason: "the authority could not be held".to_owned(),
    };
    let cases: [(&str, Outcome<'_>); 6] = [
        ("allowed", Ok((Some(7), allowed()))),
        ("held", Ok((Some(7), held()))),
        ("refused", Ok((Some(7), refused()))),
        // Refused before lookup, or not decided by the grants: no revision,
        // so never counted with the verdicts decided at one.
        ("unanswered", Ok((None, refused()))),
        ("unanswered", Ok((None, allowed()))),
        ("failed", Err(&unavailable)),
    ];
    for (name, outcome) in &cases {
        let counts = Counts::default();
        decide(&counts, outcome);
        only(&counts, name);
    }
}

#[test]
fn pages_answered_refused_and_failed_are_counted_apart() {
    let counts = Counts::default();
    counts.page(2, 1);
    only(&counts, "answered_pages");
    assert_eq!(counts.emitted.load(Ordering::Relaxed), 2);
    assert_eq!(counts.skipped.load(Ordering::Relaxed), 1);

    let counts = Counts::default();
    counts.refused_page();
    only(&counts, "refused_pages");

    let counts = Counts::default();
    counts.failed();
    only(&counts, "failed");
}

#[test]
fn the_calls_are_the_sum_of_their_outcomes() {
    let counts = Counts::default();
    let unavailable = ServerError::MembershipUnavailable {
        reason: "the grant log could not be read".to_owned(),
    };
    let decided: [Outcome<'_>; 3] = [
        Ok((Some(1), allowed())),
        Ok((Some(1), refused())),
        Ok((None, refused())),
    ];
    for outcome in &decided {
        counts.call();
        counts.hold();
        decide(&counts, outcome);
    }
    counts.call();
    decide(&counts, &Err(&unavailable));
    counts.call();
    counts.hold();
    counts.page(1, 0);
    counts.call();
    counts.hold();
    counts.refused_page();
    counts.call();
    counts.failed();

    let ended: u64 = outcomes(&counts).iter().map(|(_, n)| n).sum();
    assert_eq!(counts.calls.load(Ordering::Relaxed), 7);
    assert_eq!(ended, 7, "{:?}", outcomes(&counts));
    // A call that never held the authority is not counted as a hold.
    assert_eq!(counts.authority_holds.load(Ordering::Relaxed), 5);
    assert_eq!(counts.allowed.load(Ordering::Relaxed), 1);
    assert_eq!(counts.failed.load(Ordering::Relaxed), 2);
}
