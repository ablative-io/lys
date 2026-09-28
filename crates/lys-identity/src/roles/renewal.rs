//! Renewing a holding: a new grant of the holding with a new end date, as a
//! recorded act.
//!
//! A renewal is made by the person the holder answers to or an owner of the
//! role's project, at or before the holding's end date; anyone else is
//! refused `ROLE_RENEW_REFUSED`, a holding with no end date `no_end_date`,
//! and one whose end date has passed `holding_lapsed`. A renewal made at
//! exactly the end date is admitted: it is made before the end has passed.
//! It makes the holding's grants anew through the grant admission, each with
//! the new end date and the holder's responsible person, from the role's
//! current version's templates under
//! [`MovePolicy::MoveAtNextRenewal`] and from the held version's under
//! [`MovePolicy::DeliberateOnly`], withdraws the grants they replace, and
//! commits one holding renewed event naming the actor, its capacity, the
//! holding, the version it lands on and the new end date.
//!
//! Nothing but this act renews a holding: no clock, role edit, move, policy
//! change or retry does, and a holding nobody renews lapses at its end date
//! on the version it held.

use lys_log_store::LeafStore;

use super::check::Check;
use super::error::RoleError;
use super::events::{RoleChange, RoleEvent};
use super::holding::{
    Acting, Roles, commit_copies, end_after, judge_withdrawals, plan_copies, withdraw,
};
use super::types::{HoldingId, MovePolicy};
use crate::grants::RelationshipStore;
use crate::id::IdentityId;
use crate::operation::OperationId;

/// A request to renew a holding with a new end date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renew {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The holding.
    pub holding: HoldingId,
    /// Its new end date.
    pub ends_at: u64,
}

impl Roles {
    /// Renew a holding as new grants with a new end date.
    pub fn renew<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &Renew,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        let held = self.book().holding_or_refuse(request.holding)?.clone();
        let capacity =
            match acting.admit(request.actor, Check::Renew, &held.project, Some(held.holder)) {
                Ok(capacity) => capacity,
                Err(RoleError::NotPermitted { .. }) => {
                    return Err(RoleError::RenewRefused {
                        actor: request.actor.to_string(),
                        holding: held.id.to_string(),
                    });
                }
                Err(error) => return Err(error),
            };
        let Some(ended_at) = held.ends_at else {
            return Err(RoleError::NoEndDate {
                holding: held.id.to_string(),
            });
        };
        if acting.at > ended_at {
            return Err(RoleError::HoldingLapsed {
                holding: held.id.to_string(),
                ended_at,
            });
        }
        end_after(Some(request.ends_at), acting.at)?;
        let role = self.book().role_or_refuse(held.role)?;
        let version = match held.policy {
            MovePolicy::MoveAtNextRenewal => role.current().number(),
            MovePolicy::DeliberateOnly => held.version,
        };
        let templates = role.version_or_refuse(version)?.templates().to_vec();
        let requests = plan_copies(
            acting,
            request.actor,
            held.holder,
            &templates,
            Some(request.ends_at),
            Check::Renew,
        )?;
        judge_withdrawals(acting, request.actor, &held.grants)?;
        let grants = commit_copies(acting, &requests)?;
        let reason = format!(
            "holding {} renewed at version {version} until {}",
            held.id, request.ends_at
        );
        withdraw(acting, request.actor, &held.grants, &reason)?;
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::HoldingRenewed {
                holding: held.id,
                version,
                ends_at: request.ends_at,
                grants,
                replaced: held.grants,
            },
        )?)
    }
}
