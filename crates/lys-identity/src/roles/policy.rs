//! Each holding's move policy and what it means, and changing a holding's
//! policy or a role's default by a recorded act.
//!
//! A holding with an end date under [`MovePolicy::MoveAtNextRenewal`]
//! answers its end date as the date it will move, the words
//! [`NEXT_RENEWAL_WORDS`], and who can stop the move: the person the holder
//! answers to and each holder of the owner relation on the role's project.
//! A holding under [`MovePolicy::DeliberateOnly`], and every holding with no
//! end date whatever the role's default, answers only the words
//! [`DELIBERATE_WORDS`] and no date. A role's answer names its default and
//! marks each holding with an end date whose own policy differs from it; a
//! holding with no end date is never marked, since it has no renewal to
//! move at.
//!
//! A holding's policy is changed by the person the holder answers to or an
//! owner of the role's project, and a role's default by an owner of its
//! project; each is one recorded event, and neither changes any holding's
//! version, grants or end date. A new default applies only to holdings
//! granted after it.

use lys_log_store::LeafStore;

use super::check::{Check, responsible_for};
use super::error::RoleError;
use super::events::{RoleChange, RoleEvent};
use super::holding::{Acting, Roles};
use super::types::{Holding, HoldingId, MovePolicy, Role, RoleId};
use crate::grants::RelationshipStore;
use crate::id::{AgentId, IdentityId, PersonId};
use crate::operation::OperationId;

/// The words a holding that moves at its next renewal answers.
pub const NEXT_RENEWAL_WORDS: &str = "Its next renewal moves it to the current version";

/// The words a holding that moves only by a deliberate act answers.
pub const DELIBERATE_WORDS: &str = "Moves only by a deliberate act";

/// One holding's policy and what it means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingPolicy {
    /// The holding.
    pub holding: HoldingId,
    /// The agent holding it.
    pub agent: AgentId,
    /// The version it holds.
    pub version: u64,
    /// Its policy.
    pub policy: MovePolicy,
    /// Its end date, or none.
    pub ends_at: Option<u64>,
    /// The date it will move, or none.
    pub moves_at: Option<u64>,
    /// What its policy means.
    pub words: &'static str,
    /// Who can stop the move: none for a holding that does not move by itself.
    pub who_can_stop: Vec<PersonId>,
    /// Whether it has an end date and its policy differs from the role's default.
    pub differs_from_default: bool,
}

/// A role's default policy and each of its unlapsed holdings' policies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolePolicies {
    /// The role.
    pub role: RoleId,
    /// Its default move policy.
    pub default_policy: MovePolicy,
    /// Its unlapsed holdings, in the order of their ids.
    pub holdings: Vec<HoldingPolicy>,
}

/// A request to change one holding's move policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangePolicy {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The holding.
    pub holding: HoldingId,
    /// The policy asked for.
    pub policy: MovePolicy,
}

/// A request to change a role's default move policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeDefault {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The role.
    pub role: RoleId,
    /// The default asked for.
    pub policy: MovePolicy,
}

fn describe<S: LeafStore, R: RelationshipStore>(
    acting: &Acting<'_, S, R>,
    role: &Role,
    held: &Holding,
) -> Result<HoldingPolicy, RoleError> {
    let moves = held.policy == MovePolicy::MoveAtNextRenewal && held.ends_at.is_some();
    let mut who_can_stop = Vec::new();
    if moves {
        let facts = acting.facts();
        who_can_stop.push(responsible_for(
            acting.directory,
            held.holder,
            Check::ChangePolicy,
        )?);
        let owners = acting
            .check
            .owners(&facts, Check::ChangePolicy, &held.project, acting.at)?;
        for owner in owners {
            if !who_can_stop.contains(&owner) {
                who_can_stop.push(owner);
            }
        }
    }
    Ok(HoldingPolicy {
        holding: held.id,
        agent: held.holder,
        version: held.version,
        policy: held.policy,
        ends_at: held.ends_at,
        moves_at: if moves { held.ends_at } else { None },
        words: if moves {
            NEXT_RENEWAL_WORDS
        } else {
            DELIBERATE_WORDS
        },
        who_can_stop,
        differs_from_default: held.ends_at.is_some() && held.policy != role.default_policy(),
    })
}

impl Roles {
    /// One holding's policy and what it means.
    pub fn holding_policy<S: LeafStore, R: RelationshipStore>(
        &self,
        acting: &Acting<'_, S, R>,
        holding: HoldingId,
    ) -> Result<HoldingPolicy, RoleError> {
        let held = self.book().holding_or_refuse(holding)?;
        let role = self.book().role_or_refuse(held.role)?;
        describe(acting, role, held)
    }

    /// A role's default and each of its unlapsed holdings' policies.
    pub fn role_policies<S: LeafStore, R: RelationshipStore>(
        &self,
        acting: &Acting<'_, S, R>,
        role: RoleId,
    ) -> Result<RolePolicies, RoleError> {
        let found = self.book().role_or_refuse(role)?;
        let holdings = self
            .book()
            .holdings()
            .filter(|held| held.role == role && !held.lapsed(acting.at))
            .map(|held| describe(acting, found, held))
            .collect::<Result<_, _>>()?;
        Ok(RolePolicies {
            role,
            default_policy: found.default_policy(),
            holdings,
        })
    }

    /// Change one holding's move policy, moving nothing.
    pub fn change_policy<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &ChangePolicy,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        let held = self.book().holding_or_refuse(request.holding)?;
        let (before, ends_at, project, holder) =
            (held.policy, held.ends_at, held.project.clone(), held.holder);
        let capacity = acting.admit(request.actor, Check::ChangePolicy, &project, Some(holder))?;
        if request.policy == MovePolicy::MoveAtNextRenewal && ends_at.is_none() {
            return Err(RoleError::NoEndDate {
                holding: request.holding.to_string(),
            });
        }
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::HoldingPolicyChanged {
                holding: request.holding,
                before,
                after: request.policy,
            },
        )?)
    }

    /// Change a role's default move policy, for holdings granted after it only.
    pub fn change_default<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &ChangeDefault,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        let role = self.book().role_or_refuse(request.role)?;
        let (before, project) = (role.default_policy(), role.project().clone());
        let capacity = acting.admit(request.actor, Check::ChangeDefault, &project, None)?;
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::DefaultPolicyChanged {
                role: request.role,
                before,
                after: request.policy,
            },
        )?)
    }
}
