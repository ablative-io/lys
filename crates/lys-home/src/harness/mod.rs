//! Harness profiles: how a harness's own transcript is read into the home and
//! how a home is rendered back into the file that harness resumes from. One
//! profile per harness, each measured on its own; Claude Code first, then the
//! one translated pair, Claude Code to Codex 0.156.0 (HOME-009).

pub mod claude_code;
/// The Codex profile: a session imported from Claude Code translated into a
/// rollout in the shape Codex 0.156.0 writes for its own threads, with a
/// loss account beside it.
pub mod codex;
