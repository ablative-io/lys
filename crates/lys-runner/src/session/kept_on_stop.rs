//! Sessions survive the runner's own stop: a service restart, an update, a
//! reboot. When the runner is asked to stop it keeps every live session's
//! admitted start, with the native conversation it observed, in one private
//! file beside its record of sessions; when it next starts it resumes each,
//! in the same folder and the same conversation, through the same handover
//! the identity upgrade uses ([`Sessions::prepare_upgrade`] and
//! [`Sessions::resume_upgrade`]). A session a person stopped is never
//! restarted, and every outcome is said by name.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::Sessions;
use super::upgrade::UpgradeSession;
use crate::error::{RunnerError, said};

/// The file the kept starts are written to, inside the runner's state directory.
pub const KEPT: &str = "resume-on-start.json";

fn path(state: &Path) -> PathBuf {
    state.join(KEPT)
}

fn unkept(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("runner_resume_record_unavailable", what.to_string())
}

fn read(file: &Path) -> Result<Vec<UpgradeSession>, RunnerError> {
    match fs::read(file) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| unkept(format!("{} does not read: {error}", file.display()))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(unkept(format!(
            "{} could not be read: {error}",
            file.display()
        ))),
    }
}

fn write(file: &Path, kept: &[UpgradeSession]) -> Result<(), RunnerError> {
    let bytes = serde_json::to_vec(kept).map_err(unkept)?;
    let partial = file.with_extension("json.partial");
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&partial)
        .map_err(|error| {
            unkept(format!(
                "{} could not be written: {error}",
                partial.display()
            ))
        })?;
    out.write_all(&bytes)
        .and_then(|()| out.sync_all())
        .map_err(|error| {
            unkept(format!(
                "{} could not be written: {error}",
                partial.display()
            ))
        })?;
    fs::rename(&partial, file)
        .map_err(|error| unkept(format!("{} could not be kept: {error}", file.display())))
}

fn named(saved: &UpgradeSession) -> String {
    format!(
        "session {} (agent {}, folder {}, conversation {})",
        saved.launch.session,
        saved.agent.as_deref().unwrap_or("none"),
        saved.launch.directory,
        saved.conversation.as_deref().unwrap_or("not yet seen")
    )
}

impl Sessions {
    /// Keep every live session's start before the runner stops, beside any
    /// kept by an earlier stop that has not been resumed yet.
    ///
    /// # Errors
    /// Refuses when the file cannot be written; a session that cannot be kept
    /// is said by name and the others are kept. The stop goes on either way.
    pub fn keep_for_restart(&self, state: &Path) -> Result<usize, RunnerError> {
        let (live, failed) = self.capture_each()?;
        for (id, error) in failed {
            said(&format!(
                "runner_session_not_kept: session {id}: {error}; it will not resume"
            ));
        }
        let file = path(state);
        let mut by_id: BTreeMap<String, UpgradeSession> = read(&file)?
            .into_iter()
            .map(|saved| (saved.launch.session.clone(), saved))
            .collect();
        let count = live.len();
        for saved in live {
            by_id.insert(saved.launch.session.clone(), saved);
        }
        let kept: Vec<UpgradeSession> = by_id.into_values().collect();
        if kept.is_empty() {
            return Ok(0);
        }
        write(&file, &kept)?;
        Ok(count)
    }

    /// Resume every session kept by the last stop, each in its own
    /// conversation. A session that cannot resume is said with its reason and
    /// kept for the next start unless a person stopped it; the file is gone
    /// once nothing is left in it.
    ///
    /// # Errors
    /// Refuses when the kept file cannot be read or rewritten.
    pub fn resume_kept(self: &Arc<Self>, state: &Path) -> Result<(), RunnerError> {
        let file = path(state);
        let kept = read(&file)?;
        if kept.is_empty() {
            return Ok(());
        }
        let mut left = Vec::new();
        for saved in kept {
            let words = named(&saved);
            match self.resume_upgrade(saved.clone()) {
                Ok((pid, _)) => said(&format!(
                    "runner_session_resumed: {words} resumed as pid {pid}"
                )),
                Err(error) if error.name() == "session_stopped_by_console" => {
                    said(&format!("runner_session_not_resumed: {words}: {error}"));
                }
                Err(error) => {
                    said(&format!(
                        "runner_session_resume_failed: {words}: {error}; kept for the next start"
                    ));
                    left.push(saved);
                }
            }
        }
        if left.is_empty() {
            fs::remove_file(&file).map_err(|error| {
                unkept(format!("{} could not be removed: {error}", file.display()))
            })
        } else {
            write(&file, &left)
        }
    }
}

#[cfg(test)]
#[path = "kept_on_stop_tests.rs"]
mod tests;
