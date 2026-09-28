//! What a person cannot give from the delegation form, each with its one
//! reason (conformance row 2.4).
//!
//! The list is asked for one source grant the caller holds and one chosen
//! recipient. It holds four kinds of item: a grant in force the caller holds
//! that the recipient's kind cannot be given, a service account the caller
//! holds through such a grant, a relation of the model that no grant the
//! caller holds on the source grant's resource covers in any standing, and,
//! for an agent recipient, the caller's own sign-in identity.
//!
//! Invariants:
//!
//! - Whether a grant is in force is [`effective`]'s answer for the grant and
//!   its ancestry, the same evaluator admission uses. A grant not in force is
//!   not listed, and no item carries a standing.
//! - A grant's reasons are read from the caller's own grant alone: its
//!   pass-on, whether it names a source, and the recipient kinds its pass-on
//!   admits. Who issued it never decides a reason, and nothing about an
//!   ancestor's holder, label or id is read into an item.
//! - Coverage is the model's action sets: a grant covers a relation when the
//!   relation's actions are a subset of the grant's. No rank of names is read.
//! - Each item carries exactly one reason, the first applicable in
//!   [`CannotGiveReason::ALL`]'s order, which puts every reason no choice of
//!   recipient can change before the two about the chosen recipient.
//! - Grant and service-account items come first by the bytes of their grant
//!   id, then relation items in the model's relation order, then the sign-in
//!   identity. The answer is the same whatever order the grants were recorded in.
//! - The computation reads and changes nothing, and admits nothing: a grant
//!   absent from the list is one admission would let be passed on to that
//!   recipient's kind, and admission still judges every attempt.

use std::collections::BTreeSet;
use std::fmt;

use lys_log_store::LeafStore;

use super::admission::{Route, effective};
use super::authority::Grants;
use super::error::GrantError;
use super::model::Model;
use super::permission::RelationshipStore;
use super::projection::{GrantBook, GrantRecord};
use super::types::{Grant, GrantId, PassOn, RecipientKind, Relation, Source};
use crate::error::IdentityError;
use crate::id::IdentityId;
use crate::projection::Projection;

/// The resource kind of a service account: a grant on a resource of this kind
/// is listed as a service-account item, never as a grant item.
pub const SERVICE_ACCOUNT: &str = "service_account";

/// Why something cannot be given. The variants are declared in precedence
/// order, and an item carries the first that applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CannotGiveReason {
    /// A sign-in identity proves who the person is, and no agent holds one.
    SignInIdentity,
    /// The relation carries more than any grant the person holds on the resource.
    AboveWhatYouHold,
    /// The grant was lent to the person for their own use: it is use-only and names a source.
    LentToYou,
    /// The grant lets the person use it and pass none of it on.
    UseOnly,
    /// The grant may be passed on only to people, and the recipient is an agent.
    PeopleOnly,
    /// The grant may be passed on only to agents, and the recipient is a person.
    AgentsOnly,
}

impl CannotGiveReason {
    /// Every reason, in precedence order.
    pub const ALL: [Self; 6] = [
        Self::SignInIdentity,
        Self::AboveWhatYouHold,
        Self::LentToYou,
        Self::UseOnly,
        Self::PeopleOnly,
        Self::AgentsOnly,
    ];

    /// The reason's name, as the answer spells it.
    pub fn name(self) -> &'static str {
        match self {
            Self::SignInIdentity => "sign_in_identity",
            Self::AboveWhatYouHold => "above_what_you_hold",
            Self::LentToYou => "lent_to_you",
            Self::UseOnly => "use_only",
            Self::PeopleOnly => "people_only",
            Self::AgentsOnly => "agents_only",
        }
    }

    /// The reason `name` spells, or none when it is not one of the six.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.name() == name)
    }

    /// The one reason an item carries when every reason of `applicable`
    /// applies to it: the first in precedence order, or none when none applies.
    pub fn first(applicable: impl IntoIterator<Item = Self>) -> Option<Self> {
        applicable.into_iter().min()
    }
}

impl fmt::Display for CannotGiveReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// What one item of the list is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CannotGiveSubject {
    /// A grant the person holds, by its id.
    Grant(GrantId),
    /// A service account the person holds, by the id of the person's grant on it.
    ServiceAccount(GrantId),
    /// A relation of the model on the source grant's resource.
    Relation(Relation),
    /// The person's own sign-in identity.
    SignInIdentity,
}

/// One thing the person cannot give, with its one reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CannotGiveItem {
    /// What it is.
    pub subject: CannotGiveSubject,
    /// The one reason it cannot be given.
    pub reason: CannotGiveReason,
    /// Whether it is the grant the form was opened from.
    pub source: bool,
}

/// Everything the person cannot give the recipient, in the list's order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CannotGiveList {
    /// The grant the form was opened from.
    pub source: GrantId,
    /// The recipient chosen.
    pub recipient: IdentityId,
    /// The items, in order.
    pub items: Vec<CannotGiveItem>,
}

/// A question: what can `caller` not give `recipient`, asked from `source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CannotGiveRequest {
    /// The authenticated caller.
    pub caller: IdentityId,
    /// How the request arrived. It never changes the answer.
    pub route: Route,
    /// The grant the form was opened from, which the caller holds.
    pub source: GrantId,
    /// The recipient chosen.
    pub recipient: IdentityId,
}

/// The one reason `grant` cannot be given to a recipient of `kind`, read
/// from the grant's own members, or none when its pass-on admits that kind.
pub fn grant_reason(grant: &Grant, kind: RecipientKind) -> Option<CannotGiveReason> {
    let mut applicable = BTreeSet::new();
    match grant.pass_on() {
        PassOn::UseOnly => {
            applicable.insert(CannotGiveReason::UseOnly);
            if matches!(grant.source(), Source::Grant(_)) {
                applicable.insert(CannotGiveReason::LentToYou);
            }
        }
        PassOn::To { recipients, .. } if !recipients.contains(&kind) => {
            applicable.insert(match kind {
                RecipientKind::Agent => CannotGiveReason::PeopleOnly,
                RecipientKind::Person => CannotGiveReason::AgentsOnly,
            });
        }
        PassOn::To { .. } => {}
    }
    CannotGiveReason::first(applicable)
}

/// The cannot-give list for `request` at `at`, over the grants `book`
/// records, the identities `directory` records and the relations `model`
/// defines. Refused `SourceUnknown` when the book holds no such source,
/// `NotHolder` when the caller does not hold it, and `IdentityUnknown` when
/// the directory records no such recipient.
pub fn cannot_give(
    book: &GrantBook,
    directory: &Projection,
    model: &Model,
    request: &CannotGiveRequest,
    at: u64,
) -> Result<CannotGiveList, GrantError> {
    let source = book
        .grant(request.source)
        .ok_or_else(|| GrantError::SourceUnknown {
            grant: request.source.to_string(),
        })?;
    if source.holder() != request.caller {
        return Err(GrantError::NotHolder {
            caller: request.caller.to_string(),
            grant: request.source.to_string(),
        });
    }
    if directory.record(request.recipient).is_none() {
        return Err(IdentityError::IdentityUnknown {
            identity: request.recipient.to_string(),
        }
        .into());
    }
    let kind = RecipientKind::of(request.recipient);
    let mut items = Vec::new();
    for record in book.held_by(request.caller) {
        let grant = record.grant();
        if effective(book, directory, grant.id(), at).is_err() {
            continue;
        }
        let Some(reason) = grant_reason(grant, kind) else {
            continue;
        };
        let subject = if grant.resource().kind() == SERVICE_ACCOUNT {
            CannotGiveSubject::ServiceAccount(grant.id())
        } else {
            CannotGiveSubject::Grant(grant.id())
        };
        items.push((grant.id(), subject, reason));
    }
    // A grant id orders by its bytes, so this is the order of their bytes.
    items.sort_by_key(|(id, ..)| *id);
    let mut list: Vec<CannotGiveItem> = items
        .into_iter()
        .map(|(id, subject, reason)| CannotGiveItem {
            subject,
            reason,
            source: id == request.source,
        })
        .collect();
    for (relation, actions) in model.relations_on(source.resource().kind()) {
        let covered = book
            .held_by(request.caller)
            .map(GrantRecord::grant)
            .filter(|grant| grant.resource() == source.resource())
            .any(|grant| actions.is_subset(grant.actions()));
        if !covered {
            list.push(CannotGiveItem {
                subject: CannotGiveSubject::Relation(relation.clone()),
                reason: CannotGiveReason::AboveWhatYouHold,
                source: false,
            });
        }
    }
    if kind == RecipientKind::Agent {
        list.push(CannotGiveItem {
            subject: CannotGiveSubject::SignInIdentity,
            reason: CannotGiveReason::SignInIdentity,
            source: false,
        });
    }
    Ok(CannotGiveList {
        source: request.source,
        recipient: request.recipient,
        items: list,
    })
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// The cannot-give list for `request` at `at`, from the committed grants
    /// and the model they are judged against. It reads only: nothing is
    /// recorded, and nothing is admitted by it.
    pub fn cannot_give(
        &self,
        directory: &Projection,
        request: &CannotGiveRequest,
        at: u64,
    ) -> Result<CannotGiveList, GrantError> {
        cannot_give(&self.book, directory, &self.model, request, at)
    }
}
