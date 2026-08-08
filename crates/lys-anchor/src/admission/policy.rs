//! [`AdmissionPolicy`] — the decision an anchor makes *before* it appends — and
//! [`NotAdmitted`], the only thing a policy is able to say when the answer is
//! no.
//!
//! # One refusal, and the type is what enforces it
//!
//! Every other refusal this crate produces is detailed on purpose, and
//! [`error`](crate::error) argues at length why that is safe: no variant
//! reachable before this module existed was a *function of the submitted
//! bytes*. An admission policy is exactly that function. A policy that could
//! say "too large" in one case and "not certified" in another would hand a
//! submitter a read-out of the rule — probe until the message changes, and with
//! a size rule that is a free binary search on the threshold.
//!
//! So the answer is not "be careful what you put in the message". It is that
//! **there is no message**. [`NotAdmitted`] is a zero-sized value with no
//! fields, no `Display`, and no room for a cause; the anchor converts every
//! occurrence of it into the single fieldless
//! [`AnchorError::NotAdmitted`](crate::AnchorError::NotAdmitted). Two
//! submissions refused by two different rules — by two different *policies* —
//! come back byte-identical, and that is checked rather than intended.
//!
//! # Where the operator's detail goes, since it does not go to the submitter
//!
//! An honest implementation wants to know *why* something was refused. That
//! want is legitimate and it is not served here, for a reason that is
//! structural rather than stylistic: **the anchor is never told the reason, so
//! the anchor cannot disclose it.** There is no reason-carrying value crossing
//! from a policy into [`Anchor`](crate::Anchor) that a later "just add the
//! cause to the error" change could pick up, because there is no such value at
//! all.
//!
//! The detail therefore belongs to the policy object, which is the operator's:
//! they construct it, they own it, and an implementation that wants to record
//! its own refusals can hold whatever state or side channel its deployment
//! already uses. **This crate ships no logging framework and invents none** —
//! adding one here would be a second place for the reason to live, one level
//! away from the error type that must not carry it.
//!
//! ⚠️ **What the collapse does not buy.** It removes the *message* oracle, not
//! a *timing* one. A policy whose certificate path costs a millisecond and
//! whose length check costs a nanosecond has rebuilt the same disclosure in the
//! time domain, and no signature in this module can prevent that for an
//! implementation this crate does not own. Equally, an implementation that
//! panics with a reason has published it; `submit` catches no panics. Both are
//! stated because a defence that is silent about its edges reads as one that
//! has none.

use super::context::SubmitterContext;
use crate::wire::Submission;

/// A submission was not admitted. That is the whole of what this value says.
///
/// Deliberately empty: no cause, no reason code, no message, and no `Display`.
/// A value carrying none of those cannot be widened into an oracle by a later
/// change that only meant to be helpful — the widening has to be a visible edit
/// to a public type, and every existing `Err(NotAdmitted)` in every
/// implementation stops compiling when it happens.
///
/// The sentence a submitter eventually reads belongs to
/// [`AnchorError::NotAdmitted`](crate::AnchorError::NotAdmitted), is fixed, and
/// is the same regardless of which policy refused or which of its rules did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotAdmitted;

/// Whether an anchor takes a submission at all.
///
/// **No default is shipped and none should be added.** An anchor's admission
/// rule is an operator decision; a library that picked one would have every
/// deployment inherit a rule nobody chose. That is why
/// [`Anchor::create`](crate::Anchor::create) and
/// [`Anchor::open`](crate::Anchor::open) take a policy by value with no
/// overload that omits it, why no policy in this crate implements
/// [`Default`], and why the two that would be most tempting to default —
/// [`AcceptAll`](super::AcceptAll) and
/// [`MaxSize`](super::MaxSize) — have to be written out at the call site.
///
/// # Contract
///
/// - **The decision is made before anything is appended.** A refused submission
///   leaves no leaf, no index and no trace in the log. An anchor's log is what
///   it admitted.
/// - **A policy sees the submission, what was established about the submitter,
///   and nothing else.** It is handed no `&Anchor`, no tree, no size and no
///   origin, so admission cannot depend on the log's history and a policy
///   cannot append, publish or refuse anything other than the submission in
///   front of it.
/// - **The submitter's context arrives with its provenance attached.**
///   [`SubmitterContext`] distinguishes bytes a submitter asserted from a peer
///   a transport authenticated, and there is no accessor that collapses the
///   two. A policy that treats them alike has written both arms of a match
///   saying so — see [`context`](super::context) for why that friction is the
///   whole point.
/// - **A policy may not report why.** [`NotAdmitted`] carries nothing; see the
///   [module docs](self) for where an operator-facing reason belongs instead.
/// - **Genesis is not policed.** [`Anchor::create`](crate::Anchor::create)
///   writes leaf 0 from bytes the operator supplied at construction, not from a
///   stranger's submission, and it does not consult the policy. A policy that
///   refuses everything therefore still yields a constructible anchor — which
///   is the fail-closed direction, since the alternative is an anchor whose own
///   creation depends on its access-control rule agreeing with its operator.
/// - **`admit` is a decision, not an action.** It takes `&self`; an
///   implementation that needs to record something does so through its own
///   interior state, and this crate neither provides nor expects a channel for
///   it.
pub trait AdmissionPolicy {
    /// Decides whether `submission`, made under `context`, may be appended.
    ///
    /// # Errors
    ///
    /// Returns [`NotAdmitted`] — the only error value available — when the
    /// submission is refused, for every reason it might be refused for.
    fn admit(
        &self,
        submission: &Submission<'_>,
        context: &SubmitterContext<'_>,
    ) -> Result<(), NotAdmitted>;
}
