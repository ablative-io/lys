//! [`AnchorConfig`] — what an anchor is told at construction, and what it is
//! deliberately never told.
//!
//! # The origin is not here, and there is nowhere for it to be
//!
//! An anchor's origin is fixed when its *store* is created and is read back
//! through `LeafStore::origin`. It is therefore not a configuration value: there
//! is no field for it here, no constant for it anywhere in this crate, and no
//! default or fallback that could stand in when a caller supplies nothing. A
//! configuration struct is exactly where such a constant would otherwise
//! accumulate — a `Default` impl naming a hostname, a "dev" origin used when the
//! field is left blank — so the absence is stated rather than left to be noticed.
//!
//! # No `Default`, and that is the point of the type
//!
//! This struct carries no fields yet. It exists in the signatures of
//! [`Anchor::create`](crate::Anchor::create) and
//! [`Anchor::open`](crate::Anchor::open) from the first version so that the
//! options which follow — admission policy, signer custody — arrive as *fields
//! a caller must name*, not as new parameters that break every call site, and
//! not as values that quietly default.
//!
//! Neither of those has a safe default. An admission policy chosen for a caller
//! is an operator decision made by a library, and a signer chosen for a caller
//! is worse. So the type refuses `Default` now, while it is free, rather than
//! having to withdraw it later once callers depend on it.

/// Construction-time options for an [`Anchor`](crate::Anchor).
///
/// Deliberately empty at this version — see the module docs for why the type
/// exists before it carries anything, and why it will never implement
/// [`Default`].
///
/// # The absence of `Default` is checked, not merely intended
///
/// The two snippets below are the check. The first establishes that the probe
/// itself compiles — a `compile_fail` test passes for *any* compilation error,
/// including a malformed snippet or a name that does not resolve, so on its own
/// it cannot tell a type that lacks `Default` from a test that never worked.
/// The second differs from it in exactly one respect: the type the bound is
/// applied to.
///
/// ```
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// lys_anchor::AnchorConfig::unconfigured();
/// ```
///
/// ```compile_fail
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// requires_default::<lys_anchor::AnchorConfig>();
/// ```
#[derive(Debug, Clone)]
pub struct AnchorConfig {}

impl AnchorConfig {
    /// The configuration of an anchor with nothing configured.
    ///
    /// Named for what it is rather than `new`, because it is not a starting
    /// point that is later filled in: at this version it is the only value the
    /// type has. When a field arrives, a caller that wants the field must reach
    /// a constructor that takes it, and this one will not quietly acquire a
    /// value on their behalf.
    pub fn unconfigured() -> Self {
        Self {}
    }
}
