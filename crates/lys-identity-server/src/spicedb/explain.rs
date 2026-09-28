//! Why an identity can, or cannot, do a thing: behind DIRECTORY-006 R5's
//! explain seam, from one call to the one evaluator's traced check.
//!
//! The explanation's verdict is the traced check's verdict, and nothing
//! else decides it: this module builds no check and is no second evaluator.
//! A yes carries the grant it rests on, found where `SpiceDB`'s trace and the
//! identity's grants meet, and the path of identities from the identity to
//! the responsible person that grant's ancestry gives. A no carries the
//! reason the book names, `permission_revoked` with the withdrawn grant or
//! `no_grant`. Both carry the policy they were made under: the SHA-256 of
//! `schema.zed` and the revision the answer was read at. A check `SpiceDB`
//! does not answer, or one the projection is not current for, has no
//! explanation, and its refusal is answered instead.
//!
//! Every answer to one question set is read at one revision: the set's first
//! forward check reads at least as fresh as the projector's token, and every
//! later check and lookup of the set reads at the exact snapshot it answered
//! at.

use std::collections::BTreeSet;

use lys_identity::grants::permission::{refusal, resting};
use lys_identity::grants::relationships::{EXERCISE, GRANT};
use lys_identity::grants::{Action, GrantBook, GrantError, GrantId, Question, Resource};
use lys_identity::{IdentityId, PersonId};

use super::check::Evaluator;
use super::freshness::stale;
use super::projector::Reached;
use super::schema::digest;
use super::wire::authzed::api::v1::CheckDebugTrace;
use super::wire::authzed::api::v1::check_debug_trace::{Permissionship, Resolution};

/// The questions asked together, all read at one revision.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestionSet {
    revision: Option<String>,
}

impl QuestionSet {
    /// A set with no answer yet: its first answer fixes its revision.
    pub fn new() -> Self {
        Self::default()
    }

    /// A set continued at `revision`, a revision an earlier answer of the
    /// set was read at.
    pub fn at(revision: impl Into<String>) -> Self {
        Self {
            revision: Some(revision.into()),
        }
    }

    /// The revision the set is read at, once its first answer fixed it.
    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    pub(crate) fn fix(&mut self, revision: &str) {
        if self.revision.is_none() {
            self.revision = Some(revision.to_owned());
        }
    }
}

/// The policy an answer was made under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Policy {
    /// The SHA-256 of `schema.zed`, in lowercase hex.
    pub schema_sha256: String,
    /// The revision token the answer was read at.
    pub revision: String,
}

/// What the explanation answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The identity can: through `grant`, along `path` to `responsible`.
    Permitted {
        /// The grant the permit rests on, when the book holds a live one.
        grant: Option<GrantId>,
        /// The identities from the identity to the responsible person.
        path: Vec<IdentityId>,
        /// The person responsible, the root grant's holder.
        responsible: Option<PersonId>,
    },
    /// The identity cannot, for the named `reason`.
    Refused {
        /// The refusal's name.
        reason: String,
        /// The grant it concerns, when there is one.
        grant: Option<String>,
    },
}

/// The answer to why an identity can or cannot do a thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    /// The identity asked about.
    pub identity: IdentityId,
    /// The resource.
    pub resource: Resource,
    /// The action.
    pub action: Action,
    /// What was answered.
    pub verdict: Verdict,
    /// The policy it was answered under.
    pub policy: Policy,
}

/// Every grant object the trace shows permitting `exercise`.
fn traced_grants(trace: &CheckDebugTrace, found: &mut BTreeSet<String>) {
    let permitted = matches!(
        Permissionship::try_from(trace.result),
        Ok(Permissionship::HasPermission | Permissionship::ConditionalPermission)
    );
    if let Some(resource) = &trace.resource
        && resource.object_type == GRANT
        && trace.permission == EXERCISE
        && permitted
    {
        found.extend(resource.object_id.split(',').map(str::to_owned));
    }
    if let Some(Resolution::SubProblems(sub)) = &trace.resolution {
        for inner in &sub.traces {
            traced_grants(inner, found);
        }
    }
}

/// The identities along a grant's ancestry, from its holder to the root
/// grant's holder, each once in turn.
fn identities(book: &GrantBook, path: &[GrantId]) -> Vec<IdentityId> {
    let mut identities: Vec<IdentityId> = Vec::new();
    for grant in path.iter().filter_map(|id| book.grant(*id)) {
        if identities.last() != Some(&grant.holder()) {
            identities.push(grant.holder());
        }
    }
    identities
}

fn refused(error: &GrantError) -> Verdict {
    let text = error.to_string();
    let grant = match error {
        GrantError::PermissionRevoked { grant }
        | GrantError::Expired { grant, .. }
        | GrantError::NotStarted { grant, .. } => Some(grant.clone()),
        _ => None,
    };
    Verdict::Refused {
        reason: text.split(':').next().unwrap_or_default().to_owned(),
        grant,
    }
}

/// Why `question`'s identity can or cannot perform its action on its
/// resource, from one traced check read under `set`, with the committed
/// grant events those `book` holds up to log position `committed` and the
/// projector at `reached`.
pub fn why(
    evaluator: &Evaluator,
    book: &GrantBook,
    committed: u64,
    reached: &Reached,
    set: &mut QuestionSet,
    question: &Question<'_>,
) -> Result<Explanation, GrantError> {
    let traced = evaluator.check_traced(question, reached.token.as_deref(), set.revision())?;
    set.fix(&traced.checked_at);
    if let Some(refusal) = stale(book, reached.position, committed, question) {
        return Err(refusal);
    }
    let verdict = if traced.permitted {
        let mut shown = BTreeSet::new();
        if let Some(check) = traced.trace.as_ref().and_then(|trace| trace.check.as_ref()) {
            traced_grants(check, &mut shown);
        }
        let through_trace = book
            .held_by(question.subject)
            .map(|record| record.grant().id())
            .filter(|id| shown.contains(&id.to_string()))
            .find_map(|id| {
                let lineage = book.lineage(id).ok()?;
                lys_identity::grants::revocation::unrevoked(book, &lineage).ok()?;
                Some((id, lineage))
            });
        let resting = through_trace
            .or_else(|| resting(book, question).map(|(grant, lineage)| (grant.id(), lineage)));
        match resting {
            Some((grant, lineage)) => Verdict::Permitted {
                grant: Some(grant),
                path: identities(book, &lineage.path),
                responsible: Some(lineage.root_person),
            },
            None => Verdict::Permitted {
                grant: None,
                path: vec![question.subject],
                responsible: None,
            },
        }
    } else {
        refused(&refusal(book, question, evaluator.now()))
    };
    Ok(Explanation {
        identity: question.subject,
        resource: question.resource.clone(),
        action: question.action.clone(),
        verdict,
        policy: Policy {
            schema_sha256: digest(),
            revision: traced.checked_at,
        },
    })
}
