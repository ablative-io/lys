//! Caller cancellation closes its pending runner connection.
use super::{Hook, RunnerError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// A caller's leaving: what closes the request it was waiting on.
#[derive(Default)]
pub struct Leave {
    left: AtomicBool,
    pub(super) hook: Mutex<Option<Hook>>,
}

impl Leave {
    /// Hold `hook` to run when the caller leaves; run at once when it has.
    ///
    /// # Errors
    /// Returns a refusal when the caller cancellation lock is unavailable.
    pub fn hold(&self, hook: Hook) -> Result<(), RunnerError> {
        let mut held = match self.hook.lock() {
            Ok(held) => held,
            Err(error) => {
                hook();
                return Err(RunnerError::Unreachable {
                    reason: format!("caller leave hook unavailable: {error}"),
                });
            }
        };
        if self.left.load(Ordering::SeqCst) {
            drop(held);
            hook();
            return Ok(());
        }
        *held = Some(hook);
        Ok(())
    }

    /// The caller left.
    ///
    /// # Errors
    /// Returns a refusal when the caller cancellation lock is unavailable.
    pub fn leave(&self) -> Result<(), RunnerError> {
        self.left.store(true, Ordering::SeqCst);
        let hook = self
            .hook
            .lock()
            .map_err(|error| RunnerError::Unreachable {
                reason: format!("caller leave hook unavailable: {error}"),
            })?
            .take();
        if let Some(hook) = hook {
            hook();
        }
        Ok(())
    }

    /// Whether the caller has left.
    pub fn left(&self) -> bool {
        self.left.load(Ordering::SeqCst)
    }

    /// The request was answered: nothing is left to close.
    ///
    /// # Errors
    /// Returns a refusal when the caller cancellation lock is unavailable.
    pub fn done(&self) -> Result<(), RunnerError> {
        drop(
            self.hook
                .lock()
                .map_err(|error| RunnerError::Unreachable {
                    reason: format!("caller leave hook unavailable: {error}"),
                })?
                .take(),
        );
        Ok(())
    }
}

/// Leaves when dropped: an HTTP handler holds one while it waits, so a
/// caller that goes drops it.
pub struct LeaveOnDrop(pub Arc<Leave>);

impl Drop for LeaveOnDrop {
    fn drop(&mut self) {
        if let Err(error) = self.0.leave() {
            tracing::error!("runner caller leave refused: {error}");
        }
    }
}
