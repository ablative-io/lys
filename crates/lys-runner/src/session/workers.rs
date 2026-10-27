//! The threads that act for a session hold a share of the runner while they
//! run, and with it the lock on its state directory. The thread that sees a
//! session's exit still holds its share for a moment after it records the
//! end, so the end is not answered until that thread has been joined. A
//! caller told that a session ended is then sure the session's threads let
//! the runner go, and a runner it drops next lets its directory go before
//! the drop returns: a replacement opened at once in the same process, as an
//! upgrade's is, is never refused `runner_state_held` by a thread that was
//! merely late to finish.
//!
//! A transcript follower is not among them: it may go on reading a proxied
//! run's last calls after the end, and keeps the directory held while it
//! still writes there.

use std::thread::JoinHandle;

use super::{Sessions, Table};
use crate::error::RunnerError;

impl Table {
    /// Keep `worker` as one of session `id`'s threads. A session no longer
    /// held has nobody to answer its end, and the thread runs on alone.
    pub(super) fn keep_worker(&mut self, id: &str, worker: JoinHandle<()>) {
        if let Some(session) = self.sessions.get_mut(id) {
            session.workers.push(worker);
        }
    }
}

impl Sessions {
    /// Join the threads of each of `ids` that has ended. A thread answering
    /// an end of its own cannot join itself; it is kept for the next answer.
    pub(super) fn join_workers<'a>(
        &self,
        ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), RunnerError> {
        let here = std::thread::current().id();
        let mut joining = Vec::new();
        {
            let mut table = self.lock()?;
            for id in ids {
                let Some(session) = table
                    .sessions
                    .get_mut(id)
                    .filter(|session| session.ended.is_some())
                else {
                    continue;
                };
                let (own, others): (Vec<_>, Vec<_>) = std::mem::take(&mut session.workers)
                    .into_iter()
                    .partition(|worker| worker.thread().id() == here);
                session.workers = own;
                joining.extend(others.into_iter().map(|worker| (id.to_owned(), worker)));
            }
        }
        #[cfg(test)]
        if !joining.is_empty() {
            Probes::run(&self.worker_probes.join);
        }
        for (id, worker) in joining {
            if worker.join().is_err() {
                crate::error::said(&format!(
                    "session {id}: a thread acting for it ended abnormally"
                ));
            }
        }
        Ok(())
    }
}

/// What a test runs at the two ends of a worker's last moment: the exit
/// thread after it has recorded the end, still holding its share, and an
/// answer about to join it.
#[cfg(test)]
#[derive(Default)]
pub(super) struct Probes {
    pub(super) exit: std::sync::Mutex<Option<super::SpawnProbe>>,
    pub(super) join: std::sync::Mutex<Option<super::SpawnProbe>>,
}

#[cfg(test)]
impl Probes {
    pub(super) fn run(probe: &std::sync::Mutex<Option<super::SpawnProbe>>) {
        let taken = match probe.lock() {
            Ok(mut held) => held.take(),
            Err(error) => {
                crate::error::said(&format!("worker_probe_failed: {error}"));
                None
            }
        };
        if let Some(probe) = taken {
            probe();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, mpsc};

    use super::super::{Ask, Sessions};
    use crate::protocol::Launch;

    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    fn cat(root: &std::path::Path, session: &str) -> Launch {
        Launch {
            session: session.into(),
            program: "/bin/cat".into(),
            arguments: Vec::new(),
            directory: root.display().to_string(),
            environment: BTreeMap::new(),
            config: None,
            columns: 80,
            rows: 24,
            rotation: None,
            policy: None,
        }
    }

    /// Hold the exit thread after it records the end until the answer joins
    /// it, or until the test lets it go: without the join the answer comes
    /// back while the thread still holds its share, every time.
    fn hold_the_exit_thread(sessions: &Sessions) -> std::result::Result<mpsc::Sender<()>, String> {
        let (release, released) = mpsc::channel::<()>();
        let joining = release.clone();
        *sessions
            .worker_probes
            .exit
            .lock()
            .map_err(|error| error.to_string())? = Some(Box::new(move || {
            released.recv().unwrap_or_default();
        }));
        *sessions
            .worker_probes
            .join
            .lock()
            .map_err(|error| error.to_string())? =
            Some(Box::new(move || joining.send(()).unwrap_or_default()));
        Ok(release)
    }

    #[test]
    fn a_stop_answers_only_after_the_exit_thread_lets_the_runner_go() -> Result {
        let root = tempfile::tempdir()?;
        let sessions = Sessions::open(root.path(), 4096)?;
        let release = hold_the_exit_thread(&sessions)?;
        sessions.start(cat(root.path(), "stopped"))?;
        sessions.stop_everything(
            "a person at the console",
            "deliberate stop",
            Ask {
                kill: true,
                settling: false,
            },
            &AtomicBool::new(false),
        )?;
        let shares = Arc::strong_count(&sessions);
        drop(release);
        drop(sessions);
        let replacement = Sessions::open(root.path(), 4096);
        assert_eq!(
            shares, 1,
            "the stop answered while its exit thread held the runner"
        );
        replacement?;
        Ok(())
    }

    #[test]
    fn an_end_answers_only_after_the_exit_thread_lets_the_runner_go() -> Result {
        let root = tempfile::tempdir()?;
        let sessions = Sessions::open(root.path(), 4096)?;
        let release = hold_the_exit_thread(&sessions)?;
        sessions.start(cat(root.path(), "ended"))?;
        sessions.end("ended", &AtomicBool::new(false))?;
        let shares = Arc::strong_count(&sessions);
        drop(release);
        drop(sessions);
        let replacement = Sessions::open(root.path(), 4096);
        assert_eq!(
            shares, 1,
            "the end answered while its exit thread held the runner"
        );
        replacement?;
        Ok(())
    }
}
