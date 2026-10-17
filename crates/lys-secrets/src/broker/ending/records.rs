//! An ending retains its caller, operation and causal handle without credential material.
use super::{EndAct, EndWay};
/// How, when and by whom a lease first ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseEnd {
    /// How it ended.
    pub way: EndWay,
    /// When, in milliseconds since the epoch: the instant of the act, or
    /// the end of the window; none for an operator's drop.
    pub at_ms: Option<i64>,
    /// Who ended it, for a revoke or a relinquish.
    pub by: Option<String>,
}

/// Who ended a handle, how and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    /// The one who ended it: the person acted for, or the holder.
    pub by: String,
    /// The operation id the ending was carried under.
    pub operation: String,
    /// The handle whose ending ended this one: itself, or the handle above
    /// it that was ended.
    pub root: String,
    /// The act it was ended by.
    pub act: EndAct,
    /// When, in milliseconds since the epoch, as the broker's clock read.
    pub at_ms: i64,
}

/// What asking to end a handle came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleEnded {
    /// The handle, and every handle lent on from it that still stood, were
    /// ended now: their ids, the one asked for first.
    Ended {
        /// The handles ended.
        ended: Vec<String>,
    },
    /// The operation id was applied before to this same handle; nothing is
    /// ended again. The handles that ending ended, the one asked for first.
    Repeated {
        /// The handles ended under the operation id.
        ended: Vec<String>,
    },
    /// The handle was already dropped or ended, or one above it was;
    /// nothing is appended.
    AlreadyEnded,
}
