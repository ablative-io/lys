//! Told when a process this runner did not spawn exits, never polled for it.
//!
//! macOS registers `EVFILT_PROC`/`NOTE_EXIT` on a kqueue; Linux opens a pidfd
//! and polls it for readability. Either way the wait sleeps in the kernel until
//! the exit or the bound, with no wake between. A process already gone, or a
//! zombie the kernel will no longer name, answers gone at registration.

use std::time::{Duration, Instant};

/// Whether `pid` exits within `within`. A pid that cannot name a process is gone.
pub(super) fn gone_within(pid: u32, within: Duration) -> bool {
    let Some(id) = i32::try_from(pid)
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    else {
        return true;
    };
    exited(id, Instant::now() + within)
}

#[cfg(target_os = "macos")]
fn exited(pid: rustix::process::Pid, until: Instant) -> bool {
    use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent, Kqueue};
    let Ok(queue) = Kqueue::new() else {
        return false;
    };
    let Ok(ident) = usize::try_from(pid.as_raw_nonzero().get()) else {
        return true;
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
            value if value == isize::from(nix::errno::Errno::ESRCH as i16) => return true,
            _ => return false,
        },
        _ => return false,
    }
    let mut events = [change];
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let timeout = nix::libc::timespec {
            tv_sec: nix::libc::time_t::try_from(left.as_secs()).unwrap_or(nix::libc::time_t::MAX),
            tv_nsec: nix::libc::c_long::from(left.subsec_nanos()),
        };
        match queue.kevent(&[], &mut events, Some(timeout)) {
            Ok(0) => return false,
            Ok(_) => {
                return events[0].filter() == Ok(EventFilter::EVFILT_PROC)
                    && events[0].fflags().contains(FilterFlag::NOTE_EXIT);
            }
            Err(nix::errno::Errno::EINTR) if Instant::now() < until => {}
            Err(_) => return false,
        }
    }
}

#[cfg(target_os = "linux")]
fn exited(pid: rustix::process::Pid, until: Instant) -> bool {
    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
    use std::os::fd::AsFd;
    let descriptor = match rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty()) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::SRCH) => return true,
        Err(_) => return false,
    };
    loop {
        let left = until.saturating_duration_since(Instant::now());
        let timeout = PollTimeout::try_from(left).unwrap_or(PollTimeout::MAX);
        let mut descriptors = [PollFd::new(descriptor.as_fd(), PollFlags::POLLIN)];
        match poll(&mut descriptors, timeout) {
            Ok(0) => return false,
            Ok(_) => return true,
            Err(nix::errno::Errno::EINTR) if Instant::now() < until => {}
            Err(_) => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::gone_within;
    use std::time::{Duration, Instant};

    #[test]
    fn an_exit_is_told_at_once_and_a_live_process_holds_to_the_bound()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut child = std::process::Command::new("sleep").arg("30").spawn()?;
        let pid = child.id();
        let began = Instant::now();
        assert!(!gone_within(pid, Duration::from_millis(200)));
        assert!(began.elapsed() >= Duration::from_millis(200));
        let waiter = std::thread::spawn(move || {
            let began = Instant::now();
            (gone_within(pid, Duration::from_secs(10)), began.elapsed())
        });
        child.kill()?;
        child.wait()?;
        let (gone, took) = waiter.join().map_err(|_| "the waiter panicked")?;
        assert!(gone, "the exit was not told");
        assert!(
            took < Duration::from_secs(2),
            "the exit was told after {took:?}"
        );
        Ok(())
    }

    #[test]
    fn a_pid_with_no_process_is_gone_without_waiting() -> Result<(), Box<dyn std::error::Error>> {
        let mut child = std::process::Command::new("true").spawn()?;
        let pid = child.id();
        child.wait()?;
        let began = Instant::now();
        assert!(gone_within(pid, Duration::from_secs(5)));
        assert!(began.elapsed() < Duration::from_secs(1));
        Ok(())
    }
}
