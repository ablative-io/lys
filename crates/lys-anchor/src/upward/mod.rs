//! The upward pin — this anchor's own checkpoint, recorded by a parent anchor
//! — and the verification bundle assembled from the cascade that results.
//!
//! DP14: *"anchors can pin to other anchors kind of things so we can have a
//! cascading sort of effect"*, and — the consequence accepted with it —
//! **cascading IS the witness mechanism**. An anchor pinning to another
//! countersigns it. This module is the pinning anchor's side of the act that
//! [`witness`](crate::witness) is the recording anchor's side of, and they are
//! deliberately one mechanism rather than two: a receipt over your checkpoint
//! bytes *is* a second party's signature over a statement containing your
//! checkpoint, so there is nothing a cosignature line would add here except a
//! second format to freeze.
//!
//! # No format is frozen here, and that is the load-bearing property
//!
//! **The leaf a parent records is the child's checkpoint note, verbatim.** Not
//! wrapped, not re-encoded, not annotated with "this is a cascade" — the exact
//! bytes [`Anchor::publish_checkpoint`] produced, which are an ordinary C2SP
//! signed note that `lys-core`'s `verify_checkpoint` and Go's `sumdb/note`
//! both already read. So this module introduces **no wire format at all**:
//! there is no new domain-separation tag, no new leaf encoding, and nothing
//! here that a later change could break a historical verification by editing.
//! The first format this crate freezes is `lys/delegation/v1`, and it is
//! deliberately the last thing built.
//!
//! That is not frugality. A leaf encoding is frozen the moment something
//! durable is logged under it, so a cascade leaf invented for convenience would
//! be permanent by the second submission — and the shape that avoids it is the
//! shape the witness path already uses, which is why reusing it costs nothing.
//!
//! # Federation calls the core, and the compiler is the second party
//!
//! [`pin()`] calls [`Anchor::submit`] on the parent. It passes a
//! [`Submission`](crate::Submission) with the one field a submission has, and a
//! [`SubmitterContext`](crate::SubmitterContext) the caller established — the
//! same two arguments any other submitter passes, with nothing added. **The
//! parent's `submit` is unchanged and does not know it is being used for a
//! cascade**; there is no cascade flag, no parent-side branch, and nowhere for
//! one to go.
//!
//! The direction is one-way and enforced rather than described: everything here
//! is behind the off-by-default `federation` feature, and no item outside it
//! names anything inside it. A core path that reached into this module would
//! fail the default build, which is a gate that already runs.
//!
//! # What an upward pin does *not* do to the pinning anchor
//!
//! **It appends nothing to the child's own log.** Publishing a checkpoint is
//! not an append (`Anchor::publish_checkpoint` takes `&self`), and the leaf the
//! parent writes lands in the *parent's* log. A pinned anchor's own storage is
//! byte-for-byte what it was before, which has one consequence worth naming
//! where somebody will look for it: **this anchor's log holds no record that it
//! was ever pinned upward**, so nothing here makes
//! [`WitnessPosture`](crate::WitnessPosture) computable from it.
//! [`anchor::status`](crate::anchor::status) carries that account.
//!
//! # What a pin establishes, and what it does not
//!
//! A returned [`UpwardPin`] holds two things a third party can check: the
//! child's signed checkpoint, and the parent's receipt proving those exact
//! bytes were a leaf in the parent's tree under the root the parent signed.
//! Together they say *this parent held this child's checkpoint*. They do not
//! say the parent checked anything about it — the parent's receipt is identical
//! whether or not the parent computed a relation, by construction — and they do
//! not say the child's log is honest, or that the parent's is. A witness is a
//! durable memory, not an auditor, and the pinning side inherits every limit
//! [`witness`](crate::witness) states.
//!
//! [`Anchor::publish_checkpoint`]: crate::Anchor::publish_checkpoint
//! [`Anchor::submit`]: crate::Anchor::submit

pub mod bundle;
pub mod pin;

pub use bundle::bundle_for;
pub use pin::{UpwardPin, pin};
