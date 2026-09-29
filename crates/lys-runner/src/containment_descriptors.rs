//! A single-threaded launch helper closes inherited descriptors at exec.
//! Unlike the PTY dependency's best-effort cleanup, enumeration, marking and
//! readback failures are named refusals. Never call this in the runner daemon.

use nix::fcntl::{FcntlArg, FdFlag, fcntl};
use rustix::fs::{Dir, Mode, OFlags, open};

use crate::error::RunnerError;

fn refused(what: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("containment_descriptors_unavailable", what.to_string())
}

#[cfg(target_os = "macos")]
fn threads() -> Result<usize, RunnerError> {
    let pid = i32::try_from(std::process::id()).map_err(refused)?;
    let info = libproc::proc_pid::pidinfo::<libproc::task_info::TaskInfo>(pid, 0)
        .map_err(|error| refused(format!("process {pid}: {error}")))?;
    usize::try_from(info.pti_threadnum).map_err(refused)
}

#[cfg(target_os = "linux")]
fn threads() -> Result<usize, RunnerError> {
    let entries = std::fs::read_dir("/proc/self/task")
        .map_err(|error| refused(format!("/proc/self/task: {error}")))?;
    let mut count = 0;
    for entry in entries {
        entry.map_err(|error| refused(format!("/proc/self/task: {error}")))?;
        count += 1;
    }
    Ok(count)
}

fn mark(fd: i32) -> Result<(), RunnerError> {
    let flags = fcntl(fd, FcntlArg::F_GETFD)
        .map_err(|error| refused(format!("descriptor {fd} flags: {error}")))?;
    let desired = FdFlag::from_bits_retain(flags) | FdFlag::FD_CLOEXEC;
    fcntl(fd, FcntlArg::F_SETFD(desired))
        .map_err(|error| refused(format!("descriptor {fd} close-on-exec: {error}")))?;
    let actual = fcntl(fd, FcntlArg::F_GETFD)
        .map_err(|error| refused(format!("descriptor {fd} readback: {error}")))?;
    if !FdFlag::from_bits_retain(actual).contains(FdFlag::FD_CLOEXEC) {
        return Err(refused(format!("descriptor {fd} is still inheritable")));
    }
    Ok(())
}

/// Mark every descriptor above stdio close-on-exec and verify every result.
/// Call only in the fresh, single-threaded trusted helper immediately before
/// its exec. The helper must open nothing inheritable afterwards. A live
/// multithreaded runner is explicitly refused before its descriptors change.
/// This function proves descriptor preparation, not native sandbox readiness.
pub fn seal_for_exec() -> Result<usize, RunnerError> {
    let count = threads()?;
    if count != 1 {
        return Err(refused(format!(
            "launch helper has {count} threads; exactly one is required"
        )));
    }
    let directory = open(
        "/dev/fd",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| refused(format!("/dev/fd: {error}")))?;
    // Keep the iterator alive until marking is finished. Its descriptor must
    // not disappear from our own enumeration and be mistaken for a failure.
    let mut entries = Dir::new(directory).map_err(|error| refused(format!("/dev/fd: {error}")))?;
    let mut sealed = 0;
    for entry in &mut entries {
        let entry = entry.map_err(|error| refused(format!("/dev/fd entry: {error}")))?;
        let name = entry
            .file_name()
            .to_str()
            .map_err(|error| refused(format!("/dev/fd name: {error}")))?;
        if name == "." || name == ".." {
            continue;
        }
        let fd: i32 = name
            .parse()
            .map_err(|error| refused(format!("/dev/fd/{name}: {error}")))?;
        if fd < 0 {
            return Err(refused(format!("/dev/fd/{name} is not a descriptor")));
        }
        if fd > 2 {
            mark(fd)?;
            sealed += 1;
        }
    }
    Ok(sealed)
}

#[cfg(test)]
#[path = "containment_descriptor_tests.rs"]
mod tests;
