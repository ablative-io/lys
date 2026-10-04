//! What a follower does once its session's process has ended.
//!
//! A harness's stream ends with its process, so its follower stops. A run's
//! usage file under the proxy's state does not: a call the run made may end
//! after the run did, and its line is written when it ends. So that follower
//! reads on, woken by change notices as before and never by a timer, until
//! the proxy's journal holds no open call carrying the run's key. The proxy
//! appends a call's line before it retires the call's journal record, so a
//! run with no record left has every finished call's line in its file: the
//! file is read to its end once more and the follower stops.
//!
//! A journal that cannot be watched or read is said in the session's
//! coverage by name, `journal_unreadable`, and the follower then reads the
//! file to its end once and stops: what ends later is said to be uncounted,
//! not left unsaid.

use std::path::PathBuf;

use lys_home::proxy::journal::Journal;

use super::super::{Session, Sessions};
use super::Wake;
use super::stream::{append, stop_follower};
use crate::tracking_proxy::PROXY_ADAPTER;
use crate::tracking_store::{Body, Coverage};

/// End the following of `session`, named `id`, whose process has ended.
pub(super) fn end_follower(id: &str, session: &mut Session) {
    // The same choice `read_source` makes: the usage file is the stream of
    // a session whose harness is not tracked.
    let drains = session.guard.tracking.is_none() && session.guard.proxy.is_some();
    if !drains {
        if let Some(follower) = session.follower.take() {
            stop_follower(id, &follower);
        }
        return;
    }
    // The follower stays the session's, so a later binding of its stream
    // stops it as it stops any other.
    let gone = session
        .follower
        .as_ref()
        .is_some_and(|follower| follower.send(Wake::Drain).is_err());
    if gone {
        crate::error::said(&format!(
            "session {id}: its stream follower had already ended"
        ));
    }
}

impl Sessions {
    /// The proxy's journal: `journal`, beside `usage` in the proxy's state.
    fn proxy_journal(&self) -> Option<PathBuf> {
        let usage = self.proxy_usage.get()?;
        Some(usage.parent()?.join("journal"))
    }

    /// The run key session `id` is tracked through the proxy under.
    fn run_of(&self, id: &str) -> Option<String> {
        let table = self.lock_logged()?;
        let proxy = table.sessions.get(id)?.guard.proxy.as_ref()?;
        Some(proxy.run.clone())
    }

    /// Watch the proxy's journal as well as the usage file's directory, so
    /// a call's retirement wakes session `id`'s follower. False when it
    /// cannot be watched: that is said, the file has been read to its end
    /// once more, and the follower stops.
    pub(super) fn drain_watched(&self, id: &str, watcher: &mut notify::RecommendedWatcher) -> bool {
        let Some(journal) = self.proxy_journal() else {
            self.undrained(
                id,
                "this runner was not told where the proxy keeps its state",
            );
            return false;
        };
        let watched =
            notify::Watcher::watch(watcher, &journal, notify::RecursiveMode::NonRecursive);
        if let Err(error) = watched {
            self.undrained(
                id,
                &format!(
                    "the proxy's journal at {} cannot be watched: {error}",
                    journal.display()
                ),
            );
            return false;
        }
        true
    }

    /// Whether session `id`'s follower is done: the journal holds no open
    /// call of its run and the usage file has been read to its end once
    /// more, or the journal cannot be read and that has been said.
    pub(super) fn drained(&self, id: &str) -> bool {
        let (Some(run), Some(journal)) = (self.run_of(id), self.proxy_journal()) else {
            // The session is no longer held, so nothing is kept for it.
            return true;
        };
        match Journal::holds_run(&journal, &run) {
            Ok(true) => false,
            Ok(false) => {
                self.read_source(id, None);
                true
            }
            Err(error) => {
                self.undrained(id, &error.to_string());
                true
            }
        }
    }

    /// Say that session `id`'s run cannot be drained and why, then read its
    /// usage file to its end once more.
    fn undrained(&self, id: &str, why: &str) {
        let words = format!(
            "{why}: whether a call of this run is still open is not known, so a call of it that ends after now is not counted"
        );
        crate::error::said(&format!("session {id}: journal_unreadable: {words}"));
        if let Some(mut table) = self.lock_logged() {
            let source = table.feed.source(id).cloned().unwrap_or_default();
            let coverage = Coverage {
                adapter: Some(PROXY_ADAPTER.to_owned()),
                ..Coverage::of("journal_unreadable", &source, None, words)
            };
            let kept = append(&mut table, id, vec![Body::Coverage(coverage)], None);
            drop(table);
            if let Err(error) = kept.and_then(|()| self.writer.barrier()) {
                crate::error::said(&format!("session {id}: coverage_record_failed: {error}"));
            }
            self.wake();
        }
        self.read_source(id, None);
    }
}
