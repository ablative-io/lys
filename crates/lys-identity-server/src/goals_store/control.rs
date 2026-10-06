//! Explicit resends keep the original due time and leave normal timers alone.

use super::{Fired, GoalError, GoalStore, Item, LeafStore, Line, ServerError, span, unavailable};

impl<S: LeafStore> GoalStore<S> {
    /// Keep a responsible person's distinct resend without consuming its regular timer.
    pub fn resend(&mut self, resent: crate::goals_state::Resent) -> Result<Fired, ServerError> {
        self.settle()?;
        if self.held.kept(&resent.fired.operation) {
            if self.held.resends.get(&resent.fired.operation) != Some(&resent.prior)
                || self
                    .held
                    .firing(&resent.fired.operation)
                    .is_none_or(|kept| {
                        kept.goal != resent.fired.goal
                            || kept.reminder != resent.fired.reminder
                            || kept.due != resent.fired.due
                            || kept.fired != resent.fired.fired
                            || kept.refused != resent.fired.refused
                            || kept.sent.len() != resent.fired.sent.len()
                            || kept
                                .sent
                                .iter()
                                .zip(&resent.fired.sent)
                                .any(|(kept, asked)| {
                                    kept.operation != asked.operation
                                        || kept.session != asked.session
                                })
                    })
                || self
                    .held
                    .item(&resent.fired.goal)
                    .is_none_or(|item| item.goal.responsible != resent.by)
            {
                return Err(GoalError::Reused {
                    operation: resent.fired.operation,
                }
                .into());
            }
            return Ok(resent.fired);
        }
        self.held.check_resent(&resent).map_err(unavailable)?;
        let fired = resent.fired.clone();
        self.append(Line::Resent(resent))?;
        Ok(fired)
    }
}

pub(crate) fn text_with_words(item: &Item, words: &str, at: u64) -> String {
    let goal = &item.goal;
    let kind = match goal.kind {
        crate::goals_state::Kind::Goal => "Goal",
        crate::goals_state::Kind::Expectation => "Expectation",
        crate::goals_state::Kind::Deliverable => "Deliverable",
    };
    match goal.deadline {
        Some(deadline) => {
            let left = if deadline >= at {
                format!("{} left", span(deadline - at))
            } else {
                format!("{} past its deadline", span(at - deadline))
            };
            format!("Reminder from Lys. {kind}: {words}. {left}.")
        }
        None => format!("Reminder from Lys. {kind}: {words}."),
    }
}

pub(crate) fn occurrence_text(
    item: &Item,
    words: &str,
    due: u64,
    at: u64,
    prior: Option<&str>,
) -> Result<String, ServerError> {
    let timestamp = |value: u64| -> Result<String, ServerError> {
        let seconds = i64::try_from(value).map_err(unavailable)?;
        jiff::Timestamp::from_second(seconds)
            .map(|instant| instant.to_string())
            .map_err(unavailable)
    };
    let due_at = timestamp(due)?;
    let dispatched_at = timestamp(at)?;
    let late = at.checked_sub(due).map_or_else(String::new, |seconds| {
        if seconds == 0 {
            String::new()
        } else {
            format!(" Late by {}.", span(seconds))
        }
    });
    let prior = prior.map_or_else(String::new, |operation| {
        format!("Possible prior delivery of operation {operation}. ")
    });
    Ok(format!(
        "{prior}{} Due: {due_at}. Dispatch time: {dispatched_at}.{late}",
        text_with_words(item, words, at)
    ))
}

impl<S: LeafStore> GoalStore<S> {
    /// Prepare a distinct occurrence from one prior receipt without changing a timer.
    ///
    /// # Errors
    /// Refuses an unavailable, closed, unauthorised or reused occurrence.
    pub fn prepare_resend(
        &self,
        goal: &str,
        prior: &str,
        operation: &str,
        session: &str,
        by: &str,
        at: u64,
    ) -> Result<(crate::goals_state::Resent, String), ServerError> {
        let previous = self
            .held
            .delivery(prior)
            .map_err(unavailable)?
            .ok_or(GoalError::Unknown)?;
        if previous.item.goal.id != goal || previous.item.goal.responsible != by {
            return Err(GoalError::Unknown.into());
        }
        if previous.item.standing != crate::goals_state::Standing::Open || !previous.active {
            return Err(unavailable(
                "goal_closed_or_inactive: the prior goal cannot send another occurrence",
            ));
        }
        let source = previous.sent.session.clone();
        let fired = if let Some(kept) = self.held.firing(operation) {
            if self.held.resends.get(operation).map(String::as_str) != Some(prior)
                || kept.goal != goal
                || kept.sent.len() != 1
                || kept.sent[0].session != session
            {
                return Err(GoalError::Reused {
                    operation: operation.to_owned(),
                }
                .into());
            }
            kept.clone()
        } else {
            crate::goals_state::Fired {
                operation: operation.to_owned(),
                goal: goal.to_owned(),
                reminder: previous.fired.reminder,
                due: previous.fired.due,
                fired: at,
                late: at > previous.fired.due,
                text: occurrence_text(
                    previous.item,
                    previous.words,
                    previous.fired.due,
                    at,
                    Some(prior),
                )?,
                sent: vec![crate::goals_state::Sent {
                    session: session.to_owned(),
                    operation: super::op_id(&["resend-delivery", operation, session]),
                    state: crate::goals_state::Delivery::Pending,
                    words: String::new(),
                    at,
                }],
                refused: None,
            }
        };
        let resent = crate::goals_state::Resent {
            fired,
            prior: prior.to_owned(),
            by: by.to_owned(),
        };
        if !self.held.kept(operation) {
            self.held.check_resent(&resent).map_err(unavailable)?;
        }
        Ok((resent, source))
    }
}
