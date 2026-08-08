//! The two policies with no opinion about *who* submitted: [`AcceptAll`] and
//! [`MaxSize`].
//!
//! # Neither of these is a default, and the naming is the enforcement
//!
//! [`AcceptAll`] is the policy of an anchor that admits anything — which is
//! what a domain-agnostic anchor should do, and is the honest reading of "used
//! by anything to sign anything". It is still a *choice*, and it has to be
//! written at the construction site, in those words. That is the entire
//! difference between an operator who decided to take all comers and one who
//! never thought about it, and it is worth a type to make the two
//! distinguishable in a code review.
//!
//! Neither implements [`Default`], and the absence is checked by the
//! `compile_fail` doctests on each type rather than merely intended.
//!
//! # [`MaxSize`] bounds the statement, not the submission
//!
//! The limit is compared against the statement bytes — the thing that becomes a
//! leaf and is stored forever. A credential in the submitter's context is
//! admission-time-only and is not measured here, so [`MaxSize`] is not a
//! defence against a caller handing over an enormous credential. There is no transport in this crate, so
//! the bytes were already in the caller's memory before `submit` was reached;
//! bounding what arrives is the job of whatever accepted it, and pretending
//! otherwise here would be a limit that reads as protection and is not.

use super::context::SubmitterContext;
use super::policy::{AdmissionPolicy, NotAdmitted};
use crate::wire::Submission;

/// Admits every submission.
///
/// The right policy for an anchor that takes all comers, and it must be named
/// to get one. See the [module docs](self) for why that is not a formality.
///
/// # The absence of `Default` is checked, not merely intended
///
/// The two snippets are the check, in the shape
/// [`AnchorConfig`](crate::AnchorConfig) established: the first proves the
/// probe itself compiles — a `compile_fail` test passes for *any* compilation
/// error, including a malformed snippet — and the second differs from it in
/// exactly one respect, the type the bound is applied to.
///
/// ```
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// let _ = lys_anchor::AcceptAll;
/// ```
///
/// ```compile_fail
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// requires_default::<lys_anchor::AcceptAll>();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcceptAll;

impl AdmissionPolicy for AcceptAll {
    /// Admits `submission` unconditionally, whatever it carries and whatever
    /// was or was not established about who sent it.
    ///
    /// # Errors
    ///
    /// Never. The signature is the trait's, and a policy that cannot refuse
    /// still has to have somewhere to say so.
    fn admit(
        &self,
        _submission: &Submission<'_>,
        _context: &SubmitterContext<'_>,
    ) -> Result<(), NotAdmitted> {
        Ok(())
    }
}

/// Admits a submission whose statement is at most `max_bytes` long.
///
/// The bound is inclusive: a statement of exactly `max_bytes` is admitted, and
/// one byte more is not. `MaxSize::new(0)` therefore admits the empty statement
/// and nothing else, which is a usable — if unusual — configuration rather than
/// a degenerate one, and is tested as such.
///
/// # This policy is a function of the submitted bytes, which is the whole point
///
/// It is also exactly why its refusal cannot say what the limit is, or how far
/// over it a submission was: a refusal that named either would let a submitter
/// binary-search the threshold in a handful of tries. The refusal is
/// [`NotAdmitted`], carrying nothing, and is indistinguishable from every other
/// policy's.
///
/// # The absence of `Default` is checked, not merely intended
///
/// A default limit would be the worst kind: a number nobody chose, silently
/// governing what an anchor will accept forever. The first snippet is the
/// positive control for the second.
///
/// ```
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// let _ = lys_anchor::MaxSize::new(4096);
/// ```
///
/// ```compile_fail
/// fn requires_default<T: Default>() {}
/// requires_default::<Vec<u8>>();
/// requires_default::<lys_anchor::MaxSize>();
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaxSize {
    max_bytes: usize,
}

impl MaxSize {
    /// A policy admitting statements of at most `max_bytes` bytes.
    ///
    /// There is no default and no "sensible" value chosen here — the number is
    /// the operator's, and it is the only thing this type holds.
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    /// The limit this policy was constructed with.
    ///
    /// Readable by the operator who set it, and never reported to a submitter:
    /// nothing on the refusal path can reach this value.
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }
}

impl AdmissionPolicy for MaxSize {
    /// Admits `submission` when its statement is at most `max_bytes` long,
    /// regardless of who sent it.
    ///
    /// # Errors
    ///
    /// [`NotAdmitted`] when the statement is longer, saying nothing about the
    /// limit or the overage.
    fn admit(
        &self,
        submission: &Submission<'_>,
        _context: &SubmitterContext<'_>,
    ) -> Result<(), NotAdmitted> {
        if submission.statement.len() > self.max_bytes {
            return Err(NotAdmitted);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "trivial_tests.rs"]
mod tests;
