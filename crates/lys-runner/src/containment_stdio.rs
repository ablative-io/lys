//! Standard I/O grants are checked against the helper's actual inherited descriptors.

use std::os::fd::AsFd;

use rustix::fs::{FileType, Mode, OFlags, fstat, open};

use crate::containment_macos::Stdio;
use crate::error::RunnerError;

fn refused(words: impl Into<String>) -> RunnerError {
    RunnerError::refused("containment_stdio_unproved", words)
}

/// A stale terminal name cannot grant another session's terminal. Every stdio
/// descriptor must name the same character device, or each must be a pipe or
/// socket when the runner selected pipe transport. No ordinary file qualifies.
pub fn verify(stdio: &Stdio) -> Result<(), RunnerError> {
    let expected_device = match stdio {
        Stdio::Pipes => None,
        Stdio::Terminal { path } => {
            let device = open(
                path,
                OFlags::RDONLY
                    | OFlags::NOCTTY
                    | OFlags::NONBLOCK
                    | OFlags::NOFOLLOW
                    | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| refused(format!("terminal {}: {error}", path.display())))?;
            let stat = fstat(&device)
                .map_err(|error| refused(format!("terminal {}: {error}", path.display())))?;
            if FileType::from_raw_mode(stat.st_mode) != FileType::CharacterDevice {
                return Err(refused(format!(
                    "terminal {} is not a character device",
                    path.display()
                )));
            }
            Some(stat.st_rdev)
        }
    };
    let input = std::io::stdin();
    let output = std::io::stdout();
    let errors = std::io::stderr();
    for (number, fd) in [input.as_fd(), output.as_fd(), errors.as_fd()]
        .into_iter()
        .enumerate()
    {
        let stat = fstat(fd).map_err(|error| refused(format!("descriptor {number}: {error}")))?;
        let kind = FileType::from_raw_mode(stat.st_mode);
        match expected_device {
            Some(device) if kind == FileType::CharacterDevice && stat.st_rdev == device => {}
            None if matches!(kind, FileType::Fifo | FileType::Socket) => {}
            Some(_) => {
                return Err(refused(format!(
                    "descriptor {number} is not the declared terminal device"
                )));
            }
            None => {
                return Err(refused(format!(
                    "descriptor {number} is not a managed pipe or socket"
                )));
            }
        }
    }
    Ok(())
}
