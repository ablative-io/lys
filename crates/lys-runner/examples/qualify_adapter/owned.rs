//! The example owns a child session and cleans up its pipes on every exit.

use super::{Result, code, io_failed};
use lys_runner::harness_control::{Executable, Update, claude, codex, process};
use lys_runner::peer::{Leader, start_identity};
use serde_json::{Value, json};
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use tokio::signal::unix::{Signal, SignalKind, signal};
use tokio::sync::mpsc;

pub(super) struct Cancellation {
    interrupt: Signal,
    terminate: Signal,
    hangup: Signal,
}
impl Cancellation {
    pub(super) fn new() -> Result<Self> {
        Ok(Self {
            interrupt: signal(SignalKind::interrupt()).map_err(io_failed)?,
            terminate: signal(SignalKind::terminate()).map_err(io_failed)?,
            hangup: signal(SignalKind::hangup()).map_err(io_failed)?,
        })
    }
    async fn received(&mut self) {
        tokio::select! { _ = self.interrupt.recv() => {}, _ = self.terminate.recv() => {}, _ = self.hangup.recv() => {} }
    }
}

pub(super) enum Message {
    Frame(Value),
    Version(Vec<u8>),
}
pub(super) struct Owned {
    child: Option<Child>,
    pub(super) leader: Option<Leader>,
    writer: Option<ChildStdin>,
    frames: mpsc::Receiver<Result<Message>>,
    reader: Option<JoinHandle<()>>,
}

impl Owned {
    pub(super) async fn start(
        program: &Executable,
        directory: &Path,
        mode: &str,
        conversation: &str,
        cancel: &mut Cancellation,
    ) -> Result<Self> {
        let executable = std::env::current_exe().map_err(io_failed)?;
        let mut child = Command::new(executable)
            .arg("--owned-child")
            .arg(mode)
            .arg(&program.path)
            .arg(directory)
            .arg(conversation)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(io_failed)?;
        let (send, frames) = mpsc::channel(32);
        let mut owned = Self {
            writer: child.stdin.take(),
            child: Some(child),
            leader: None,
            frames,
            reader: None,
        };
        let stdout = owned
            .child
            .as_mut()
            .and_then(|child| child.stdout.take())
            .ok_or("qualification_pipe_missing")?;
        let probe = mode == "probe";
        owned.reader = Some(
            std::thread::Builder::new()
                .name("qualification-reader".to_owned())
                .spawn(move || {
                    let mut reader = BufReader::new(stdout);
                    let ready = process::frame(&mut reader)
                        .map(Message::Frame)
                        .map_err(|error| error.name());
                    let failed = ready.is_err();
                    if send.blocking_send(ready).is_err() || failed {
                        return;
                    }
                    if probe {
                        let mut bytes = Vec::new();
                        let result = reader
                            .take(129)
                            .read_to_end(&mut bytes)
                            .map_err(io_failed)
                            .and_then(|_| {
                                if bytes.len() > 128 {
                                    Err("qualification_version_report_invalid".to_owned())
                                } else {
                                    Ok(Message::Version(bytes))
                                }
                            });
                        if let Err(error) = send.blocking_send(result) {
                            drop(error);
                            eprintln!("qualification_receiver_ended");
                        }
                    } else {
                        loop {
                            let result = process::frame(&mut (&mut reader).take(1_048_576))
                                .map(Message::Frame)
                                .map_err(|error| error.name());
                            let failed = result.is_err();
                            if send.blocking_send(result).is_err() || failed {
                                break;
                            }
                        }
                    }
                })
                .map_err(io_failed)?,
        );
        let Message::Frame(ready) = owned.next(cancel).await? else {
            return Err("qualification_child_unproved".to_owned());
        };
        let pid = owned
            .child
            .as_ref()
            .ok_or("qualification_child_missing")?
            .id();
        let id = rustix::process::Pid::from_raw(
            i32::try_from(pid).map_err(|error| format!("qualification_child_unproved:{error}"))?,
        )
        .ok_or("qualification_child_unproved")?;
        if ready != json!({"qualification_child":pid,"executable":program})
            || rustix::process::getpgid(Some(id)).map_err(io_failed_errno)? != id
            || rustix::process::getsid(Some(id)).map_err(io_failed_errno)? != id
        {
            return Err("qualification_child_unproved".to_owned());
        }
        owned.leader = Some(Leader {
            pid,
            start: start_identity(pid).map_err(|error| error.name())?,
        });
        owned
            .writer
            .as_mut()
            .ok_or("qualification_pipe_missing")?
            .write_all(b"execute\n")
            .map_err(io_failed)?;
        Ok(owned)
    }
    pub(super) async fn next(&mut self, cancel: &mut Cancellation) -> Result<Message> {
        tokio::select! { frame = self.frames.recv() => frame.ok_or_else(|| "qualification_pipe_ended".to_owned())?, () = cancel.received() => Err("qualification_cancelled".to_owned()) }
    }
    pub(super) fn write(&mut self, update: &Update) -> Result<()> {
        let writer = self.writer.as_mut().ok_or("qualification_pipe_missing")?;
        for dispatch in &update.dispatches {
            serde_json::to_writer(&mut *writer, &dispatch.frame)
                .map_err(|error| format!("qualification_frame_write_failed:{error}"))?;
            writer
                .write_all(b"\n")
                .and_then(|()| writer.flush())
                .map_err(io_failed)?;
        }
        Ok(())
    }
    pub(super) fn stop(&mut self, force: bool) -> Result<ExitStatus> {
        self.frames.close();
        self.writer.take();
        let child = self.child.as_mut().ok_or("qualification_child_missing")?;
        if child.try_wait().map_err(io_failed)?.is_none() {
            if let Some(leader) = &self.leader {
                if force {
                    if start_identity(leader.pid).map_err(|error| error.name())? == leader.start {
                        lys_runner::pty::end_group(leader.pid).map_err(|error| error.name())?;
                    } else {
                        return Err("qualification_child_start_changed".to_owned());
                    }
                } else {
                    lys_runner::pty::end(leader).map_err(|error| error.name())?;
                }
            } else {
                child.kill().map_err(io_failed)?;
            }
        }
        self.wait_exit()
    }
    pub(super) fn wait_exit(&mut self) -> Result<ExitStatus> {
        self.frames.close();
        self.writer.take();
        let status = self
            .child
            .as_mut()
            .ok_or("qualification_child_missing")?
            .wait()
            .map_err(io_failed)?;
        if let Some(leader) = &self.leader {
            match lys_runner::pty::end_left_group(leader).map_err(|error| error.name())? {
                lys_runner::pty::Left::Gone | lys_runner::pty::Left::Ended { reason: None } => {}
                lys_runner::pty::Left::Ended {
                    reason: Some(reason),
                }
                | lys_runner::pty::Left::Unended { reason } => {
                    return Err(format!("qualification_cleanup_failed:{reason}"));
                }
                lys_runner::pty::Left::Reused => {
                    return Err("qualification_child_start_changed".to_owned());
                }
            }
        }
        if let Some(reader) = self.reader.take() {
            reader.join().map_err(|panic| {
                format!(
                    "qualification_reader_panicked:{:?}",
                    panic.as_ref().type_id()
                )
            })?;
        }
        self.child.take();
        Ok(status)
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        if self.child.is_some()
            && let Err(error) = self.stop(true)
        {
            eprintln!("{}", code(&error));
        }
    }
}
fn io_failed_errno(error: rustix::io::Errno) -> String {
    format!("qualification_child_unproved:{error}")
}

pub(super) fn child_entry(args: &[String]) -> Result<()> {
    let [flag, mode, program, directory, conversation] = args else {
        return Err("qualification_child_arguments_invalid".to_owned());
    };
    if flag != "--owned-child" {
        return Err("qualification_child_arguments_invalid".to_owned());
    }
    let arguments = match mode.as_str() {
        "probe" => vec!["--version".to_owned()],
        "claude-code" => claude::arguments(&[], conversation).map_err(|error| error.name())?,
        "codex" => codex::arguments(&[]).map_err(|error| error.name())?,
        _ => return Err("qualification_child_arguments_invalid".to_owned()),
    };
    rustix::process::setsid().map_err(io_failed_errno)?;
    let mut stdout = std::io::stdout();
    serde_json::to_writer(&mut stdout, &json!({"qualification_child":std::process::id(),"executable":process::executable(Path::new(program)).map_err(|error| error.name())?})).map_err(|error| format!("qualification_child_ready_failed:{error}"))?;
    stdout
        .write_all(b"\n")
        .and_then(|()| stdout.flush())
        .map_err(io_failed)?;
    {
        let mut input = File::from(rustix::io::dup(std::io::stdin()).map_err(io_failed_errno)?);
        permission(&mut input)?;
    }
    Err(io_failed(
        Command::new(program)
            .args(arguments)
            .current_dir(directory)
            .exec(),
    ))
}

fn permission(input: &mut impl Read) -> Result<()> {
    // Buffered startup input can consume the next process's first protocol frame.
    let mut bytes = [0; 8];
    input.read_exact(&mut bytes).map_err(io_failed)?;
    if &bytes != b"execute\n" {
        return Err("qualification_child_not_authorised".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launch_permission_leaves_the_first_protocol_frame_in_the_pipe()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let (mut reader, mut writer) = std::io::pipe()?;
        writer.write_all(b"execute\n{\"id\":\"next-frame\"}\n")?;
        drop(writer);
        permission(&mut reader)?;
        let mut remaining = String::new();
        reader.read_to_string(&mut remaining)?;
        assert_eq!(remaining, "{\"id\":\"next-frame\"}\n");
        Ok(())
    }
}
