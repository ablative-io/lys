//! Lowercase hex encoding for public key and digest output.
//!
//! `lys-core` keeps its own hex helper crate-private, so the CLI carries
//! this small mirror. Used only for public material — public keys, payload
//! hashes — never for seeds or signatures' private inputs.

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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn encodes_known_bytes() {
        assert_eq!(hex_lower(&[0x00, 0x0f, 0xa5, 0xff]), "000fa5ff");
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
