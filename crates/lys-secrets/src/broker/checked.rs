//! Permission asked before the broker is taken. The permission source may
//! be a remote engine whose answer takes as long as the network does, so a
//! proxy asks it the questions a use's admission will ask
//! ([`Broker::asks_for`]) while holding nothing, and hands the answers to
//! the admission ([`Broker::admit_use_checked`]) and to the forward boundary
//! ([`Broker::at_forward_boundary_checked`]).
//!
//! An answer is used only for the exact question it answers: the relation,
//! the identity and the secret or scope. A question admission asks that no
//! answer covers, because the secret's scope changed in between, is asked
//! of the permission source there, as it always was.

use crate::handle::HandleToken;
use crate::permission::{Denied, PermissionCheck, Permitted, Relation};
use crate::store::Scope;

use super::Broker;

/// One question admission asks of the permission source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// The relation asked for.
    pub relation: Relation,
    /// The identity it is asked for.
    pub identity: String,
    /// The secret, or for `member` the scope's target.
    pub target: String,
}

impl Ask {
    /// The permission source's answer to this question.
    fn put<P: PermissionCheck + ?Sized>(&self, permissions: &P) -> Result<Permitted, Denied> {
        match self.relation {
            Relation::Use => permissions.may_use(&self.identity, &self.target),
            Relation::Read => permissions.may_read(&self.identity, &self.target),
            Relation::Lend => permissions.may_lend(&self.identity, &self.target),
            Relation::Member => permissions.member_of(&self.identity, &self.target),
        }
    }
}

/// The permission source's answers to a use's questions, got before the
/// broker was taken.
#[derive(Debug, Clone, Default)]
pub struct Checked {
    answers: Vec<(Ask, Result<Permitted, Denied>)>,
}

impl Checked {
    /// Asks `permissions` each of `asks`, once.
    pub fn answer<P: PermissionCheck + ?Sized>(asks: &[Ask], permissions: &P) -> Self {
        Self {
            answers: asks
                .iter()
                .map(|ask| (ask.clone(), ask.put(permissions)))
                .collect(),
        }
    }

    /// The answer to `relation` for `identity` on `target`, when one was got.
    pub(super) fn get(
        &self,
        relation: Relation,
        identity: &str,
        target: &str,
    ) -> Option<Result<Permitted, Denied>> {
        self.answers
            .iter()
            .find(|(ask, _)| {
                ask.relation == relation && ask.identity == identity && ask.target == target
            })
            .map(|(_, answer)| answer.clone())
    }
}

impl<P: PermissionCheck> Broker<P> {
    /// The questions admitting a use of `token` asks the permission source:
    /// `use` on its secret, and `member` of the secret's scope when only the
    /// permission source can place its holder inside it. None for a token
    /// that opens no handle. Nothing is asked here and nothing recorded.
    pub fn asks_for(&self, token: &HandleToken) -> Vec<Ask> {
        let Some(record) = self.find(token) else {
            return Vec::new();
        };
        let mut asks = vec![Ask {
            relation: Relation::Use,
            identity: record.identity.clone(),
            target: record.secret.clone(),
        }];
        if let Some(target) = self.scope_question(&record.identity, &record.secret) {
            asks.push(Ask {
                relation: Relation::Member,
                identity: record.identity.clone(),
                target,
            });
        }
        asks
    }

    /// The scope target `member` is asked on, when `identity` is inside
    /// the scope of `secret` only if the permission source says so.
    fn scope_question(&self, identity: &str, secret: &str) -> Option<String> {
        let entry = self.store.entry(secret)?;
        let scope = self.store.scope(secret)?;
        let own = entry.owner == identity
            || matches!(&scope, Scope::Personal(person) if person == identity);
        (!own).then(|| scope.target())
    }
}
