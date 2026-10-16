//! Keep a stand-in available until its caller has completed the server act.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Lifetime {
    socket: PathBuf,
    ended: Arc<AtomicBool>,
}

impl Lifetime {
    pub fn new(socket: &Path) -> Self {
        Self {
            socket: socket.to_owned(),
            ended: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.ended)
    }

    pub fn finish(&self) -> std::io::Result<()> {
        self.ended.store(true, Ordering::Release);
        std::os::unix::net::UnixStream::connect(&self.socket).map(|_| ())
    }
}

impl Drop for Lifetime {
    fn drop(&mut self) {
        if !self.ended.swap(true, Ordering::AcqRel) {
            // Wake the acceptor; the test separately joins and checks its result.
            let result = std::os::unix::net::UnixStream::connect(&self.socket);
            if let Err(error) = result {
                eprintln!("stand-in shutdown failed: {error}");
            }
        }
    }
}

pub fn status<'a>(
    sessions: impl Iterator<Item = &'a String>,
    wanted: Option<&str>,
) -> lys_runner::Answer {
    lys_runner::Answer::Status {
        status: lys_runner::protocol::StatusView {
            runner: "fixture".to_owned(),
            protocol: lys_runner::protocol::PROTOCOL_VERSION,
            sessions: sessions
                .filter(|id| wanted.is_none_or(|wanted| wanted == id.as_str()))
                .map(|id| lys_runner::protocol::SessionView {
                    qualification: None,
                    session: id.clone(),
                    pid: Some(42),
                    started_at: 1,
                    columns: 80,
                    rows: 24,
                    oldest: 0,
                    cursor: 0,
                    account: None,
                    moves: Vec::new(),
                    ended: None,
                    policy: None,
                    control: None,
                })
                .collect(),
        },
    }
}

pub fn liveness(
    native: &std::collections::BTreeMap<String, String>,
    wanted: Option<&str>,
) -> lys_runner::Answer {
    lys_runner::Answer::Liveness {
        sessions: native
            .iter()
            .filter(|(id, _)| wanted.is_none_or(|wanted| wanted == id.as_str()))
            .map(|(id, conversation)| lys_runner::liveness::LivenessView {
                session: id.clone(),
                pid: Some(42),
                alive: true,
                harness_session: Some(conversation.clone()),
                turn_active: false,
                last_signal_at: None,
                started_at: 1,
                ended: false,
                harness: lys_runner::liveness::HarnessLiveness {
                    managed: false,
                    started_fresh: false,
                },
            })
            .collect(),
    }
}
