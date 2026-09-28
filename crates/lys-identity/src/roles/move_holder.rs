//! Moving a holding to a newer version, by a deliberate act that shows what
//! changes first.
//!
//! A preview names the grants a move adds and the grants it removes, from
//! the difference between the held version's templates and the target
//! version's, each in the grant fields relation, resource, actions,
//! pass-on, window and responsible, and commits nothing. A move is taken
//! only by confirming a preview that is still the move the holding would
//! take, with a timing ([`Timing::NextStart`] when none is given), and only
//! for the person the holder answers to or an owner of the role's project.
//! It makes the added grants through the grant admission with the holding's
//! end date and the holder's responsible person, withdraws the removed ones,
//! and commits one holding moved event naming the actor, its capacity, the
//! from and to versions, the timing and the grants added and removed. It
//! never changes the holding's end date or its policy.
//!
//! A move [`Timing::Now`] stops every open session of the agent first, and
//! only the configured administrator may take it; anyone else is refused
//! `now_not_operator` and may take the move at the next start. The session
//! stop it asks for is not yet built, so the administrator's move now is
//! refused `now_unavailable` and nothing is committed.

use std::collections::BTreeSet;

use lys_log_store::LeafStore;

use super::check::{Check, responsible_for};
use super::error::RoleError;
use super::events::{RoleChange, RoleEvent};
use super::holders::difference;
use super::holding::{Acting, Roles, commit_copies, judge_withdrawals, plan_copies, withdraw};
use super::types::{HoldingId, Template, Timing};
use crate::grants::{Action, PassOn, Relation, RelationshipStore, Resource, Window};
use crate::id::{IdentityId, PersonId};
use crate::operation::OperationId;
use crate::projection::Projection;

/// One grant a move adds or removes, in the grant fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantView {
    /// The relation.
    pub relation: Relation,
    /// The resource.
    pub resource: Resource,
    /// The actions.
    pub actions: BTreeSet<Action>,
    /// The pass-on.
    pub pass_on: PassOn,
    /// The window, ending at the holding's end date.
    pub window: Window,
    /// The holder's responsible person.
    pub responsible: PersonId,
}

/// What a move would add and remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// The holding.
    pub holding: HoldingId,
    /// The version it holds.
    pub from: u64,
    /// The version it would move to.
    pub to: u64,
    /// The grants the move would add.
    pub added: Vec<GrantView>,
    /// The grants the move would remove.
    pub removed: Vec<GrantView>,
}

/// A request to take a move, confirming its preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmMove {
    /// The caller's operation id.
    pub operation: OperationId,
    /// The authenticated actor.
    pub actor: IdentityId,
    /// The preview confirmed.
    pub preview: Preview,
    /// When the move takes effect; at the next start when none is given.
    pub timing: Option<Timing>,
}

fn view(
    template: &Template,
    ends_at: Option<u64>,
    responsible: PersonId,
) -> Result<GrantView, RoleError> {
    Ok(GrantView {
        relation: template.relation().clone(),
        resource: template.resource().clone(),
        actions: template.actions().clone(),
        pass_on: template.pass_on().clone(),
        window: Window::new(template.window().starts_at(), ends_at)?,
        responsible,
    })
}

impl Roles {
    /// What moving `holding` to version `to` would add and remove. Commits nothing.
    pub fn preview(
        &self,
        directory: &Projection,
        holding: HoldingId,
        to: u64,
    ) -> Result<Preview, RoleError> {
        let held = self.book().holding_or_refuse(holding)?;
        if to <= held.version {
            return Err(RoleError::NotNewer {
                held: held.version,
                target: to,
            });
        }
        let role = self.book().role_or_refuse(held.role)?;
        let from = role.version_or_refuse(held.version)?;
        let target = role.version_or_refuse(to)?;
        let responsible = responsible_for(directory, held.holder, Check::Move)?;
        let (added, dropped) = difference(from, target);
        Ok(Preview {
            holding,
            from: held.version,
            to,
            added: added
                .into_iter()
                .map(|template| view(template, held.ends_at, responsible))
                .collect::<Result<_, _>>()?,
            removed: dropped
                .iter()
                .filter_map(|index| from.templates().get(*index))
                .map(|template| view(template, held.ends_at, responsible))
                .collect::<Result<_, _>>()?,
        })
    }

    /// Take the move a confirmed preview shows.
    pub fn confirm_move<S: LeafStore, R: RelationshipStore>(
        &mut self,
        acting: &mut Acting<'_, S, R>,
        request: &ConfirmMove,
    ) -> Result<RoleEvent, RoleError> {
        self.fresh(request.operation)?;
        let preview = &request.preview;
        let held = self.book().holding_or_refuse(preview.holding)?.clone();
        if preview.to <= held.version {
            return Err(RoleError::NotNewer {
                held: held.version,
                target: preview.to,
            });
        }
        let capacity = acting.admit(request.actor, Check::Move, &held.project, Some(held.holder))?;
        let timing = request.timing.unwrap_or(Timing::NextStart);
        if timing == Timing::Now {
            return Err(if acting.is_administrator(request.actor) {
                RoleError::NowUnavailable
            } else {
                RoleError::NowNotOperator {
                    actor: request.actor.to_string(),
                }
            });
        }
        if let Some(ended_at) = held.ends_at
            && held.lapsed(acting.at)
        {
            return Err(RoleError::HoldingLapsed {
                holding: held.id.to_string(),
                ended_at,
            });
        }
        if &self.preview(acting.directory, preview.holding, preview.to)? != preview {
            return Err(RoleError::PreviewStale);
        }
        let role = self.book().role_or_refuse(held.role)?;
        let from = role.version_or_refuse(held.version)?;
        let target = role.version_or_refuse(preview.to)?;
        let (added, dropped) = difference(from, target);
        let added: Vec<Template> = added.into_iter().cloned().collect();
        let removed: Vec<_> = dropped
            .iter()
            .filter_map(|index| held.grants.get(*index).copied())
            .collect();
        let requests = plan_copies(
            acting,
            request.actor,
            held.holder,
            &added,
            held.ends_at,
            Check::Move,
        )?;
        judge_withdrawals(acting, request.actor, &removed)?;
        let made = commit_copies(acting, &requests)?;
        let reason = format!(
            "holding {} moved from version {} to version {} of role {}",
            held.id, held.version, preview.to, held.role
        );
        withdraw(acting, request.actor, &removed, &reason)?;
        self.commit(RoleEvent::new(
            request.operation,
            request.actor,
            capacity,
            acting.at,
            RoleChange::HoldingMoved {
                holding: held.id,
                from: held.version,
                to: preview.to,
                timing,
                added: made,
                removed,
            },
        )?)
    }
}
