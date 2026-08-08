//! Admission — whether an anchor takes a submission at all, and the single
//! shape its refusal is permitted to have.
//!
//! # Why this is a policy object and not a rule
//!
//! DP9 rules the write path certificate-gated. DP13 rules the anchor
//! domain-agnostic — *"used by anything to sign anything"*. Those are
//! genuinely contradictory as shipped behaviour: a certificate gate is an
//! opinion about who submitters are, and an opinion about submitters is a
//! domain. DP23 resolves it by making admission an object: the anchor core
//! stays ignorant, the cert-gated rule becomes one policy among several
//! ([`RecognisedCertificate`]), and **no default is shipped**, so no deployment
//! can inherit an admission rule nobody chose.
//!
//! The absence of a default is the load-bearing part. There is no `Default` on
//! any policy here, no policy-defaulting constructor on
//! [`Anchor`](crate::Anchor), and none should be added: a convenience that
//! picks an admission rule for the caller defeats the ruling in one line, which
//! is how a fail-closed control is usually disarmed — by someone being helpful.
//!
//! # One refusal, and it is enforced by the type rather than by care
//!
//! [`crate::error`]'s module docs argued, before this module existed, that this
//! crate's detailed error variants are not a parsing oracle *because no variant
//! is a function of the submitted bytes*, and recorded that the argument would
//! expire the moment an admission policy arrived. It has arrived. An admission
//! policy is exactly a function of the submitted bytes.
//!
//! So [`NotAdmitted`] carries nothing at all, and every occurrence of it
//! becomes the one fieldless
//! [`AnchorError::NotAdmitted`](crate::AnchorError::NotAdmitted). A submitter
//! cannot learn which rule refused them, and cannot learn which *policy* the
//! anchor runs. [`policy`] says where an operator-facing reason belongs
//! instead, and why it is not routed through the anchor.

pub mod certificate;
pub mod context;
pub mod policy;
pub mod trivial;

pub use certificate::RecognisedCertificate;
pub use context::{AuthenticatedPeer, SubmitterContext};
pub use policy::{AdmissionPolicy, NotAdmitted};
pub use trivial::{AcceptAll, MaxSize};
