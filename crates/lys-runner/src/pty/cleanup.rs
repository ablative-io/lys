use std::collections::BTreeMap;
use std::os::fd::BorrowedFd;

use super::{Left, group};
use crate::error::RunnerError;
use crate::peer::{Leader, StartIdentity};

pub(super) trait Operations {
    fn start(&mut self, pid: u32) -> Result<Option<StartIdentity>, RunnerError>;
    fn members(&mut self, group: u32) -> Result<Vec<u32>, RunnerError>;
    fn prove(
        &mut self,
        pid: u32,
        start: &StartIdentity,
        leader: &Leader,
    ) -> Result<(), RunnerError>;
    fn basename(&mut self, pid: u32) -> Result<String, RunnerError>;
    fn register(&mut self, pid: u32) -> Result<bool, RunnerError>;
    fn signal(&mut self, pid: u32) -> Result<(), RunnerError>;
    fn wait(
        &mut self,
        pending: &BTreeMap<u32, StartIdentity>,
        cancel: Option<BorrowedFd<'_>>,
    ) -> Result<Vec<u32>, RunnerError>;
}

pub(super) struct Plan<O> {
    leader: Leader,
    pub(super) operations: O,
    pending: BTreeMap<u32, StartIdentity>,
    receipts: BTreeMap<u32, StartIdentity>,
    words: Vec<String>,
    failed: bool,
    signalled: bool,
    result: Option<Left>,
}

impl<O: Operations> Plan<O> {
    pub(super) fn prepare(leader: Leader, operations: O, probe: rustix::io::Result<()>) -> Self {
        let mut plan = Self {
            leader,
            operations,
            pending: BTreeMap::new(),
            receipts: BTreeMap::new(),
            words: Vec::new(),
            failed: false,
            signalled: false,
            result: None,
        };
        match probe {
            Err(rustix::io::Errno::SRCH) => plan.result = Some(Left::Gone),
            Err(rustix::io::Errno::PERM) => {
                plan.words.push(format!(
                    "process group {} probe: EPERM (errno 1)",
                    plan.leader.pid
                ));
                plan.begin();
            }
            Err(error) => plan.fail(format!("process group {} probe: {error}", plan.leader.pid)),
            Ok(()) => plan.begin(),
        }
        plan
    }

    fn fail(&mut self, words: String) {
        self.failed = true;
        self.words.push(words);
    }

    fn begin(&mut self) {
        match self.operations.start(self.leader.pid) {
            Ok(Some(start)) if start != self.leader.start => {
                self.result = Some(Left::Reused);
                return;
            }
            Ok(_) => {}
            Err(error) => {
                self.fail(error.to_string());
                return;
            }
        }
        let members = match self.operations.members(self.leader.pid) {
            Ok(members) => members
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>(),
            Err(error) => {
                self.fail(error.to_string());
                return;
            }
        };
        for pid in members {
            let start = match self.operations.start(pid) {
                Ok(Some(start)) => start,
                Ok(None) => continue,
                Err(error) => {
                    self.fail(format!("process {pid}: {error}"));
                    continue;
                }
            };
            if self.receipts.get(&pid) == Some(&start) {
                continue;
            }
            let basename = match self.operations.basename(pid) {
                Ok(basename) => basename,
                Err(error) => {
                    self.fail(format!("process {pid}: basename unreadable: {error}"));
                    continue;
                }
            };
            let checked = self
                .operations
                .prove(pid, &start, &self.leader)
                .and_then(|()| self.operations.register(pid));
            match checked {
                Ok(false) => {
                    match self.operations.start(pid) {
                        Ok(current) if current.as_ref().is_none_or(|current| current == &start) => {
                            self.receipts.insert(pid, start);
                        }
                        Ok(_) => self.fail(format!(
                            "process {pid} ({basename}): identity changed after registration ESRCH"
                        )),
                        Err(error) => self.fail(format!("process {pid} ({basename}): {error}")),
                    }
                    self.words.push(format!(
                        "process {pid} ({basename}): registration ESRCH (errno 3)"
                    ));
                    continue;
                }
                Err(error) => {
                    self.fail(format!("process {pid} ({basename}): {error}"));
                    continue;
                }
                Ok(true) => {}
            }
            match self.operations.start(pid) {
                Ok(Some(current)) if current == start => {}
                Ok(None) => {
                    self.pending.insert(pid, start);
                    continue;
                }
                Ok(Some(_)) => {
                    self.fail(format!(
                        "process {pid} ({basename}): identity changed after exit registration"
                    ));
                    continue;
                }
                Err(error) => {
                    self.fail(format!("process {pid} ({basename}): {error}"));
                    continue;
                }
            }
            match self.operations.signal(pid) {
                Ok(()) => {
                    self.signalled = true;
                    self.pending.insert(pid, start);
                    self.words.push(format!(
                        "process {pid} ({basename}): SIGKILL accepted (errno 0); exit awaited"
                    ));
                }
                Err(error) => self.fail(format!("process {pid} ({basename}): {error}")),
            }
        }
        if self.pending.is_empty() && !self.failed {
            self.result = Some(if self.signalled || !self.words.is_empty() {
                Left::Ended {
                    reason: Some(self.words.join("; ")),
                }
            } else {
                Left::Gone
            });
        }
    }

    pub(super) fn wait(&mut self, cancel: Option<BorrowedFd<'_>>) -> Left {
        loop {
            if let Some(result) = self.result.take() {
                return result;
            }
            if self.failed {
                return self.refuse("cleanup refused");
            }
            if self.pending.is_empty() {
                self.begin();
                continue;
            }
            let events = match self.operations.wait(&self.pending, cancel) {
                Ok(events) if !events.is_empty() => events,
                Ok(_) => return self.refuse("exit notification returned no receipts"),
                Err(error) => return self.refuse(&error.to_string()),
            };
            for pid in events {
                let Some(start) = self.pending.remove(&pid) else {
                    return self.refuse(&format!("unexpected exit receipt for process {pid}"));
                };
                self.receipts.insert(pid, start);
                self.words
                    .push(format!("process {pid}: kernel exit observed"));
            }
            if self.pending.is_empty() {
                self.begin();
            }
        }
    }

    pub(super) fn refuse(&self, reason: &str) -> Left {
        let pending = self
            .pending
            .keys()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        Left::Unended {
            reason: format!(
                "{}; {reason}; exits still awaited: [{pending}]",
                self.words.join("; ")
            ),
        }
    }

    pub(super) fn decline(mut self) -> Left {
        match self.result.take() {
            Some(result) => result,
            None => self.refuse("exit wait not performed"),
        }
    }
}

pub(super) struct Kernel {
    #[cfg(target_os = "macos")]
    queue: nix::sys::event::Kqueue,
    #[cfg(target_os = "linux")]
    descriptors: BTreeMap<u32, std::os::fd::OwnedFd>,
}

fn refused(error: impl std::fmt::Display) -> RunnerError {
    RunnerError::refused("end_failed", error.to_string())
}

impl Kernel {
    #[cfg(target_os = "macos")]
    pub(super) fn new() -> Result<Self, RunnerError> {
        Ok(Self {
            queue: nix::sys::event::Kqueue::new().map_err(refused)?,
        })
    }
    #[cfg(target_os = "linux")]
    pub(super) fn new() -> Self {
        Self {
            descriptors: BTreeMap::new(),
        }
    }

    #[cfg(target_os = "macos")]
    fn add(
        &self,
        ident: usize,
        filter: nix::sys::event::EventFilter,
        flags: nix::sys::event::FilterFlag,
    ) -> Result<bool, RunnerError> {
        use nix::sys::event::{EventFlag, KEvent};
        let change = KEvent::new(
            ident,
            filter,
            EventFlag::EV_ADD | EventFlag::EV_ONESHOT | EventFlag::EV_RECEIPT,
            flags,
            0,
            0,
        );
        let mut receipt = [change];
        let count = self
            .queue
            .kevent(&[change], &mut receipt, None)
            .map_err(refused)?;
        if count != 1 || !receipt[0].flags().contains(EventFlag::EV_ERROR) {
            return Err(refused("exit registration receipt missing"));
        }
        match receipt[0].data() {
            0 => Ok(true),
            value if value == isize::from(nix::errno::Errno::ESRCH as i16) => Ok(false),
            value => Err(refused(format!("exit registration errno {value}"))),
        }
    }
}

impl Operations for Kernel {
    fn start(&mut self, pid: u32) -> Result<Option<StartIdentity>, RunnerError> {
        #[cfg(target_os = "macos")]
        {
            // The nonzero selector includes an exited process before its parent reaps it.
            match libproc::proc_pid::pidinfo::<libproc::bsd_info::BSDInfo>(
                group(pid)?.as_raw_nonzero().get(),
                1,
            ) {
                Ok(info) => Ok(Some(StartIdentity(format!(
                    "macos:{}.{:06}",
                    info.pbi_start_tvsec, info.pbi_start_tvusec
                )))),
                Err(error) => match rustix::process::test_kill_process(group(pid)?) {
                    Err(rustix::io::Errno::SRCH) => Ok(None),
                    probe => Err(refused(format!(
                        "process {pid} start unreadable: {error}; existence check: {probe:?}"
                    ))),
                },
            }
        }
        #[cfg(target_os = "linux")]
        {
            crate::peer::present_start(pid)
        }
    }
    fn members(&mut self, pid: u32) -> Result<Vec<u32>, RunnerError> {
        crate::peer::group_members(pid).or_else(|error| {
            match rustix::process::test_kill_process_group(group(pid)?) {
                Err(rustix::io::Errno::SRCH) => Ok(Vec::new()),
                _ => Err(error),
            }
        })
    }
    fn prove(
        &mut self,
        pid: u32,
        start: &StartIdentity,
        leader: &Leader,
    ) -> Result<(), RunnerError> {
        let member = group(pid)?;
        let recorded = group(leader.pid)?;
        if rustix::process::getsid(Some(member)).map_err(refused)? != recorded
            || rustix::process::getpgid(Some(member)).map_err(refused)? != recorded
        {
            return Err(refused(
                "member no longer belongs to the recorded session and group",
            ));
        }
        if !crate::peer::started_not_before(start, &leader.start)? {
            return Err(refused("member started before the recorded leader"));
        }
        Ok(())
    }
    fn basename(&mut self, pid: u32) -> Result<String, RunnerError> {
        #[cfg(target_os = "macos")]
        let path = std::path::PathBuf::from(
            libproc::proc_pid::pidpath(group(pid)?.as_raw_nonzero().get()).map_err(refused)?,
        );
        #[cfg(target_os = "linux")]
        let path = std::fs::read_link(format!("/proc/{pid}/exe")).map_err(refused)?;
        path.file_name()
            .and_then(std::ffi::OsStr::to_str)
            .map(str::to_owned)
            .ok_or_else(|| refused("executable basename unreadable"))
    }
    fn register(&mut self, pid: u32) -> Result<bool, RunnerError> {
        #[cfg(target_os = "macos")]
        {
            self.add(
                usize::try_from(pid).map_err(refused)?,
                nix::sys::event::EventFilter::EVFILT_PROC,
                nix::sys::event::FilterFlag::NOTE_EXIT,
            )
        }
        #[cfg(target_os = "linux")]
        {
            match rustix::process::pidfd_open(group(pid)?, rustix::process::PidfdFlags::empty()) {
                Ok(fd) => {
                    self.descriptors.insert(pid, fd);
                    Ok(true)
                }
                Err(rustix::io::Errno::SRCH) => Ok(false),
                Err(error) => Err(refused(error)),
            }
        }
    }
    fn signal(&mut self, pid: u32) -> Result<(), RunnerError> {
        #[cfg(target_os = "macos")]
        let answer = rustix::process::kill_process(group(pid)?, rustix::process::Signal::KILL);
        #[cfg(target_os = "linux")]
        let answer = rustix::process::pidfd_send_signal(
            self.descriptors
                .get(&pid)
                .ok_or_else(|| refused("exit descriptor missing"))?,
            rustix::process::Signal::KILL,
        );
        match answer {
            Ok(()) | Err(rustix::io::Errno::SRCH) => Ok(()),
            Err(error) => Err(refused(error)),
        }
    }
    fn wait(
        &mut self,
        pending: &BTreeMap<u32, StartIdentity>,
        cancel: Option<BorrowedFd<'_>>,
    ) -> Result<Vec<u32>, RunnerError> {
        #[cfg(target_os = "macos")]
        {
            use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent};
            use std::os::fd::AsRawFd;
            if let Some(fd) = cancel {
                if !self.add(
                    usize::try_from(fd.as_raw_fd()).map_err(refused)?,
                    EventFilter::EVFILT_READ,
                    FilterFlag::empty(),
                )? {
                    return Err(refused("cancellation descriptor disappeared"));
                }
            }
            let mut events = [KEvent::new(
                0,
                EventFilter::EVFILT_PROC,
                EventFlag::empty(),
                FilterFlag::empty(),
                0,
                0,
            ); 64];
            loop {
                let count = match self.queue.kevent(&[], &mut events, None) {
                    Err(nix::errno::Errno::EINTR) => continue,
                    answer => answer.map_err(refused)?,
                };
                let mut exits = Vec::new();
                for event in &events[..count] {
                    if event.flags().contains(EventFlag::EV_ERROR) {
                        return Err(refused(format!("exit event errno {}", event.data())));
                    }
                    match event.filter().map_err(refused)? {
                        EventFilter::EVFILT_READ => return Err(refused("cleanup cancelled")),
                        EventFilter::EVFILT_PROC
                            if event.fflags().contains(FilterFlag::NOTE_EXIT) =>
                        {
                            let pid = u32::try_from(event.ident()).map_err(refused)?;
                            if !pending.contains_key(&pid) {
                                return Err(refused(format!(
                                    "unexpected exit event for process {pid}"
                                )));
                            }
                            exits.push(pid);
                        }
                        _ => return Err(refused("exit event kind unsupported")),
                    }
                }
                return Ok(exits);
            }
        }
        #[cfg(target_os = "linux")]
        {
            use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
            use std::os::fd::AsFd;
            let pids: Vec<_> = pending.keys().copied().collect();
            let mut descriptors = pids
                .iter()
                .map(|pid| {
                    self.descriptors
                        .get(pid)
                        .map(|fd| PollFd::new(fd.as_fd(), PollFlags::POLLIN))
                        .ok_or_else(|| {
                            refused(format!("exit descriptor missing for process {pid}"))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            if let Some(fd) = cancel {
                descriptors.push(PollFd::new(fd, PollFlags::POLLIN));
            }
            loop {
                match poll(&mut descriptors, PollTimeout::NONE) {
                    Err(nix::errno::Errno::EINTR) => continue,
                    result => {
                        result.map_err(refused)?;
                        break;
                    }
                }
            }
            let mut exits = Vec::new();
            for (index, descriptor) in descriptors.iter().enumerate() {
                let flags = descriptor
                    .revents()
                    .ok_or_else(|| refused("exit readiness flags unsupported"))?;
                if flags.intersects(PollFlags::POLLERR | PollFlags::POLLNVAL) {
                    return Err(refused("exit descriptor readiness failed"));
                }
                if flags.intersects(PollFlags::POLLIN | PollFlags::POLLHUP) {
                    if index == pending.len() {
                        return Err(refused("cleanup cancelled"));
                    }
                    exits.push(
                        *pids
                            .get(index)
                            .ok_or_else(|| refused("exit descriptor index invalid"))?,
                    );
                }
            }
            Ok(exits)
        }
    }
}

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;
