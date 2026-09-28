//! The `resume-check` command: what a fork of a rendered transcript added,
//! measured by record and by `tool_use` id.

use std::path::Path;

use serde_json::Value;

use crate::error::HomeError;

/// What a resume check measured. `repeated_tool_use_ids` is exactly that: ids
/// that appear more times in the fork than in the rendered file; it says
/// nothing about an action repeated under a fresh id. `new_tool_uses` counts
/// every `tool_use` part in the fork's own records (those not copied from the
/// rendered file), which for a one-turn question answerable without tools is
/// expected to be 0.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct ResumeReport {
    /// Records in the rendered file.
    pub rendered_records: u64,
    /// Records in the fork.
    pub forked_records: u64,
    /// Records in the fork whose uuid is not in the rendered file.
    pub forked_new_records: u64,
    /// `tool_use` ids appearing more times in the fork than in the rendered file.
    pub repeated_tool_use_ids: u64,
    /// `tool_use` parts in the fork's new records.
    pub new_tool_uses: u64,
}

/// One record's uuid and its `tool_use` ids.
fn tool_uses(path: &Path) -> Result<Vec<(String, Vec<String>)>, HomeError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| HomeError::io("reading a transcript", path, e))?;
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line).map_err(|e| HomeError::Malformed {
            path: path.to_path_buf(),
            line: n + 1,
            what: "Claude Code record",
            reason: e.to_string(),
        })?;
        let uuid = v
            .get("uuid")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let mut ids = Vec::new();
        if let Some(parts) = v
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(Value::as_array)
        {
            for p in parts {
                if p.get("type").and_then(Value::as_str) == Some("tool_use")
                    && let Some(id) = p.get("id").and_then(Value::as_str)
                {
                    ids.push(id.to_owned());
                }
            }
        }
        out.push((uuid, ids));
    }
    Ok(out)
}

pub(crate) fn resume_check(rendered: &Path, forked: &Path) -> Result<ResumeReport, HomeError> {
    let before = tool_uses(rendered)?;
    let after = tool_uses(forked)?;
    let rendered_uuids: std::collections::BTreeSet<&str> =
        before.iter().map(|(u, _)| u.as_str()).collect();
    let count = |records: &[(String, Vec<String>)], id: &str| -> u64 {
        u64::try_from(
            records
                .iter()
                .flat_map(|(_, ids)| ids.iter())
                .filter(|x| x.as_str() == id)
                .count(),
        )
        .unwrap_or(u64::MAX)
    };
    // A tool action the fork *inherited* by copying the rendered records is not a repeat;
    // a repeat is an id that appears more times in the fork than in the rendered file.
    let mut repeated = 0u64;
    for id in before
        .iter()
        .flat_map(|(_, ids)| ids.iter())
        .collect::<std::collections::BTreeSet<_>>()
    {
        repeated += count(&after, id).saturating_sub(count(&before, id));
    }
    let new: Vec<&(String, Vec<String>)> = after
        .iter()
        .filter(|(u, _)| !rendered_uuids.contains(u.as_str()))
        .collect();
    Ok(ResumeReport {
        rendered_records: before.len() as u64,
        forked_records: after.len() as u64,
        forked_new_records: new.len() as u64,
        repeated_tool_use_ids: repeated,
        new_tool_uses: new.iter().map(|(_, ids)| ids.len() as u64).sum(),
    })
}
