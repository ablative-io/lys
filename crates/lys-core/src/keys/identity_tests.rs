#![cfg(test)]

use super::*;

#[path = "identity_tests/canonical_keys.rs"]
mod canonical_keys;
#[path = "identity_tests/from_env.rs"]
mod from_env;
#[path = "identity_tests/persistence.rs"]
mod persistence;
#[path = "identity_tests/signing.rs"]
mod signing;
#[path = "identity_tests/x25519.rs"]
mod x25519;

/// The name of the environment variable read by [`Ed25519Identity::from_env`],
/// which its error messages are checked against.
const TEST_ENV_VAR: &str = "LYS_IDENTITY_KEY";

fn identity_from_seed(seed: [u8; 32]) -> Ed25519Identity {
    Ed25519Identity::from_seed(&Zeroizing::new(seed))
}
