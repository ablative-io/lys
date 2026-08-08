//! [`AnchorStatus`] and [`WitnessPosture`] — what an anchor says about itself
//! when nobody asked it to sign anything.
//!
//! # The disclosure is a value, not a message
//!
//! `BUILD-PLAN.md` §2.2 requires four places to state that a standalone anchor
//! can equivocate undetectably, and this is the one the *library* owns. The
//! shape it asks for is precise, and it is the shape here:
//!
//! > `AnchorStatus` carries a non-optional `posture: WitnessPosture` field;
//! > `WitnessPosture::Unwitnessed`'s `Display` carries the sentence. A caller
//! > cannot obtain a status without obtaining the posture, and cannot format
//! > the posture without the sentence appearing.
//!
//! Both halves are structural. [`AnchorStatus::posture`] is a plain field with
//! no `Option` around it and no constructor that omits it, and
//! [`WitnessPosture`]'s only `Display` output is [`STANDALONE_DISCLOSURE`] — so
//! a renderer that formats the posture at all has rendered the disclosure, and
//! one that drops it has to drop the posture, which is visible in the code that
//! drops it. **A disclosure that can be silently dropped is not a disclosure**,
//! and this is the version that cannot be.
//!
//! The division of labour is the recovery notice's, for the recovery notice's
//! reason: **the library returns, the renderer prints, and neither may drop
//! it.** A library that wrote this to stderr would have decided for its caller
//! how it gets reported.
//!
//! `crates/lys-anchor-cli` currently prints its own copy of the sentence as an
//! unconditional constant, and says in its own docs that it does so only
//! because this value did not exist. **This is the value it should render**;
//! the constant there is now a duplicate awaiting removal, and it is named here
//! so the next pass finds the instruction rather than the duplicate.
//!
//! # The posture is computed, never stored — and what that does and does not
//! mean today
//!
//! §2.2's rule is that the posture is *"computed from the anchor's own log —
//! whether it has ever recorded a receipt from a parent — and not from
//! configuration, so it cannot be set to a comfortable value by an operator who
//! has not actually arranged a witness."*
//!
//! The half of that which is held right now, structurally: **there is nowhere
//! for a posture to be stored.** [`Anchor`] has no posture field,
//! [`AnchorConfig`](crate::AnchorConfig) has no field at all and will never
//! implement `Default`, no constructor takes a posture, and there is no setter.
//! [`Anchor::status`] reads the log and nothing else. An operator cannot
//! configure this value because there is no configuration surface it could be
//! configured through.
//!
//! The half that is **not** yet a computation over leaf contents, stated
//! plainly rather than implied: [`WitnessPosture`] has one variant, and
//! `status` returns it unconditionally. That is not a stub standing in for a
//! scan that was skipped — it is that the evidence a witnessed posture would
//! require does not exist to be scanned for. The leaf that would carry it is
//! written by the upward-pin path (BUILD-PLAN increment 8), which is not built,
//! and there is no ratified format for a parent's receipt over this anchor's
//! checkpoint for a scan to recognise. Writing a recogniser for a format that
//! does not exist would be inventing the format, in the crate whose entire
//! discipline is that formats are frozen the moment something durable is signed
//! under them.
//!
//! So the honest statement is: the value is **unreachable-by-configuration
//! today and unreachable-by-content today**, and increment 8 makes the second
//! half a real scan. This module is where that scan goes, and it is the only
//! place that has to change.
//!
//! **The consequence for tests, said out loud:** an assertion that a fresh
//! anchor's posture is `Unwitnessed` cannot fail while the enum has one
//! variant. It is written anyway, because §7.2 asks for it and because it
//! becomes load-bearing the moment a second variant exists — but it is not
//! evidence of anything now, and the assertion that *is* load-bearing today is
//! the one on the `Display` string, which a person editing this file can break.
//!
//! # Why `tree_size` is a method and not a field
//!
//! [`AnchorStatus::root`] is a [`RootHash`], which carries the leaf count it
//! was computed at, and [`AnchorStatus::tree_size`] reads that count back out.
//! A second `tree_size` field would be a second name for one fact and a second
//! thing to keep in step; there is no assignment here that could put the size
//! and the root out of agreement, because there is only one of them.

use std::fmt;

use lys_core::merkle::RootHash;
use lys_log_store::LeafStore;

use super::open::Anchor;

/// The sentence a standalone anchor's operator must be told, in the words
/// [the crate's own module docs](crate) use.
///
/// Not a warning that can be silenced, not conditional on a flag, and not
/// something a caller assembles: it is the entire `Display` output of
/// [`WitnessPosture::Unwitnessed`], so it cannot be rendered in part.
pub const STANDALONE_DISCLOSURE: &str = "this anchor has no witnesses: an anchor with no \
     witnesses can equivocate undetectably — it can hold two histories and show each observer \
     whichever suits, and no local check catches that. What changes it is one external party \
     keeping its own durable memory of this anchor's checkpoints.";

/// Whether anything outside this anchor keeps a durable memory of its
/// checkpoints.
///
/// Computed from the log by [`Anchor::status`], never stored and never
/// configured — see the [module docs](self) for what that guarantees today and
/// what it does not yet.
///
/// `#[non_exhaustive]` because the second variant is scheduled rather than
/// hypothetical: increment 8's upward pin is what makes a witnessed anchor
/// expressible, and a renderer written now should be forced to say what it will
/// do with a posture it has not seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WitnessPosture {
    /// No external party is known to keep a memory of this anchor's
    /// checkpoints.
    ///
    /// Its [`Display`](std::fmt::Display) is [`STANDALONE_DISCLOSURE`], whole.
    Unwitnessed,
}

impl fmt::Display for WitnessPosture {
    /// Writes the disclosure for this posture.
    ///
    /// There is deliberately no shorter rendering and no `Debug`-style one-word
    /// form on this path. A caller who wants the variant's name has
    /// [`Debug`](std::fmt::Debug); a caller who *displays* a posture to an
    /// operator gets the sentence, because that is what the display is for.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unwitnessed => f.write_str(STANDALONE_DISCLOSURE),
        }
    }
}

/// What an anchor reports about itself: identity, extent, root, posture, and
/// whether opening it repaired anything.
///
/// **Nothing here is signed**, and the type offers no way to make it look as
/// though it were. It is the answer to "what does this anchor hold", read off
/// the operator's own log; the answer to "what will this anchor vouch for" is a
/// checkpoint, and producing one requires a signer this value never touches.
///
/// Owned rather than borrowed, so a renderer can hold it after the anchor is
/// gone. It is a snapshot: an anchor that is appended to afterwards has moved
/// on, and this value does not follow it.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct AnchorStatus {
    /// The log's origin, as its store reports it.
    pub origin: String,
    /// The RFC 6962 Merkle Tree Hash of the log at the moment this was read,
    /// together with the leaf count it was computed at.
    pub root: RootHash,
    /// Whether anything outside this anchor remembers its checkpoints.
    ///
    /// Not an `Option`, and there is no constructor that omits it.
    pub posture: WitnessPosture,
    /// The tree size an interrupted append was recovered to when this anchor
    /// was opened, or `None` if it opened clean.
    ///
    /// Present so a renderer emitting a status has the repair in hand without a
    /// second call. A repair the operator never hears about is
    /// indistinguishable from one that never happened.
    pub recovered_to: Option<u64>,
}

impl AnchorStatus {
    /// The number of leaves in the log, genesis included.
    ///
    /// Read out of [`root`](Self::root) rather than stored beside it — see the
    /// [module docs](self) for why one fact gets one field.
    pub fn tree_size(&self) -> u64 {
        self.root.num_leaves()
    }
}

impl<S: LeafStore, K, P> Anchor<S, K, P> {
    /// Reads the anchor's current status.
    ///
    /// **Signs nothing and appends nothing.** On the unbounded `impl` block, so
    /// it is reachable on a read-only anchor — which is the point: a `status`
    /// command should not have to supply a signing key to answer questions it
    /// never signs for, nor an admission policy to answer questions no policy
    /// governs.
    ///
    /// Every field is read from the log on this call. Nothing is cached, so
    /// this cannot disagree with storage; and the posture is derived here
    /// rather than looked up, so there is no stored value for it to disagree
    /// with either.
    pub fn status(&self) -> AnchorStatus {
        AnchorStatus {
            origin: self.origin().to_string(),
            root: self.root(),
            // Derived on every call from what this anchor holds. There is no
            // field, no config entry and no setter anywhere that could make
            // this read anything else — and the module docs say exactly what is
            // and is not yet established by that.
            posture: WitnessPosture::Unwitnessed,
            recovered_to: self.recovered_to(),
        }
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
