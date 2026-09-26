//! The Claude Code profile.
//!
//! Claude Code 2.1.281 writes one JSONL file per session under
//! `~/.claude/projects/<cwd-slug>/<session id>.jsonl`, where the slug is the
//! working directory with every `/` replaced by `-`. Every record carries
//! `uuid`, `parentUuid`, `type`, `timestamp`, `sessionId` and `cwd`; `user` and
//! `assistant` records carry a `message`. The file is already a tree.
//!
//! Measured on 24 September 2026: `claude --resume <path>` resumes directly
//! from a file anywhere on disk (Waffles 13:36, Archie 13:37) and writes its
//! continuation beside that file as `<sessionId>.jsonl`, leaving the passed
//! file unchanged.
//!
//! A launch therefore resumes with `--fork-session`: a bare resume writes its
//! continuation as `<sessionId>.jsonl` beside the passed file, and under the
//! render that is the rendered file's own name (its session id is the chosen
//! uuid), so a bare resume would write onto the rendered file and break the
//! hash recorded for it. With `--fork-session` the continuation lands under
//! `~/.claude/projects/<cwd-slug>/` for the run's working directory, under
//! the uuid Claude Code assigns at the fork (PROOF-FEWSHOT.md measured the
//! bare case on 2.1.281, PROOF-RESUME.md the fork on 2.1.281, and
//! PROOF-LAUNCH.md the launch line on 2.1.283).

pub mod events;
#[cfg(test)]
mod events_tests;
pub mod import;
#[cfg(test)]
pub(crate) mod import_tests;
pub mod launch;
pub mod launch_env;
pub mod render;
#[cfg(test)]
mod render_tests;
pub mod template;
#[cfg(test)]
mod template_tests;

/// The harness name as it appears in lys entries.
pub const HARNESS: &str = "claude-code";
/// The provider Claude Code talks to.
pub const PROVIDER: &str = "anthropic";
/// The api Claude Code speaks.
pub const API: &str = "anthropic-messages";
/// The model value that marks a hand-authored turn.
pub const AUTHORED: &str = "authored";

/// The directory Claude Code keeps a session in for a working directory.
#[must_use]
pub fn projects_slug(cwd: &str) -> String {
    cwd.replace('/', "-")
}
