//! A reversible membership migration copies only the teams it changes.
//! A full catalogue is materialized only when a caller asks for its slice.

use std::collections::{HashMap, HashSet, hash_map::Entry};
use std::sync::OnceLock;

use crate::teams_state::{Held, Line, Refused, Team};

#[derive(Default)]
pub(super) struct Overlay {
    teams: HashMap<String, Team>,
    operations: HashSet<String>,
    checked: bool,
    catalogue: OnceLock<Vec<Team>>,
}

impl Overlay {
    pub(super) fn team(&self, id: &str) -> Option<&Team> {
        self.teams.get(id)
    }

    pub(super) fn view<'a>(&'a self, base: &'a Held, team: &'a Team) -> &'a Team {
        self.teams
            .get(&team.created.id)
            .filter(|_| {
                base.team(&team.created.id)
                    .is_some_and(|first| std::ptr::eq(first, team))
            })
            .unwrap_or(team)
    }

    pub(super) fn teams(&self, base: &Held) -> &[Team] {
        self.catalogue.get_or_init(|| {
            base.teams
                .iter()
                .map(|team| self.view(base, team).clone())
                .collect()
        })
    }

    pub(super) fn hold(&mut self, base: &Held, line: Line) -> Result<(), String> {
        if self.operations.contains(line.operation()) {
            return Err(format!(
                "operation `{}` already names a line",
                line.operation()
            ));
        }
        let operation = line.operation().to_owned();
        match line {
            Line::Held(hold) => {
                let team = match self.teams.entry(hold.team.clone()) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        let team = base
                            .team(&hold.team)
                            .ok_or_else(|| format!("team `{}` was never created", hold.team))?;
                        entry.insert(team.clone())
                    }
                };
                if team.held.iter().any(|held| held.member == hold.member) {
                    return Err(format!(
                        "line `{}` on team `{}` is refused: {:?}",
                        hold.operation,
                        hold.team,
                        Refused::Held
                    ));
                }
                if team.lead.as_ref() == Some(&hold.member) {
                    team.lead = None;
                }
                team.held.push(hold.clone());
                team.changes.push(Line::Held(hold));
            }
            Line::Checked(_) => {
                if self.checked {
                    return Err(format!(
                        "line `{operation}` on team `` is refused: {:?}",
                        Refused::Checked
                    ));
                }
                self.checked = true;
            }
            other => {
                return Err(format!(
                    "line `{}` is not a migration line",
                    other.operation()
                ));
            }
        }
        self.operations.insert(operation);
        Ok(())
    }
}
