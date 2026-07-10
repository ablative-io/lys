//! Hex encoding and parsing for public key and digest values.
//!
//! `lys-core` keeps its own hex helper crate-private, so the CLI carries
//! this small mirror, plus strict parsing for 32-byte keys supplied as
//! command-line arguments. Used only for public material — public keys,
//! payload hashes — never for seeds or signatures' private inputs.

/// Lowercase hex encoding of a byte slice.
pub fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        // Deliberate discard: `fmt::Write` for `String` is infallible —
        // writing to an in-memory String can never return an error.
        let _ = s.write_fmt(format_args!("{b:02x}"));
    }
    s
}

/// Decodes exactly 64 hexadecimal characters into 32 bytes.
///
/// Accepts upper- or lowercase digits; any other length or character yields
/// `None`. Used to parse public keys supplied on the command line (the
/// format `lys key inspect` prints).
pub fn parse_hex_32(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let high = char::from(pair[0]).to_digit(16)?;
        let low = char::from(pair[1]).to_digit(16)?;
        *slot = u8::try_from((high << 4) | low).ok()?;
    }
    Some(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
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
}
