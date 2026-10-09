//! The membership page cursors this service keeps (ACCESS-006 R3, R5).
//!
//! A cursor's text is a random handle, 32 bytes as unpadded URL-safe
//! base64, that the service looks up; nothing a caller could read or edit
//! is in it, so it cannot be forged or moved. The kept record binds the
//! asker, the question, the grant log and epoch, the revision it was issued
//! at and the last key decided. It never grants anything: every row of a
//! later page is decided again.
//!
//! Retention is finite and stated by the operator: a cursor is kept for
//! `cursor_seconds` and at most `cursors_max` are kept at once. The cursors
//! of one listing form a chain; the page that completes the listing
//! releases its whole chain, a cursor from another log or epoch releases
//! its chain as reset, and a restart keeps none. A cursor no longer kept is
//! refused [`CURSOR_EXPIRED`]; a page needing one more than the setting
//! allows is refused [`CURSORS_FULL`]. Using a cursor does not release it,
//! so a page whose answer was lost can be asked again.

use std::collections::{BTreeMap, BTreeSet};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use lys_pass::membership::GrantLog;
use lys_pass::membership_pages::{
    CURSOR_EXPIRED, CURSOR_FOREIGN, CURSOR_MALFORMED, CURSOR_RESET, CURSORS_FULL,
};

use crate::channel_membership::Named;

/// The bytes of randomness behind one cursor.
pub(crate) const HANDLE_BYTES: usize = 32;

/// What a page continues: who asks, the question, the log served and the
/// grants' revision now.
pub(crate) struct Asked<'a> {
    pub(crate) asker: Option<&'a str>,
    pub(crate) question: &'a str,
    pub(crate) served: &'a GrantLog,
    pub(crate) revision: u64,
}

/// One kept cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Kept {
    asker: Option<String>,
    question: String,
    log: GrantLog,
    revision: u64,
    after: String,
    chain: u64,
    expires: u64,
}

/// A continuation a kept cursor allows: the last key decided, and the
/// listing it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Continued {
    pub(crate) after: String,
    pub(crate) chain: u64,
}

/// The kept cursors.
#[derive(Debug)]
pub(crate) struct CursorBook {
    seconds: u64,
    most: usize,
    kept: BTreeMap<String, Kept>,
    expiring: BTreeSet<(u64, String)>,
    chains: BTreeMap<u64, BTreeSet<String>>,
    next_chain: u64,
    released: u64,
}

fn named(name: &str, reason: &str) -> Named {
    (name.to_owned(), reason.to_owned())
}

impl CursorBook {
    /// A book keeping each cursor `seconds` and at most `most` at once.
    pub(crate) fn new(seconds: u64, most: u32) -> Self {
        Self {
            seconds,
            most: usize::try_from(most).unwrap_or(usize::MAX),
            kept: BTreeMap::new(),
            expiring: BTreeSet::new(),
            chains: BTreeMap::new(),
            next_chain: 0,
            released: 0,
        }
    }

    /// How many cursors are kept now.
    pub(crate) fn retained(&self) -> usize {
        self.kept.len()
    }

    /// How many cursors have been released, by completion, expiry or reset.
    pub(crate) fn released(&self) -> u64 {
        self.released
    }

    fn drop_handle(&mut self, handle: &str) {
        if let Some(kept) = self.kept.remove(handle) {
            self.expiring.remove(&(kept.expires, handle.to_owned()));
            if let Some(chain) = self.chains.get_mut(&kept.chain) {
                chain.remove(handle);
                if chain.is_empty() {
                    self.chains.remove(&kept.chain);
                }
            }
            self.released += 1;
        }
    }

    /// Release every cursor whose retention ended at or before `now`,
    /// earliest first, reading only those.
    pub(crate) fn expire(&mut self, now: u64) {
        while let Some((expires, handle)) = self.expiring.first().cloned() {
            if expires > now {
                break;
            }
            self.drop_handle(&handle);
        }
    }

    /// Release every cursor of the listing `chain`.
    pub(crate) fn complete(&mut self, chain: u64) {
        let handles = self.chains.remove(&chain).unwrap_or_default();
        for handle in handles {
            self.drop_handle(&handle);
        }
    }

    /// The continuation `handle` allows for `asked` at `now`, or why not,
    /// by name.
    pub(crate) fn take(
        &mut self,
        handle: &str,
        asked: &Asked<'_>,
        now: u64,
    ) -> Result<Continued, Named> {
        self.expire(now);
        let shaped = URL_SAFE_NO_PAD
            .decode(handle)
            .is_ok_and(|bytes| bytes.len() == HANDLE_BYTES);
        if !shaped {
            return Err(named(
                CURSOR_MALFORMED,
                "the cursor is not one this service issues",
            ));
        }
        let Some(kept) = self.kept.get(handle) else {
            return Err(named(
                CURSOR_EXPIRED,
                "the cursor is no longer kept: its retention passed, its listing completed or the service restarted",
            ));
        };
        if &kept.log != asked.served || kept.revision > asked.revision {
            let chain = kept.chain;
            self.complete(chain);
            return Err(named(
                CURSOR_RESET,
                "the cursor was issued from another grant log, epoch or later revision",
            ));
        }
        if kept.asker.as_deref() != asked.asker || kept.question != asked.question {
            return Err(named(
                CURSOR_FOREIGN,
                "the cursor was issued for another asker or question",
            ));
        }
        Ok(Continued {
            after: kept.after.clone(),
            chain: kept.chain,
        })
    }

    /// Keep a cursor continuing `asked` after `after`, in the listing
    /// `chain` or a new one, under the handle `random` gives, at `now`.
    pub(crate) fn issue(
        &mut self,
        asked: &Asked<'_>,
        (after, chain): (&str, Option<u64>),
        random: [u8; HANDLE_BYTES],
        now: u64,
    ) -> Result<String, Named> {
        self.expire(now);
        if self.kept.len() >= self.most {
            return Err(named(
                CURSORS_FULL,
                "this service already keeps as many cursors as its setting allows",
            ));
        }
        let handle = URL_SAFE_NO_PAD.encode(random);
        if self.kept.contains_key(&handle) {
            return Err(named(CURSORS_FULL, "the cursor handle is already kept"));
        }
        let chain = chain.unwrap_or_else(|| {
            self.next_chain += 1;
            self.next_chain
        });
        let expires = now.saturating_add(self.seconds);
        self.kept.insert(
            handle.clone(),
            Kept {
                asker: asked.asker.map(str::to_owned),
                question: asked.question.to_owned(),
                log: asked.served.clone(),
                revision: asked.revision,
                after: after.to_owned(),
                chain,
                expires,
            },
        );
        self.expiring.insert((expires, handle.clone()));
        self.chains.entry(chain).or_default().insert(handle.clone());
        Ok(handle)
    }
}

#[cfg(test)]
#[path = "channel_membership_cursors_tests.rs"]
mod tests;
