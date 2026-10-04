//! A terminal's runner end as the runner's own descriptor: read, written,
//! resized and waited on through it, and kept by number when the runner
//! hands its sessions to a new build.

use std::fs::File;
use std::io::Read;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd, RawFd};

use crate::error::RunnerError;

fn failed(refusal: &str, error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused(refusal, error.to_string())
}

/// The runner's end of one session's terminal.
#[derive(Debug)]
pub struct Master {
    fd: OwnedFd,
}

/// The terminal's output. The end of output, once every process holding
/// the terminal has closed it, reads as the end of the file, as the kernel's
/// own "input/output error" says it on macOS.
#[derive(Debug)]
pub struct Reader {
    file: File,
}

impl Master {
    /// The terminal end `raw` names, made this runner's own.
    pub(crate) fn own(raw: RawFd) -> Result<Self, RunnerError> {
        let fd = crate::fd_own::own(raw)?;
        Self::checked(fd)
    }

    /// The terminal end `fd`, refused unless it is a terminal.
    pub(crate) fn checked(fd: OwnedFd) -> Result<Self, RunnerError> {
        if !rustix::termios::isatty(&fd) {
            return Err(RunnerError::refused(
                "terminal_invalid",
                "the descriptor kept for a session's terminal is not a terminal",
            ));
        }
        Ok(Self { fd })
    }

    /// The name of the terminal's other end, the one its processes hold.
    pub(crate) fn name(&self) -> Result<String, RunnerError> {
        let name = rustix::pty::ptsname(&self.fd, Vec::new())
            .map_err(|error| failed("terminal_unnamed", error))?;
        name.into_string()
            .map_err(|error| failed("terminal_unnamed", error))
    }

    /// A reader of the terminal's output.
    pub fn reader(&self) -> Result<Reader, RunnerError> {
        let fd = self
            .fd
            .try_clone()
            .map_err(|error| failed("spawn_failed", error))?;
        Ok(Reader {
            file: File::from(fd),
        })
    }

    /// A writer of the terminal's input.
    pub fn writer(&self) -> Result<File, RunnerError> {
        let fd = self
            .fd
            .try_clone()
            .map_err(|error| failed("spawn_failed", error))?;
        Ok(File::from(fd))
    }

    /// Resize the terminal.
    pub fn resize(&self, columns: u16, rows: u16) -> Result<(), RunnerError> {
        if columns == 0 || rows == 0 {
            return Err(RunnerError::refused(
                "size_invalid",
                "a terminal is at least one column wide and one row high",
            ));
        }
        rustix::termios::tcsetwinsize(
            &self.fd,
            rustix::termios::Winsize {
                ws_row: rows,
                ws_col: columns,
                ws_xpixel: 0,
                ws_ypixel: 0,
            },
        )
        .map_err(|error| failed("resize_failed", error))
    }
}

impl AsFd for Master {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.fd.as_fd()
    }
}

impl Read for Reader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self.file.read(buffer) {
            Err(error) if error.raw_os_error() == Some(rustix::io::Errno::IO.raw_os_error()) => {
                Ok(0)
            }
            read => read,
        }
    }
}

impl AsFd for Reader {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}
