//! Team creation and changes update retained records and indexes together.

use super::{Held, Line, Operation, Team};
use std::sync::Arc;

impl Held {
    /// Fold one line. A line the lines before it do not allow is refused by
    /// reason, since every kept line was checked against what came before.
    ///
    /// # Errors
    /// Refuses a reused operation or a team act the preceding lines do not
    /// allow, including an act on a team never created.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        if self.index.operations.contains_key(line.operation()) {
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
        let Some(line) = self.begin_line(line) else {
            return Ok(());
        };
        let position = *self
            .index
            .positions
            .get(line.team())
            .ok_or_else(|| format!("team `{}` was never created", line.team()))?;
        let team = &mut self.teams[position];
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
                let index = Arc::make_mut(&mut self.index);
                if let Some(parent) = &team.parent {
                    if let Some(children) = index.children.get_mut(parent) {
                        children.remove(&team.created.id);
                        if children.is_empty() {
                            index.children.remove(parent);
                        }
                    }
                }
                if let Some(parent) = &nested.parent {
                    index
                        .children
                        .entry(parent.clone())
                        .or_default()
                        .insert(team.created.id.clone());
                }
                team.parent.clone_from(&nested.parent);
                team.lead.clone_from(&nested.lead);
            }
            Line::Created(_) | Line::CreatedV1(_) | Line::Checked(_) => {}
        }
        Arc::make_mut(&mut self.index).operations.insert(
            line.operation().to_owned(),
            Operation::Change(position, team.changes.len()),
        );
        team.changes.push(line);
        Ok(())
    }

    fn begin_line(&mut self, line: Line) -> Option<Line> {
        match line {
            Line::Created(created) => {
                Arc::make_mut(&mut self.index)
                    .operations
                    .insert(created.id.clone(), Operation::Created(self.teams.len()));
                Arc::make_mut(&mut self.index)
                    .positions
                    .insert(created.id.clone(), self.teams.len());
                self.teams.push(Team {
                    created,
                    parent: None,
                    lead: None,
                    members: Vec::new(),
                    retired: None,
                    held: Vec::new(),
                    changes: Vec::new(),
                });
                None
            }
            Line::CreatedV1(created) => {
                let index = Arc::make_mut(&mut self.index);
                index.operations.insert(
                    created.created.id.clone(),
                    Operation::Change(self.teams.len(), 0),
                );
                index
                    .positions
                    .insert(created.created.id.clone(), self.teams.len());
                if let Some(parent) = &created.parent {
                    index
                        .children
                        .entry(parent.clone())
                        .or_default()
                        .insert(created.created.id.clone());
                }
                self.teams.push(Team {
                    created: created.created.clone(),
                    parent: created.parent.clone(),
                    lead: created.lead.clone(),
                    members: Vec::new(),
                    retired: None,
                    held: Vec::new(),
                    changes: vec![Line::CreatedV1(created)],
                });
                None
            }
            Line::Checked(checked) => {
                Arc::make_mut(&mut self.index)
                    .operations
                    .insert(checked.operation.clone(), Operation::Checked);
                self.checked = Some(checked);
                None
            }
            other => Some(other),
        }
    }
}
