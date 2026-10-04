//! Who stopped a session and why, as its end records it.

use serde::{Deserialize, Serialize};

/// The words a stop of everything left on each session it ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stopped {
    /// Who pulled the cord: the person the server verified, or the runner
    /// itself when it was asked to stop by a signal.
    pub by: String,
    /// Why, in their words.
    pub reason: String,
    /// When the cord was pulled, in milliseconds since the Unix epoch.
    pub at: u64,
}
