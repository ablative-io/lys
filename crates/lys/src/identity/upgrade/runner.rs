//! The runner is stopped only after its live sessions have been ruled out.
//! A replacement is ready only when its own process holds the exit lock,
//! has said it listens and answers with Status.

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_runner::{Act, Answer, Client};

use super::super::error::{ErrorKind, IdentityError, IdentityResult};
use super::super::install::exit_wait::{self, ExitWatch};
use super::super::install::layout::Layout;
use super::super::install::log_wait::{self, LogCursor};
use super::super::install::login;
use super::super::private_files;
use super::{Ready, Unit};

fn refuse(name: &str, detail: impl std::fmt::Display, path: &Path) -> IdentityError {
    IdentityError::new(
        ErrorKind::UpgradeFailed,
        "upgrade",
        "runner",
        format!("{name}: {detail}"),
    )
    .at(path)
}

fn pid_at(path: &Path) -> IdentityResult<u32> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| refuse("runner_process_unreadable", error, path))?;
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|pid| *pid > 1 && *pid != std::process::id())
        .ok_or_else(|| refuse("runner_process_unreadable", "no usable process id", path))
}

fn status(client: &Client, socket: &Path) -> IdentityResult<lys_runner::protocol::StatusView> {
    match client.ask(&Act::Status { session: None }) {
        Ok(Answer::Status { status })
            if status.protocol == lys_runner::protocol::PROTOCOL_VERSION =>
        {
            Ok(status)
        }
        Ok(Answer::Refused { refusal, words, .. }) => Err(refuse(
            "runner_status_unreadable",
            format!("{refusal}: {words}"),
            socket,
        )),
        Ok(_) => Err(refuse(
            "runner_status_unreadable",
            "Status did not answer with Status in the supported protocol",
            socket,
        )),
        Err(error) => Err(refuse("runner_status_unreadable", error, socket)),
    }
}

/// The answering runner and each replacement started by this upgrade.
pub struct Restart {
    /// The runner as a binary in the install.
    pub unit: Unit,
    client: Client,
    socket: std::path::PathBuf,
    exit: ExitWatch,
    pid: u32,
    child: Option<Child>,
    recorded: bool,
}

impl Restart {
    /// Reads the existing key and Status before any file or process changes.
    /// When an upgrade stopped part-way, which the upgrade ends first, the
    /// runner is read as for [`Restart::prepare_for_recovery`].
    pub fn prepare(layout: &Layout) -> IdentityResult<Self> {
        Self::capture(layout, super::intent::Intent::read(layout)?.is_some())
    }

    /// The runner of an upgrade that stopped part-way. An upgrade stops the
    /// runner before it places anything, so this runner is usually down: one
    /// whose recorded process holds no exit lock runs no session, needs no
    /// Status, and is started once the previous build is back. One that is
    /// up is still asked, and refused while any session lives.
    pub fn prepare_for_recovery(layout: &Layout) -> IdentityResult<Self> {
        Self::capture(layout, true)
    }

    fn capture(layout: &Layout, down_allowed: bool) -> IdentityResult<Self> {
        let socket = layout.runner_socket();
        let key = Arc::new(
            Ed25519Identity::load(&layout.service_key()).map_err(|error| {
                refuse("runner_status_unreadable", error, &layout.service_key())
            })?,
        );
        let client = Client::new(socket.clone(), Arc::clone(&key));
        let pid_file = layout.run_dir().join("runner.pid");
        let down = down_allowed && ExitWatch::open(&pid_file)?.exited()?;
        if !down {
            Self::empty(&client, &socket)?;
        }
        let public = layout.run_dir().join("runner-server.pub");
        let bytes = std::fs::read_to_string(&public)
            .map_err(|error| refuse("runner_key_unreadable", error, &public))?;
        if bytes.trim() != lys_runner::protocol::hex(&key.public_key_bytes()) {
            return Err(refuse(
                "runner_key_differs",
                "the runner's public key differs from the existing service key",
                &public,
            ));
        }
        let pid = pid_at(&pid_file)?;
        let exit = ExitWatch::open(&pid_file)?;
        if !down && exit.exited()? {
            return Err(refuse(
                "runner_process_unreadable",
                "Status answered but the recorded runner holds no exit lock",
                &pid_file,
            ));
        }
        let args = vec![
            "runner".into(),
            "serve".into(),
            "--socket".into(),
            socket.display().to_string(),
            "--state".into(),
            layout.data_dir().join("runner").display().to_string(),
            "--server-key".into(),
            public.display().to_string(),
        ];
        Ok(Self {
            unit: Unit {
                binary: "lys",
                args,
                log: layout.logs_dir().join("runner.log"),
                pid: pid_file,
                ready: Ready {
                    says: format!("listening {} as runner", socket.display()),
                    answers: None,
                },
            },
            client,
            socket,
            exit,
            pid,
            child: None,
            recorded: true,
        })
    }

    fn empty(client: &Client, socket: &Path) -> IdentityResult<()> {
        let live: Vec<_> = status(client, socket)?
            .sessions
            .into_iter()
            .filter(|session| session.ended.is_none())
            .map(|session| session.session)
            .collect();
        if live.is_empty() {
            return Ok(());
        }
        Err(refuse(
            "runner_sessions_live",
            format!("end these sessions before upgrading: {}", live.join(", ")),
            socket,
        ))
    }

    /// Stops the captured process, after checking Status again if it lives.
    /// A failed replacement already observed exited needs no Status answer.
    pub fn stop(&mut self, say: &mut dyn FnMut(&str)) -> IdentityResult<()> {
        if self.recorded && pid_at(&self.unit.pid)? != self.pid {
            return Err(refuse(
                "runner_process_changed",
                "the process file changed; no process was stopped",
                &self.unit.pid,
            ));
        }
        if !self.exit.exited()? {
            Self::empty(&self.client, &self.socket)?;
            let result = Command::new("kill")
                .arg(self.pid.to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|error| refuse("runner_stop_refused", error, &self.unit.pid))?;
            if !result.success() {
                return Err(refuse(
                    "runner_stop_refused",
                    format!("stop signal for process {} exited {result}", self.pid),
                    &self.unit.pid,
                ));
            }
            self.exit.wait()?;
            say("runner stopped through its exit lock");
        }
        if let Some(mut child) = self.child.take() {
            child
                .wait()
                .map_err(|error| refuse("runner_exit_unreadable", error, &self.unit.pid))?;
        }
        Ok(())
    }

    /// Starts the placed executable, retaining its pid even if recording it
    /// fails, and waits on a fresh ready line, typed Status and its exit lock.
    pub fn start(
        &mut self,
        layout: &Layout,
        say: &mut dyn FnMut(&str),
        word: &str,
    ) -> IdentityResult<()> {
        if !self.exit.exited()? {
            return Err(refuse(
                "runner_start_refused",
                "the previous process has not exited",
                &self.unit.pid,
            ));
        }
        let before = std::fs::metadata(&self.unit.log)
            .map_err(|error| refuse("runner_log_unreadable", error, &self.unit.log))?
            .len();
        let out = OpenOptions::new()
            .append(true)
            .open(&self.unit.log)
            .map_err(|error| refuse("runner_log_unreadable", error, &self.unit.log))?;
        let err = out
            .try_clone()
            .map_err(|error| refuse("runner_log_unreadable", error, &self.unit.log))?;
        let lock = exit_wait::hold(&self.unit.pid)?;
        // The runner starts with the login's environment, as every service
        // does, never the shell that ran the upgrade.
        let environment = login::login()?;
        let mut command = Command::new(layout.binary("lys"));
        command
            .env_clear()
            .envs(environment.variables())
            .args(&self.unit.args)
            .stdin(lock)
            .stdout(out)
            .stderr(err);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command
            .spawn()
            .map_err(|error| refuse("runner_start_refused", error, &layout.binary("lys")))?;
        drop(command);
        self.pid = child.id();
        self.child = Some(child);
        self.recorded = false;
        private_files::write(&self.unit.pid, self.pid.to_string().as_bytes()).map_err(|error| {
            refuse(
                "runner_process_unrecorded",
                format!(
                    "process {} started but its process file could not be recorded: {error}",
                    self.pid
                ),
                &self.unit.pid,
            )
        })?;
        self.recorded = true;
        let mut cursor = LogCursor::at(&self.unit.log, before);
        let mut last = None;
        let outcome = log_wait::wait_until("runner", &self.unit.log, &self.unit.pid, &mut || {
            if !cursor.says(&self.unit.ready.says) {
                return false;
            }
            match status(&self.client, &self.socket) {
                Ok(_) => match self.exit.exited() {
                    Ok(false) => true,
                    Ok(true) => {
                        last = Some("the replacement exited before ready".to_owned());
                        false
                    }
                    Err(error) => {
                        last = Some(error.to_string());
                        false
                    }
                },
                Err(error) => {
                    last = Some(error.to_string());
                    false
                }
            }
        });
        if let Err(error) = outcome {
            let named = self.named_output(before)?;
            return Err(refuse(
                "runner_start_refused",
                format!(
                    "{error}; {}; {named}",
                    last.as_deref().unwrap_or("no Status readiness")
                ),
                &self.unit.log,
            ));
        }
        if self.exit.exited()? {
            return Err(refuse(
                "runner_start_refused",
                self.named_output(before)?,
                &self.unit.log,
            ));
        }
        say(&format!("runner {word} on {}", self.socket.display()));
        Ok(())
    }

    fn named_output(&self, from: u64) -> IdentityResult<String> {
        let reading = |error| refuse("runner_log_unreadable", error, &self.unit.log);
        let mut log = std::fs::File::open(&self.unit.log).map_err(reading)?;
        log.seek(SeekFrom::Start(from)).map_err(reading)?;
        let mut text = String::new();
        log.read_to_string(&mut text).map_err(reading)?;
        Ok(text
            .lines()
            .find_map(|line| {
                let named = line.strip_prefix("error: ").unwrap_or(line);
                named.starts_with("runner_").then(|| named.to_owned())
            })
            .unwrap_or_else(|| "the replacement exited without a named runner diagnostic".into()))
    }
}
