//! The names Claude Code's entries carry in lys: the harness, the provider,
//! the api and the model value that marks a hand-authored turn.

/// The harness name as it appears in lys entries.
pub const HARNESS: &str = "claude-code";
/// The provider Claude Code talks to.
pub const PROVIDER: &str = "anthropic";
/// The api Claude Code speaks.
pub const API: &str = "anthropic-messages";
/// The model value that marks a hand-authored turn.
pub const AUTHORED: &str = "authored";
