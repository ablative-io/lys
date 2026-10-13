//! One secret-free timing summary, including exchanges that refuse an early act.

use std::time::Instant;

#[derive(Clone, Copy)]
pub(super) enum Act {
    TakeGrant,
    IsLive,
    Pass,
    Issue,
}

pub(super) struct Timing<'a> {
    holder_kind: &'static str,
    app: &'a str,
    started: Instant,
    take_grant: Option<u128>,
    is_live: Option<u128>,
    pass: Option<u128>,
    issue: Option<u128>,
    cache: &'static str,
}

impl<'a> Timing<'a> {
    pub(super) fn new(holder_kind: &'static str, app: &'a str) -> Self {
        Self {
            holder_kind,
            app,
            started: Instant::now(),
            take_grant: None,
            is_live: None,
            pass: None,
            issue: None,
            cache: "not_run",
        }
    }

    pub(super) fn measure<T>(&mut self, act: Act, run: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let result = run();
        let elapsed = Some(started.elapsed().as_millis());
        match act {
            Act::TakeGrant => self.take_grant = elapsed,
            Act::IsLive => self.is_live = elapsed,
            Act::Pass => self.pass = elapsed,
            Act::Issue => self.issue = elapsed,
        }
        result
    }

    pub(super) fn cache(&mut self, hit: bool) {
        self.cache = if hit { "hit" } else { "miss" };
    }
}

impl Drop for Timing<'_> {
    fn drop(&mut self) {
        tracing::info!(target: "lys_identity_server::passfast",
            holder_kind = self.holder_kind, app = self.app,
            take_grant_elapsed_ms = ?self.take_grant, is_live_elapsed_ms = ?self.is_live,
            pass_elapsed_ms = ?self.pass, cache = self.cache, issue_elapsed_ms = ?self.issue,
            elapsed_ms = self.started.elapsed().as_millis(), "token exchange");
    }
}
