//! Derived handles and the counts a line of handles shares. A holder may
//! pass its access to another identity as a derived handle only when it
//! owns the secret or holds the right to lend it, and the recipient is
//! itself permitted to use it. A derived handle stays inside its live
//! ancestry: every use counts against each handle above it, every spend
//! settles against each, and a dropped or expired ancestor ends it. A
//! handle issued on its own is no one's descendant, so dropping another
//! handle never touches it.

use std::collections::BTreeMap;

use crate::audit::AuditKind;
use crate::encoding::hex;
use crate::error::{LendingRefusal, SecretsError};
use crate::handle::{HandleId, HandleToken, Holder, IssuedHandle, Presentation};
use crate::permission::PermissionCheck;

use super::inflight::CANCELLED_AT_BOUNDARY;
use super::owner::{Admission, Call, OwnerChanged, RECIPIENTS};
use super::{Broker, HandleRecord};

/// The deepest a line of derived handles goes.
const MAX_DEPTH: usize = 16;

/// `id` and every handle above it, nearest first.
pub(super) fn chain(handles: &BTreeMap<String, HandleRecord>, id: &str) -> Vec<String> {
    let mut line = Vec::new();
    let mut at = Some(id.to_owned());
    while let Some(current) = at {
        if line.len() >= MAX_DEPTH || line.contains(&current) {
            break;
        }
        at = handles
            .get(&current)
            .and_then(|record| record.parent.clone());
        line.push(current);
    }
    line
}

/// Counts an admitted use of `id` on it and on every handle above it.
pub(super) fn admitted(
    handles: &mut BTreeMap<String, HandleRecord>,
    id: &str,
    (operation, mark): (&str, &str),
    reserved: u64,
) {
    for at in chain(handles, id) {
        if let Some(record) = handles.get_mut(&at) {
            record.used = record.used.saturating_add(1);
            record.open.insert(operation.to_owned(), reserved);
            if at == id {
                record.operations.insert(
                    operation.to_owned(),
                    ("admitted".to_owned(), mark.to_owned()),
                );
            }
        }
    }
}

/// Settles `operation` of `id` on it and on every handle above it; a call
/// cancelled at the forward boundary gives its use back.
pub(super) fn settled(
    handles: &mut BTreeMap<String, HandleRecord>,
    id: &str,
    operation: &str,
    outcome: &str,
    spend: u64,
) {
    for at in chain(handles, id) {
        if let Some(record) = handles.get_mut(&at) {
            record.open.remove(operation);
            record.settled = record.settled.saturating_add(spend);
            if outcome == CANCELLED_AT_BOUNDARY {
                record.used = record.used.saturating_sub(1);
            }
            if at == id {
                if let Some(entry) = record.operations.get_mut(operation) {
                    outcome.clone_into(&mut entry.0);
                }
            }
        }
    }
}

impl<P: PermissionCheck> Broker<P> {
    /// Whether every handle above `record` still stands and, when a use
    /// reserving `reserve` is asked, has room for it.
    pub(super) fn ancestry_admits(
        &self,
        record: &HandleRecord,
        reserve: Option<u64>,
    ) -> Result<(), SecretsError> {
        let now = (self.clock)();
        for at in chain(&self.handles, &record.id).into_iter().skip(1) {
            let Some(above) = self.handles.get(&at) else {
                return Err(SecretsError::HandleDropped { handle: at });
            };
            if above.dropped {
                return Err(SecretsError::HandleDropped { handle: at });
            }
            if now > above.not_after_ms {
                return Err(SecretsError::LeaseWindowClosed { handle: at });
            }
            let Some(reserve) = reserve else {
                continue;
            };
            if above.used >= above.max_uses {
                return Err(SecretsError::LeaseExhausted { handle: at });
            }
            if let Some(cap) = above.spend_cap {
                if reserve == 0 {
                    return Err(SecretsError::ReservationMissing { handle: at });
                }
                let held = above
                    .open
                    .values()
                    .fold(above.settled, |sum, open| sum.saturating_add(*open));
                let left = cap.saturating_sub(held);
                if reserve > left {
                    return Err(SecretsError::SpendCapReached {
                        handle: at,
                        cap,
                        left,
                        asked: reserve,
                    });
                }
            }
        }
        Ok(())
    }

    /// Whether the recipient policy of `secret` admits `recipient`.
    pub(super) fn recipient_admitted(
        &self,
        recipient: &str,
        secret: &str,
    ) -> Result<(), SecretsError> {
        if self.store.recipients(secret).admits(recipient) {
            return Ok(());
        }
        Err(SecretsError::from(LendingRefusal::RecipientRefused {
            recipient: recipient.to_owned(),
            secret: secret.to_owned(),
        }))
    }

    /// Sets the recipient policy of `secret`, as its owner, from the
    /// owner's own command. The change carries no operation id.
    ///
    /// # Errors
    ///
    /// `LendingNotPermitted` when `owner` is not the secret's owner,
    /// `SecretUnknown`, and the store's and audit log's refusals.
    pub fn set_recipients(
        &mut self,
        owner: &str,
        secret: &str,
        policy: crate::store::Recipients,
    ) -> Result<(), SecretsError> {
        self.owns(owner, secret)?;
        self.apply_recipients(owner, secret, policy, None, None)
    }

    /// Sets the recipient policy of `secret`, as its owner, under the
    /// operation id `operation`, asked through the screen service `via` when
    /// one carried the owner's word; the audit line names both. An id
    /// already applied to the secret with this same change answers the
    /// outcome recorded then, and applies nothing.
    ///
    /// # Errors
    ///
    /// As `set_recipients`, `OperationMissing` for no operation id or one of
    /// the wrong shape, and `OperationReused` for an id already applied to
    /// the secret with another change.
    pub fn set_recipients_via(
        &mut self,
        owner: &str,
        secret: &str,
        policy: crate::store::Recipients,
        via: Option<&str>,
        operation: Option<&str>,
    ) -> Result<OwnerChanged, SecretsError> {
        self.owns(owner, secret)?;
        let change = format!("{RECIPIENTS}{}", policy.label());
        match self.owner_admission(secret, operation, &change)? {
            Admission::Repeated(outcome) => Ok(OwnerChanged::Repeated { outcome }),
            Admission::Fresh(call) => {
                self.apply_recipients(owner, secret, policy, via, Some(&call))?;
                Ok(OwnerChanged::Applied)
            }
        }
    }

    fn apply_recipients(
        &mut self,
        owner: &str,
        secret: &str,
        policy: crate::store::Recipients,
        via: Option<&str>,
        call: Option<&Call>,
    ) -> Result<(), SecretsError> {
        self.store.set_recipients(secret, policy)?;
        let outcome = super::scope::with_via(format!("{RECIPIENTS}{}", policy.label()), via);
        self.record_owner_change(owner, secret, &outcome, call)
    }

    /// Whether `handle` or any handle above it was dropped.
    pub(super) fn line_dropped(&self, handle: &str) -> bool {
        let line = chain(&self.handles, handle);
        line.is_empty()
            || line
                .iter()
                .any(|at| self.handles.get(at).is_none_or(|record| record.dropped))
    }

    /// Derives a handle on the same secret for `child` from the handle
    /// `token`, presented by its holder: at most the uses left above it,
    /// ending no later than it, and with a spend cap within what is left.
    ///
    /// # Errors
    ///
    /// The presentation and lease refusals of a use, `LendingNotPermitted`
    /// when the holder neither owns the secret nor holds the right to lend
    /// it, `PermissionDenied` when `child` may not use it, and
    /// `BeyondAncestry` when a bound reaches past the handle above, and
    /// `LendingTooDeep` below the deepest line the broker counts.
    pub fn derive(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        child: &Holder,
        (max_uses, not_after_ms): (u64, i64),
        spend_cap: Option<u64>,
    ) -> Result<IssuedHandle, SecretsError> {
        let parent = self.presented(token, presentation)?;
        self.live(parent, presentation)?;
        self.permitted(parent)?;
        self.ancestry_admits(parent, None)?;
        let depth = chain(&self.handles, &parent.id).len();
        if depth >= MAX_DEPTH {
            return Err(SecretsError::from(LendingRefusal::TooDeep {
                handle: parent.id.clone(),
                depth,
            }));
        }
        let owner = self
            .store
            .entry(&parent.secret)
            .is_some_and(|entry| entry.owner == parent.identity);
        if !owner
            && self
                .permissions
                .may_lend(&parent.identity, &parent.secret)
                .is_err()
        {
            return Err(SecretsError::from(LendingRefusal::NotPermitted {
                holder: parent.identity.clone(),
                secret: parent.secret.clone(),
            }));
        }
        self.recipient_admitted(&child.identity, &parent.secret)?;
        if let Err(reason) = self.within_scope(&child.identity, &parent.secret) {
            return Err(SecretsError::PermissionDenied {
                holder: child.identity.clone(),
                secret: parent.secret.clone(),
                reason,
            });
        }
        if let Err(denied) = self.permissions.may_use(&child.identity, &parent.secret) {
            return Err(SecretsError::PermissionDenied {
                holder: child.identity.clone(),
                secret: parent.secret.clone(),
                reason: denied.reason,
            });
        }
        let uses_left = parent.max_uses.saturating_sub(parent.used);
        let spend_left = parent.spend_cap.map(|cap| {
            cap.saturating_sub(
                parent
                    .open
                    .values()
                    .fold(parent.settled, |sum, open| sum.saturating_add(*open)),
            )
        });
        let within_spend = match (parent.spend_cap, spend_cap, spend_left) {
            (None, _, _) => true,
            (Some(_), Some(asked), Some(left)) => asked <= left,
            (Some(_), _, _) => false,
        };
        if max_uses == 0
            || max_uses > uses_left
            || not_after_ms > parent.not_after_ms
            || not_after_ms <= (self.clock)()
            || !within_spend
        {
            return Err(SecretsError::from(LendingRefusal::BeyondAncestry {
                handle: parent.id.clone(),
            }));
        }
        let (parent_id, secret) = (parent.id.clone(), parent.secret.clone());
        let id = HandleId::generate()?;
        let child_token = HandleToken::generate()?;
        let record = HandleRecord {
            id: id.as_str().to_owned(),
            digest: hex(&child_token.digest()),
            identity: child.identity.clone(),
            holder_key: hex(&child.key),
            secret: secret.clone(),
            max_uses,
            not_after_ms,
            used: 0,
            dropped: false,
            operations: BTreeMap::new(),
            spend_cap,
            settled: 0,
            open: BTreeMap::new(),
            parent: Some(parent_id.clone()),
        };
        let outcome = format!("derived from {parent_id}");
        self.record(
            AuditKind::Issue,
            (Some(id.as_str()), Some(&child.identity), Some(&secret)),
            None,
            Some(0),
            &outcome,
        )?;
        self.handles.insert(record.id.clone(), record);
        self.write_handles()?;
        Ok(IssuedHandle {
            id,
            token: child_token,
        })
    }
}
