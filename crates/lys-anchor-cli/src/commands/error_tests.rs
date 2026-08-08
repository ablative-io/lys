#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

use super::*;

/// A refusal reaches the operator with the library's words and nothing added.
///
/// The second party is `lys-anchor`'s own `AnchorError::NotAdmitted`, whose
/// message is fixed there and whose whole design is that it carries nothing.
/// This asserts the CLI is a pass-through: equality with the library's
/// `Display`, not a "contains" check, because appending a hint about the
/// configured policy is exactly the drift that would still pass a substring
/// test.
#[test]
fn an_admission_refusal_is_forwarded_verbatim() {
    let library = AnchorError::NotAdmitted.to_string();
    let cli = CliError::from(AnchorError::NotAdmitted).to_string();
    assert_eq!(cli, library);
    assert!(
        !cli.contains("--admit") && !cli.contains("max-bytes"),
        "the CLI must not reconstruct the rule the library refused to disclose: {cli}"
    );
}

/// An uninitialized directory gets the remedy the store cannot know.
///
/// The store reports "not initialized"; only the CLI knows the command that
/// fixes it. Asserting the remedy names *this* binary, because the mapping was
/// copied from `crates/lys` and a leftover `lys log init` would send an operator
/// to a command that cannot create an anchor.
#[test]
fn an_uninitialized_directory_names_the_command_that_creates_one() {
    let path = PathBuf::from("/tmp/no-such-anchor");
    let err = CliError::from(StoreError::NotInitialized { path });
    assert!(matches!(err, CliError::AnchorDirMissing { .. }), "{err}");
    let message = err.to_string();
    assert!(message.contains("not initialized"), "{message}");
    assert!(message.contains("lys-anchor init"), "{message}");
    assert!(
        !message.contains("lys log init"),
        "the remedy must be this binary's, not the log CLI's: {message}"
    );
}

/// Re-initialization explains that the origin is pinned, rather than reporting a
/// bare file conflict.
#[test]
fn re_initialization_explains_the_pin() {
    let path = PathBuf::from("/tmp/an-anchor");
    let err = CliError::from(StoreError::AlreadyInitialized { path });
    let message = err.to_string();
    assert!(
        matches!(err, CliError::AnchorDirInvalid { .. }),
        "{message}"
    );
    assert!(message.contains("origin is pinned at init"), "{message}");
}

/// `CliResult` stays small enough that `result_large_err` is satisfied without
/// an allow.
///
/// The boxing in this module exists for this reason and nothing else; a future
/// variant that inlined a `TrustError` would grow every `Result` in the binary,
/// and the lint would be the thing that noticed. Asserting it here states the
/// budget where the boxing decision is, so removing the boxes fails a test as
/// well as a lint.
#[test]
fn the_error_type_stays_small() {
    assert!(
        size_of::<CliError>() <= 128,
        "CliError is {} bytes",
        size_of::<CliError>()
    );
}
