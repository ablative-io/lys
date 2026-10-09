//! Authenticate a complete tail inside its provider's current-head reading,
//! and name every authority effect of every event in it.
//!
//! A tail is read only on explicit demand, and only between the owner's
//! trusted frontier and a freshly certified head; no permission read
//! acquires one, and no reconciliation failure is waived by one. The
//! classification answers, for one selected lineage of the folded book,
//! whether any included event can change it: a revoke or a second issue of
//! any grant on it, or the spend of a one-time grant on it. An event whose
//! subject neither the book nor the tail itself names cannot be classified,
//! so it cannot prove disjointness either.

use std::collections::{BTreeMap, BTreeSet};

use lys_log_store::witness::TailWitness;
use lys_log_store::{Frontier, LeafStore, PinnedRoot};

use super::authority::Grants;
use super::permission::RelationshipStore;
use super::projection::GrantBook;
use super::types::{GrantId, Source};
use super::{GrantChange, GrantError, GrantEvent, SignedGrantEvent, verify_grant_event};
use crate::{IdentityError, IdentityId};

/// What one authenticated tail event does to authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TailEffect {
    /// A grant is issued from its source.
    Issued {
        /// The grant issued.
        grant: GrantId,
        /// What it derives from.
        source: Source,
    },
    /// A grant is revoked, and with it everything derived from it.
    Revoked {
        /// The grant revoked.
        grant: GrantId,
    },
    /// A one-time grant is spent by the use the event records.
    Spent {
        /// The grant spent.
        grant: GrantId,
    },
    /// A use that changes no authority: the grant is not one-time, is
    /// already ended, or the use is not its holder's, which the book refuses.
    Used {
        /// The grant used.
        grant: GrantId,
    },
    /// A revoke or use of a grant neither the folded book nor an earlier
    /// tail event issued: its effect cannot be classified.
    Unknown {
        /// The grant the event names.
        grant: GrantId,
    },
}

impl TailEffect {
    fn subject(&self) -> GrantId {
        match self {
            Self::Issued { grant, .. }
            | Self::Revoked { grant }
            | Self::Spent { grant }
            | Self::Used { grant }
            | Self::Unknown { grant } => *grant,
        }
    }
}

/// One tail event's effect at its exact log index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailEffectAt {
    /// The log index of the event.
    pub index: u64,
    /// What it does to authority.
    pub effect: TailEffect,
}

/// How a complete tail bears on one selected lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TailBearing {
    /// No included event can change the lineage.
    Disjoint,
    /// The first included event that changes a grant on the lineage.
    Relevant {
        /// Its log index.
        index: u64,
        /// Its effect.
        effect: TailEffect,
    },
    /// The first included event whose effect cannot be classified.
    Unclassifiable {
        /// Its log index.
        index: u64,
        /// Its effect.
        effect: TailEffect,
    },
}

/// The authority effects of a complete, authenticated tail for one lineage of
/// the folded book, with the work the classification did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailAuthority {
    /// The selected grant and every ancestor, from the folded book.
    pub lineage: Vec<GrantId>,
    /// Every included event's effect, in log order.
    pub effects: Vec<TailEffectAt>,
    /// Whether the tail can change the lineage.
    pub bearing: TailBearing,
    /// The number of tail events read.
    pub tail_len: u64,
    /// The number of lineage membership checks made: one per tail event.
    pub lineage_visits: u64,
}

pub(super) fn unavailable(reason: impl Into<String>) -> GrantError {
    GrantError::Identity(IdentityError::LogUnavailable {
        reason: reason.into(),
    })
}

pub(super) fn authenticate(
    witness: &TailWitness,
    settled: &Frontier,
    origin: &str,
    key: &[u8; 32],
) -> Result<Vec<SignedGrantEvent>, GrantError> {
    if witness.origin != origin {
        return Err(unavailable("tail witness names a different grant log"));
    }
    let lower = PinnedRoot {
        tree_size: settled.size(),
        root: settled.root(),
    };
    if witness.lower != lower {
        return Err(unavailable(
            "tail witness differs from the trusted grant frontier",
        ));
    }
    let count = u64::try_from(witness.leaves.len())
        .map_err(|error| unavailable(format!("tail leaf count cannot be represented: {error}")))?;
    if witness.upper.tree_size.checked_sub(lower.tree_size) != Some(count) {
        return Err(unavailable(
            "tail witness does not contain the complete grant range",
        ));
    }
    let mut frontier = settled.clone();
    let mut events = Vec::with_capacity(witness.leaves.len());
    for leaf in &witness.leaves {
        if leaf.index != frontier.size() {
            return Err(unavailable(
                "grant tail indexes are omitted, repeated or reordered",
            ));
        }
        let event = verify_grant_event(&leaf.bytes, key)?;
        frontier.push(&leaf.bytes);
        events.push(event);
    }
    if frontier.size() != witness.upper.tree_size || frontier.root() != witness.upper.root {
        return Err(unavailable(
            "authenticated grant tail does not reach its certified root",
        ));
    }
    Ok(events)
}

/// The grants the tail itself issued, whether each is one-time and who holds
/// it, with the grants the tail ended.
#[derive(Default)]
struct Seen {
    issued: BTreeMap<GrantId, (bool, IdentityId)>,
    ended: BTreeSet<GrantId>,
}

fn effect_of(book: &GrantBook, seen: &mut Seen, event: &GrantEvent) -> TailEffect {
    let known = |seen: &Seen, grant: GrantId| {
        book.grant(grant).is_some() || seen.issued.contains_key(&grant)
    };
    match event.change() {
        GrantChange::Issue(grant) => {
            seen.issued
                .insert(grant.id(), (grant.is_once(), grant.holder()));
            TailEffect::Issued {
                grant: grant.id(),
                source: grant.source(),
            }
        }
        GrantChange::Revoke { grant, .. } => {
            if !known(seen, *grant) {
                return TailEffect::Unknown { grant: *grant };
            }
            seen.ended.insert(*grant);
            TailEffect::Revoked { grant: *grant }
        }
        GrantChange::Use { grant, .. } => {
            let (once, holder) = match book.record(*grant) {
                Some(record) => (
                    record.grant().is_once() && record.revoked().is_none(),
                    record.grant().holder(),
                ),
                None => match seen.issued.get(grant) {
                    Some(issued) => *issued,
                    None => return TailEffect::Unknown { grant: *grant },
                },
            };
            if once && holder == event.caller() && seen.ended.insert(*grant) {
                TailEffect::Spent { grant: *grant }
            } else {
                TailEffect::Used { grant: *grant }
            }
        }
    }
}

fn classify(
    book: &GrantBook,
    lineage: Vec<GrantId>,
    first: u64,
    events: &[SignedGrantEvent],
) -> TailAuthority {
    let on_path: BTreeSet<GrantId> = lineage.iter().copied().collect();
    let mut seen = Seen::default();
    let mut effects = Vec::with_capacity(events.len());
    let mut bearing = TailBearing::Disjoint;
    let (mut tail_len, mut lineage_visits) = (0, 0);
    for (index, signed) in (first..).zip(events) {
        tail_len += 1;
        let effect = effect_of(book, &mut seen, signed.event());
        lineage_visits += 1;
        let touches = on_path.contains(&effect.subject());
        if bearing == TailBearing::Disjoint {
            bearing = match &effect {
                TailEffect::Unknown { .. } => TailBearing::Unclassifiable {
                    index,
                    effect: effect.clone(),
                },
                TailEffect::Issued { .. }
                | TailEffect::Revoked { .. }
                | TailEffect::Spent { .. }
                    if touches =>
                {
                    TailBearing::Relevant {
                        index,
                        effect: effect.clone(),
                    }
                }
                TailEffect::Issued { .. }
                | TailEffect::Revoked { .. }
                | TailEffect::Spent { .. }
                | TailEffect::Used { .. } => TailBearing::Disjoint,
            };
        }
        effects.push(TailEffectAt { index, effect });
    }
    TailAuthority {
        lineage,
        effects,
        bearing,
        tail_len,
        lineage_visits,
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// Name every authority effect of the complete tail `witness` certifies,
    /// and how it bears on `grant`'s lineage in the folded book, within one
    /// current-head reading of the explicit tail capability.
    ///
    /// This is evidence on explicit demand. It records nothing, permits
    /// nothing, and does not change the rule that a reconciliation failure
    /// refuses every permission read with its original error.
    ///
    /// # Errors
    /// Refuses when the folded book does not stand at the trusted frontier,
    /// `grant` has no checked lineage in it, the capability is absent, or the
    /// provider, a signature or a bound refuses the witness, each by its
    /// original name.
    pub fn tail_authority(
        &self,
        witness: &TailWitness,
        grant: GrantId,
    ) -> Result<TailAuthority, GrantError> {
        if self.folded != self.ledger.trusted_size() {
            return Err(unavailable(
                "the folded grant book does not stand at the trusted frontier",
            ));
        }
        let lineage = self.book.lineage(grant)?.path;
        let first = witness.lower.tree_size;
        self.ledger.with_verified_tail(witness, |events| {
            Ok(classify(&self.book, lineage, first, events))
        })
    }

    /// Refuse `claimed` unless it is exactly what [`Grants::tail_authority`]
    /// derives for `grant` from the same witness now.
    ///
    /// # Errors
    /// The errors of [`Grants::tail_authority`], and a named refusal when
    /// any claimed lineage, effect, bearing or count differs.
    pub fn confirm_tail_authority(
        &self,
        witness: &TailWitness,
        grant: GrantId,
        claimed: &TailAuthority,
    ) -> Result<(), GrantError> {
        if &self.tail_authority(witness, grant)? == claimed {
            Ok(())
        } else {
            Err(unavailable(
                "the claimed tail authority differs from the authenticated tail",
            ))
        }
    }
}
