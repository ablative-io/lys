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
//! # ⚠️ This boundary is usable for the ROOT key and reserved for everything else
//!
//! **An HSM, a KMS or any other remote signer can implement [`Signer`] today,
//! issue an anchor's genesis delegation, and still not be able to publish a
//! checkpoint or a receipt.** That is the current state stated precisely,
//! because the previous version of this paragraph said the boundary was
//! unusable full stop, and increment 11 made half of that false — a doc that
//! overstates a gap misdirects an integrator just as surely as one that
//! understates it.
//!
//! The half that works is the offline key, which is the half DP16 cares about
//! most. `lys-core`'s delegation format exposes a two-phase
//! preimage-then-assemble path *because* the root key is meant to live somewhere
//! this process cannot reach, so `Anchor::create_with_delegated_genesis` needs
//! only [`Signer`] — no `Ed25519Identity`, no private key in this process.
//!
//! The half that does not work is everything an anchor signs afterwards, and the
//! reason is one level down. `lys-core`'s signing entry points take a
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
//! anchor operation that signs *through `lys-core`'s concrete entry points*
//! requires [`InProcessSigner`], and a remote signer cannot satisfy it. **The
//! compiler refuses the swap; the docs only explain why.** When `lys-core` grows
//! signer-generic entry points, the migration is mechanical: relax the bounds
//! from [`InProcessSigner`] to [`Signer`] and delete this file's second trait.
//! Nothing else moves.
//!
//! Genesis is the one operation that already needed no such migration, and the
//! reason is instructive rather than lucky: the delegation format was specified
//! with a two-phase signing API **because** its signer is the offline root key.
//! An entry point designed for absent key material is one a custody boundary
//! passes through for free. That is the shape the two routes above would give
//! every other signing path.

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
/// # This surface is live on exactly one path
///
/// [`sign`](Self::sign) was reserved — nothing in this crate called it — until
/// genesis became a delegation. `Anchor::create_with_delegated_genesis` (named
/// in plain text: a link from these ungated docs to a gated item resolves under
/// `--all-features` and breaks the default `cargo doc`, which is a gate) hands
/// it the RFC 9052 `Sig_structure` for a `lys/delegation/v1` claim and
/// takes 64 bytes back, so **an offline or remote root signer can issue an
/// anchor's genesis delegation today** with no `Ed25519Identity` in the path.
///
/// Everything else an anchor signs — checkpoints, receipts, inclusion artifacts
/// — still goes through `lys-core`'s concrete entry points and still requires
/// [`InProcessSigner`]. That asymmetry is the custody story rather than an
/// inconsistency: the key that must be online is the one this crate has to hold,
/// and the key DP16 wants kept offline is the one that no longer needs to be.
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
