//! Byte comparison with the early exit removed, in one place.
//!
//! # Why this is a module rather than a local helper
//!
//! Two verifiers in this crate compare a caller-supplied expected key against
//! the key an artifact carries, and both must do it without the comparison
//! itself revealing *how far* the two agreed. Written locally in each, that is
//! two copies of a primitive whose value is a property no test observes — and
//! two copies of a rule are guarded by neither, because nothing makes them
//! fail together when one drifts.
//!
//! So there is exactly one implementation, and it is `pub(crate)` so no
//! consumer depends on it as API.
//!
//! # ⚠️ What this is NOT
//!
//! **This is early-exit removal, not a constant-time guarantee, and the
//! function is named for what it does rather than for what would be nicer to
//! claim.** The length check is a branch; a subject value's length is already
//! observable from the artifact's size; and nothing here forbids the optimiser
//! reintroducing a branch inside the loop. A real constant-time claim needs a
//! primitive that can forbid that, which this crate does not depend on.
//!
//! This caveat is carried verbatim in substance from the delegation verifier's
//! copy, **because the version before it overclaimed a related property and was
//! wrong**. Merging two copies into one is exactly the moment such a caveat
//! gets dropped as boilerplate, so it is restated rather than summarised.
//!
//! Private key material is never compared here. Every caller compares *public*
//! values whose comparison result must not leak a prefix.

/// Compare two byte slices without returning early on the first differing
/// byte.
///
/// Returns `false` immediately when the lengths differ — a length is not
/// secret in any caller here, since every one of them compares fixed-width
/// keys, and a length mismatch means the caller passed the wrong kind of
/// thing. Otherwise every byte of both inputs is read and accumulated, so the
/// work done does not depend on *where* they first differ.
///
/// See the module docs for the limits of that claim.
pub(crate) fn bytes_eq_no_early_exit(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (l, r) in left.iter().zip(right.iter()) {
        difference |= l ^ r;
    }
    difference == 0
}

#[cfg(test)]
#[path = "compare_tests.rs"]
mod tests;
