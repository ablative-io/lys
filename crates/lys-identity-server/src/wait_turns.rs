//! A subject claims its turn under a short lock and waits on its predecessor's
//! recorded outcome. The claim holds no lock while its owner calls a service.

use crate::error::ServerError;
use std::collections::BTreeMap;
use std::sync::Mutex;
use tokio::sync::watch;

/// Independently claimed subjects; empty entries disappear on settlement or drop.
#[derive(Default)]
pub struct Turns(Mutex<BTreeMap<String, watch::Sender<()>>>);

/// A recorded claim, withdrawn by the request's own drop.
pub struct Claim<'a> {
    turns: &'a Turns,
    subject: String,
}

impl Turns {
    /// Claim a subject or wait for the subject's current outcome to be recorded.
    ///
    /// # Errors
    /// Refuses unavailable claim records.
    pub async fn take(&self, subject: &str) -> Result<Claim<'_>, ServerError> {
        loop {
            let mut previous = {
                let mut held = self
                    .0
                    .lock()
                    .map_err(|error| ServerError::WaitsUnavailable {
                        reason: error.to_string(),
                    })?;
                if let Some(previous) = held.get(subject) {
                    previous.subscribe()
                } else {
                    let (outcome, receiving) = watch::channel(());
                    drop(receiving);
                    held.insert(subject.to_owned(), outcome);
                    return Ok(Claim {
                        turns: self,
                        subject: subject.to_owned(),
                    });
                }
            };
            // Removing the predecessor's sender is its settled/withdrawn event.
            match previous.changed().await {
                Ok(()) | Err(_) => {}
            }
        }
    }
}

impl Drop for Claim<'_> {
    fn drop(&mut self) {
        match self.turns.0.lock() {
            Ok(mut held) => {
                held.remove(&self.subject);
            }
            Err(error) => tracing::error!("claim_withdrawal_failed: {error}"),
        }
    }
}
