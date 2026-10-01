//! Ordered batch admission and durable receipts.

use std::collections::HashMap;

use lys_log_store::LeafStore;

use super::{Directory, IdentityError, IdentityEvent, Receipt, SignedEvent, sign_event};

enum Answer {
    Recorded(Box<Receipt>),
    Appended(usize),
}

impl<S: LeafStore> Directory<S> {
    /// Judge an ordered batch, sign its new events, and record them with one pin.
    /// Admission runs against a private projection so any invalid request refuses
    /// the whole batch before writing. This clones the projection once per batch.
    /// Existing and repeated operations obey the same retry rule as single changes.
    /// A storage failure can commit a prefix; retrying the batch returns its saved
    /// receipts and records only the remainder. This is not an atomic transaction.
    ///
    /// # Errors
    /// Any admission or signing refusal before writing, or a named log failure.
    /// An uncertain outcome holds current reads until reconciliation succeeds.
    pub fn commit_batch(
        &mut self,
        events: &[IdentityEvent],
    ) -> Result<Vec<Receipt>, IdentityError> {
        self.settle()?;
        if events.is_empty() {
            return Ok(Vec::new());
        }
        let mut projection = self.projection.clone();
        let mut next = self.folded;
        let mut signed: Vec<SignedEvent> = Vec::with_capacity(events.len());
        let mut planned = HashMap::new();
        let mut answers = Vec::with_capacity(events.len());
        for event in events {
            if let Some((_, receipt)) = self.retry(
                event.operation(),
                event.actor(),
                Some(event.identity()),
                event.change(),
            )? {
                answers.push(Answer::Recorded(Box::new(receipt)));
                continue;
            }
            if let Some(&position) = planned.get(&event.operation()) {
                let original: &SignedEvent = &signed[position];
                if !super::same_actor(original.event()?.actor(), event.actor())
                    || original.event()?.identity() != event.identity()
                    || original.event()?.change() != event.change()
                {
                    return Err(IdentityError::OperationReused {
                        operation: event.operation().to_string(),
                    });
                }
                answers.push(Answer::Appended(position));
                continue;
            }
            projection.apply(event, next)?;
            next = next
                .checked_add(1)
                .ok_or_else(|| IdentityError::LogUnavailable {
                    reason: "batch exceeds the leaf index range".to_owned(),
                })?;
            let position = signed.len();
            signed.push(sign_event(event.clone(), &self.key)?);
            planned.insert(event.operation(), position);
            answers.push(Answer::Appended(position));
        }
        let coordinates = match self.log.append_batch(&signed) {
            Ok(coordinates) => coordinates,
            Err(failure) => {
                if self.log.is_uncertain() {
                    self.settle()?;
                }
                return Err(failure);
            }
        };
        let receipts: Vec<Receipt> = signed
            .iter()
            .zip(coordinates)
            .map(|(event, coordinate)| Receipt::of(event, coordinate))
            .collect::<Result<_, _>>()?;
        self.projection = projection;
        self.folded = next;
        self.snapshot();
        answers
            .into_iter()
            .map(|answer| match answer {
                Answer::Recorded(receipt) => Ok(*receipt),
                Answer::Appended(position) => {
                    receipts
                        .get(position)
                        .cloned()
                        .ok_or_else(|| IdentityError::LogUnavailable {
                            reason: "batch log omitted an admitted event's coordinate".to_owned(),
                        })
                }
            })
            .collect()
    }
}
