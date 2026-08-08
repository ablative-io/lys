#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Gates on [`AcceptAll`] and [`MaxSize`].
//!
//! # Where the second party comes from
//!
//! These policies are small enough that a test written from the implementation
//! would restate it, so the checks are keyed on things the implementation did
//! not choose:
//!
//! - **The written contract.** [`MaxSize`]'s docs say the bound is *inclusive*
//!   and that `MaxSize::new(0)` admits the empty statement and nothing else.
//!   That sentence was written before this file and can be wrong, which is what
//!   makes disagreeing with it possible.
//! - **A sweep with a counted outcome split**, rather than a pair of hand-picked
//!   cases. A loop that ran zero times would satisfy every assertion inside it,
//!   so the number of admissions *and* the number of refusals are asserted
//!   against the arithmetic of the boundary rather than against what the sweep
//!   happened to produce.
//! - **The other input.** [`MaxSize`] is documented as bounding the statement
//!   and not the credential; a case where the credential is enormous and the
//!   statement is tiny separates the two, and no assertion about statement
//!   length alone could.

use super::*;
use crate::admission::AuthenticatedPeer;
use crate::wire::Submission;

/// Nothing was established about whoever sent these. Neither policy here looks
/// at the context, which is what the last two cases of the first test check.
const NOBODY: SubmitterContext<'static> = SubmitterContext::Unidentified;

/// A submission of `statement`.
fn bare(statement: &[u8]) -> Submission<'_> {
    Submission { statement }
}

#[test]
fn accept_all_admits_everything_it_is_shown() {
    let long = vec![0xab_u8; 100_000];
    let credential = vec![0x01_u8; 4096];
    let authenticated = SubmitterContext::AuthenticatedByTransport(
        AuthenticatedPeer::verified_by_transport(&credential),
    );
    // Every provenance, because AcceptAll is documented as ignoring all of
    // them: a policy that refused an unidentified submitter would pass a sweep
    // that always supplied a credential, and vice versa.
    let cases: [(Submission<'_>, SubmitterContext<'_>); 6] = [
        (bare(b""), NOBODY),
        (bare(b"an ordinary statement"), NOBODY),
        (bare(&[0xff, 0x00, 0xc3, 0x28]), NOBODY),
        (bare(&long), NOBODY),
        (
            bare(b"asserted something meaningless"),
            SubmitterContext::AssertedBySubmitter(&credential),
        ),
        (bare(b"authenticated by somebody"), authenticated),
    ];

    let mut admitted = 0;
    for (submission, context) in cases {
        assert_eq!(
            AcceptAll.admit(&submission, &context),
            Ok(()),
            "AcceptAll refused a submission"
        );
        admitted += 1;
    }
    // Count what fired: an empty case array satisfies the loop without
    // admitting anything at all.
    assert_eq!(admitted, 6, "the sweep must have run every case");
}

#[test]
fn max_size_admits_up_to_and_including_the_limit_and_refuses_past_it() {
    const LIMIT: usize = 4;
    let policy = MaxSize::new(LIMIT);

    let mut admitted = 0;
    let mut refused = 0;
    let mut cases = 0;
    for length in 0_usize..=8 {
        let statement = vec![b'x'; length];
        let outcome = policy.admit(&bare(&statement), &NOBODY);
        // The expectation comes from the *documented* bound — inclusive — not
        // from what the policy happened to answer, so a policy that made the
        // bound exclusive fails at the boundary rather than being described by
        // it.
        if length <= LIMIT {
            assert_eq!(
                outcome,
                Ok(()),
                "a statement of {length} bytes was refused inside a limit of {LIMIT}"
            );
            admitted += 1;
        } else {
            assert_eq!(
                outcome,
                Err(NotAdmitted),
                "a statement of {length} bytes was admitted past a limit of {LIMIT}"
            );
            refused += 1;
        }
        cases += 1;
    }

    // Assert rejections, not just successes, and assert the split rather than
    // the total: a policy that admitted everything and one that refused
    // everything both produce nine cases.
    assert_eq!(cases, 9);
    assert_eq!(admitted, LIMIT + 1, "0..=LIMIT must be admitted");
    assert_eq!(refused, 8 - LIMIT, "LIMIT+1..=8 must be refused");
}

#[test]
fn a_zero_limit_admits_the_empty_statement_and_nothing_else() {
    // The docs call this usable rather than degenerate. If that is wrong, it is
    // wrong here.
    let policy = MaxSize::new(0);
    assert_eq!(policy.admit(&bare(b""), &NOBODY), Ok(()));
    assert_eq!(policy.admit(&bare(b"x"), &NOBODY), Err(NotAdmitted));
    assert_eq!(policy.max_bytes(), 0);
}

#[test]
fn max_size_bounds_the_statement_and_not_the_submitters_credential() {
    // Documented behaviour, and the case that separates the two fields: a
    // policy that measured `statement.len() + credential.len()`, or that
    // measured the wrong field entirely, refuses this and passes every
    // statement-only test above.
    let policy = MaxSize::new(8);
    let enormous = vec![0x00_u8; 1_000_000];
    let with_enormous_credential = SubmitterContext::AssertedBySubmitter(&enormous);
    assert_eq!(
        policy.admit(&bare(b"short"), &with_enormous_credential),
        Ok(())
    );

    // The mirror: a long statement is refused whatever came alongside it.
    let long = vec![b'y'; 9];
    assert_eq!(
        policy.admit(
            &bare(&long),
            &SubmitterContext::AssertedBySubmitter(b"tiny")
        ),
        Err(NotAdmitted)
    );
}

#[test]
fn the_limit_is_readable_by_the_operator_and_unreachable_from_a_refusal() {
    let policy = MaxSize::new(1234);
    assert_eq!(policy.max_bytes(), 1234);

    // The refusal for a statement one byte over is the same value as the
    // refusal for a statement a megabyte over. There is no arithmetic a
    // submitter can do on it.
    let just_over = vec![b'z'; 1235];
    let far_over = vec![b'z'; 1_048_576];
    let near = policy.admit(&bare(&just_over), &NOBODY).unwrap_err();
    let far = policy.admit(&bare(&far_over), &NOBODY).unwrap_err();
    assert_eq!(near, far);
    assert_eq!(format!("{near:?}"), format!("{far:?}"));
    assert_eq!(format!("{near:?}"), "NotAdmitted");
}
