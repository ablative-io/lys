//! The identity settings an install takes from the deployment configuration's
//! `[identity]` table (ACCESS-002 R2), writes into the directory service's
//! configuration and records in `install/build.json`.
//!
//! `pass_lifetime` is how long a pass, the access token Lys gives a product,
//! lives, in seconds. Its default is the ten minutes ACCESS-002 R2 states
//! (Tom to confirm, design page section 7); a deployment configuration
//! without the table, as every one written before it existed is, takes it.
//! It is written into `identity.json` as `provider.pass_seconds`, and
//! `build.json` records what that file says the service runs with.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::private_files;
use super::layout::Layout;

/// A pass's lifetime when the deployment configuration names none, in
/// seconds: ten minutes, as ACCESS-002 R2 states it.
pub const PASS_LIFETIME_SECONDS: u64 = 600;

/// The `[identity]` table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySettings {
    /// How long a pass lives, in seconds; never zero.
    #[serde(default = "pass_lifetime")]
    pub pass_lifetime: u64,
}

fn pass_lifetime() -> u64 {
    PASS_LIFETIME_SECONDS
}

impl Default for IdentitySettings {
    fn default() -> Self {
        Self {
            pass_lifetime: PASS_LIFETIME_SECONDS,
        }
    }
}

/// The settings `install/build.json` records, as the written service
/// configuration states them; absent when it states none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RecordedSettings {
    /// `identity.pass_lifetime`, from `provider.pass_seconds`.
    #[serde(rename = "identity.pass_lifetime")]
    pub pass_lifetime: Option<u64>,
}

/// What the service configuration written under `layout` says the settings
/// are, or nothing for each it does not name.
pub fn recorded(layout: &Layout) -> IdentityResult<RecordedSettings> {
    let path = layout.service_config();
    let Some(bytes) = private_files::read(&path)? else {
        return Ok(RecordedSettings {
            pass_lifetime: None,
        });
    };
    let written: Value = serde_json::from_slice(&bytes).map_err(|error| {
        IdentityError::new(
            ErrorKind::ConfigInvalid,
            "read",
            "identity.json",
            error.to_string(),
        )
        .at(&path)
    })?;
    Ok(RecordedSettings {
        pass_lifetime: written
            .get("provider")
            .and_then(|provider| provider.get("pass_seconds"))
            .and_then(Value::as_u64),
    })
}
