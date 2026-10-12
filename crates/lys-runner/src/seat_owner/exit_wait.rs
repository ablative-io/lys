//! Told when a process this runner did not spawn exits, never polled for it.
//!
//! macOS registers `EVFILT_PROC`/`NOTE_EXIT` on a kqueue; Linux opens a pidfd
//! and polls it for readability. Either way the wait sleeps in the kernel until
//! the exit or the bound, with no wake between. A process already gone, or a
//! zombie the kernel will no longer name, answers gone at registration.
//!
//! The answer has three states, never two: a wait that could not be set up
//! is [`Exit::Unwatched`] with its cause, never read as a process still
//! running. A bare pid can be reused, so a caller that holds a start identity
//! re-checks it on any answer but [`Exit::Gone`].

use std::time::{Duration, Instant};

/// What a bounded wait on a process's exit saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Exit {
    /// It exited, or no process has the pid.
    Gone,
    /// It was still running at the bound.
    Running,
    /// The kernel could not be asked to tell its exit; the cause is said.
    Unwatched(String),
}

/// Whether `pid` exits within `within`. A pid that cannot name a process is gone.
pub(super) fn exit_within(pid: u32, within: Duration) -> Exit {
    let Some(id) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    else {
        return Exit::Gone;
    };
    exited(id, Instant::now() + within)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn exited(_pid: rustix::process::Pid, _until: Instant) -> Exit {
    Exit::Unwatched("this system has no kernel exit notice this runner uses".to_owned())
}

#[cfg(target_os = "macos")]
fn exited(pid: rustix::process::Pid, until: Instant) -> Exit {
    use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent, Kqueue};
    let queue = match Kqueue::new() {
        Ok(queue) => queue,
        Err(error) => return Exit::Unwatched(format!("kqueue: {error}")),
    };
    let Ok(ident) = usize::try_from(pid.as_raw_nonzero().get()) else {
        return Exit::Gone;
    };
    let change = KEvent::new(
        ident,
        EventFilter::EVFILT_PROC,
        EventFlag::EV_ADD | EventFlag::EV_ONESHOT | EventFlag::EV_RECEIPT,
        FilterFlag::NOTE_EXIT,
        0,
        0,
    );
    let mut receipt = [change];
    match queue.kevent(&[change], &mut receipt, None) {
        Ok(1) if receipt[0].flags().contains(EventFlag::EV_ERROR) => match receipt[0].data() {
            0 => {}
            value if value == isize::from(nix::errno::Errno::ESRCH as i16) => return Exit::Gone,
            value => return Exit::Unwatched(format!("NOTE_EXIT registration: errno {value}")),
        },
        Ok(count) => {
            return Exit::Unwatched(format!("NOTE_EXIT registration answered {count} events"));
        }
        Err(error) => return Exit::Unwatched(format!("NOTE_EXIT registration: {error}")),
    }
    let mut events = [change];
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let timeout = nix::libc::timespec {
            tv_sec: nix::libc::time_t::try_from(left.as_secs()).unwrap_or(nix::libc::time_t::MAX),
            tv_nsec: nix::libc::c_long::from(left.subsec_nanos()),
        };
        match queue.kevent(&[], &mut events, Some(timeout)) {
            Ok(0) => return Exit::Running,
            Ok(_) => {
                return if events[0].filter() == Ok(EventFilter::EVFILT_PROC)
                    && events[0].fflags().contains(FilterFlag::NOTE_EXIT)
                {
                    Exit::Gone
                } else {
                    Exit::Unwatched("the kqueue answered an event that is not the exit".to_owned())
                };
            }
            Err(nix::errno::Errno::EINTR) if Instant::now() < until => {}
            Err(error) => return Exit::Unwatched(format!("kevent: {error}")),
        }
    }
}

#[cfg(target_os = "linux")]
fn exited(pid: rustix::process::Pid, until: Instant) -> Exit {
    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
    use std::os::fd::AsFd;
    let descriptor = match rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty()) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::SRCH) => return Exit::Gone,
        Err(error) => return Exit::Unwatched(format!("pidfd_open: {error}")),
    };
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let timeout = PollTimeout::try_from(left).unwrap_or(PollTimeout::MAX);
        let mut descriptors = [PollFd::new(descriptor.as_fd(), PollFlags::POLLIN)];
        match poll(&mut descriptors, timeout) {
            Ok(0) => return Exit::Running,
            Ok(_) => return Exit::Gone,
            Err(nix::errno::Errno::EINTR) if Instant::now() < until => {}
            Err(error) => return Exit::Unwatched(format!("poll on the pidfd: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Exit, exit_within};
    use std::time::{Duration, Instant};

    #[test]
    fn an_exit_is_told_at_once_and_a_live_process_holds_to_the_bound()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut child = std::process::Command::new("sleep").arg("30").spawn()?;
        let pid = child.id();
        let began = Instant::now();
        assert_eq!(exit_within(pid, Duration::from_millis(200)), Exit::Running);
        assert!(began.elapsed() >= Duration::from_millis(200));
        let waiter = std::thread::spawn(move || {
            let began = Instant::now();
            (exit_within(pid, Duration::from_secs(10)), began.elapsed())
        });
        child.kill()?;
        child.wait()?;
        let (gone, took) = waiter.join().map_err(|_| "the waiter panicked")?;
        assert_eq!(gone, Exit::Gone, "the exit was not told");
        assert!(
            took < Duration::from_secs(2),
            "the exit was told after {took:?}"
        );
        Ok(())
    }

    /// A reaped pid is gone; if the kernel has meanwhile given the pid to
    /// another process, the wait may run to its bound, and the pid then names
    /// a process with another start identity, which is what a caller re-checks.
    #[test]
    fn a_pid_with_no_process_is_gone_without_waiting() -> Result<(), Box<dyn std::error::Error>> {
        let mut child = std::process::Command::new("sleep").arg("30").spawn()?;
        let pid = child.id();
        let start = crate::peer::start_identity(pid)?;
        child.kill()?;
        child.wait()?;
        let began = Instant::now();
        match exit_within(pid, Duration::from_secs(5)) {
            Exit::Gone => assert!(began.elapsed() < Duration::from_secs(1)),
            other => assert!(
                crate::peer::start_identity(pid).ok() != Some(start),
                "pid {pid} answered {other:?} and still names the reaped process"
            ),
        }
        Ok(())
    }
}
