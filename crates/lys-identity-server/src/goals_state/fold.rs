//! Each kept leaf changes folded state and its derived locations together.

use super::{Held, Item, Line, Remind, Standing, Timer, first_due};
use std::sync::Arc;

impl Held {
    /// Fold one leaf. A leaf that contradicts what came before is refused,
    /// since every kept leaf was checked against it.
    pub fn hold(&mut self, line: Line) -> Result<(), String> {
        match line {
            Line::Set(goal) => {
                goal.check().map_err(|error| error.to_string())?;
                if self.item(&goal.id).is_some() {
                    return Err(format!("goal `{}` is already set", goal.id));
                }
                let timers = goal
                    .reminders
                    .iter()
                    .map(|remind| Timer {
                        remind: remind.clone(),
                        next_due: first_due(&goal, remind),
                    })
                    .collect();
                Arc::make_mut(&mut self.index)
                    .items
                    .insert(goal.id.clone(), self.items.len());
                self.items.push(Item {
                    goal,
                    standing: Standing::Open,
                    marked: None,
                    timers,
                    fired: Vec::new(),
                    changes: Vec::new(),
                });
            }
            Line::Marked(marked) => {
                let position = self.position(&marked.goal)?;
                let operation = marked.operation.clone();
                let item = self.item_mut(&marked.goal)?;
                if item.standing != Standing::Open || marked.standing == Standing::Open {
                    return Err(format!("goal `{}` cannot be marked so", marked.goal));
                }
                item.standing = marked.standing;
                item.marked = Some(marked);
                for timer in &mut item.timers {
                    timer.next_due = None;
                }
                Arc::make_mut(&mut self.index).mark(position, &operation);
            }
            Line::Changed(changed) => {
                changed.change.check().map_err(|error| error.to_string())?;
                if self.changed(&changed.operation).is_some() {
                    return Err(format!(
                        "operation `{}` already names a change",
                        changed.operation
                    ));
                }
                let item = self.item_mut(&changed.goal)?;
                if item.standing != Standing::Open {
                    return Err(format!("goal `{}` is already closed", changed.goal));
                }
                let change = item.changes.len();
                let position = self.position(&changed.goal)?;
                Arc::make_mut(&mut self.index).change(position, change, &changed);
                self.items[position].changes.push(changed);
            }
            Line::Fired(fired) => self.fire(fired)?,
            Line::Resent(resent) => {
                self.check_resent(&resent)?;
                let position = self.position(&resent.fired.goal)?;
                let firing = self.items[position].fired.len();
                Arc::make_mut(&mut self.index).fire(position, firing, &resent.fired);
                Arc::make_mut(&mut self.index)
                    .prior_resends
                    .insert(resent.prior.clone(), (position, firing));
                self.resends
                    .insert(resent.fired.operation.clone(), resent.prior);
                self.items[position].fired.push(resent.fired);
            }
            Line::Answered(answered) => self.answer(&answered)?,
            Line::Evented(evented) => {
                if self.kept(&evented.operation) {
                    return Err(format!("operation `{}` is already kept", evented.operation));
                }
                self.events.push(evented.operation.clone());
                Arc::make_mut(&mut self.index)
                    .kept
                    .insert(evented.operation.clone());
                for goal in &evented.goals {
                    let item = self.item_mut(goal)?;
                    if item.standing != Standing::Open {
                        continue;
                    }
                    for timer in &mut item.timers {
                        if timer.remind
                            == (Remind::On {
                                event: evented.event,
                            })
                        {
                            timer.next_due = Some(timer.next_due.unwrap_or(evented.at));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
