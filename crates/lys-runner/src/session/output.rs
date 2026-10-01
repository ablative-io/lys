//! Terminal bytes and their waiters belong to one session and generation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};

use crate::error::RunnerError;
use crate::protocol::Ended;
use crate::rotation::RotationState;
use crate::scrollback::Scrollback;

pub(crate) struct OutputHandle {
    state: Mutex<OutputState>,
    changed: Condvar,
}

pub(crate) struct OutputState {
    scrollback: Scrollback,
    ended: Option<Ended>,
    generation: u64,
    stopping: bool,
    words: Vec<String>,
    longest_word: usize,
    tripped: bool,
}

impl OutputState {
    pub(crate) fn scrollback(&self) -> &Scrollback {
        &self.scrollback
    }

    pub(crate) fn ended(&self) -> Option<Ended> {
        self.ended.clone()
    }
}

fn poisoned(error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("session_output_poisoned", error.to_string())
}

impl OutputHandle {
    pub(crate) fn new(limit: usize, ended: Option<Ended>) -> Self {
        Self {
            state: Mutex::new(OutputState {
                scrollback: Scrollback::new(limit),
                ended,
                generation: 0,
                stopping: false,
                words: Vec::new(),
                longest_word: 0,
                tripped: false,
            }),
            changed: Condvar::new(),
        }
    }

    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, OutputState>, RunnerError> {
        self.state.lock().map_err(poisoned)
    }

    pub(crate) fn begin(
        &self,
        generation: u64,
        rotation: Option<&RotationState>,
        tracked_claude: bool,
    ) -> Result<(), RunnerError> {
        let mut state = self.lock()?;
        state.generation = generation;
        state.ended = None;
        state.stopping = false;
        state.words = if tracked_claude {
            Vec::new()
        } else {
            rotation
                .and_then(RotationState::words)
                .unwrap_or_default()
                .to_vec()
        };
        state.longest_word = state.words.iter().map(String::len).max().unwrap_or(0);
        state.tripped = false;
        self.changed.notify_all();
        Ok(())
    }

    pub(crate) fn push(&self, generation: u64, bytes: &[u8]) -> Result<bool, RunnerError> {
        let mut state = self.lock()?;
        if state.generation != generation || state.ended.is_some() {
            return Ok(false);
        }
        state.scrollback.push(bytes);
        let trip = if state.tripped || state.words.is_empty() {
            false
        } else {
            let lookback = bytes
                .len()
                .saturating_add(state.longest_word.saturating_sub(1));
            let from = state
                .scrollback
                .end()
                .saturating_sub(lookback as u64)
                .max(state.scrollback.oldest());
            let bytes = state.scrollback.from(from)?;
            let text = String::from_utf8_lossy(&bytes);
            state.words.iter().any(|word| text.contains(word))
        };
        state.tripped |= trip;
        self.changed.notify_all();
        Ok(trip)
    }

    pub(crate) fn finish(&self, generation: u64, ended: Ended) -> Result<(), RunnerError> {
        let mut state = self.lock()?;
        if state.generation == generation {
            state.ended = Some(ended);
            self.changed.notify_all();
        }
        Ok(())
    }

    pub(crate) fn wake(&self) -> Result<(), RunnerError> {
        let state = self.lock()?;
        self.changed.notify_all();
        drop(state);
        Ok(())
    }

    pub(crate) fn stop(&self) -> Result<(), RunnerError> {
        let mut state = self.lock()?;
        state.stopping = true;
        self.changed.notify_all();
        Ok(())
    }

    pub(crate) fn until<T>(
        &self,
        id: &str,
        left: &AtomicBool,
        mut check: impl FnMut(&mut OutputState, &str) -> Option<Result<T, RunnerError>>,
    ) -> Result<T, RunnerError> {
        let mut state = self.lock()?;
        loop {
            if let Some(answer) = check(&mut state, id) {
                return answer;
            }
            if left.load(Ordering::SeqCst) {
                return Err(RunnerError::refused(
                    "caller_left",
                    "the caller closed the request",
                ));
            }
            if state.stopping {
                return Err(RunnerError::refused(
                    "runner_stopping",
                    "the runner is stopping",
                ));
            }
            state = self.changed.wait(state).map_err(poisoned)?;
        }
    }
}
