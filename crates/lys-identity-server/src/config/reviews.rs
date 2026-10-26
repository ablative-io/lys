//! The review cadence is a written setting, never a runtime default.
use serde::Deserialize;

use crate::error::ServerError;
use super::invalid;

/// How often a standing grant needs the person's check.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSettings {
    /// A positive whole number of days since the last check.
    pub period_days: u32,
}

impl ReviewSettings {
    pub(crate) fn validate(&self) -> Result<(), ServerError> {
        if self.period_days == 0 {
            return Err(invalid(
                "[reviews] period_days must be a whole number of days of at least 1",
            ));
        }
        Ok(())
    }
}

pub(super) fn written(text: &str) -> Result<(), ServerError> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| invalid(format!("configuration: {error}")))?;
    let reviews = value
        .get("reviews")
        .ok_or_else(|| invalid("[reviews] period_days is missing"))?;
    let settings: ReviewSettings = serde_json::from_value(reviews.clone())
        .map_err(|error| invalid(format!("[reviews] period_days: {error}")))?;
    settings.validate()
}

#[cfg(test)]
#[path = "reviews_tests.rs"]
mod tests;
