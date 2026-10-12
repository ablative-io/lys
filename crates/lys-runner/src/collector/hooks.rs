//! Passive hooks bind sources and report boundaries for terminal sessions.

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

use super::{boundary, plain, slug, text};
use crate::error::RunnerError;
use crate::session::{Sessions, Table, append};
use crate::tracking::Harness;
use crate::tracking_store::{Body, Coverage, SourceState};

impl Sessions {
    pub(super) fn hook(
        self: &Arc<Self>,
        id: &str,
        event: &str,
        input: &Value,
    ) -> Result<String, RunnerError> {
        if event == "Stop" && input.get("stop_hook_active").and_then(Value::as_bool) == Some(true) {
            return Ok("stop_hook_active: the harness is already continuing from a stop hook, so this is no boundary".to_owned());
        }
        if !matches!(
            event,
            "SessionStart"
                | "UserPromptSubmit"
                | "Stop"
                | "SessionEnd"
                | "PreCompact"
                | "PostCompact"
                | "PostToolUse"
                | "PostToolUseFailure"
        ) {
            return Err(RunnerError::refused(
                "hook_unknown",
                format!("the collector keeps no hook {event}"),
            ));
        }
        let untracked = self.hook_untracked(id, event)?;
        let words = match event {
            "SessionStart" if untracked => {
                let mut table = self.lock()?;
                append(&mut table, id, vec![boundary("session_start", None)], None)?;
                Ok("session start kept without binding a transcript".to_owned())
            }
            "SessionStart" => self.bind_claude(id, input),
            "UserPromptSubmit" => {
                let mut table = self.lock()?;
                if let Some(session) = table.sessions.get_mut(id) {
                    session.guard.idle = false;
                }
                append(&mut table, id, vec![boundary("turn_start", None)], None)?;
                Ok("a turn began".to_owned())
            }
            "Stop" | "SessionEnd" => self.hook_stop(id, event),
            "PreCompact" => {
                let mut table = self.lock()?;
                append(&mut table, id, vec![boundary("compacting", None)], None)?;
                if table
                    .sessions
                    .get(id)
                    .is_none_or(|session| session.managed.is_none())
                {
                    crate::operations::compacting(&mut table, id);
                }
                drop(table);
                self.wake();
                Ok("compacting kept".to_owned())
            }
            "PostCompact" => self.post_compact(id, input),
            _ => Ok("an outcome observation carries no usage or policy refusal".to_owned()),
        }?;
        Ok(if untracked {
            format!("hook_untracked: {event}; {words}")
        } else {
            words
        })
    }

    fn hook_untracked(&self, id: &str, event: &str) -> Result<bool, RunnerError> {
        let mut table = self.lock()?;
        let session = table
            .sessions
            .get(id)
            .ok_or_else(|| crate::session::unknown(id))?;
        let untracked = session.guard.tracking.is_none() && session.managed.is_none();
        if untracked {
            observe(
                &mut table,
                id,
                "hook_untracked",
                format!(
                    "{event}: no harness tracking is configured; terminal boundaries are kept without transcript attribution"
                ),
            )?;
        }
        Ok(untracked)
    }

    fn post_compact(self: &Arc<Self>, id: &str, input: &Value) -> Result<String, RunnerError> {
        if !input.is_object() {
            return Err(RunnerError::refused(
                "hook_malformed",
                "PostCompact input must be an object",
            ));
        }
        let mut kept = serde_json::Map::new();
        for name in ["trigger", "custom_instructions"] {
            if let Some(value) = input.get(name) {
                if !value.is_string() {
                    return Err(RunnerError::refused(
                        "hook_malformed",
                        format!("PostCompact {name} must be text"),
                    ));
                }
                kept.insert(name.to_owned(), value.clone());
            }
        }
        let words = serde_json::to_string(&kept).map_err(|error| {
            RunnerError::refused(
                "hook_malformed",
                format!("PostCompact input was not recorded: {error}"),
            )
        })?;
        self.read_source(id, None);
        let mut table = self.lock()?;
        let managed = table
            .sessions
            .get(id)
            .is_some_and(|session| session.managed.is_some());
        super::flushed(&mut table, self.runner(), id, "compacted")?;
        observe(&mut table, id, "compacted_input", words)?;
        if !managed {
            crate::operations::compacting(&mut table, id);
            if let Some(session) = table.sessions.get_mut(id) {
                session.guard.idle = true;
            }
            crate::operations::deliver(&mut table, id);
        }
        drop(table);
        self.wake();
        Ok("compacted kept".to_owned())
    }

    /// Bind session `id` to the transcript a `SessionStart` names, once it
    /// is the file its isolated configuration home keeps for that session.
    fn bind_claude(self: &Arc<Self>, id: &str, input: &Value) -> Result<String, RunnerError> {
        let unbound = |words: String| RunnerError::refused("transcript_unbound", words);
        let claude = text(input, "session_id").filter(|claude| plain(claude));
        let given = text(input, "transcript_path");
        let (Some(claude), Some(given)) = (claude, given) else {
            return Err(unbound(
                "the hook names no session id and transcript".to_owned(),
            ));
        };
        let mut table = self.lock()?;
        let session = table
            .sessions
            .get(id)
            .ok_or_else(|| crate::session::unknown(id))?;
        let tracking = session
            .guard
            .tracking
            .as_ref()
            .filter(|tracking| tracking.harness == Harness::ClaudeCode)
            .ok_or_else(|| unbound(format!("session {id} is not a tracked Claude Code session")))?;
        let launched = session
            .launch
            .as_ref()
            .map(|launch| launch.directory.clone());
        let expected = [Some(session.guard.cwd.clone()), launched]
            .into_iter()
            .flatten()
            .map(|dir| {
                Path::new(&tracking.config_home)
                    .join("projects")
                    .join(slug(&dir))
                    .join(format!("{claude}.jsonl"))
            })
            .any(|path| path == Path::new(given));
        if !expected {
            let words = format!(
                "{given} is not the transcript the session's configuration home keeps for {claude}"
            );
            let refused = SourceState {
                path: given.to_owned(),
                ..SourceState::default()
            };
            let coverage = Coverage::of("source_refused", &refused, None, words.clone());
            append(&mut table, id, vec![Body::Coverage(coverage)], None)?;
            return Err(unbound(words));
        }
        self.bind(&mut table, id, (given, claude), false)
    }
}

fn observe(table: &mut Table, id: &str, state: &str, words: String) -> Result<(), RunnerError> {
    let source = table.feed.source(id);
    let coverage = Coverage {
        state: state.to_owned(),
        source: source.map(|source| source.path.clone()),
        generation: source.map_or(0, |source| source.generation),
        offset: source.map(|source| source.offset),
        words,
        executable: None,
        harness_version: None,
        adapter: None,
    };
    append(table, id, vec![Body::Coverage(coverage)], None)
}
