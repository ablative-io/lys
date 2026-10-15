/// The bytes a request's signature is over. The runner id and the
/// challenge are hexadecimal, so no field can carry the newline that ends
/// the one before it.
#[must_use]
pub fn signed_bytes(version: u32, runner: &str, challenge: &str, act: &str) -> Vec<u8> {
    format!("{REQUEST_DOMAIN}\n{version}\n{runner}\n{challenge}\n{act}").into_bytes()
}

const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Lowercase hex of `bytes`.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The bytes `text` spells in lowercase hex, `None` when it does not.
#[must_use]
pub fn unhex(text: &str) -> Option<Vec<u8>> {
    let digit = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0 {
        return None;
    }
    bytes
        .chunks(2)
        .map(|pair| Some((digit(pair[0])? << 4) | digit(pair[1])?))
        .collect()
}

/// A fresh nonce: 16 random bytes in hex.
#[must_use]
pub fn nonce() -> String {
    hex(&rand::random::<[u8; 16]>())
}
