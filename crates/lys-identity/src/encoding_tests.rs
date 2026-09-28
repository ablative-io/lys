//! The canonical head writer against RFC 8949.

use super::{MAJOR_UNSIGNED, head};

/// The head writer against RFC 8949 Appendix A at every width boundary, so a drift from
/// the shortest form fails here on its own line, not only through a round trip.
#[test]
fn heads_match_rfc_8949_appendix_a_at_every_boundary() {
    let vectors: [(u64, &[u8]); 9] = [
        (0, &[0x00]),
        (23, &[0x17]),
        (24, &[0x18, 0x18]),
        (255, &[0x18, 0xff]),
        (256, &[0x19, 0x01, 0x00]),
        (65_535, &[0x19, 0xff, 0xff]),
        (65_536, &[0x1a, 0x00, 0x01, 0x00, 0x00]),
        (4_294_967_295, &[0x1a, 0xff, 0xff, 0xff, 0xff]),
        (
            4_294_967_296,
            &[0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00],
        ),
    ];
    for (value, expected) in vectors {
        let mut out = Vec::new();
        head(&mut out, MAJOR_UNSIGNED, value);
        assert_eq!(out, expected, "the head of {value}");
    }
}
