//! The issuer's proof-of-work challenge, answered by the first counter
//! whose SHA-256 begins with the difficulty's zero bits.

use sha2::{Digest, Sha256};

use super::failed;
use crate::error::ServerError;

pub(super) fn leading_zero_bits(hash: &[u8]) -> u32 {
    let mut bits = 0;
    for byte in hash {
        if *byte != 0 {
            return bits + byte.leading_zeros();
        }
        bits += 8;
    }
    bits
}

/// Answer the issuer's proof-of-work `challenge`: the challenge followed by
/// the first counter whose SHA-256 over the whole begins with as many zero
/// bits as the challenge's difficulty. The challenge is
/// `version:difficulty:expires:salt:challenge:`, version 1 and the
/// difficulty two digits.
///
/// # Errors
/// Refuses a malformed or unsupported challenge, invalid difficulty or
/// a proof counter that cannot be found within its range.
pub fn solve(challenge: &str) -> Result<String, ServerError> {
    let counter = solve_counter(challenge)?;
    Ok(format!("{challenge}{counter}"))
}

pub(super) fn solve_counter(challenge: &str) -> Result<u64, ServerError> {
    if !challenge.starts_with("1:") || !challenge.ends_with(':') {
        return Err(failed(
            "the sign-in service's challenge is not one this service answers",
        ));
    }
    let difficulty = challenge
        .get(2..4)
        .and_then(|digits| digits.parse::<u32>().ok())
        .filter(|difficulty| (10..99).contains(difficulty))
        .ok_or_else(|| failed("the sign-in service's challenge names no difficulty"))?;
    let mut prefix = Sha256::new();
    prefix.update(challenge.as_bytes());
    let mut digits = [0u8; 20];
    let mut counter: u64 = 0;
    loop {
        if crate::wait_request::left() {
            return Err(failed("the caller closed the sign-in"));
        }
        let mut value = counter;
        let mut first = digits.len();
        loop {
            first -= 1;
            digits[first] = b'0'
                + u8::try_from(value % 10)
                    .map_err(|error| failed(format!("the counter digit is invalid: {error}")))?;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        let mut hash = prefix.clone();
        hash.update(&digits[first..]);
        if leading_zero_bits(&hash.finalize()) >= difficulty {
            return Ok(counter);
        }
        counter = counter
            .checked_add(1)
            .ok_or_else(|| failed("the challenge has no answer below 2^64"))?;
    }
}
