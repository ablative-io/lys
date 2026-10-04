//! A session process's exit, seen by its parent: this runner's process,
//! whichever build of it is running.
//!
//! The exit is first seen without being collected, so the process stays
//! waitable: a runner that replaces its program between seeing an exit and
//! recording it leaves the exit for the new build, which sees it again with
//! its status. It is collected only once its end is recorded.

use rustix::process::{Pid, WaitId, WaitIdOptions, WaitOptions};

/// How a process ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Exit {
    /// Its exit status, when it exited.
    pub(crate) status: Option<u32>,
    /// The signal that ended it, by name, when one did.
    pub(crate) signal: Option<String>,
}

fn pid_of(pid: u32) -> Result<Pid, String> {
    i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or_else(|| format!("{pid} is not a process id"))
}

/// The name of signal `number`, as the records say it.
pub(crate) fn signal_name(number: i32) -> String {
    match number {
        1 => "SIGHUP".to_owned(),
        2 => "SIGINT".to_owned(),
        3 => "SIGQUIT".to_owned(),
        4 => "SIGILL".to_owned(),
        5 => "SIGTRAP".to_owned(),
        6 => "SIGABRT".to_owned(),
        9 => "SIGKILL".to_owned(),
        11 => "SIGSEGV".to_owned(),
        13 => "SIGPIPE".to_owned(),
        14 => "SIGALRM".to_owned(),
        15 => "SIGTERM".to_owned(),
        other => format!("signal {other}"),
    }
}

/// Wait for child `pid` to exit and answer how, leaving it waitable.
pub(crate) fn observe(pid: u32) -> Result<Exit, String> {
    let id = pid_of(pid)?;
    loop {
        match rustix::process::waitid(
            WaitId::Pid(id),
            WaitIdOptions::EXITED | WaitIdOptions::NOWAIT,
        ) {
            Ok(Some(status)) => {
                let signal = status.terminating_signal().map(signal_name);
                let code = if signal.is_some() {
                    None
                } else {
                    status
                        .exit_status()
                        .and_then(|code| u32::try_from(code).ok())
                };
                return Ok(Exit {
                    status: code,
                    signal,
                });
            }
            Ok(None) => {}
            Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(format!("process {pid}'s exit could not be seen: {error}")),
        }
    }
}

/// Collect child `pid`, whose exit [`observe`] saw, so it leaves no record
/// behind in the kernel.
pub(crate) fn reap(pid: u32) -> Result<(), String> {
    let id = pid_of(pid)?;
    loop {
        match rustix::process::waitpid(Some(id), WaitOptions::NOHANG) {
            Ok(_) => return Ok(()),
            Err(rustix::io::Errno::INTR) => {}
            Err(error) => return Err(format!("process {pid} could not be collected: {error}")),
        }
    }
}
