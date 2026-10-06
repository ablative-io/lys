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
        self.refuse_second_resend(&resent.prior, &resent.fired.operation)?;
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
    /// The already kept resend, borrowing its identity from the derived index.
    pub fn resent_occurrence(&self, prior: &str) -> Option<&str> {
        self.held.resent_occurrence(prior)
    }

    /// Read a kept occurrence only for an uncertain delivery of this aim.
    ///
    /// # Errors
    /// Names an unknown prior, a different aim or a delivery that is not uncertain.
    pub fn resend_lookup(&self, goal: &str, prior: &str) -> Result<Option<&Fired>, ServerError> {
        let previous = self
            .held
            .delivery(prior)
            .map_err(unavailable)?
            .ok_or_else(|| ServerError::Runner {
                refusal: "goal_prior_unknown".to_owned(),
                words: "the prior delivery is not held for this goal".to_owned(),
            })?;
        if previous.item.goal.id != goal {
            return Err(ServerError::Runner {
                refusal: "goal_prior_unknown".to_owned(),
                words: "the prior delivery is not held for this goal".to_owned(),
            });
        }
        if previous.sent.state != crate::goals_state::Delivery::Uncertain {
            return Err(ServerError::Runner {
                refusal: "control_not_uncertain".to_owned(),
                words: "the prior delivery is not uncertain".to_owned(),
            });
        }
        Ok(self.held.resent_firing(prior))
    }

    fn refuse_second_resend(&self, prior: &str, asked: &str) -> Result<(), ServerError> {
        if let Some(kept) = self.held.resent_occurrence(prior)
            && kept != asked
        {
            return Err(ServerError::Runner {
                refusal: "goal_prior_already_resent".to_owned(),
                words: format!(
                    "prior `{prior}` already has occurrence `{kept}`; finish its recorded decision instead of creating another delivery"
                ),
            });
        }
        Ok(())
    }

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
        let source = previous.sent.session.clone();
        self.refuse_second_resend(prior, operation)?;
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
            if previous.item.standing != crate::goals_state::Standing::Open || !previous.active {
                return Err(unavailable(
                    "goal_closed_or_inactive: the prior goal cannot send another occurrence",
                ));
            }
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

/// One currently admitted holder, naming the person whose aims it may fire.
pub struct CompactionHolder {
    /// The currently covered agent or team.
    pub holder: super::Holder,
    /// Its current responsible person or team owner.
    pub responsible: String,
}

/// A feed observation and the current authority captured before its fold.
#[derive(Clone, Copy)]
pub struct ControlAnswer<'a> {
    /// The runner's retained operation outcome.
    pub outcome: &'a lys_runner::operations::OperationOutcome,
    /// The holders admitted for the current source agent.
    pub holders: &'a [CompactionHolder],
    /// The service's observation instant in seconds.
    pub at: u64,
}

impl<S: LeafStore> GoalStore<S> {
    /// Fold the runner's answer through the existing delivery state owner.
    ///
    /// # Errors
    /// Propagates an unavailable or invalid goals log.
    pub fn control_answer(&mut self, answer: ControlAnswer<'_>) -> Result<bool, ServerError> {
        self.settle()?;
        let answered = crate::goals_state::Answered::from_runner(answer.outcome, answer.at);
        let changed = self
            .held
            .sent(&answered.operation)
            .is_some_and(|sent| sent.state.unsettled() && sent.state != answered.state);
        self.answer(answered)?;
        if !actual_compaction(answer.outcome) {
            return Ok(changed);
        }
        let marker = super::op_id(&["compaction-complete", &answer.outcome.operation]);
        if self.held.kept(&marker) {
            return Ok(changed);
        }
        let mut unique = std::collections::HashSet::with_capacity(answer.holders.len());
        let mut lines = Vec::with_capacity(answer.holders.len() + 1);
        let at = answer.outcome.at / 1000;
        for target in answer.holders {
            let goals = self
                .held
                .compaction_goals(&target.holder, &target.responsible)
                .map_err(unavailable)?;
            if goals.is_empty() {
                continue;
            }
            let kind = match target.holder.kind {
                crate::goals_state::HolderKind::Agent => "agent",
                crate::goals_state::HolderKind::Team => "team",
            };
            if !unique.insert((kind, target.holder.id.as_str())) {
                return Err(ServerError::Runner {
                    refusal: "compaction_holder_repeated".to_owned(),
                    words: "a compaction batch names each holder once".to_owned(),
                });
            }
            let operation = super::op_id(&[
                "compaction-holder",
                &answer.outcome.operation,
                kind,
                &target.holder.id,
            ]);
            lines.push(Line::Evented(crate::goals_state::Evented {
                operation,
                event: crate::goals_state::Event::Compaction,
                goals,
                at,
            }));
        }
        lines.push(Line::Evented(crate::goals_state::Evented {
            operation: marker,
            event: crate::goals_state::Event::Compaction,
            goals: Vec::new(),
            at,
        }));
        self.compaction_batch(lines)?;
        Ok(true)
    }
}

/// Only an actual harness completion can advance compaction reminders.
pub(crate) fn actual_compaction(outcome: &lys_runner::operations::OperationOutcome) -> bool {
    outcome.state == lys_runner::operations::OperationState::Confirmed
        && matches!(outcome.request.as_str(), "compact" | "context_compact")
}

impl<S: LeafStore> GoalStore<S> {
    pub(crate) fn compaction_kept(
        &self,
        outcome: &lys_runner::operations::OperationOutcome,
    ) -> bool {
        self.held
            .kept(&super::op_id(&["compaction-complete", &outcome.operation]))
    }

    /// Current holders are derived and are never decoded from stored history here.
    pub fn compaction_holders(&self, agent: &str) -> Vec<super::Holder> {
        self.held.compaction_holders(agent)
    }

    fn compaction_batch(&mut self, lines: Vec<Line>) -> Result<(), ServerError> {
        let count = u64::try_from(lines.len()).map_err(unavailable)?;
        let since = self
            .since_snapshot
            .checked_add(count)
            .ok_or_else(|| unavailable("goals snapshot count is exhausted"))?;
        let bytes = lines
            .iter()
            .map(serde_json::to_vec)
            .collect::<Result<Vec<_>, _>>()
            .map_err(unavailable)?;
        let first = self.log.len();
        if let Err(failure) = self
            .log
            .append_batch(&bytes.iter().map(Vec::as_slice).collect::<Vec<_>>())
        {
            self.uncertain = true;
            self.settle()?;
            for (offset, expected) in bytes.iter().enumerate() {
                let index = first
                    .checked_add(u64::try_from(offset).map_err(unavailable)?)
                    .ok_or_else(|| unavailable("goals batch index is exhausted"))?;
                match self.log.leaf_bytes(index).map_err(unavailable)? {
                    Some(kept) if kept == *expected => {}
                    Some(_) => {
                        return Err(unavailable(format!(
                            "leaf {index} was written by another writer: {failure}"
                        )));
                    }
                    None => return Err(unavailable(failure)),
                }
            }
            return Ok(());
        }
        for line in lines {
            if let Err(reason) = self.held.hold(line) {
                self.uncertain = true;
                return Err(unavailable(reason));
            }
        }
        self.since_snapshot = since;
        if self.since_snapshot >= lys_identity::SNAPSHOT_EVERY.get() {
            self.write_snapshot();
        }
        Ok(())
    }
}
