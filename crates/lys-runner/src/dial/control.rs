//! Owner-only local commands interrupt reconnect backoff and read its current status.

use std::io::{Read, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::super::reconnect::RetryWait;
use crate::RunnerError;

fn failed(error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("runner_reconnect_control_failed", error.to_string())
}

fn path(socket: &Path) -> PathBuf {
    socket.with_extension("reconnect.sock")
}

pub(super) fn ask(socket: &Path, command: u8) -> Result<serde_json::Value, RunnerError> {
    let mut stream = UnixStream::connect(path(socket)).map_err(failed)?;
    stream.write_all(&[command]).map_err(failed)?;
    let mut bytes = Vec::new();
    stream
        .take(1_048_576)
        .read_to_end(&mut bytes)
        .map_err(failed)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        failed(format!(
            "invalid control response: {:?} at {}:{}",
            error.classify(),
            error.line(),
            error.column()
        ))
    })?;
    if let Some(reason) = value.get("refusal").and_then(serde_json::Value::as_str) {
        return Err(failed(reason));
    }
    Ok(value)
}

pub(super) struct Control {
    socket: PathBuf,
    worker: Option<std::thread::JoinHandle<Result<(), RunnerError>>>,
}

impl Control {
    pub(super) fn open(socket: &Path, wait: Arc<RetryWait>) -> Result<Self, RunnerError> {
        let control = path(socket);
        // The bridge's exclusive outbox lease proves an earlier bridge is no longer serving this path.
        match std::fs::symlink_metadata(&control) {
            Ok(metadata)
                if metadata.file_type().is_socket()
                    && metadata.uid() == rustix::process::getuid().as_raw() =>
            {
                std::fs::remove_file(&control).map_err(failed)?;
            }
            Ok(_) => return Err(failed("the control path is not this user's Unix socket")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(failed(error)),
        }
        let listener = UnixListener::bind(&control).map_err(failed)?;
        std::fs::set_permissions(&control, std::fs::Permissions::from_mode(0o600))
            .map_err(failed)?;
        let worker = std::thread::Builder::new()
            .name("runner-reconnect-control".to_owned())
            .spawn(move || serve(&listener, &wait))
            .map_err(failed)?;
        Ok(Self {
            socket: socket.to_path_buf(),
            worker: Some(worker),
        })
    }

    pub(super) fn finish(mut self) -> Result<(), RunnerError> {
        self.close()
    }

    fn close(&mut self) -> Result<(), RunnerError> {
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        let asked = ask(&self.socket, b'X');
        let joined = match worker.join() {
            Ok(answer) => answer,
            Err(payload) => {
                // Panic payloads can contain replies; retain only the failure's name.
                drop(payload);
                return Err(failed("the control worker panicked"));
            }
        };
        let removed = std::fs::remove_file(path(&self.socket)).map_err(failed);
        match (asked, joined, removed) {
            (Ok(_), Ok(()), Ok(())) => Ok(()),
            (asked, joined, removed) => {
                let errors = [asked.map(drop), joined, removed]
                    .into_iter()
                    .filter_map(Result::err)
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>();
                Err(failed(errors.join("; ")))
            }
        }
    }
}

impl Drop for Control {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            crate::error::said(&error.to_string());
        }
    }
}

fn serve(listener: &UnixListener, wait: &RetryWait) -> Result<(), RunnerError> {
    loop {
        let (mut stream, _) = listener.accept().map_err(failed)?;
        let mut command = [0];
        if let Err(error) = stream.read_exact(&mut command) {
            crate::error::said(&failed(error).to_string());
            continue;
        }
        let (closed, answer) = match command[0] {
            b'R' => (
                false,
                serde_json::json!({"reconnecting":true,"generation":wait.reconnect()?}),
            ),
            b'S' => (false, wait.status()?),
            b'X' => (true, serde_json::json!({"closed":true})),
            _ => (
                false,
                serde_json::json!({"refusal":"runner_reconnect_command_unknown"}),
            ),
        };
        serde_json::to_writer(&mut stream, &answer).map_err(failed)?;
        stream.flush().map_err(failed)?;
        if closed {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn local_reconnect_dispatches_a_wake_and_status_names_the_next_dial()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let socket = root.path().join("runner.sock");
        let wait = std::sync::Arc::new(super::RetryWait::default());
        wait.report(Some(&crate::dial::reconnect::RetryStatus {
            refused_at: 1000,
            next_dial: 1250,
            attempt: 1,
            reason: "connection refused".to_owned(),
        }))?;
        let control = super::Control::open(&socket, wait)?;
        let status = super::ask(&socket, b'S')?;
        assert!(
            status
                .to_string()
                .contains("Lys refused at 1000, next dial at 1250, attempt 1")
        );
        let answer = super::ask(&socket, b'R')?;
        assert_eq!(answer["reconnecting"], true);
        assert_eq!(answer["generation"], 1);
        control.finish()?;
        Ok(())
    }
}
