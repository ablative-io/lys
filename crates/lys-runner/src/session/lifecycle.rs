//! A session's process: started in its terminal, its output kept, its exit
//! seen, and at a usage limit moved to its next account. One thread reads
//! the terminal and one waits on the process; neither runs on a clock.

use std::io::Read;
use std::sync::Arc;

use portable_pty::Child;

use super::{Live, Session, Sessions, now_ms, unknown};
use crate::error::RunnerError;
use crate::protocol::{Ended, EndedHow};

impl Sessions {
    /// Run the session's process, on its rotation's account when it has one,
    /// and start the threads that read its output and see its exit.
    pub(super) fn run(
        self: &Arc<Self>,
        id: &str,
        session: &mut Session,
        resumed: bool,
    ) -> Result<u32, RunnerError> {
        let launch = session.launch.as_ref().ok_or_else(|| unknown(id))?;
        let mut environment = launch.environment.clone();
        let mut arguments = launch.arguments.clone();
        if let Some(rotation) = &session.rotation {
            environment.insert(
                rotation.variable().to_owned(),
                rotation.account().to_owned(),
            );
            if let (true, Some(resume)) = (resumed, rotation.resume_arguments()) {
                arguments = resume.to_vec();
            }
        }
        let spawned = crate::pty::spawn(&crate::pty::Spawn {
            program: &launch.program,
            arguments: &arguments,
            directory: &launch.directory,
            environment: &environment,
            columns: session.columns,
            rows: session.rows,
        })?;
        session.generation += 1;
        session.pid = Some(spawned.pid);
        session.leader_started = match crate::pty::leader_started(spawned.pid) {
            Ok(started) => started,
            Err(error) => {
                crate::error::said(&format!(
                    "session {id}: process {pid} will not be signalled by a restart: {error}",
                    pid = spawned.pid
                ));
                None
            }
        };
        session.live = Some(Live {
            writer: spawned.writer,
            master: spawned.master,
            pid: spawned.pid,
        });
        let generation = session.generation;
        let (reader, child) = (spawned.reader, spawned.child);
        let pumped = Arc::clone(self);
        let owned = id.to_owned();
        let pump = std::thread::spawn(move || pumped.pump(&owned, generation, reader));
        let watched = Arc::clone(self);
        let owned = id.to_owned();
        std::thread::spawn(move || watched.watch(&owned, generation, child, pump));
        Ok(spawned.pid)
    }

    /// Keep the output of generation `generation` of session `id` until its
    /// terminal closes.
    fn pump(&self, id: &str, generation: u64, mut reader: Box<dyn Read + Send>) {
        let mut buffer = [0_u8; 8192];
        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    crate::error::said(&format!("session {id}: the terminal closed: {error}"));
                    break;
                }
            };
            let mut table = self.lock();
            if let Some(session) = table
                .sessions
                .get_mut(id)
                .filter(|s| s.generation == generation)
            {
                session.scrollback.push(&buffer[..read]);
                trip_on_words(session, read);
            }
            drop(table);
            self.wake();
        }
    }

    /// See the exit of generation `generation` of session `id`: move it to
    /// its next account at a usage limit, or record its end. The end is
    /// recorded once the exit is seen and every byte its terminal gave is
    /// kept, so a read that answers an end has read everything before it.
    fn watch(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        mut child: Box<dyn Child + Send + Sync>,
        pump: std::thread::JoinHandle<()>,
    ) {
        let waited = child.wait();
        let at = now_ms();
        if pump.join().is_err() {
            crate::error::said(&format!(
                "session {id}: the thread keeping its output ended abnormally"
            ));
        }
        let mut table = self.lock();
        let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|s| s.generation == generation)
        else {
            return;
        };
        session.live = None;
        let (status, signal) = match &waited {
            Ok(exit) => match exit.signal() {
                Some(signal) => (None, Some(signal.to_owned())),
                None => (Some(exit.exit_code()), None),
            },
            Err(error) => {
                crate::error::said(&format!(
                    "session {id}: the process's exit could not be read: {error}"
                ));
                (None, None)
            }
        };
        let limit = !session.ending
            && session
                .rotation
                .as_ref()
                .is_some_and(|rotation| rotation.limit_at_exit(status));
        let mut how = EndedHow::Exited;
        if limit {
            let next = session
                .rotation
                .as_mut()
                .and_then(|rotation| rotation.advance(at));
            if let Some(moved) = next {
                crate::error::said(&format!(
                    "session {id} reached its usage limit and moves to account {moved}"
                ));
                match self.run(id, session, true) {
                    Ok(_) => {
                        self.persist_logged(&table);
                        drop(table);
                        self.wake();
                        return;
                    }
                    Err(error) => crate::error::said(&format!(
                        "session {id} could not move to its next account: {error}"
                    )),
                }
            } else {
                crate::error::said(&format!(
                    "session {id} reached its usage limit on the last of its accounts"
                ));
                how = EndedHow::AccountsExhausted;
            }
        }
        session.ended = Some(Ended {
            how,
            at,
            status,
            signal,
        });
        self.persist_logged(&table);
        drop(table);
        self.wake();
    }
}

/// Mark the session's usage-limit words seen in the last `read` bytes, and
/// end its process so its exit moves it to the next account.
fn trip_on_words(session: &mut Session, read: usize) {
    let Some(rotation) = session.rotation.as_mut() else {
        return;
    };
    if session.ending || rotation.tripped() {
        return;
    }
    let reach = read + rotation.longest_word().saturating_sub(1);
    let from = session
        .scrollback
        .end()
        .saturating_sub(reach as u64)
        .max(session.scrollback.oldest());
    let Ok(bytes) = session.scrollback.from(from) else {
        return;
    };
    if rotation.words_in(&String::from_utf8_lossy(&bytes)) {
        rotation.trip();
        if let Some(live) = &session.live {
            live.end("at its usage limit");
        }
    }
}
