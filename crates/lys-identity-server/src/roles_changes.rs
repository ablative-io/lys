use serde::{Deserialize, Serialize};

use super::{Kept, unavailable};
use crate::error::ServerError;
use crate::roles_records::{Ending, Holding, Move, Role, Version};

#[derive(Serialize, Deserialize)]
#[serde(tag = "change", deny_unknown_fields)]
pub(super) enum Change {
    Make {
        role: Role,
    },
    Revise {
        role: usize,
        version: Version,
    },
    Assign {
        role: usize,
        holding: Holding,
    },
    Move {
        role: usize,
        holding: usize,
        moved: Move,
    },
    End {
        role: usize,
        holding: usize,
        ending: Ending,
    },
}

impl Change {
    pub(super) fn validate(&self, kept: &Kept) -> Result<(), ServerError> {
        let (role, holding) = match self {
            Self::Make { .. } => return Ok(()),
            Self::Revise { role, .. } | Self::Assign { role, .. } => (*role, None),
            Self::Move { role, holding, .. } | Self::End { role, holding, .. } => {
                (*role, Some(*holding))
            }
        };
        let role = kept
            .roles
            .get(role)
            .ok_or_else(|| unavailable("role change names an absent role"))?;
        if holding.is_some_and(|position| role.holdings.get(position).is_none()) {
            return Err(unavailable("role change names an absent holding"));
        }
        Ok(())
    }

    pub(super) fn apply(self, kept: &mut Kept) -> Result<(), ServerError> {
        self.validate(kept)?;
        match self {
            Self::Make { role } => kept.roles.push(role),
            Self::Revise { role, version } => {
                role_mut(kept, role)?.versions.push(version);
            }
            Self::Assign { role, holding } => {
                role_mut(kept, role)?.holdings.push(holding);
            }
            Self::Move {
                role,
                holding,
                moved,
            } => {
                let holding = holding_mut(kept, role, holding)?;
                holding.version = moved.to;
                holding.moves.push(moved);
            }
            Self::End {
                role,
                holding,
                ending,
            } => holding_mut(kept, role, holding)?.ended = Some(ending),
        }
        Ok(())
    }
}

fn role_mut(kept: &mut Kept, role: usize) -> Result<&mut Role, ServerError> {
    kept.roles
        .get_mut(role)
        .ok_or_else(|| unavailable("role change names an absent role"))
}

fn holding_mut(kept: &mut Kept, role: usize, holding: usize) -> Result<&mut Holding, ServerError> {
    role_mut(kept, role)?
        .holdings
        .get_mut(holding)
        .ok_or_else(|| unavailable("role change names an absent holding"))
}
