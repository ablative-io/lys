//! The freshness rule (C25) behind `SpiceDB`, as DIRECTORY-006 R4 builds it:
//! a check is answered only from a projection that has applied every
//! committed grant event it depends on, and is otherwise refused
//! `StaleDecision`.
//!
//! A check depends on an unapplied event when the event issues or revokes a
//! grant on the checked identity's path to the resource: one of the
//! identity's grants carrying the action on the resource, or a grant one of
//! those derives from. Such a check still makes its one `CheckPermission`
//! call, so every check asks `SpiceDB`, and its verdict is discarded, even a
//! permit. A check that depends on no unapplied event is answered as the
//! evaluator answers it, however far behind the projector is, so a lagging
//! projection never stops unrelated authority. The refusal names the
//! revision the decision needs (the unapplied event's log position), the
//! revision the relationships stand at (the projector's position) and the
//! affected grant.

use std::collections::BTreeSet;

use lys_identity::grants::{
    Answer, Evaluator as Decides, GrantBook, GrantError, GrantId, Question,
};

use super::check::Evaluator;
use super::projector::Reached;

/// The grants on `question`'s identity's path to its resource: the
/// identity's grants carrying the action on the resource, and every grant
/// they derive from.
fn path(book: &GrantBook, question: &Question<'_>) -> BTreeSet<GrantId> {
    let mut path = BTreeSet::new();
    for record in book.held_by(question.subject) {
        let grant = record.grant();
        if grant.resource() != question.resource || !grant.actions().contains(question.action) {
            continue;
        }
        path.insert(grant.id());
        if let Ok(lineage) = book.lineage(grant.id()) {
            path.extend(lineage.path);
        }
    }
    path
}

/// The `StaleDecision` refusal of `question` when it depends on a committed
/// event after log position `reached` and up to `committed`, naming the
/// latest such event; none when it depends on none.
pub fn stale(
    book: &GrantBook,
    reached: u64,
    committed: u64,
    question: &Question<'_>,
) -> Option<GrantError> {
    if reached >= committed {
        return None;
    }
    let unapplied = |index: u64| {
        let position = index + 1;
        (position > reached && position <= committed).then_some(position)
    };
    path(book, question)
        .into_iter()
        .filter_map(|grant| {
            let record = book.record(grant)?;
            let issued = unapplied(record.index());
            let revoked = record
                .revoked()
                .and_then(|revocation| unapplied(revocation.index));
            issued.max(revoked).map(|position| (position, grant))
        })
        .max()
        .map(|(required, grant)| GrantError::StaleDecision {
            required,
            projected: reached,
            grant: Some(grant.to_string()),
        })
}

/// The evaluator behind DIRECTORY-006 R4's decision: the one `SpiceDB` check,
/// under the freshness rule, against the projection as it stands.
#[derive(Debug)]
pub struct Fresh<'a> {
    evaluator: &'a Evaluator,
    reached: &'a Reached,
}

impl<'a> Fresh<'a> {
    /// The evaluator asking through `evaluator` with the projector at `reached`.
    pub fn new(evaluator: &'a Evaluator, reached: &'a Reached) -> Self {
        Self { evaluator, reached }
    }
}

impl Decides for Fresh<'_> {
    fn evaluate(
        &self,
        book: &GrantBook,
        committed: u64,
        question: &Question<'_>,
    ) -> Result<Answer, GrantError> {
        let checked = self
            .evaluator
            .check(question, self.reached.token.as_deref())?;
        if let Some(refusal) = stale(book, self.reached.position, committed, question) {
            return Err(refusal);
        }
        Ok(Answer {
            permitted: checked.permitted,
            revision: self.reached.position,
        })
    }
}
