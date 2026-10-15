//! One announced return owns one connection attempt, shared by both delivery lanes.

use super::{RetryWait, now};
use crate::RunnerError;

pub(super) struct Plan {
    expected: u64,
    connecting: bool,
    result: Option<Result<String, (String, String)>>,
}

impl RetryWait {
    pub(crate) fn resume_at(
        &self,
        expected: u64,
        take: impl FnOnce() -> Result<String, RunnerError>,
    ) -> Result<String, RunnerError> {
        self.resume_at_with(expected, take, |expected| {
            let current = now()?;
            if expected > current {
                self.wait(std::time::Duration::from_millis(expected - current))?;
            }
            Ok(())
        })
    }

    fn resume_at_with(
        &self,
        expected: u64,
        take: impl FnOnce() -> Result<String, RunnerError>,
        at_return: impl FnOnce(u64) -> Result<(), RunnerError>,
    ) -> Result<String, RunnerError> {
        let mut held = self.state.lock().map_err(lock_failed)?;
        loop {
            if held.stopped {
                return Err(RunnerError::refused(
                    "runner_dial_stopped",
                    "the dial bridge has ended",
                ));
            }
            let plan = held.plan.get_or_insert(Plan {
                expected,
                connecting: false,
                result: None,
            });
            if plan.expected != expected && !plan.connecting {
                *plan = Plan {
                    expected,
                    connecting: false,
                    result: None,
                };
            }
            if let Some(result) = &plan.result {
                return result
                    .clone()
                    .map_err(|(name, words)| RunnerError::refused(&name, words));
            }
            if !plan.connecting {
                plan.connecting = true;
                break;
            }
            held = self.changed.wait(held).map_err(lock_failed)?;
        }
        drop(held);
        let result = at_return(expected).and_then(|()| take());
        let remembered = result
            .as_ref()
            .cloned()
            .map_err(|error| (error.name(), error.to_string()));
        let mut held = self.state.lock().map_err(lock_failed)?;
        held.plan = Some(Plan {
            expected,
            connecting: false,
            result: Some(remembered),
        });
        self.changed.notify_all();
        result
    }
}

fn lock_failed(error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("runner_planned_return_failed", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn an_announced_return_dials_at_its_instant_and_uses_no_failed_attempt_backoff()
    -> Result<(), RunnerError> {
        let clock = Cell::new(0);
        let dials = Cell::new(0);
        let returned = Cell::new(0);
        let wait = RetryWait::default();
        let epoch = wait.resume_at_with(
            1000,
            || {
                assert_eq!(
                    clock.get(),
                    1000,
                    "the runner dialled before the announced return event"
                );
                dials.set(dials.get() + 1);
                Ok("epoch".to_owned())
            },
            |expected| {
                clock.set(expected);
                returned.set(returned.get() + 1);
                Ok(())
            },
        )?;
        let second = wait.resume_at_with(
            1000,
            || {
                dials.set(dials.get() + 1);
                Ok("another-epoch".to_owned())
            },
            |_| {
                returned.set(returned.get() + 1);
                Ok(())
            },
        )?;
        assert_eq!(second, epoch);
        assert_eq!(epoch, "epoch");
        assert_eq!(dials.get(), 1);
        assert_eq!(returned.get(), 1);
        Ok(())
    }
}
