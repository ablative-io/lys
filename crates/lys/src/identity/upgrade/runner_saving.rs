//! Private resume records survive an interrupted binary replacement.

use std::collections::BTreeMap;
use std::path::Path;

use lys_runner::seat_owner::{sessions::OwnedSeat, spawn::OwnerPlan};
use lys_runner::session::upgrade::UpgradeSession;
use lys_runner::{Act, Answer, Client};
use serde::{Deserialize, Serialize};

use super::super::super::error::IdentityResult;
use super::super::super::private_files;
use super::{Restart, refuse, status};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    seat: OwnedSeat,
    agent: String,
    directory: String,
    harness: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    sessions: Vec<UpgradeSession>,
    owners: Vec<Owner>,
}

fn read(path: &Path) -> IdentityResult<Kept> {
    match private_files::read(path)? {
        Some(bytes) => serde_json::from_slice(&bytes)
            .map_err(|error| refuse("runner_resume_record_unreadable", error, path)),
        None => Ok(Kept::default()),
    }
}

fn write(path: &Path, saved: &Kept) -> IdentityResult<()> {
    let bytes = serde_json::to_vec(saved)
        .map_err(|error| refuse("runner_resume_record_unwritable", error, path))?;
    private_files::write(path, &bytes).map(drop)
}

fn named(saved: &UpgradeSession) -> String {
    format!(
        "agent {} session {} folder {:?} harness {:?} conversation {}",
        saved.agent.as_deref().unwrap_or("unassigned"),
        saved.launch.session,
        saved.launch.directory,
        saved
            .launch
            .config
            .as_ref()
            .and_then(|config| config.harness),
        saved.conversation.as_deref().unwrap_or("not yet observed")
    )
}

impl Restart {
    /// Release a capture when the transaction cannot begin.
    pub fn cancel(&self) -> IdentityResult<()> {
        match self.client.ask(&Act::CancelUpgrade) {
            Ok(Answer::UpgradeCancelled) => Ok(()),
            answer => Err(refuse(
                "runner_upgrade_cancel_failed",
                format!("the start fence could not be cancelled: {answer:?}"),
                &self.socket,
            )),
        }
    }

    /// Retain every direct launch and independent owner before stopping any service.
    pub fn retain(&self) -> IdentityResult<()> {
        if self.exit.exited()? {
            return Ok(());
        }
        let before = status(&self.client, &self.socket)?;
        let sessions = match self.client.ask(&Act::UpgradeSessions) {
            Ok(Answer::UpgradeSessions { sessions }) => sessions,
            Ok(Answer::Refused { words, .. })
                if words.contains("unknown variant")
                    && words.contains("upgrade_sessions")
                    && before
                        .sessions
                        .iter()
                        .all(|session| session.ended.is_some()) =>
            {
                Vec::new()
            }
            answer => {
                return Err(refuse(
                    "runner_upgrade_capture_unavailable",
                    format!(
                        "running launches could not be retained; no process was stopped: {answer:?}"
                    ),
                    &self.socket,
                ));
            }
        };
        let result = self.retain_with(sessions);
        if result.is_err() {
            match self.client.ask(&Act::CancelUpgrade) {
                Ok(Answer::UpgradeCancelled) => {}
                answer => {
                    return Err(refuse(
                        "runner_upgrade_cancel_failed",
                        format!("{result:?}; the start fence could not be cancelled: {answer:?}"),
                        &self.socket,
                    ));
                }
            }
        }
        result
    }

    fn retain_with(&self, sessions: Vec<UpgradeSession>) -> IdentityResult<()> {
        let mut saved = read(&self.saved_path)?;
        let mut by_id: BTreeMap<_, _> = saved
            .sessions
            .into_iter()
            .map(|saved| (saved.launch.session.clone(), saved))
            .collect();
        for session in sessions {
            by_id.insert(session.launch.session.clone(), session);
        }
        saved.sessions = by_id.into_values().collect();
        let owners = match self.client.ask(&Act::Owned) {
            Ok(Answer::Owned {
                owners,
                unreachable,
            }) if unreachable.is_empty() => owners,
            answer => {
                return Err(refuse(
                    "runner_owned_capture_unavailable",
                    format!(
                        "independent owners could not be confirmed; no process was stopped: {answer:?}"
                    ),
                    &self.socket,
                ));
            }
        };
        saved.owners.clear();
        for seat in owners {
            let owner = Client::new(
                seat.endpoint.socket.clone(),
                std::sync::Arc::clone(&self.key),
            );
            if status(&owner, &seat.endpoint.socket)?
                .sessions
                .iter()
                .all(|session| session.ended.is_some())
            {
                continue;
            }
            let plan = OwnerPlan::read(&seat.endpoint.dir).map_err(|error| {
                refuse(
                    "runner_owned_capture_unavailable",
                    error,
                    &seat.endpoint.dir,
                )
            })?;
            saved.owners.push(Owner {
                agent: plan.managed.launch.policy.as_ref().map_or_else(
                    || seat.binding.seat.clone(),
                    |admitted| admitted.policy.agent.clone(),
                ),
                directory: plan.managed.launch.directory,
                harness: format!("{:?}", plan.managed.transport),
                seat,
            });
        }
        write(&self.saved_path, &saved)
    }

    pub(super) fn resume_saved(&self, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
        let mut saved = read(&self.saved_path)?;
        let mut failed = Vec::new();
        for session in saved.sessions {
            let description = named(&session);
            match self.client.ask(&Act::ResumeUpgrade {
                saved: Box::new(session.clone()),
            }) {
                Ok(Answer::Started { .. }) => say(&format!("{description}: resumed")),
                Ok(Answer::Refused { refusal, words, .. }) => {
                    say(&format!("{description}: failed: {refusal}: {words}"));
                    failed.push(session);
                }
                answer => {
                    say(&format!(
                        "{description}: failed: runner_resume_unconfirmed: {answer:?}"
                    ));
                    failed.push(session);
                }
            }
        }
        saved.sessions = failed;
        let mut owners_failed = Vec::new();
        for owner in saved.owners {
            let client = Client::new(
                owner.seat.endpoint.socket.clone(),
                std::sync::Arc::clone(&self.key),
            );
            let description = format!(
                "agent {} session {} folder {:?} harness {} conversation {}",
                owner.agent,
                owner.seat.binding.session,
                owner.directory,
                owner.harness,
                owner.seat.binding.conversation
            );
            match status(&client, &owner.seat.endpoint.socket) {
                Ok(status)
                    if status.sessions.iter().any(|session| {
                        session.session == owner.seat.binding.session && session.ended.is_none()
                    }) =>
                {
                    say(&format!(
                        "{description}: resumed through its unchanged independent owner"
                    ));
                }
                answer => {
                    say(&format!(
                        "{description}: failed: runner_owned_resume_unconfirmed: {answer:?}"
                    ));
                    owners_failed.push(owner);
                }
            }
        }
        saved.owners = owners_failed;
        if saved.sessions.is_empty() && saved.owners.is_empty() {
            if self.saved_path.exists() {
                std::fs::remove_file(&self.saved_path).map_err(|error| {
                    refuse("runner_resume_record_unwritable", error, &self.saved_path)
                })?;
            }
        } else {
            write(&self.saved_path, &saved)?;
        }
        Ok(())
    }
}
