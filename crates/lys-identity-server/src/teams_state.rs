//! What the teams' log folds to, and how that fold is sealed in the log's
//! signed snapshot so a start reads only the leaves after it.
//!
//! A team is a named group of people and agents, owned by one person. It
//! carries no grant and confers no authority: membership says who is in it
//! and nothing more. It is created under the operation id that names it, its
//! members are added and removed one line at a time, and it is retired once.
//! Every operation id names one line only.
//!
//! A membership kept before the rule that an agent joins only by its
//! operator's act, and a person only by their own or the administrator's, is
//! checked once under that rule. One the rule would refuse is held: it stays
//! on the record, its member is sent no team goal, and the administrator
//! confirms or removes it. The check is recorded once, so it never runs
//! again.

use serde::{Deserialize, Serialize};

use crate::read_views::Login;

/// The snapshot domain the teams' folded state is sealed under.
pub const DOMAIN: &str = "lys/identity/teams-state/v1";

const FORMAT: &str = "lys-teams-state/v2";
const BEFORE_NESTING: &str = "lys-teams-state/v1";

/// A team as it was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Created {
    /// The operation id it was created with, which names it.
    pub id: String,
    /// The person who owns it.
    pub owner: String,
    /// Its name.
    pub name: String,
    /// What it is for, in its creator's words; empty when they said nothing.
    pub description: String,
    /// The login that created it.
    pub by: Login,
    /// When it was created, in seconds since the Unix epoch.
    pub at: u64,
}

/// A change to a team after its creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Changed {
    /// The operation id it was made with.
    pub operation: String,
    /// The team changed.
    pub team: String,
    /// The person or agent added or removed; empty for a retirement.
    pub member: String,
    /// The login that made it.
    pub by: Login,
    /// When it was made, in seconds since the Unix epoch.
    pub at: u64,
}

/// A membership held because the rule would now refuse how it was added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Hold {
    /// The operation id the hold was kept under.
    pub operation: String,
    /// The team.
    pub team: String,
    /// The person or agent held.
    pub member: String,
    /// Why the rule refuses how it was added.
    pub reason: String,
    /// When it was held, in seconds since the Unix epoch.
    pub at: u64,
}

/// The one check of the memberships kept before the rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checked {
    /// The operation id it was kept under.
    pub operation: String,
    /// When it was made, in seconds since the Unix epoch.
    pub at: u64,
}

/// One line of the log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum Line {
    /// A team created.
    Created(Created),
    /// A creation carrying explicit nesting under its versioned event.
    CreatedV1(crate::teams_nesting::CreatedV1),
    /// A parent and lead changed under their versioned event.
    NestedV1(crate::teams_nesting::NestedV1),
    /// A member added.
    Added(Changed),
    /// A member removed.
    Removed(Changed),
    /// A team retired.
    Retired(Changed),
    /// A membership held under the rule.
    Held(Hold),
    /// A held membership the administrator confirmed.
    Confirmed(Changed),
    /// The memberships kept before the rule were checked.
    Checked(Checked),
}

impl Line {
    /// The operation id the line was kept under.
    pub fn operation(&self) -> &str {
        match self {
            Self::Created(created) => &created.id,
            Self::CreatedV1(created) => &created.created.id,
            Self::NestedV1(nested) => &nested.operation,
            Self::Added(changed)
            | Self::Removed(changed)
            | Self::Retired(changed)
            | Self::Confirmed(changed) => &changed.operation,
            Self::Held(hold) => &hold.operation,
            Self::Checked(checked) => &checked.operation,
        }
    }

    /// The team the line is about.
    pub fn team(&self) -> &str {
        match self {
            Self::Created(created) => &created.id,
            Self::CreatedV1(created) => &created.created.id,
            Self::NestedV1(nested) => &nested.team,
            Self::Added(changed)
            | Self::Removed(changed)
            | Self::Retired(changed)
            | Self::Confirmed(changed) => &changed.team,
            Self::Held(hold) => &hold.team,
            Self::Checked(_) => "",
        }
    }

    /// The same line received at `at`, so two sendings compare as one act.
    #[must_use]
    pub fn at(&self, at: u64) -> Self {
        let mut line = self.clone();
        match &mut line {
            Self::Created(created) => created.at = at,
            Self::CreatedV1(created) => created.created.at = at,
            Self::NestedV1(nested) => nested.at = at,
            Self::Added(changed)
            | Self::Removed(changed)
            | Self::Retired(changed)
            | Self::Confirmed(changed) => changed.at = at,
            Self::Held(hold) => hold.at = at,
            Self::Checked(checked) => checked.at = at,
        }
        line
    }

    fn received(&self) -> u64 {
        match self {
            Self::Created(created) => created.at,
            Self::CreatedV1(created) => created.created.at,
            Self::NestedV1(nested) => nested.at,
            Self::Added(changed)
            | Self::Removed(changed)
            | Self::Retired(changed)
            | Self::Confirmed(changed) => changed.at,
            Self::Held(hold) => hold.at,
            Self::Checked(checked) => checked.at,
        }
    }

    /// Whether `other` is the same act as this line, whenever each was received.
    pub fn same_act(&self, other: &Self) -> bool {
        *self == other.at(self.received())
    }
}

/// One team, with its members and its retirement once it has one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Team {
    /// How it was created.
    pub created: Created,
    /// Its parent; absent in stores written before nesting.
    #[serde(default)]
    pub parent: Option<String>,
    /// Its lead; absent in stores written before nesting.
    #[serde(default)]
    pub lead: Option<String>,
    /// Its members, in the order added.
    pub members: Vec<String>,
    /// How it was retired, null while it is in use.
    pub retired: Option<Changed>,
    /// Its memberships held under the rule, until confirmed or removed.
    #[serde(default)]
    pub held: Vec<Hold>,
    /// Every line kept on it after its creation, in order.
    pub changes: Vec<Line>,
}

/// The teams as their log folds them, in the order created.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The teams.
    pub teams: Vec<Team>,
    /// The one check of the memberships kept before the rule, once made.
    #[serde(default)]
    pub checked: Option<Checked>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    held: Held,
}

/// Why a line cannot be kept on the teams as they stand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// The team was never created.
    Unknown,
    /// The team is retired.
    Retired,
    /// The member is already in the team.
    Held,
    /// The member is not in the team.
    Absent,
    /// The membership is not held, so there is nothing to confirm.
    NotHeld,
    /// The memberships were already checked.
    Checked,
    /// The requested parent closes a cycle.
    ParentCycle {
        /// The team whose parent would change.
        team: String,
        /// The requested parent that would close a cycle.
        parent: String,
    },
    /// The requested lead is not an admitted member.
    LeadNotMember {
        /// The team requiring an admitted lead.
        team: String,
        /// The requested lead that is not an admitted member.
        lead: String,
    },
}

impl Held {
    /// The team named `id`.
    pub fn team(&self, id: &str) -> Option<&Team> {
        self.teams.iter().find(|team| team.created.id == id)
    }

    /// The line kept under `operation`, whichever kind it is.
    pub fn operation(&self, operation: &str) -> Option<Line> {
        self.teams
            .iter()
            .find_map(|team| {
                if team.created.id == operation {
                    return Some(
                        team.changes
                            .iter()
                            .find(|line| matches!(line, Line::CreatedV1(_)))
                            .cloned()
                            .unwrap_or_else(|| Line::Created(team.created.clone())),
                    );
                }
                team.changes
                    .iter()
                    .find(|line| line.operation() == operation)
                    .cloned()
            })
            .or_else(|| {
                self.checked
                    .as_ref()
                    .filter(|checked| checked.operation == operation)
                    .cloned()
                    .map(Line::Checked)
            })
    }

    /// Whether `line` may be kept on the teams as they stand, by reason.
    pub fn allows(&self, line: &Line) -> Result<(), Refused> {
        match line {
            Line::CreatedV1(created) => {
                return crate::teams_nesting::allows(
                    self,
                    &created.created.id,
                    created.parent.as_deref(),
                    created.lead.as_deref(),
                );
            }
            Line::NestedV1(nested) => crate::teams_nesting::allows(
                self,
                &nested.team,
                nested.parent.as_deref(),
                nested.lead.as_deref(),
            )?,
            _ => {}
        }
        match line {
            Line::Checked(_) if self.checked.is_some() => return Err(Refused::Checked),
            Line::Created(_) | Line::Checked(_) => return Ok(()),
            _ => {}
        }
        let team = self.team(line.team()).ok_or(Refused::Unknown)?;
        if team.retired.is_some() {
            return Err(Refused::Retired);
        }
        match line {
            Line::Added(changed) if team.members.contains(&changed.member) => Err(Refused::Held),
            Line::Removed(changed) if !team.members.contains(&changed.member) => {
                Err(Refused::Absent)
            }
            Line::Held(hold) if !team.members.contains(&hold.member) => Err(Refused::Absent),
            Line::Held(hold) if team.held.iter().any(|held| held.member == hold.member) => {
                Err(Refused::Held)
            }
            Line::Confirmed(changed)
                if !team.held.iter().any(|held| held.member == changed.member) =>
            {
                Err(Refused::NotHeld)
            }
            _ => Ok(()),
        }
    }

    /// Fold one line. A line the lines before it do not allow is refused by
    /// reason, since every kept line was checked against what came before.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        if self.operation(line.operation()).is_some() {
            return Err(format!(
                "operation `{}` already names a line",
                line.operation()
            ));
        }
        self.allows(&line).map_err(|refused| {
            format!(
                "line `{}` on team `{}` is refused: {refused:?}",
                line.operation(),
                line.team()
            )
        })?;
        match line {
            Line::Created(created) => {
                self.teams.push(Team {
                    created,
                    parent: None,
                    lead: None,
                    members: Vec::new(),
                    retired: None,
                    held: Vec::new(),
                    changes: Vec::new(),
                });
                return Ok(());
            }
            Line::CreatedV1(created) => {
                self.teams.push(Team {
                    created: created.created.clone(),
                    parent: created.parent.clone(),
                    lead: created.lead.clone(),
                    members: Vec::new(),
                    retired: None,
                    held: Vec::new(),
                    changes: vec![Line::CreatedV1(created)],
                });
                return Ok(());
            }
            Line::Checked(checked) => {
                self.checked = Some(checked);
                return Ok(());
            }
            _ => {}
        }
        let team = self
            .teams
            .iter_mut()
            .find(|team| team.created.id == line.team())
            .ok_or_else(|| format!("team `{}` was never created", line.team()))?;
        match &line {
            Line::Added(changed) => team.members.push(changed.member.clone()),
            Line::Removed(changed) => {
                team.members.retain(|member| *member != changed.member);
                team.held.retain(|held| held.member != changed.member);
                if team.lead.as_ref() == Some(&changed.member) {
                    team.lead = None;
                }
            }
            Line::Retired(changed) => team.retired = Some(changed.clone()),
            Line::Held(hold) => {
                if team.lead.as_ref() == Some(&hold.member) {
                    team.lead = None;
                }
                team.held.push(hold.clone());
            }
            Line::Confirmed(changed) => team.held.retain(|held| held.member != changed.member),
            Line::NestedV1(nested) => {
                team.parent.clone_from(&nested.parent);
                team.lead.clone_from(&nested.lead);
            }
            Line::Created(_) | Line::CreatedV1(_) | Line::Checked(_) => {}
        }
        team.changes.push(line);
        Ok(())
    }

    /// Fold every leaf of `tail`, in order.
    pub fn fold(&mut self, tail: &lys_log_store::Tail) -> Result<(), String> {
        for (index, bytes) in (tail.from..).zip(&tail.leaves) {
            let line = serde_json::from_slice(bytes)
                .map_err(|error| format!("leaf {index} is not a team line: {error}"))?;
            self.hold(line)
                .map_err(|reason| format!("leaf {index}: {reason}"))?;
        }
        Ok(())
    }

    /// The state a snapshot seals.
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(&Sealed {
            format: FORMAT.to_owned(),
            held: self.clone(),
        })
        .map_err(|error| format!("teams state: {error}"))
    }

    /// The state a snapshot sealed, refused by reason unless it reads whole
    /// in this format.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let sealed: Sealed =
            serde_json::from_slice(bytes).map_err(|error| format!("teams state: {error}"))?;
        if sealed.format != FORMAT && sealed.format != BEFORE_NESTING {
            return Err(format!(
                "teams state is in format {}, not {FORMAT}",
                sealed.format
            ));
        }
        Ok(sealed.held)
    }
}
