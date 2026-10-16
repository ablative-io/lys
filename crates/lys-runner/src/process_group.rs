//! The group and session the kernel names for a process.
//!
//! Linux answers 0 for a process whose group or session it has already
//! detached as the process exits. That is no group at all, never a group
//! numbered 0: rustix's own `getpgid` asserts it never sees 0 and, past the
//! assertion, would carry 0 as a process id that cannot exist, so a stop that
//! raced a member's exit ended the runner. These read the kernel's answer
//! raw and say `None` for 0.

use rustix::io::Errno;
use rustix::process::Pid;

/// The process group the kernel names for `pid`, or `None` when the
/// process is exiting and belongs to no group any more.
///
/// # Errors
/// The kernel's refusal, `SRCH` when no such process exists.
pub(crate) fn group_of(pid: Pid) -> Result<Option<Pid>, Errno> {
    nix::unistd::getpgid(Some(native(pid)))
        .map(|group| Pid::from_raw(group.as_raw()))
        .map_err(errno)
}

/// The session the kernel names for `pid`, or `None` when the process is
/// exiting and belongs to no session any more.
///
/// # Errors
/// The kernel's refusal, `SRCH` when no such process exists.
pub(crate) fn session_of(pid: Pid) -> Result<Option<Pid>, Errno> {
    nix::unistd::getsid(Some(native(pid)))
        .map(|session| Pid::from_raw(session.as_raw()))
        .map_err(errno)
}

fn native(pid: Pid) -> nix::unistd::Pid {
    nix::unistd::Pid::from_raw(pid.as_raw_nonzero().get())
}

fn errno(error: nix::errno::Errno) -> Errno {
    Errno::from_raw_os_error(error as i32)
}

#[cfg(test)]
mod tests {
    #[test]
    fn this_process_is_named_in_its_own_group_and_session() -> Result<(), rustix::io::Errno> {
        let me = rustix::process::getpid();
        assert_eq!(super::group_of(me)?, Some(rustix::process::getpgrp()));
        assert_eq!(super::session_of(me)?, Some(rustix::process::getsid(None)?));
        Ok(())
    }

    #[test]
    fn a_process_that_does_not_exist_is_refused_by_the_kernel() {
        let gone = rustix::process::Pid::from_raw(i32::MAX);
        assert!(gone.is_some());
        if let Some(gone) = gone {
            assert_eq!(super::group_of(gone), Err(rustix::io::Errno::SRCH));
            assert_eq!(super::session_of(gone), Err(rustix::io::Errno::SRCH));
        }
    }
}
