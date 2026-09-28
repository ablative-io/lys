//! The handles an identity holds, as a screen shows them: what each stands
//! for, how far it has been used and when it ends, whether and by whom it
//! was ended, and where the provider's part of its revocation stands. Never
//! a token, a digest or a key, and only the handles on a secret the asker
//! may discover.

use crate::permission::PermissionCheck;

use super::{Broker, Ended, UpstreamRevocation};

/// One handle as a screen shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldHandle {
    /// The handle's id, which every record names. It is not the handle.
    pub id: String,
    /// The secret the handle stands for.
    pub secret: String,
    /// How many uses it was issued for.
    pub max_uses: u64,
    /// How many uses the log shows.
    pub used: u64,
    /// When it ends, in milliseconds since the epoch.
    pub not_after_ms: i64,
    /// Whether it was dropped, itself or by a drop above it.
    pub dropped: bool,
    /// Who ended it, under which operation and from which handle, when a
    /// person it acts for ended it or a handle above it.
    pub ended: Option<Ended>,
    /// Where the provider's part of its revocation stands; never inferred
    /// from its ending here.
    pub upstream: UpstreamRevocation,
    /// The most it may spend, when it is capped.
    pub spend_cap: Option<u64>,
    /// The spend settled so far.
    pub settled: u64,
    /// The handle it was derived from, when it was.
    pub parent: Option<String>,
}

impl<P: PermissionCheck> Broker<P> {
    /// The handles `holder` holds, in order of id, to `identity`: each only
    /// when `identity` may discover its secret. A handle on a secret it may
    /// not discover is left out, as one never issued.
    /// Each secret is asked of the permission source once, however many of
    /// the handles stand for it.
    pub fn held_by(&self, identity: &str, holder: &str) -> Vec<HeldHandle> {
        let mut discovery = self.discovery(identity);
        self.handles
            .values()
            .filter(|record| record.identity == holder)
            .filter(|record| discovery.discovers(&record.secret))
            .map(|record| HeldHandle {
                id: record.id.clone(),
                secret: record.secret.clone(),
                max_uses: record.max_uses,
                used: record.used,
                not_after_ms: record.not_after_ms,
                dropped: self.line_dropped(&record.id),
                ended: record.ended.clone(),
                upstream: record.upstream.clone(),
                spend_cap: record.spend_cap,
                settled: record.settled,
                parent: record.parent.clone(),
            })
            .collect()
    }
}
