//! Owned starts: a session begun for a verified responsible person, whose
//! input then needs a verified caller and a live grant (see
//! [`crate::legacy_input`]).

use std::sync::Arc;

use crate::error::RunnerError;
use crate::judge::Policy;
use crate::protocol::Launch;
use crate::tracking::Tracking;

use super::Sessions;

impl Sessions {
    /// Start with a verified responsible person supplied by the admitting caller.
    ///
    /// # Errors
    /// Returns invalid responsibility, launch or persistence errors by name.
    pub fn start_for(
        self: &Arc<Self>,
        launch: Launch,
        responsible: &str,
    ) -> Result<(u32, u64), RunnerError> {
        self.begin_for(launch, None, None, responsible)
    }

    /// Begin an owned session without deriving a person from a server key.
    ///
    /// # Errors
    /// Returns invalid responsibility, tracking, launch or persistence errors by name.
    pub fn begin_for(
        self: &Arc<Self>,
        launch: Launch,
        policy: Option<Policy>,
        tracking: Option<Tracking>,
        responsible: &str,
    ) -> Result<(u32, u64), RunnerError> {
        if responsible.is_empty()
            || responsible.trim() != responsible
            || responsible.chars().any(char::is_control)
            || responsible == "lys"
        {
            return Err(RunnerError::refused(
                "SessionResponsibleInvalid",
                "a verified responsible person is required",
            ));
        }
        self.begin_owned(launch, policy, tracking, Some(responsible.to_owned()))
    }
}
