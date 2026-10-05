//! A grant's ancestry, walked from the grant to its root and checked at every hop.
//!
//! Every grant is bounded by an acyclic ancestry ending at a root held by its
//! responsible person. Each hop is judged again here, whoever wrote it: the
//! grant was issued by its source's holder or the person answering for both
//! agents, is on the same resource, carries
//! only actions its source let be passed on, to a kind of recipient its source
//! names, and ends no later than its source. A grant that does not pass is
//! refused by the boundary it broke, so a forged or hand-written grant carries
//! no more authority than one admitted today.

use std::collections::BTreeSet;

use super::error::GrantError;
use super::types::{Grant, GrantId, PassOn, RecipientKind, Source};
use crate::id::{IdentityId, PersonId};

/// A grant's checked ancestry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    /// The grant and every ancestor, from the grant to its root.
    pub path: Vec<GrantId>,
    /// The person the root grant is held by: the human authority of the chain.
    pub root_person: PersonId,
    /// The earliest end on the path, and the grant it belongs to.
    pub ends: Option<(u64, GrantId)>,
}

/// Whether `identity` is an agent, whose responsible person may pass on
/// between agents they answer for.
pub(super) fn is_agent(identity: IdentityId) -> bool {
    match identity {
        IdentityId::Agent(_) => true,
        IdentityId::Person(_) | IdentityId::ServiceAccount(_) | IdentityId::Connector(_) => false,
    }
}

/// Refuse `grant` unless it lies within what `source` let be passed on.
pub fn check_hop(grant: &Grant, source: &Grant) -> Result<(), GrantError> {
    let parts = grant.parts();
    let responsible_pass = is_agent(source.holder())
        && is_agent(grant.holder())
        && parts.issuer == IdentityId::Person(source.responsible())
        && grant.responsible() == source.responsible();
    if parts.issuer != source.holder() && !responsible_pass {
        return Err(GrantError::IssuerNotHolder {
            grant: grant.id().to_string(),
            source_grant: source.id().to_string(),
        });
    }
    if grant.resource() != source.resource() {
        return Err(GrantError::ResourceOutside {
            requested: grant.resource().to_string(),
            source_grant: source.id().to_string(),
        });
    }
    let PassOn::To {
        actions,
        recipients,
    } = source.pass_on()
    else {
        return Err(GrantError::UseOnly {
            grant: source.id().to_string(),
        });
    };
    let kind = RecipientKind::of(grant.holder());
    if !recipients.contains(&kind) {
        return Err(GrantError::RecipientRefused {
            kind,
            grant: source.id().to_string(),
        });
    }
    let outside: Vec<&str> = grant
        .actions()
        .difference(actions)
        .map(super::types::Action::as_str)
        .collect();
    if !outside.is_empty() {
        return Err(GrantError::ActionsOutside {
            relation: parts.relation.to_string(),
            outside: outside.join(", "),
            model_version: parts.model_version,
        });
    }
    if let PassOn::To {
        actions: onward,
        recipients: onward_to,
    } = grant.pass_on()
        && (!onward.is_subset(actions) || !onward_to.is_subset(recipients))
    {
        return Err(GrantError::PassOnBeyondSource {
            source_grant: source.id().to_string(),
        });
    }
    check_end(grant.window().ends_at(), source)
}

/// Refuse an end later than `source`'s own end. A missing end under a source
/// that ends is later than it, and nothing is clamped.
pub fn check_end(requested: Option<u64>, source: &Grant) -> Result<(), GrantError> {
    let Some(bound) = source.window().ends_at() else {
        return Ok(());
    };
    if requested.is_none_or(|ends| ends > bound) {
        return Err(GrantError::ExpiryBeyondSource {
            requested: requested.map_or_else(|| "no end".to_owned(), |ends| ends.to_string()),
            source_grant: source.id().to_string(),
            source_ends: bound,
        });
    }
    Ok(())
}

/// Walk `start` to its root through `lookup`, checking every hop. An
/// ancestry of any length is walked; a grant met twice is a cycle.
pub fn resolve<'a>(
    start: GrantId,
    lookup: impl Fn(GrantId) -> Option<&'a Grant>,
) -> Result<Lineage, GrantError> {
    let unknown = |grant: GrantId| GrantError::SourceUnknown {
        grant: grant.to_string(),
    };
    let mut current = lookup(start).ok_or_else(|| unknown(start))?;
    let mut path = vec![start];
    let mut seen = BTreeSet::from([start]);
    let mut ends = current.window().ends_at().map(|ends| (ends, start));
    loop {
        let source_id = match current.source() {
            Source::Root => {
                let person = match current.holder() {
                    IdentityId::Person(person) => person,
                    IdentityId::Agent(_)
                    | IdentityId::ServiceAccount(_)
                    | IdentityId::Connector(_) => {
                        return Err(GrantError::LineageMalformed {
                            reason: "a root grant is held by its responsible person",
                        });
                    }
                };
                return Ok(Lineage {
                    path,
                    root_person: person,
                    ends,
                });
            }
            Source::Grant(source_id) => source_id,
        };
        if !seen.insert(source_id) {
            return Err(GrantError::LineageCycle {
                grant: source_id.to_string(),
            });
        }
        let source = lookup(source_id).ok_or_else(|| unknown(source_id))?;
        check_hop(current, source)?;
        if let Some(source_ends) = source.window().ends_at()
            && ends.is_none_or(|(earliest, _)| source_ends <= earliest)
        {
            ends = Some((source_ends, source_id));
        }
        path.push(source_id);
        current = source;
    }
}
