#![cfg(test)]
//! A full input pipe must leave the session table available to another session.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, TryLockError, mpsc};

use super::Sessions;
use crate::protocol::Launch;

struct GatedPipe {
    pipe: UnixStream,
    blocked: mpsc::SyncSender<()>,
    resume: mpsc::Receiver<()>,
    entered: bool,
}

impl Write for GatedPipe {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.entered {
            match self.pipe.write(bytes) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                other => {
                    return Err(io::Error::other(format!(
                        "the input pipe was not full: {other:?}"
                    )));
                }
            }
            self.blocked.send(()).map_err(io::Error::other)?;
            self.resume.recv().map_err(io::Error::other)?;
            self.pipe.set_nonblocking(false)?;
            self.entered = true;
        }
        self.pipe.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.pipe.flush()
    }
}

fn launch(session: &str) -> Launch {
    Launch {
        session: session.to_owned(),
        program: "/bin/cat".to_owned(),
        arguments: Vec::new(),
        directory: "/".to_owned(),
        environment: BTreeMap::new(),
        columns: 80,
        rows: 24,
        rotation: None,
        policy: None,
    }
}

#[test]
fn a_full_unwatched_input_pipe_leaves_the_second_session_answering() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let sessions = Sessions::open(dir.path(), 4096)?;
    sessions.start(launch("blocked"))?;
    sessions.start(launch("second"))?;
    let result = exercise(&sessions);
    sessions.stop_all();
    result
}

fn exercise(sessions: &Arc<Sessions>) -> Result<(), Box<dyn Error>> {
    let (mut pipe, mut reader) = UnixStream::pair()?;
    pipe.set_nonblocking(true)?;
    let buffer = [0_u8; 8192];
    let mut filled = 0;
    loop {
        match pipe.write(&buffer) {
            Ok(0) => return Err("input pipe closed before becoming full".into()),
            Ok(written) => filled += written,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error) => return Err(error.into()),
        }
    }
    assert!(filled > 0);
    let (blocked, waiting) = mpsc::sync_channel(1);
    let (release, resume) = mpsc::sync_channel(1);
    {
        let mut table = sessions.lock();
        let live = table
            .sessions
            .get_mut("blocked")
            .ok_or("blocked session missing")?
            .live("blocked")?;
        live.writer = Box::new(GatedPipe {
            pipe,
            blocked,
            resume,
            entered: false,
        });
    }
    let writing = Arc::clone(sessions);
    let writer = std::thread::spawn(move || writing.write("blocked", b"x"));
    waiting.recv()?;
    let available = match sessions.table.try_lock() {
        Ok(table) => {
            drop(table);
            true
        }
        Err(TryLockError::WouldBlock) => false,
        Err(TryLockError::Poisoned(error)) => return Err(error.to_string().into()),
    };
    let second = if available {
        Some(sessions.status(Some("second")))
    } else {
        None
    };
    let mut drained = vec![0; filled];
    reader.read_exact(&mut drained)?;
    release.send(())?;
    writer.join().expect("input writer panicked")?;
    let answer = second.unwrap_or_else(|| sessions.status(Some("second")))?;
    assert_eq!(answer.sessions.len(), 1);
    assert_eq!(answer.sessions[0].session, "second");
    assert!(
        available,
        "the full input pipe held the shared table and stopped the second session answering"
    );
    Ok(())
}
