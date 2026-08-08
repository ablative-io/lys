//! [`Relation`] and [`Observation`] — the witness's report, which nobody signs.
//!
//! # Invariants
//!
//! - **Nothing in this file is signed, and there is nowhere for a signature to
//!   attach.** [`Observation::recorded`] holds the anchor's receipt, produced by
//!   the ordinary submit path over the submitted bytes; [`Observation::previous`]
//!   and [`Observation::relation`] are this witness's own report and are outside
//!   every signature. A consumer who is handed an `Observation` by a third party
//!   is being told what that party says it remembers, and has no cryptographic
//!   reason to believe the last two fields.
//! - **A relation exists only where a comparison happened.** `previous` and
//!   `relation` are `Some` together and `None` together; a witness with nothing
//!   to compare reports nothing rather than a value that reads as assent.
//! - **A [`Relation`] is a statement about two roots, never about the entries
//!   between them.** Every variant below is scoped to the two `(size, root)`
//!   pairs the witness holds.

use super::projection::OriginState;
use crate::wire::SubmissionOutcome;

// Scaffolding for both witness test files. Declared here rather than in
// `mod.rs`, which carries only module declarations, re-exports and docs.
#[cfg(test)]
#[path = "fixture.rs"]
pub(super) mod fixture;

/// How a newly recorded checkpoint sits against the one this witness last
/// recorded for the same origin.
///
/// Each variant is a statement about exactly two `(size, root)` pairs — the one
/// in the witness's own log and the one just recorded. None of them says
/// anything about the entries in between, about the honesty of the log, or
/// about the well-formedness of what it contains.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Relation {
    /// The new root was proven, by an RFC 6962 consistency proof, to be the
    /// root of a tree the previously recorded one is a prefix of.
    ///
    /// This is the only variant carrying cryptographic weight, and it weighs
    /// exactly this: the two roots are consistent. It is not a statement that
    /// the entries added in between are well-formed, admissible, or true.
    Extends,

    /// The new checkpoint states the same size and the same root as the
    /// recorded one.
    ///
    /// A no-op, and not an error. The same reasoning the storage layer applies
    /// to re-pinning an identical root applies here: an idempotent repeat is
    /// precisely the door the equivocation check has to stand in, so it must be
    /// distinguishable from — and permitted where — a differing root is not.
    Identical,

    /// The new checkpoint states a smaller tree size than the recorded one.
    ///
    /// An append-only log's size never decreases, so the log this checkpoint
    /// came from either lost history or is presenting a shorter one on purpose.
    /// The witness reports it and refuses nothing; the evidence is both notes,
    /// durable in its own log.
    Rollback,

    /// The new checkpoint states the **same** size as the recorded one with a
    /// **different** root.
    ///
    /// ⭐ **This is the observation the whole mechanism exists to produce.** An
    /// append-only tree has exactly one root per size, so two roots at one size
    /// are two histories. It is the registry-level twin of the storage layer's
    /// refusal to re-pin a changed root at an unchanged size, for the reason
    /// written there: accepting it would record that the log's history is two
    /// different things.
    ///
    /// It is an observation, not a verdict. It says two notes this witness
    /// holds disagree — not which one is the honest one, and not that the
    /// submitter forged anything. Neither note's signature was checked here.
    Conflicting,

    /// The witness could **not establish** that the new root extends the one it
    /// holds.
    ///
    /// ⚠️ Read the name narrowly. It does not assert that the two trees are
    /// unrelated; it reports that this witness has no proof they are related.
    /// It is the answer when a consistency proof failed to verify, when the
    /// proof bytes were malformed, and when no proof was offered at all —
    /// because in all three cases the witness holds the same thing, which is
    /// nothing.
    Unrelated,
}

/// What a witness reports after recording a checkpoint note.
///
/// # What is signed, and what is only reported
///
/// - **`recorded`** carries the anchor's receipt: a signature over a Merkle
///   root the anchor derived, whose proven leaf is the submitted note verbatim.
///   That receipt is byte-identical to the one an ordinary submission of the
///   same bytes at the same tree state would have produced, and deliberately
///   so — see the [module docs](super) for why a witness must have no artifact
///   a plain recorder could not emit.
/// - **`previous`** and **`relation`** are signed by nobody. They are the
///   witness's report of its own memory, and they are outside every signature
///   in the struct.
///
/// # The coupled invariant
///
/// `previous.is_some() == relation.is_some()`. Both are `None` when the witness
/// had nothing to compare, which happens in exactly two ways: the recorded
/// bytes are not a checkpoint note this crate can read at all, or they are one
/// for an origin this witness has never recorded before. In both cases the
/// honest report is that no comparison was made — a witness with no memory of
/// an origin must say so rather than emit a value a reader could take for
/// assent, which is why `relation` is an [`Option`] and why no
/// "nothing-to-compare" [`Relation`] variant exists to be misread.
///
/// An operator who needs to tell those two cases apart reads the recorded
/// leaf: it is durable, it is the submitted bytes verbatim, and parsing it is
/// their business rather than this crate's.
#[derive(Clone, Debug)]
pub struct Observation {
    /// The note, recorded as a leaf, with the anchor's receipt for it.
    ///
    /// Its presence is the proof that the record preceded the check: this value
    /// only exists because a submission completed.
    pub recorded: SubmissionOutcome,

    /// The `(origin, size, root)` this witness itself last recorded for the
    /// origin named in the note — never a state fetched from anywhere else, and
    /// never one the submitter supplied.
    pub previous: Option<OriginState>,

    /// How the recorded checkpoint sits against `previous`.
    ///
    /// `Some` exactly when `previous` is `Some`.
    pub relation: Option<Relation>,
}
