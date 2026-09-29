//! Harness profiles: how a harness's own transcript is read into the home and
//! how a home is rendered back into the file that harness resumes from. One
//! profile per harness, each measured on its own; Claude Code first, then
//! the one translation from Claude Code to Codex.

pub mod claude_code;
/// The Codex profile: a home session translated into a rollout in the shape
/// Codex 0.156.0 writes and reads for its own threads, with a loss account
/// beside it (HOME-009). The translation is a fork of the session, never the
/// same session: the rollout opens with an in-band marker saying so, and the
/// durable link back is the account and a `lys.translation` side leaf.
pub mod codex;
/// The pinned model, permission, transport and rendering capabilities.
pub mod description;
/// What a recorded profile gives a launch, in words neither harness owns:
/// the declared build, the models, the MCP servers with their settings,
/// handles and channel policy, and the kept skills (HOME-037).
pub mod launch_fields;
mod launch_fields_tests;
/// The native rendering registry, addressed by description contract identifier.
pub mod rendering;
/// Kept skills written into a session's own config directory, each checked
/// against the hash Lys keeps it by (HOME-037 R3).
pub mod skills;
mod skills_tests;

mod rendering_permissions;
