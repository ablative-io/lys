//! One managed harness channel per owned session. Manual PTYs do not supply
//! control boundaries. Event consumers share the runner's existing durable
//! tracking feed and cursor, never a second reader of the harness process.

pub mod claude;
pub mod codex;
pub mod dispatcher;
pub mod events;
pub mod process;
