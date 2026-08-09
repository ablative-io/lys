#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::bytes_eq_no_early_exit as bytes_eq;

// ⚠️ WHAT THESE TESTS CAN AND CANNOT SEE, STATED BEFORE THEY ARE READ.
//
// They establish the FUNCTIONAL claim — `bytes_eq_no_early_exit` agrees with
// equality — and the second party for it is `std`'s slice `==`, which is a
// genuinely different algorithm (it is free to short-circuit and to use
// `memcmp`) written by people who never saw this crate.
//
// They CANNOT see the constant-time claim, and no outcome-based test can.
// Adding an early `return false` inside the loop changes the work done and
// changes no result, so every assertion below stays green while the property
// the module exists for is gone. That is the same shape as the delegation
// verifier's timing defect, which needed a counting instrument rather than a
// wall-clock assertion — see `delegation/sign.rs`.
//
// What actually guards it here is that there is exactly ONE implementation, so
// the property has one place to be reviewed rather than two places to drift
// apart. Saying so is the point: a reader who assumes these tests cover timing
// would be wrong, and would be wrong in the direction of doing less review.

/// The matrix, and both outcomes must occur — a comparison that always
/// returned `false` would satisfy every rejection case on its own.
#[test]
fn it_agrees_with_slice_equality_on_every_case_and_both_outcomes_occur() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"", b""),
        (b"", b"a"),
        (b"a", b""),
        (b"a", b"a"),
        (b"a", b"b"),
        (b"abc", b"abc"),
        (b"abc", b"abd"),  // differs in the LAST byte
        (b"abc", b"zbc"),  // differs in the FIRST byte
        (b"abc", b"abcd"), // prefix, so length differs
        (b"abcd", b"abc"),
        (&[0x00], &[0x00]),
        (&[0x00], &[0xff]),
        (&[0xff; 32], &[0xff; 32]),
        (&[0xff; 32], &[0xfe; 32]),
    ];

    let mut accepted = 0usize;
    let mut rejected = 0usize;

    for (left, right) in cases {
        let expected = left == right;
        assert_eq!(
            bytes_eq(left, right),
            expected,
            "disagreed with slice equality on {left:?} vs {right:?}"
        );
        if expected {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }

    // Count what fired, not what passed: a loop that runs zero times satisfies
    // every assertion inside it.
    assert_eq!(cases.len(), 14, "the case table changed size unnoticed");
    // 5 and 9, and the numbers are hand-counted deliberately. My first attempt
    // asserted 6 and 8 — wrong, and this assertion is what caught it. A test
    // that derived the expected counts from the same loop that produced them
    // would have agreed with any miscount, including one that left the table
    // with no accepting cases at all.
    assert_eq!(
        accepted, 5,
        "not enough ACCEPTING cases to prove it can accept"
    );
    assert_eq!(rejected, 9, "not enough rejecting cases");
}

/// A difference in the last byte must be caught as surely as one in the first.
/// This is the case an implementation that stopped early would still pass, so
/// it is here for the functional half only — it is deliberately NOT presented
/// as evidence about timing.
#[test]
fn a_difference_in_the_final_byte_is_still_a_difference() {
    let mut a = [7u8; 32];
    let mut b = [7u8; 32];
    assert!(bytes_eq(&a, &b));

    b[31] ^= 0x01;
    assert!(!bytes_eq(&a, &b));

    a[31] ^= 0x01;
    assert!(bytes_eq(&a, &b), "flipping both back must agree again");
}

/// Every single-byte position, so no position is skipped by an off-by-one in
/// the zip.
#[test]
fn every_byte_position_is_actually_examined() {
    let base = [0x5au8; 32];
    let mut flipped_positions = 0usize;

    for position in 0..base.len() {
        let mut other = base;
        other[position] ^= 0xff;
        assert!(
            !bytes_eq(&base, &other),
            "a flip at position {position} was not detected"
        );
        flipped_positions += 1;
    }

    assert_eq!(flipped_positions, 32, "not every position was tried");
}
