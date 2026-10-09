//! The operator's reset of a refused grant checkpoint (ACCESS-006 R5).
//!
//! A grant log whose checkpoint is refused, damaged, foreign or unreadable,
//! holds the grants unready by the refusal's name
//! ([`GrantError::CheckpointRefused`]); nothing is answered from them and
//! nothing rebuilds them by itself. [`Grants::open_reset`] is the one way
//! back: the operator names the refusal it discards, by its `Snapshot…`
//! name, the reset confirms that is the refusal now standing, and only then
//! folds every leaf of the verified log into a fresh book and writes a new
//! checkpoint. It answers the refusal it discarded, words and all. A reset
//! of grants whose checkpoint is not refused, or naming another refusal,
//! is refused [`GrantError::ResetRefused`] and changes nothing.

use std::collections::BTreeMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_log_store::witness::TailWitnessProvider;
use lys_log_store::{LeafStore, SnapshotRefusal};

use super::authority::Grants;
use super::error::GrantError;
use super::events::SignedGrantEvent;
use super::model::Model;
use super::permission::RelationshipStore;
use super::projection::GrantBook;
use super::recovery::GrantLedger;
use super::state;
use crate::id::PersonId;
use crate::log::{Coordinate, Reopen};

/// The stable name of a snapshot refusal: the words before its first colon.
#[must_use]
pub fn refusal_name(refusal: &SnapshotRefusal) -> String {
    let words = refusal.to_string();
    words
        .split_once(':')
        .map_or(words.as_str(), |(name, _)| name)
        .to_owned()
}

fn confirm(refusal: &SnapshotRefusal, discard: &str) -> Result<String, GrantError> {
    let name = refusal_name(refusal);
    if name == discard {
        Ok(refusal.to_string())
    } else {
        Err(GrantError::ResetRefused {
            reason: format!(
                "the refused checkpoint is {name}, not {discard}; nothing is discarded"
            ),
        })
    }
}

impl<S: LeafStore, R: RelationshipStore> Grants<S, R> {
    /// The grants from an opened ledger, the book its checkpoint held (or
    /// a fresh one) folded through `folded`, and the events after it.
    pub(super) fn opened(
        ledger: GrantLedger<S>,
        (book, folded, events): (GrantBook, u64, Vec<(SignedGrantEvent, Coordinate)>),
        (key, relationships, model, root_authority): (Ed25519Identity, R, Model, PersonId),
    ) -> Result<Self, GrantError> {
        let mut grants = Self {
            model,
            root_authority,
            key,
            ledger,
            book,
            folded,
            relationships,
            unreported: BTreeMap::new(),
            startup_degraded: None,
        };
        for (signed, coordinate) in events {
            grants.record_committed(&signed, coordinate)?;
        }
        grants.snapshot();
        grants.startup_degraded = grants.project_reading()?.degraded;
        Ok(grants)
    }

    /// The operator's reset: open the grants held unready by the refused
    /// checkpoint named `discard`, discarding it, from every leaf of their
    /// verified log, and answer them with the refusal discarded.
    ///
    /// # Errors
    /// [`GrantError::ResetRefused`] when no checkpoint is refused or the
    /// one refused is not `discard`; any failure opening or folding the log
    /// keeps its own name.
    pub fn open_reset(
        reopen: Reopen<S>,
        parts: (Ed25519Identity, R, Model, PersonId),
        every: NonZeroU64,
        tail_provider: Option<Arc<dyn TailWitnessProvider + Send + Sync>>,
        discard: &str,
    ) -> Result<(Self, String), GrantError> {
        let key = &parts.0;
        let checked = GrantLedger::open_checked(reopen, key, every, tail_provider.clone())?;
        let (ledger, events, discarded) = match checked {
            Err((refusal, reopen)) => {
                let discarded = confirm(&refusal, discard)?;
                let (ledger, opening) =
                    GrantLedger::open_rebuilt(reopen, key, every, tail_provider, refusal)?;
                (ledger, opening.events, discarded)
            }
            Ok((mut ledger, opening)) => {
                let Some(Err(reason)) = opening
                    .state
                    .as_deref()
                    .map(|held| state::decode(held, opening.size))
                else {
                    return Err(GrantError::ResetRefused {
                        reason: "the grant checkpoint is not refused; nothing is discarded"
                            .to_owned(),
                    });
                };
                let refusal = SnapshotRefusal::StateUnreadable {
                    reason: reason.clone(),
                };
                let discarded = confirm(&refusal, discard)?;
                let events = ledger.refuse_state(reason, key)?;
                (ledger, events, discarded)
            }
        };
        let grants = Self::opened(ledger, (GrantBook::new(), 0, events), parts)?;
        Ok((grants, discarded))
    }
}
