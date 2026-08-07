//! [`Signer`] — the custody boundary, and [`InProcessSigner`] — the bound that
//! records it is not one yet.
//!
//! # What this is for
//!
//! An anchor's whole value is the signature on what it publishes, so the key
//! that produces it is the thing an operator most wants to keep out of the
//! process. The trait exists now, before there is a second signing call site,
//! because "just give me the private key" leaks into every layer that touches
//! signing the moment it is allowed once, and retrofitting a boundary through
//! those layers afterwards is a rewrite.
//!
//! # ⚠️ This boundary is reserved, not usable — say so before anyone plans on it
//!
//! **An HSM, a KMS or any other remote signer can implement [`Signer`] today
//! and still cannot be used with this crate.** That is not an oversight to be
//! discovered at integration time; it is the current state, and it is written
//! here because claiming a swap is possible when it is not would be worse than
//! the gap itself.
//!
//! The reason is one level down. `lys-core`'s signing entry points take a
//! concrete `&Ed25519Identity` — `checkpoint::sign_note`, `receipt::sign_receipt`
//! and `tlog::build_inclusion_artifact` all do — so producing a lys artifact
//! requires handing over an in-process key. There are exactly two ways out and
//! this crate takes neither:
//!
//! - **Make `lys-core` signer-generic.** It is published and semver-bound, and
//!   the addition belongs behind its off-by-default, semver-exempt
//!   `unstable-anchor` feature rather than in a crate consumers depend on. Out
//!   of scope here, and named so the route is known.
//! - **Assemble the signed bytes in `lys-anchor` instead** — the note envelope,
//!   or a COSE `Sig_structure`. That would put a *second copy of a canonical
//!   encoder* in the workspace, which is the failure this repository guards
//!   against hardest: two encoders drift silently, and every round-trip test on
//!   either side keeps passing while they do.
//!
//! So the gap is expressed as a trait bound rather than as a comment. Every
//! anchor operation that signs requires [`InProcessSigner`], and a remote signer
//! cannot satisfy it. **The compiler refuses the swap; the docs only explain
//! why.** When `lys-core` grows signer-generic entry points, the migration is
//! mechanical: relax the bounds from [`InProcessSigner`] to [`Signer`] and
//! delete this file's second trait. Nothing else moves.

use lys_core::Ed25519Identity;

use crate::error::AnchorResult;

/// Custody of the anchor's signing key.
///
/// The narrow surface is deliberate: a public key to publish, and the ability
/// to produce a detached Ed25519 signature over bytes somebody else chose.
/// Nothing here exposes the private key, and nothing here should ever be
/// widened to.
///
/// # Contract
///
/// - [`public_key`](Self::public_key) is stable for the lifetime of the value.
///   A signer whose key changes underneath a running anchor would publish two
///   checkpoints that no single verifier key can check, which is
///   indistinguishable from equivocation to everyone downstream.
/// - A signature returned by [`sign`](Self::sign) verifies against
///   [`public_key`](Self::public_key) under **strict** Ed25519 verification
///   (`Ed25519Identity::verify`) for the message it was given.
///
/// # Reserved surface
///
/// Nothing in this crate calls [`sign`](Self::sign) yet: signing goes through
/// `lys-core`'s concrete entry points, which need an
/// [`InProcessSigner`]. This method is the part of the boundary that survives
/// that gap being closed — see the module docs.
pub trait Signer {
    /// The 32-byte Ed25519 public key this signer signs under.
    fn public_key(&self) -> [u8; 32];

    /// Signs `message`, returning the 64-byte detached Ed25519 signature.
    ///
    /// # Errors
    ///
    /// Returns an [`AnchorError`](crate::AnchorError) if the signer could not
    /// produce a signature. Ed25519 signing is itself infallible, so a local
    /// implementation never fails here — the fallibility is for the remote
    /// custody this trait is shaped for, where the network, the device or the
    /// operator's authorization can all decline.
    fn sign(&self, message: &[u8]) -> AnchorResult<[u8; 64]>;
}

/// A [`Signer`] whose private key is held in this process, as an
/// `Ed25519Identity`.
///
/// **This is the gap, made into a bound.** Every anchor operation that signs
/// requires it, because `lys-core`'s signing entry points take a concrete
/// `&Ed25519Identity`; a remote signer can implement [`Signer`] and will never
/// be able to implement this. The module docs say what would have to change and
/// what this crate must not do instead.
///
/// # Contract
///
/// [`identity`](Self::identity) returns the identity whose public key is
/// exactly [`Signer::public_key`]. An implementation that returned a different
/// identity would publish under one key and advertise another, and every
/// verifier configured from the advertised key would reject every artifact
/// while the anchor looked healthy from the inside.
pub trait InProcessSigner: Signer {
    /// The in-process identity `lys-core`'s signing entry points require.
    fn identity(&self) -> &Ed25519Identity;
}
