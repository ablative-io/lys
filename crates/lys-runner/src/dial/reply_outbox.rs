//! Completed acts remain on disk until the server acknowledges their replies.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use crate::RunnerError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reply {
    pub(super) epoch: String,
    pub(super) ticket: String,
    pub(super) body: String,
}

pub(super) struct Outbox {
    root: PathBuf,
    lease: File,
    writing: std::sync::Mutex<()>,
}

fn failed(action: &str, error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(
        "runner_reply_outbox_failed",
        format!("could not {action} the completed reply: {error}"),
    )
}

impl Outbox {
    pub(super) fn open(socket: &Path) -> Result<Self, RunnerError> {
        let root = socket.with_extension("completed-replies");
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)
            .map_err(|error| failed("open storage for", error))?;
        let metadata =
            fs::symlink_metadata(&root).map_err(|error| failed("inspect storage for", error))?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(failed(
                "open storage for",
                "storage must be an owner-only directory",
            ));
        }
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(root.join("lease"))
            .map_err(|error| failed("lock storage for", error))?;
        rustix::fs::flock(&lease, rustix::fs::FlockOperation::NonBlockingLockExclusive)
            .map_err(|error| failed("claim storage for", error))?;
        Ok(Self {
            root,
            lease,
            writing: std::sync::Mutex::new(()),
        })
    }

    fn path(&self, ticket: &str) -> Result<PathBuf, RunnerError> {
        if ticket.is_empty()
            || ticket.len() > 128
            || !ticket
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
        {
            return Err(failed("name", "the ticket is invalid"));
        }
        Ok(self.root.join(format!("{ticket}.json")))
    }

    pub(super) fn save(&self, reply: &Reply) -> Result<(), RunnerError> {
        let guard = self.writing.lock().map_err(|error| failed("lock", error))?;
        let saved = self.save_locked(reply);
        drop(guard);
        saved
    }

    fn save_locked(&self, reply: &Reply) -> Result<(), RunnerError> {
        let path = self.path(&reply.ticket)?;
        if path.exists() {
            let existing = Self::read(&path)?;
            if existing.body == reply.body && existing.ticket == reply.ticket {
                return Ok(());
            }
            return Err(failed(
                "replace",
                "another completion is already recorded for this ticket",
            ));
        }
        let temporary = self.root.join(format!(
            ".{}.pending",
            crate::protocol::hex(&rand::random::<[u8; 16]>())
        ));
        let saved = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)
                .map_err(|error| failed("stage", error))?;
            serde_json::to_writer(&mut file, reply)
                .map_err(|error| failed("encode", format!("{:?}", error.classify())))?;
            file.flush()
                .and_then(|()| file.sync_all())
                .map_err(|error| failed("flush", error))?;
            fs::rename(&temporary, path).map_err(|error| failed("publish", error))?;
            self.sync()
        })();
        if let Err(error) = saved {
            return match fs::remove_file(&temporary) {
                Ok(()) => Err(error),
                Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => Err(error),
                Err(cleanup) => Err(failed("clean up", format!("{error}; {cleanup}"))),
            };
        }
        Ok(())
    }

    fn read(path: &Path) -> Result<Reply, RunnerError> {
        let metadata = fs::symlink_metadata(path).map_err(|error| failed("inspect", error))?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(failed("read", "the completion is not an owner-only file"));
        }
        let file = File::open(path).map_err(|error| failed("open", error))?;
        serde_json::from_reader(file).map_err(|error| {
            failed(
                "decode",
                format!(
                    "{:?} at {}:{}",
                    error.classify(),
                    error.line(),
                    error.column()
                ),
            )
        })
    }

    pub(super) fn pending(&self) -> Result<Vec<Reply>, RunnerError> {
        let guard = self.writing.lock().map_err(|error| failed("lock", error))?;
        let replies = self.read_pending();
        drop(guard);
        replies
    }

    fn read_pending(&self) -> Result<Vec<Reply>, RunnerError> {
        let mut replies = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|error| failed("list", error))? {
            let path = entry.map_err(|error| failed("inspect", error))?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "pending")
            {
                let reply = Self::read(&path)?;
                let target = self.path(&reply.ticket)?;
                if target.exists() {
                    let held = Self::read(&target)?;
                    if held.body != reply.body {
                        return Err(failed(
                            "recover",
                            "staged and published completions disagree",
                        ));
                    }
                    fs::remove_file(&path)
                        .map_err(|error| failed("retire recovered staging for", error))?;
                } else {
                    fs::rename(&path, &target).map_err(|error| failed("recover", error))?;
                    replies.push(reply);
                }
                self.sync()?;
            } else if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                let reply = Self::read(&path)?;
                if self.path(&reply.ticket)? != path {
                    return Err(failed("read", "the ticket and file disagree"));
                }
                replies.push(reply);
            }
        }
        Ok(replies)
    }

    pub(super) fn acknowledge(&self, ticket: &str) -> Result<(), RunnerError> {
        let guard = self.writing.lock().map_err(|error| failed("lock", error))?;
        let answer = match fs::remove_file(self.path(ticket)?) {
            Ok(()) => self.sync(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(failed("acknowledge", error)),
        };
        drop(guard);
        answer
    }

    fn sync(&self) -> Result<(), RunnerError> {
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| failed("flush storage for", error))
    }
}

impl Drop for Outbox {
    fn drop(&mut self) {
        if let Err(error) = rustix::fs::flock(&self.lease, rustix::fs::FlockOperation::Unlock) {
            crate::error::said(&failed("release storage for", error).to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn completed_replies_survive_reopen_until_acknowledged()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let socket = root.path().join("runner.sock");
        let outbox = super::Outbox::open(&socket)?;
        outbox.save(&super::Reply {
            epoch: "epoch".to_owned(),
            ticket: "ticket".to_owned(),
            body: "completed".to_owned(),
        })?;
        drop(outbox);
        let outbox = super::Outbox::open(&socket)?;
        let replies = outbox.pending()?;
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].body, "completed");
        outbox.acknowledge("ticket")?;
        drop(outbox);
        assert!(super::Outbox::open(&socket)?.pending()?.is_empty());
        Ok(())
    }
    #[test]
    fn a_second_bridge_cannot_claim_the_completion_outbox() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = tempfile::tempdir()?;
        let socket = root.path().join("runner.sock");
        let first = super::Outbox::open(&socket)?;
        let refused = super::Outbox::open(&socket)
            .err()
            .ok_or("two bridges claimed the same outbox")?;
        assert_eq!(refused.name(), "runner_reply_outbox_failed");
        drop(first);
        super::Outbox::open(&socket)?;
        Ok(())
    }

    #[test]
    fn a_flushed_completion_is_recovered_from_interrupted_publication()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let root = tempfile::tempdir()?;
        let socket = root.path().join("runner.sock");
        let outbox = super::Outbox::open(&socket)?;
        let staging = outbox.root.join(".interrupted.pending");
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&staging)?;
        file.write_all(&serde_json::to_vec(&super::Reply {
            epoch: "epoch".to_owned(),
            ticket: "ticket".to_owned(),
            body: "finished".to_owned(),
        })?)?;
        file.sync_all()?;
        drop(file);
        let pending = outbox.pending()?;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].body, "finished");
        assert!(!staging.exists());
        assert!(outbox.root.join("ticket.json").exists());
        Ok(())
    }

    #[test]
    fn another_outcome_cannot_replace_an_existing_completed_reply()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let outbox = super::Outbox::open(&root.path().join("runner.sock"))?;
        outbox.save(&super::Reply {
            epoch: "epoch".to_owned(),
            ticket: "ticket".to_owned(),
            body: "first".to_owned(),
        })?;
        let refused = outbox
            .save(&super::Reply {
                epoch: "epoch".to_owned(),
                ticket: "ticket".to_owned(),
                body: "second".to_owned(),
            })
            .err()
            .ok_or("the first completion was replaced")?;
        assert_eq!(refused.name(), "runner_reply_outbox_failed");
        assert_eq!(outbox.pending()?[0].body, "first");
        Ok(())
    }
}
