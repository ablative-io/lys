//! CRC-32C (Castagnoli), table-driven, in the crate: the record checksum of
//! the segment layout. No dependency is added for it, so `Cargo.lock` does
//! not move under the battery.

/// The reflected Castagnoli polynomial.
const POLYNOMIAL: u32 = 0x82F6_3B78;

const fn table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut index: u32 = 0;
    while index < 256 {
        let mut crc = index;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                POLYNOMIAL ^ (crc >> 1)
            } else {
                crc >> 1
            };
            bit += 1;
        }
        table[index as usize] = crc;
        index += 1;
    }
    table
}

static TABLE: [u32; 256] = table();

/// The CRC-32C of `bytes`.
pub(crate) fn crc32c(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc = TABLE[((crc ^ u32::from(byte)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::crc32c;

    #[test]
    fn the_check_value_of_the_standard_vector_is_answered() {
        // The CRC-32C check value from the CRC catalogue: "123456789".
        assert_eq!(crc32c(b"123456789"), 0xE306_9283);
        assert_eq!(crc32c(b""), 0);
        assert_ne!(crc32c(b"leaf"), crc32c(b"leaf!"));
    }
}
