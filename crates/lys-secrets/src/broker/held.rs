//! The handles an identity holds, as a screen shows them: what each stands
//! for, how far it has been used and when it ends, whether and by whom it
//! was ended, and where the provider's part of its revocation stands. Never
//! a token, a digest or a key, and only the handles the access seam lets
//! the asker discover: its own, and those acted for it.
//!
//! One lease as its lease route reads it ([`LeaseView`]) is answered only
//! to an identity the access seam lets discover it: its holder and the
//! person it is acted for. Anyone else is answered as for a lease that does
//! not exist.

use serde_json::{Value, json};

use crate::error::LeaseRefusal;
use crate::handle::HandleId;
use crate::permission::PermissionCheck;

use super::{Broker, Ended, LeaseEnd, UpstreamRevocation};

/// One lease as its lease route reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseView {
    /// The lease's id. It is not the handle.
    pub id: String,
    /// The secret it was issued from.
    pub secret: String,
    /// The identity holding it.
    pub holder: String,
    /// How, when and by whom it ended, when it has.
    pub end: Option<LeaseEnd>,
    /// Where the provider's part of its revocation stands.
    pub upstream: UpstreamRevocation,
}

impl LeaseView {
    /// Whether it still issues: `live` until it ends, `stopped` after.
    pub fn issuing(&self) -> &'static str {
        if self.end.is_some() {
            "stopped"
        } else {
            "live"
        }
    }

    /// Its provider state as the route reads it: `not_asked`; `pending`
    /// until the system behind confirms; `confirmed` once it has.
    pub fn upstream_label(&self) -> &'static str {
        match self.upstream {
            UpstreamRevocation::NotAsked => "not_asked",
            UpstreamRevocation::Unconfirmed(_) => "pending",
            UpstreamRevocation::Confirmed => "confirmed",
        }
    }

    /// The JSON body the lease route answers.
    pub fn body(&self) -> Value {
        let end = self.end.as_ref();
        json!({
            "lease": self.id,
            "secret": self.secret,
            "holder": self.holder,
            "issuing": self.issuing(),
            "ended_by": end.map(|end| end.way.label()),
            "ended_at_ms": end.and_then(|end| end.at_ms),
            "ended_by_identity": end.and_then(|end| end.by.clone()),
            "upstream": self.upstream_label(),
        })
    }
}

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
    /// when the access seam lets `identity` discover it, as its holder or the
    /// person it is acted for. Any other is left out, as one never issued.
    pub fn held_by(&self, identity: &str, holder: &str) -> Vec<HeldHandle> {
        self.handles
            .values()
            .filter(|record| record.identity == holder)
            .filter(|record| self.discovers_record(identity, &record.id))
            .map(|record| HeldHandle {
                id: record.id.clone(),
                secret: record.secret.clone(),
                max_uses: record.max_uses,
                used: record.used,
                not_after_ms: record.not_after_ms,
                dropped: self.line_dropped(&record.id),
                ended: record.ended.clone(),
                upstream: record.upstream.state().clone(),
                spend_cap: record.spend_cap,
                settled: record.settled,
                parent: record.parent.clone(),
            })
            .collect()
    }

    /// The lease `id` as `identity` may read it: only its holder and the
    /// person it is acted for may.
    ///
    /// # Errors
    ///
    /// `NotFound` for a lease never issued or one `identity` may not
    /// discover; the two answer alike and name nothing.
    pub fn lease_view(&self, identity: &str, id: &HandleId) -> Result<LeaseView, LeaseRefusal> {
        if !self.discovers_lease(identity, id) {
            return Err(LeaseRefusal::NotFound);
        }
        let record = self
            .handles
            .get(id.as_str())
            .ok_or(LeaseRefusal::NotFound)?;
        Ok(LeaseView {
            id: record.id.clone(),
            secret: record.secret.clone(),
            holder: record.identity.clone(),
            end: self.lease_end(&record.id),
            upstream: record.upstream.state().clone(),
        })
    }
}
