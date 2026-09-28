#![cfg(test)]

use super::*;

mod canonical_keys;
mod from_env;
mod persistence;
mod signing;
mod x25519;

/// The name of the environment variable read by [`Ed25519Identity::from_env`],
/// which its error messages are checked against.
const TEST_ENV_VAR: &str = "LYS_IDENTITY_KEY";

fn identity_from_seed(seed: [u8; 32]) -> Ed25519Identity {
    Ed25519Identity::from_seed(&Zeroizing::new(seed))
}
