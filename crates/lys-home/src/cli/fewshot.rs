//! The `fewshot` command: an authored transcript written in Claude Code's
//! shape from a file of turns.

use std::io::Write;
use std::path::Path;

use rand::RngCore;
use serde_json::json;

use crate::error::HomeError;
use crate::harness::claude_code::AUTHORED;
use crate::harness::claude_code::render::uuid_string;
use crate::record::canon::Role;
use crate::record::now;

/// Write the authored file: user records as plain strings, assistant records
/// with model `authored`, the parent chain intact, a fresh session id.
pub(crate) fn fewshot(out: &Path, turns: &Path, cwd: &str) -> Result<u64, HomeError> {
    let turns = crate::record::canon::parse_turns(turns)?;
    let session_id = random_uuid();
    let mut prev: Option<String> = None;
    let mut lines = Vec::new();
    for (n, (role, body)) in turns.into_iter().enumerate() {
        let uuid = random_uuid();
        let (kind, message) = match role {
            Role::User => ("user", json!({"role": "user", "content": body})),
            Role::Assistant => (
                "assistant",
                json!({"id": format!("msg_authored_{n}"), "type": "message", "role": "assistant", "model": AUTHORED,
                "content": [{"type": "text", "text": body}], "stop_reason": "end_turn", "stop_sequence": null,
                "usage": {"input_tokens": 0, "output_tokens": 0}}),
            ),
        };
        lines.push(json!({
            "parentUuid": prev, "isSidechain": false, "userType": "external", "cwd": cwd,
            "sessionId": session_id, "version": "2.1.281", "gitBranch": "", "uuid": uuid,
            "timestamp": now(), "type": kind, "message": message,
        }));
        prev = Some(uuid);
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)
            .map_err(|e| HomeError::io("creating the output directory", dir, e))?;
    }
    let mut body = String::new();
    for l in &lines {
        body.push_str(&serde_json::to_string(l).map_err(|source| HomeError::Json {
            context: "a record could not be serialised",
            source,
        })?);
        body.push('\n');
    }
    // Never over an existing transcript: the create is exclusive, so a second
    // writer between a check and a write cannot slip in; then the file and its
    // directory are synced before the path is reported.
    let mut file = match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out)
    {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(HomeError::Exists {
                path: out.to_path_buf(),
            });
        }
        Err(e) => return Err(HomeError::io("creating the authored file", out, e)),
    };
    file.write_all(body.as_bytes())
        .map_err(|e| HomeError::io("writing the authored file", out, e))?;
    file.sync_all()
        .map_err(|e| HomeError::io("syncing the authored file", out, e))?;
    if let Some(dir) = out.parent() {
        crate::record::blocks::sync_dir(dir)?;
    }
    Ok(lines.len() as u64)
}

/// A fresh id in Claude Code's uuid shape: 16 random bytes with the version
/// nibble 4 and the variant nibble 8, written by [`uuid_string`].
fn random_uuid() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x0f) | 0x80;
    uuid_string(&bytes)
}
