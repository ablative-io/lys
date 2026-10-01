//! Usage is validated before an event is charged or an act is requested.

use crate::budgets_state::Usage;
use crate::error::ServerError;

/// Validate reported fields before charging any event or asking any act.
pub fn checked(usage: &Usage) -> Result<(), ServerError> {
    if usage.event.is_empty()
        || usage.at_ms < 0
        || usage.context_percent.is_some_and(|value| value > 100)
        || usage.account.as_ref().is_some_and(String::is_empty)
    {
        return Err(ServerError::RequestMalformed { reason: "usage names an event and nonnegative observation instant, context is 0 to 100, and a reported account is nonempty".to_owned() });
    }
    if let Some(windows) = &usage.plan_windows {
        for (index, window) in windows.iter().enumerate() {
            if windows[..index]
                .iter()
                .any(|earlier| earlier.duration_minutes == window.duration_minutes)
                || window
                    .duration_minutes
                    .checked_mul(60_000)
                    .is_none_or(|duration| window.resets_at_ms < duration)
                || window.duration_minutes == 0
                || window.resets_at_ms > i64::MAX.unsigned_abs()
                || lys_runner::tracking_budget::percent(&serde_json::Value::Number(
                    window.used_percent.clone(),
                ))
                .is_none()
            {
                return Err(ServerError::RequestMalformed { reason: "a reported plan window names a positive duration, representable reset instant and percentage from 0 to 100".to_owned() });
            }
        }
    }
    Ok(())
}
