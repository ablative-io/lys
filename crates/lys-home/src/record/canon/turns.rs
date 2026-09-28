//! The turns of an authored example, read from a file.

use std::path::Path;

use crate::error::HomeError;

/// A turn's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// The person's turn.
    User,
    /// The assistant's turn.
    Assistant,
}

/// Parse a turns file: `user: text` or `assistant: text` per line, blank
/// lines skipped. A line that names another role, has no role, or holds a
/// thinking block is refused by line number: thinking is never authored.
pub fn parse_turns(path: &Path) -> Result<Vec<(Role, String)>, HomeError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| HomeError::io("reading the turns file", path, e))?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let malformed = |what: &'static str, reason: &str| HomeError::Malformed {
            path: path.to_path_buf(),
            line: n + 1,
            what,
            reason: reason.to_owned(),
        };
        let (role, body) = line.split_once(": ").ok_or_else(|| {
            malformed(
                "turn (`user: text` or `assistant: text`)",
                "no `role: ` prefix",
            )
        })?;
        if body.contains("\"type\":\"thinking\"") || body.contains("\"type\": \"thinking\"") {
            return Err(malformed("turn", "a thinking block is never authored"));
        }
        let role = match role {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            _ => return Err(malformed("turn", "role must be user or assistant")),
        };
        out.push((role, body.to_owned()));
    }
    if out.is_empty() {
        return Err(HomeError::Malformed {
            path: path.to_path_buf(),
            line: 0,
            what: "turns file",
            reason: "no turns".to_owned(),
        });
    }
    Ok(out)
}
