//! The display profile, and what a profile change may hold.
//!
//! A profile is how an identity is shown. It never establishes identity: two
//! people may share a display name, and a changed name is the same person
//! (P1). It carries no email: an email is not identity either, and a profile
//! that carried one would invite matching on it.

use crate::error::IdentityError;

/// The longest display name accepted, in characters.
pub const DISPLAY_NAME_MAX_CHARS: usize = 200;

/// How an identity is shown.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Profile {
    display_name: String,
}

impl Profile {
    /// A profile showing `display_name`, refused by name if the name is not one this directory records.
    pub fn new(display_name: &str) -> Result<Self, IdentityError> {
        if display_name.trim().is_empty() {
            return Err(IdentityError::ProfileInvalid {
                reason: "the display name is empty",
            });
        }
        if display_name.trim() != display_name {
            return Err(IdentityError::ProfileInvalid {
                reason: "the display name carries surrounding whitespace",
            });
        }
        if display_name.chars().count() > DISPLAY_NAME_MAX_CHARS {
            return Err(IdentityError::ProfileInvalid {
                reason: "the display name is longer than 200 characters",
            });
        }
        if display_name.chars().any(char::is_control) {
            return Err(IdentityError::ProfileInvalid {
                reason: "the display name carries a control character",
            });
        }
        Ok(Self {
            display_name: display_name.to_owned(),
        })
    }

    /// The display name.
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}
