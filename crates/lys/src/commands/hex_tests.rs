#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn encodes_known_bytes() {
    assert_eq!(hex_lower(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
}

#[test]
fn parse_hex_32_round_trips_encoding() {
    let bytes: [u8; 32] = core::array::from_fn(|i| u8::try_from(i * 7 % 256).unwrap());
    assert_eq!(parse_hex_32(&hex_lower(&bytes)), Some(bytes));
}

#[test]
fn parse_hex_32_accepts_uppercase() {
    let hex = "AB".repeat(32);
    assert_eq!(parse_hex_32(&hex), Some([0xabu8; 32]));
}

#[test]
fn parse_hex_32_rejects_wrong_length_and_non_hex() {
    assert_eq!(parse_hex_32(""), None);
    assert_eq!(parse_hex_32(&"ab".repeat(31)), None);
    assert_eq!(parse_hex_32(&"ab".repeat(33)), None);
    let mut bad = "ab".repeat(32);
    bad.replace_range(0..2, "zz");
    assert_eq!(parse_hex_32(&bad), None);
}

#[test]
fn empty_slice_yields_empty_string() {
    assert_eq!(hex_lower(&[]), "");
}

#[test]
fn thirty_two_bytes_yield_sixty_four_chars() {
    let bytes = [0xabu8; 32];
    let hex = hex_lower(&bytes);
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c == 'a' || c == 'b'));
}
