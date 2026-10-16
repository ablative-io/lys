use super::{Boundary, Controller, Kind, Observation, Transport, Update, receipt};
use crate::error::RunnerError;

impl Controller {
    pub(super) fn apply(&mut self, observation: Observation) -> Result<Update, RunnerError> {
        let mut update = Update::default();
        match observation {
            Observation::Other | Observation::Accepted => {}
            Observation::Ready { compact } => {
                if self.transport == Transport::Claude {
                    return self.claude_version(compact);
                }
                if self.ready {
                    return Ok(update);
                }
                self.ready = true;
                self.compact = compact;
                self.boundary = Boundary::Idle;
                update.events.push(self.event("control_bound", None, None));
                self.request_boundary(&mut update)?;
            }
            Observation::Started { turn } => {
                if self.boundary == Boundary::Reserved || self.boundary == Boundary::Idle {
                    self.turn = Some(turn.clone());
                    self.boundary = Boundary::Active(turn.clone());
                    update
                        .events
                        .push(self.event("turn_start", Some(turn), None));
                }
            }
            Observation::Admitted { uuid, turn } => {
                if self.transport == Transport::Claude && self.defer_claude_admission(&uuid) {
                    return Ok(update);
                }
                if let Some(current) = &self.current {
                    if current.uuid == uuid && !self.admitted {
                        update.admissions.push((
                            current.id.clone(),
                            self.binding.clone(),
                            uuid,
                            turn.clone(),
                        ));
                        self.admitted = true;
                        if let Some(turn) = turn {
                            self.turn = Some(turn.clone());
                            self.boundary = Boundary::Active(turn);
                        }
                        if current.kind != Kind::Compact {
                            update.receipts.push(receipt(
                                &current.id,
                                crate::operations::OperationState::Confirmed,
                                "harness_admitted",
                            ));
                        }
                        update.events.push(self.event(
                            "harness_admitted",
                            self.turn.clone(),
                            Some(current.id.clone()),
                        ));
                    }
                }
            }
            Observation::Compacted { turn } => {
                if self.compaction_matches(turn.as_deref()) {
                    self.compacted = Some(turn.unwrap_or_default());
                }
            }
            Observation::Completed { turn, failed } => {
                return self.completed(turn.as_deref(), failed);
            }
            Observation::Refused => {
                if let Some(current) = self.current.take() {
                    update.receipts.push(receipt(
                        &current.id,
                        crate::operations::OperationState::Refused,
                        "harness_refused",
                    ));
                }
                for pending in self.pending.drain(..) {
                    update.receipts.push(receipt(
                        &pending.id,
                        crate::operations::OperationState::Refused,
                        "harness_refused: queued input was not written",
                    ));
                }
                self.pending_ids.clear();
                self.ready = false;
                self.closed = true;
                self.boundary = Boundary::Unknown;
                self.turn = None;
                self.compacted = None;
                self.admitted = false;
                update
                    .events
                    .push(self.event("control_request_refused", None, None));
            }
        }
        Ok(update)
    }

    fn completed(&mut self, turn: Option<&str>, failed: bool) -> Result<Update, RunnerError> {
        let mut update = Update::default();
        if self.transport == Transport::Claude && !self.initialized {
            return Err(self.claude_unproved());
        }
        let matches = if self.transport == Transport::Claude {
            self.current.is_some() && self.boundary != Boundary::Unknown
        } else {
            turn.is_some()
                && turn == self.turn.as_deref()
                && matches!(self.boundary, Boundary::Active(_))
        };
        if !matches {
            return Ok(update);
        }
        if let Some(current) = self.current.take() {
            if current.kind == Kind::Compact {
                let (state, reason) = if self.compacted.is_some() && !failed {
                    (
                        crate::operations::OperationState::Confirmed,
                        "harness_compacted",
                    )
                } else {
                    (crate::operations::OperationState::Refused, "not_compacted")
                };
                update.receipts.push(receipt(&current.id, state, reason));
            } else if !self.admitted {
                update.receipts.push(receipt(
                    &current.id,
                    crate::operations::OperationState::Uncertain,
                    "admission_unconfirmed",
                ));
            }
            update
                .events
                .push(self.event("turn_end", self.turn.clone(), Some(current.id)));
        } else {
            update
                .events
                .push(self.event("turn_end", self.turn.clone(), None));
        }
        self.boundary = Boundary::Idle;
        self.turn = None;
        self.compacted = None;
        self.request_boundary(&mut update)?;
        self.dispatch(&mut update)?;
        Ok(update)
    }
}
