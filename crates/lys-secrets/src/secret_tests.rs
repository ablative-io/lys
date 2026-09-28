#![cfg(test)]
//! The redacting type's fields are exactly the ones it was landed with: the
//! pattern below names each of them and no rest pattern, so the crate's
//! tests stop compiling the moment a field is added to it.

use zeroize::Zeroizing;

use super::{REDACTED, Secret};

#[test]
fn secret_fields_unchanged() {
    let secret = Secret::from_slice(b"generated test bytes");
    assert_eq!(format!("{secret:?}"), REDACTED);
    let Secret(buffer) = secret;
    let buffer: Zeroizing<Vec<u8>> = buffer;
    assert_eq!(buffer.as_slice(), b"generated test bytes");
}
