#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// The second party here is not the encoder: it is a vector produced by
/// something else entirely.
///
/// `hex_lower` and `parse_hex_32` are each other's inverse, so a round trip
/// through both proves only that they agree — any symmetric change is invisible
/// to it. The literal below is the hex of a key printed by `lys key inspect`
/// style output, checked against `printf` rather than against this file's own
/// encoder.
#[test]
fn hex_lower_matches_an_externally_produced_vector() {
    assert_eq!(hex_lower(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
    assert_eq!(hex_lower(&[]), "");
}

/// Refusals are counted, not merely permitted: every one of these must be
/// rejected, and the count is asserted so a loop that stopped running would be
/// visible.
#[test]
fn parse_hex_32_refuses_everything_that_is_not_64_hex_digits() {
    let rejected = [
        "",
        "00",
        &"0".repeat(63),
        &"0".repeat(65),
        &format!("{}zz", "0".repeat(62)),
        &format!("{} ", "0".repeat(63)),
    ];
    let mut refusals = 0;
    for candidate in rejected {
        assert!(parse_hex_32(candidate).is_none(), "accepted: {candidate:?}");
        refusals += 1;
    }
    assert_eq!(refusals, 6, "every case must have been exercised");
}

/// The positive control for the refusals above: a well-formed key parses, in
/// both cases, to the bytes an independent reading of the string gives.
#[test]
fn parse_hex_32_accepts_64_digits_in_either_case() {
    let lower = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
    let upper = lower.to_uppercase();
    let expected: [u8; 32] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
        0xee, 0xff,
    ];
    assert_eq!(parse_hex_32(lower), Some(expected));
    assert_eq!(parse_hex_32(&upper), Some(expected));
}
