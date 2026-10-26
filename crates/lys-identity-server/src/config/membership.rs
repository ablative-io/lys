//! Written membership page bounds.
use serde::Deserialize;

use super::invalid;
use crate::error::ServerError;

/// The operator's bounds on channel membership pages: the most rows and
/// bytes any page may ask for, and how long and how many continuation
/// cursors are kept. No member has a default; each must be positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MembershipSettings {
    /// The most rows a page may ask for.
    pub page_rows_max: u32,
    /// The most bytes a page's rows may encode to, as JSON.
    pub page_bytes_max: u32,
    /// How long an issued cursor is kept, in seconds; after it the cursor is
    /// refused `membership_cursor_expired`.
    pub cursor_seconds: u64,
    /// The most cursors kept at once; a page that would need one more is
    /// refused `membership_cursors_full`.
    pub cursors_max: u32,
}

impl MembershipSettings {
    /// Refuse a setting of zero, by its member's name.
    ///
    /// # Errors
    /// Refuses a zero membership setting, naming the setting.
    pub fn validate(&self) -> Result<(), ServerError> {
        for (name, value) in [
            ("page_rows_max", u64::from(self.page_rows_max)),
            ("page_bytes_max", u64::from(self.page_bytes_max)),
            ("cursor_seconds", self.cursor_seconds),
            ("cursors_max", u64::from(self.cursors_max)),
        ] {
            if value == 0 {
                return Err(invalid(format!("membership.{name} is zero")));
            }
        }
        Ok(())
    }
}
