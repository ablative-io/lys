//! Harness profiles: how a harness's own transcript is read into the home and
//! how a home is rendered back into the file that harness resumes from. One
//! profile per harness, each measured on its own; Claude Code first, then
//! the one translation from Claude Code to Codex.

pub mod claude_code;
pub mod codex;
