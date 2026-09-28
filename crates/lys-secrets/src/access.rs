//! Who asks a secrets list or a lease route, and the one seam every check
//! those routes make is a call on.
//!
//! The mounting server authenticates the caller: a signed-in person by its
//! own sign-in, an agent by the signed presentation of a handle it holds.
//! It hands the broker an [`Asker`]: the identity, whether it is a person or
//! an agent, and, for a person only, the team ids [`crate::team_ids`] reads
//! from the group claims on the person's token. An agent's claims are never
//! read.
//!
//! The seam answers three questions and nothing else asks them:
//!
//! - whether an asker may see a secret ([`crate::Broker::sees`]): a secret
//!   whose scope is an organisation's; a team's secret whose team is one of
//!   the asker's team ids; a personal secret the asker owns; and a secret
//!   with no scope set, its owner alone;
//! - whether an identity may discover a lease
//!   ([`crate::Broker::discovers_lease`]): only its holder and the person it
//!   is acted for, which is what the lease reads, the handles list and the
//!   revocation read all ask;
//! - whether an identity may revoke a lease
//!   ([`crate::Broker::revokes_lease`]): only the person it is acted for.
//!
//! The person a lease is acted for is the person its holder acts for: the
//! holder itself when it is a person, or the person the permission source's
//! `member` relation on `person/<id>` says it acts for; for a handle lent
//! on, also the person a handle above it is so acted for.
//!
//! Seeing the secret a lease was issued from makes no lease discoverable,
//! and owning it confers no revoke (ADR-095, proposed): while ADR-095 is
//! proposed the set of who may revoke beyond the person acted for is empty.
//! The seam answers from the store's scope and owner fields, the asker's
//! team ids and that acted-for rule, which the broker's ending already
//! kept. The proxy's use checks on a handle are not the seam's and stay
//! where they are.

use serde_json::Value;

use crate::teams::team_ids;

/// Whether the one asking is a person or an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskerKind {
    /// A signed-in person.
    Person,
    /// An agent, which does not sign in and never uses the secrets list.
    Agent,
}

/// The one asking a secrets list or a lease route, as its mounting server
/// authenticated it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asker {
    identity: String,
    kind: AskerKind,
    teams: Vec<String>,
}

impl Asker {
    /// The asker `identity` of `kind`, with the group claims on its token.
    /// A person's team ids are read from `claims` through
    /// [`crate::team_ids`]; an agent's claims are not read at all.
    pub fn new(identity: &str, kind: AskerKind, claims: &Value) -> Self {
        let teams = match kind {
            AskerKind::Person => team_ids(claims),
            AskerKind::Agent => Vec::new(),
        };
        Self {
            identity: identity.to_owned(),
            kind,
            teams,
        }
    }

    /// The identity asking.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Whether it is a person or an agent.
    pub fn kind(&self) -> AskerKind {
        self.kind
    }

    /// The team ids of a person asking; none for an agent.
    pub fn teams(&self) -> &[String] {
        &self.teams
    }

    /// Whether the asker belongs to the team `team`.
    pub fn in_team(&self, team: &str) -> bool {
        self.teams.iter().any(|mine| mine == team)
    }
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
