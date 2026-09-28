//! A use in two acts. Admission checks the handle, the presentation, the
//! lease and the permission, appends the use line (with the reservation on
//! a capped lease) and hands back a ticket holding the credential.
//! Settlement appends the call's outcome and what it spent, and closes the
//! reservation. A call admitted and never settled when the broker stops is
//! settled at start as `outcome_unknown`, at its full reservation, and is
//! never forwarded again.

use crate::audit::AuditKind;
use crate::encoding::hex;
use crate::error::SecretsError;
use crate::handle::{HandleToken, Presentation};
use crate::permission::PermissionCheck;
use crate::secret::Secret;
use crate::store::EntryClass;

use super::admit::Admission;
use super::checked::Checked;
use super::{Broker, UseError, Used};

const COMPLETED: &str = "completed";
const UPSTREAM_FAILED: &str = "upstream_failed";
const COMPLETED_UNMETERED: &str = "completed_unmetered";
const COMPLETED_AFTER_DROP: &str = "completed_after_drop";
const OUTCOME_UNKNOWN: &str = "outcome_unknown";

/// An admitted call: the credential for its one forward, and what it
/// reserved. Settle it with [`Broker::settle`] once the call has an outcome.
#[derive(Debug)]
pub struct Ticket {
    pub(super) handle: String,
    pub(super) identity: String,
    pub(super) secret: String,
    pub(super) operation: String,
    pub(super) mark: String,
    pub(super) reserved: Option<u64>,
    /// The store entry opened: the secret's current account.
    pub(super) entry: String,
    /// The entry's class.
    pub(super) class: EntryClass,
    pub(super) uses_left: u64,
    pub(super) credential: Secret,
}

impl Ticket {
    /// The credential, for the one forward this ticket admits.
    pub fn credential(&self) -> &Secret {
        &self.credential
    }

    /// Uses left on the lease after this one.
    pub fn uses_left(&self) -> u64 {
        self.uses_left
    }

    /// The reservation, on a capped lease.
    pub fn reserved(&self) -> Option<u64> {
        self.reserved
    }
}

/// How a forwarded call ended, for [`Broker::settle_checked`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settled {
    /// The upstream answered and reported what the call spent.
    Spent(u64),
    /// The upstream answered without reporting what it spent.
    Unmetered,
    /// The upstream never answered, with what the call may have spent.
    Failed(u64),
}

/// What admission answers.
#[derive(Debug)]
pub enum Admitted {
    /// A fresh call, to forward and then settle.
    Fresh(Ticket),
    /// The same operation was presented before; its recorded outcome is
    /// answered and nothing is forwarded again.
    Retried {
        /// The recorded outcome.
        outcome: String,
    },
}

impl<P: PermissionCheck> Broker<P> {
    /// Swaps `token` for its credential inside `forward` and settles the
    /// call at once, spending nothing. For a lease with a spend cap use
    /// [`Broker::admit_use`] and [`Broker::settle`].
    ///
    /// # Errors
    ///
    /// As [`Broker::admit_use`] and [`Broker::settle`].
    pub fn use_handle<R>(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        forward: impl FnOnce(&Secret) -> R,
    ) -> Result<Used<R>, UseError> {
        match self.admit_use(token, presentation, 0)? {
            Admitted::Retried { outcome } => Ok(Used::Retried { outcome }),
            Admitted::Fresh(ticket) => {
                let ticket = self.at_forward_boundary(ticket)?;
                let answer = forward(ticket.credential());
                let uses_left = ticket.uses_left;
                self.settle(ticket, 0)?;
                Ok(Used::Forwarded { answer, uses_left })
            }
        }
    }

    /// Admits one call of `token`, reserving `reserve` against a capped
    /// lease. The use counts from its audit line.
    ///
    /// # Errors
    ///
    /// One named refusal per attempt, each recorded in the audit log.
    pub fn admit_use(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        reserve: u64,
    ) -> Result<Admitted, UseError> {
        self.admit_use_as(token, presentation, reserve, None)
    }

    /// Admits one call of `token` as [`Broker::admit_use`] does, taking the
    /// permission source's answers from `checked`, asked before the broker
    /// was taken (see [`Broker::asks_for`]), where it holds them.
    ///
    /// # Errors
    ///
    /// As [`Broker::admit_use`].
    pub fn admit_use_checked(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        reserve: u64,
        checked: &Checked,
    ) -> Result<Admitted, UseError> {
        self.admit_use_as(token, presentation, reserve, Some(checked))
    }

    fn admit_use_as(
        &mut self,
        token: &HandleToken,
        presentation: &Presentation,
        reserve: u64,
        checked: Option<&Checked>,
    ) -> Result<Admitted, UseError> {
        let operation = hex(&presentation.operation_id);
        let mark = self.request_mark(&presentation.request)?;
        let call = Some((operation.as_str(), mark.as_str()));
        let record = self.find(token);
        let asked = (reserve, checked);
        let admitted = self.admit(record, presentation, (&operation, &mark), asked);
        let admission = match admitted {
            Ok(admission) => admission,
            Err(refusal) => {
                let found = record.map(|record| {
                    (
                        record.id.clone(),
                        record.identity.clone(),
                        record.secret.clone(),
                    )
                });
                let subject = found
                    .as_ref()
                    .map_or((None, None, None), |(id, who, what)| {
                        (Some(id.as_str()), Some(who.as_str()), Some(what.as_str()))
                    });
                self.record(AuditKind::Use, subject, call, None, refusal.name())?;
                return Err(refusal);
            }
        };
        let (id, identity, secret, uses_left, used, reserved) = match admission {
            Admission::Retry(outcome) => return Ok(Admitted::Retried { outcome }),
            Admission::Fresh {
                id,
                identity,
                secret,
                uses_left,
                used,
                reserved,
            } => (id, identity, secret, uses_left, used, reserved),
        };
        let (entry, account) = match self.store.current_entry(&secret) {
            Ok(current) => current,
            Err(refusal) => {
                let subject = (
                    Some(id.as_str()),
                    Some(identity.as_str()),
                    Some(secret.as_str()),
                );
                self.record(AuditKind::Use, subject, call, None, refusal.name())?;
                return Err(refusal);
            }
        };
        let used_as = format!("{secret}@{account}");
        let subject = (
            Some(id.as_str()),
            Some(identity.as_str()),
            Some(used_as.as_str()),
        );
        let mut line = self.line(AuditKind::Use, subject, call, Some(used), "admitted");
        line.spend = reserved;
        self.append_unanchored(&line)?;
        super::lineage::admitted(
            &mut self.handles,
            &id,
            (&operation, &mark),
            reserved.unwrap_or(0),
        );
        let opened = match self.store.entry(&entry).map(|view| view.class) {
            Some(class) => self
                .store
                .open_for_use(&self.store_key, &entry, class)
                .map(|credential| (credential, class)),
            None => Err(SecretsError::SecretUnknown {
                name: entry.clone(),
            }),
        };
        let (credential, class) = match opened {
            Ok(opened) => opened,
            Err(refusal) => {
                let spend = reserved.map(|_reserved| 0);
                self.close(
                    &id,
                    (&identity, &secret),
                    (&operation, &mark),
                    spend,
                    refusal.name(),
                )?;
                return Err(refusal);
            }
        };
        Ok(Admitted::Fresh(Ticket {
            handle: id,
            identity,
            secret,
            operation,
            mark,
            reserved,
            entry,
            class,
            uses_left,
            credential,
        }))
    }

    /// Settles an admitted call: records its outcome and what it spent, and
    /// closes its reservation. On a capped lease the spend is what passed
    /// through, even past the reservation, so the next reservation sees it.
    ///
    /// # Errors
    ///
    /// The audit log's refusals.
    pub fn settle(&mut self, ticket: Ticket, spent: u64) -> Result<(), SecretsError> {
        self.settle_as(ticket, (spent, COMPLETED), None)
    }

    /// Settles a call whose upstream answered without reporting what it
    /// spent, as `completed_unmetered`: the full reservation stays counted,
    /// and the line says it was not measured.
    ///
    /// # Errors
    ///
    /// The audit log's refusals.
    pub fn settle_unmetered(&mut self, ticket: Ticket) -> Result<(), SecretsError> {
        let reserved = ticket.reserved.unwrap_or(0);
        self.settle_as(ticket, (reserved, COMPLETED_UNMETERED), None)
    }

    /// Settles a call whose upstream never answered, as `upstream_failed`,
    /// with what it may have spent.
    ///
    /// # Errors
    ///
    /// The audit log's refusals.
    pub fn settle_failed(&mut self, ticket: Ticket, spent: u64) -> Result<(), SecretsError> {
        self.settle_as(ticket, (spent, UPSTREAM_FAILED), None)
    }

    /// Settles a call as [`Broker::settle`], [`Broker::settle_unmetered`]
    /// or [`Broker::settle_failed`] does, as `settled` says, taking the
    /// permission source's answers from `checked`, asked after the call and
    /// before the broker was taken, where it holds them.
    ///
    /// # Errors
    ///
    /// The audit log's refusals.
    pub fn settle_checked(
        &mut self,
        ticket: Ticket,
        settled: Settled,
        checked: &Checked,
    ) -> Result<(), SecretsError> {
        let settlement = match settled {
            Settled::Spent(spent) => (spent, COMPLETED),
            Settled::Unmetered => (ticket.reserved.unwrap_or(0), COMPLETED_UNMETERED),
            Settled::Failed(spent) => (spent, UPSTREAM_FAILED),
        };
        self.settle_as(ticket, settlement, Some(checked))
    }

    fn settle_as(
        &mut self,
        ticket: Ticket,
        (spent, outcome): (u64, &str),
        checked: Option<&Checked>,
    ) -> Result<(), SecretsError> {
        let Ticket {
            handle,
            identity,
            secret,
            operation,
            mark,
            reserved,
            ..
        } = ticket;
        let settled = reserved.map(|_reserved| spent);
        let finished = outcome == COMPLETED || outcome == COMPLETED_UNMETERED;
        let outcome = if finished && self.cut_off(&handle, (&identity, &secret), checked) {
            COMPLETED_AFTER_DROP
        } else {
            outcome
        };
        self.close(
            &handle,
            (&identity, &secret),
            (&operation, &mark),
            settled,
            outcome,
        )
    }

    /// Settles every call the log shows admitted and never settled, as
    /// `outcome_unknown` at its full reservation.
    pub(super) fn settle_unknown_outcomes(&mut self) -> Result<(), SecretsError> {
        let mut unsettled = Vec::new();
        for record in self.handles.values() {
            for (operation, (_outcome, mark)) in &record.operations {
                let Some(reserved) = record.open.get(operation) else {
                    continue;
                };
                unsettled.push((
                    record.id.clone(),
                    (record.identity.clone(), record.secret.clone()),
                    (operation.clone(), mark.clone()),
                    record.spend_cap.map(|_cap| *reserved),
                ));
            }
        }
        for (handle, (identity, secret), (operation, mark), spend) in unsettled {
            self.close(
                &handle,
                (&identity, &secret),
                (&operation, &mark),
                spend,
                OUTCOME_UNKNOWN,
            )?;
        }
        Ok(())
    }

    pub(super) fn close(
        &mut self,
        handle: &str,
        (identity, secret): (&str, &str),
        (operation, mark): (&str, &str),
        spend: Option<u64>,
        outcome: &str,
    ) -> Result<(), SecretsError> {
        let subject = (Some(handle), Some(identity), Some(secret));
        let mut line = self.line(
            AuditKind::Settlement,
            subject,
            Some((operation, mark)),
            None,
            outcome,
        );
        line.spend = spend;
        self.append(&line)?;
        super::lineage::settled(
            &mut self.handles,
            handle,
            operation,
            outcome,
            spend.unwrap_or(0),
        );
        Ok(())
    }
}
