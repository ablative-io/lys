//! The roles and holdings as their committed role events fold them, and who
//! holds a role, answered as a query over the holdings.
//!
//! The book is derived from the role events and from nothing else. A role
//! keeps no member list: who holds it, and on which version, is read from
//! the holding records each time it is asked, grouped by version, and a
//! lapsed holding is not listed as holding the role. An event the book
//! refuses changes nothing: a version that skips a number is refused
//! `version_gap`, one that would replace a committed version
//! `version_immutable`, and a holding outside its role's project
//! `project_mismatch`.

use std::collections::{BTreeMap, BTreeSet};

use super::error::RoleError;
use super::events::{RoleChange, RoleEvent};
use super::holding::Roles;
use super::types::{Holding, HoldingId, MovePolicy, Role, RoleId, Template, Version};
use crate::grants::{GrantId, Resource};
use crate::id::AgentId;
use crate::operation::OperationId;

/// The roles and holdings the committed role events make.
#[derive(Debug, Clone, Default)]
pub struct RoleBook {
    roles: BTreeMap<RoleId, Role>,
    holdings: BTreeMap<HoldingId, Holding>,
    operations: BTreeSet<OperationId>,
}

/// What one admitted event does to the book.
enum Step {
    Open(Role),
    Push(RoleId, Version),
    Title(RoleId, String),
    Default(RoleId, MovePolicy),
    Hold(Holding),
}

/// The templates `to` adds over `from`, in `to`'s order, and the indices of
/// `from`'s templates that `to` drops. Templates are compared whole, in
/// every grant field, so a changed template is one dropped and one added.
pub fn difference<'a>(from: &Version, to: &'a Version) -> (Vec<&'a Template>, Vec<usize>) {
    let added = to
        .templates()
        .iter()
        .filter(|template| !from.templates().contains(template))
        .collect();
    let dropped = from
        .templates()
        .iter()
        .enumerate()
        .filter(|(_, template)| !to.templates().contains(template))
        .map(|(index, _)| index)
        .collect();
    (added, dropped)
}

/// Refuse unless `project` is the one `role` is defined in.
fn same_project(role: &Role, project: &Resource, what: &'static str) -> Result<(), RoleError> {
    if project == role.project() {
        return Ok(());
    }
    Err(RoleError::ProjectMismatch {
        what,
        named: project.to_string(),
        role: role.id().to_string(),
        project: role.project().to_string(),
    })
}

fn mismatch(reason: &'static str) -> RoleError {
    RoleError::GrantsMismatch { reason }
}

/// The grants a holding carries after moving from `from` to `to` with
/// `added` made, and the grants it withdraws.
pub fn moved_grants(
    holding: &Holding,
    from: &Version,
    to: &Version,
    added: &[GrantId],
) -> Result<(Vec<GrantId>, Vec<GrantId>), RoleError> {
    let mut made = added.iter();
    let mut grants = Vec::new();
    for template in to.templates() {
        let grant = match from.templates().iter().position(|held| held == template) {
            Some(index) => holding.grants.get(index),
            None => made.next(),
        };
        grants.push(*grant.ok_or_else(|| {
            mismatch("a move keeps each grant it keeps and makes one for each new template")
        })?);
    }
    if made.next().is_some() {
        return Err(mismatch("a move makes one grant for each new template and no more"));
    }
    let (_, dropped) = difference(from, to);
    let removed = dropped
        .iter()
        .filter_map(|index| holding.grants.get(*index).copied())
        .collect();
    Ok((grants, removed))
}

impl RoleBook {
    /// The role `id`, if it is made.
    pub fn role(&self, id: RoleId) -> Option<&Role> {
        self.roles.get(&id)
    }

    /// The role `id`, or its refusal by name.
    pub fn role_or_refuse(&self, id: RoleId) -> Result<&Role, RoleError> {
        self.role(id).ok_or_else(|| RoleError::RoleUnknown {
            role: id.to_string(),
        })
    }

    /// Every role, in the order of their ids.
    pub fn roles(&self) -> impl Iterator<Item = &Role> {
        self.roles.values()
    }

    /// The holding `id`, if it is granted.
    pub fn holding(&self, id: HoldingId) -> Option<&Holding> {
        self.holdings.get(&id)
    }

    /// The holding `id`, or its refusal by name.
    pub fn holding_or_refuse(&self, id: HoldingId) -> Result<&Holding, RoleError> {
        self.holding(id).ok_or_else(|| RoleError::HoldingUnknown {
            holding: id.to_string(),
        })
    }

    /// Every holding, in the order of their ids.
    pub fn holdings(&self) -> impl Iterator<Item = &Holding> {
        self.holdings.values()
    }

    /// Every holding `agent` has of `role`.
    pub fn held_by(&self, agent: AgentId, role: RoleId) -> impl Iterator<Item = &Holding> {
        self.holdings
            .values()
            .filter(move |holding| holding.holder == agent && holding.role == role)
    }

    /// Whether `operation` names a committed role event.
    pub fn answered(&self, operation: OperationId) -> bool {
        self.operations.contains(&operation)
    }

    /// Refuse `event` by name unless it would apply to the book as it stands.
    pub fn check(&self, event: &RoleEvent) -> Result<(), RoleError> {
        self.step(event).map(drop)
    }

    /// Advance the book by `event`, or refuse it by name and change nothing.
    pub fn apply(&mut self, event: &RoleEvent) -> Result<(), RoleError> {
        let step = self.step(event)?;
        self.operations.insert(event.operation());
        match step {
            Step::Open(role) => {
                self.roles.insert(role.id(), role);
            }
            Step::Push(id, version) => {
                if let Some(role) = self.roles.get_mut(&id) {
                    role.push(version);
                }
            }
            Step::Title(id, title) => {
                if let Some(role) = self.roles.get_mut(&id) {
                    role.set_title(title);
                }
            }
            Step::Default(id, policy) => {
                if let Some(role) = self.roles.get_mut(&id) {
                    role.set_default(policy);
                }
            }
            Step::Hold(holding) => {
                self.holdings.insert(holding.id, holding);
            }
        }
        Ok(())
    }

    fn step(&self, event: &RoleEvent) -> Result<Step, RoleError> {
        if self.answered(event.operation()) {
            return Err(RoleError::OperationReused {
                operation: event.operation().to_string(),
            });
        }
        match event.change() {
            RoleChange::VersionMade {
                role,
                project,
                version,
                templates,
                opening,
            } => {
                let made = Version::new(*version, templates.clone())?;
                let Some(existing) = self.roles.get(role) else {
                    return match opening {
                        Some(opening) if *version == 1 => Ok(Step::Open(Role::open(
                            *role,
                            project.clone(),
                            opening.title.clone(),
                            opening.default_policy,
                            made,
                        ))),
                        _ => Err(RoleError::VersionGap {
                            role: role.to_string(),
                            last: 0,
                            version: *version,
                        }),
                    };
                };
                let last = existing.current().number();
                if *version <= last {
                    return Err(RoleError::VersionImmutable {
                        role: role.to_string(),
                        version: *version,
                    });
                }
                if *version != last + 1 {
                    return Err(RoleError::VersionGap {
                        role: role.to_string(),
                        last,
                        version: *version,
                    });
                }
                if opening.is_some() {
                    return Err(RoleError::EventMalformed {
                        reason: "only version 1 opens a role",
                    });
                }
                same_project(existing, project, "a version")?;
                Ok(Step::Push(*role, made))
            }
            RoleChange::TitleChanged {
                role,
                before,
                after,
            } => {
                let existing = self.role_or_refuse(*role)?;
                if existing.title() != before {
                    return Err(RoleError::EventMalformed {
                        reason: "a title change names the role's title before it",
                    });
                }
                Ok(Step::Title(*role, super::types::title(after)?))
            }
            RoleChange::DefaultPolicyChanged {
                role,
                before,
                after,
            } => {
                if self.role_or_refuse(*role)?.default_policy() != *before {
                    return Err(RoleError::EventMalformed {
                        reason: "a default change names the role's default before it",
                    });
                }
                Ok(Step::Default(*role, *after))
            }
            RoleChange::HoldingGranted(holding) => self.grant_step(holding),
            RoleChange::HoldingMoved {
                holding,
                from,
                to,
                added,
                removed,
                ..
            } => {
                let held = self.holding_or_refuse(*holding)?;
                let role = self.role_or_refuse(held.role)?;
                if *from != held.version {
                    return Err(RoleError::EventMalformed {
                        reason: "a move names the version its holding holds",
                    });
                }
                if to <= from {
                    return Err(RoleError::NotNewer {
                        held: *from,
                        target: *to,
                    });
                }
                let (grants, withdrawn) = moved_grants(
                    held,
                    role.version_or_refuse(*from)?,
                    role.version_or_refuse(*to)?,
                    added,
                )?;
                if &withdrawn != removed {
                    return Err(mismatch(
                        "a move withdraws exactly the grants of the templates it drops",
                    ));
                }
                Ok(Step::Hold(Holding {
                    version: *to,
                    grants,
                    ..held.clone()
                }))
            }
            RoleChange::HoldingRenewed {
                holding,
                version,
                ends_at,
                grants,
                replaced,
            } => {
                let held = self.holding_or_refuse(*holding)?;
                if held.ends_at.is_none() {
                    return Err(RoleError::NoEndDate {
                        holding: holding.to_string(),
                    });
                }
                if &held.grants != replaced {
                    return Err(mismatch("a renewal replaces every grant its holding carries"));
                }
                let landed = self.role_or_refuse(held.role)?.version_or_refuse(*version)?;
                if *version < held.version || grants.len() != landed.templates().len() {
                    return Err(mismatch(
                        "a renewal lands on the held or a newer version with one grant for each template",
                    ));
                }
                let renewed = Holding {
                    version: *version,
                    grants: grants.clone(),
                    ends_at: Some(*ends_at),
                    ..held.clone()
                };
                renewed.check()?;
                Ok(Step::Hold(renewed))
            }
            RoleChange::HoldingPolicyChanged {
                holding,
                before,
                after,
            } => {
                let held = self.holding_or_refuse(*holding)?;
                if held.policy != *before {
                    return Err(RoleError::EventMalformed {
                        reason: "a policy change names the holding's policy before it",
                    });
                }
                let changed = Holding {
                    policy: *after,
                    ..held.clone()
                };
                changed.check()?;
                Ok(Step::Hold(changed))
            }
        }
    }

    fn grant_step(&self, holding: &Holding) -> Result<Step, RoleError> {
        holding.check()?;
        let role = self.role_or_refuse(holding.role)?;
        same_project(role, &holding.project, "a holding")?;
        let version = role.version_or_refuse(holding.version)?;
        if holding.grants.len() != version.templates().len() {
            return Err(mismatch(
                "a holding carries one grant for each template of its version",
            ));
        }
        if self.holdings.contains_key(&holding.id) {
            return Err(RoleError::HoldingExists {
                holding: holding.id.to_string(),
            });
        }
        Ok(Step::Hold(holding.clone()))
    }
}

/// One holder of a role, as the holding records give it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    /// The holding.
    pub holding: HoldingId,
    /// The agent holding the role.
    pub agent: AgentId,
    /// The version it holds.
    pub version: u64,
    /// Its end date, or none.
    pub ends_at: Option<u64>,
    /// Its move policy.
    pub policy: MovePolicy,
}

/// The holders of one version of a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionHolders {
    /// The version's number.
    pub version: u64,
    /// Whether it is the role's current version.
    pub current: bool,
    /// Its holders, in the order of their holding ids.
    pub holders: Vec<Holder>,
}

impl Roles {
    /// Who holds `role` at `at`, on each of its versions, first to current,
    /// read from the holding records; a lapsed holding is not listed.
    pub fn holders(&self, role: RoleId, at: u64) -> Result<Vec<VersionHolders>, RoleError> {
        let book = self.book();
        let found = book.role_or_refuse(role)?;
        let current = found.current().number();
        Ok(found
            .versions()
            .map(|version| VersionHolders {
                version: version.number(),
                current: version.number() == current,
                holders: book
                    .holdings()
                    .filter(|holding| {
                        holding.role == role
                            && holding.version == version.number()
                            && !holding.lapsed(at)
                    })
                    .map(|holding| Holder {
                        holding: holding.id,
                        agent: holding.holder,
                        version: holding.version,
                        ends_at: holding.ends_at,
                        policy: holding.policy,
                    })
                    .collect(),
            })
            .collect())
    }
}
