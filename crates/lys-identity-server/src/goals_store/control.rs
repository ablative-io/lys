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
