//! Told when a process this runner did not spawn exits, never polled for it.
//!
//! macOS registers `EVFILT_PROC`/`NOTE_EXIT` on a kqueue; Linux opens a pidfd
//! and waits for readability. The kernel waits for the exit event. A process
//! already gone, or a zombie the kernel will no longer name, answers gone at registration.
//!
//! A wait that could not be set up
//! is [`Exit::Unwatched`] with its cause, never read as a process still
//! running. A bare pid can be reused, so a caller that holds a start identity
//! re-checks it on any answer but [`Exit::Gone`].

/// What the kernel exit watch saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Exit {
    /// It exited, or no process has the pid.
    Gone,
    /// The kernel could not be asked to tell its exit; the cause is said.
    Unwatched(String),
}

/// Wait until `pid` exits. A pid that cannot name a process is gone.
pub(super) fn wait(pid: u32) -> Exit {
    let id = match i32::try_from(pid) {
        Ok(raw) if raw > 1 => match rustix::process::Pid::from_raw(raw) {
            Some(id) => id,
            None => return Exit::Unwatched(format!("{pid} is not a process id")),
        },
        Ok(_) | Err(_) => return Exit::Unwatched(format!("{pid} is not a process id")),
    };
    exited(id)
}

impl crate::protocol::Ended {
    /// Require the status or signal returned by the harness child's reap.
    ///
    /// # Errors
    ///
    /// `process_exit_unconfirmed` when the runner could not read its child's exit.
    pub fn require_reaped(&self, session: &str) -> Result<(), crate::error::RunnerError> {
        if self.at == 0 || (self.status.is_none() && self.signal.is_none()) {
            return Err(crate::error::RunnerError::refused(
                "process_exit_unconfirmed",
                format!("session {session}: the harness has no reaped exit status or signal"),
            ));
        }
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn exited(_pid: rustix::process::Pid) -> Exit {
    Exit::Unwatched("this system has no kernel exit notice this runner uses".to_owned())
}

#[cfg(target_os = "macos")]
fn exited(pid: rustix::process::Pid) -> Exit {
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
        match queue.kevent(&[], &mut events, None) {
            Ok(0) => return Exit::Unwatched("the exit watch ended without an event".to_owned()),
            Ok(_) => {
                return if events[0].filter() == Ok(EventFilter::EVFILT_PROC)
                    && events[0].fflags().contains(FilterFlag::NOTE_EXIT)
                {
                    Exit::Gone
                } else {
                    Exit::Unwatched("the kqueue answered an event that is not the exit".to_owned())
                };
            }
            Err(nix::errno::Errno::EINTR) => {}
            Err(error) => return Exit::Unwatched(format!("kevent: {error}")),
        }
    }
}

#[cfg(target_os = "linux")]
fn exited(pid: rustix::process::Pid) -> Exit {
    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
    use std::os::fd::AsFd;
    let descriptor = match rustix::process::pidfd_open(pid, rustix::process::PidfdFlags::empty()) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::SRCH) => return Exit::Gone,
        Err(error) => return Exit::Unwatched(format!("pidfd_open: {error}")),
    };
    loop {
        let mut descriptors = [PollFd::new(descriptor.as_fd(), PollFlags::POLLIN)];
        match poll(&mut descriptors, PollTimeout::NONE) {
            Ok(0) => return Exit::Unwatched("the exit watch ended without an event".to_owned()),
            Ok(_) => return Exit::Gone,
            Err(nix::errno::Errno::EINTR) => {}
            Err(error) => return Exit::Unwatched(format!("poll on the pidfd: {error}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Exit, wait};

    #[test]
    fn exit_rejects_invalid_process_ids_as_unwatched() {
        for pid in [0, 1, u32::MAX] {
            assert!(matches!(wait(pid), Exit::Unwatched(_)));
        }
    }

    #[test]
    fn process_exit_releases_the_kernel_watch() -> Result<(), Box<dyn std::error::Error>> {
        let mut child = std::process::Command::new("/bin/cat")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()?;
        let pid = child.id();
        let (start_sender, started) = std::sync::mpsc::channel();
        let (complete, completed) = std::sync::mpsc::channel();
        let watcher = std::thread::spawn(move || {
            start_sender.send(()).map_err(|error| error.to_string())?;
            complete.send(wait(pid)).map_err(|error| error.to_string())
        });
        started.recv()?;
        assert!(matches!(
            completed.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
        child.kill()?;
        child.wait()?;
        assert_eq!(completed.recv()?, Exit::Gone);
        watcher
            .join()
            .map_err(|error| format!("exit watcher panicked: {error:?}"))??;
        Ok(())
    }
}
