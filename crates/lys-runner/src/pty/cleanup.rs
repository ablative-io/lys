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
            self.end_member(pid);
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

    fn end_member(&mut self, pid: u32) {
        let start = match self.operations.start(pid) {
            Ok(Some(start)) => start,
            Ok(None) => return,
            Err(error) => {
                self.fail(format!("process {pid}: {error}"));
                return;
            }
        };
        if self.receipts.get(&pid) == Some(&start) {
            return;
        }
        let Some(basename) = self.member_name(pid, &start) else {
            return;
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
                return;
            }
            Err(error) => {
                self.fail(format!("process {pid} ({basename}): {error}"));
                return;
            }
            Ok(true) => {}
        }
        match self.operations.start(pid) {
            Ok(Some(current)) if current == start => {}
            Ok(None) => {
                self.pending.insert(pid, start);
                return;
            }
            Ok(Some(_)) => {
                self.fail(format!(
                    "process {pid} ({basename}): identity changed after exit registration"
                ));
                return;
            }
            Err(error) => {
                self.fail(format!("process {pid} ({basename}): {error}"));
                return;
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

    fn member_name(&mut self, pid: u32, start: &StartIdentity) -> Option<String> {
        match self.operations.basename(pid) {
            Ok(basename) => Some(basename),
            Err(error) => {
                match self.exited_before_name(pid, start) {
                    Ok(true) => {
                        self.receipts.insert(pid, start.clone());
                        self.words.push(format!(
                            "process {pid}: kernel exit confirmed before basename"
                        ));
                    }
                    Ok(false) => self.fail(format!("process {pid}: basename unreadable: {error}")),
                    Err(proof) => self.fail(format!(
                        "process {pid}: basename unreadable: {error}; exit unconfirmed: {proof}"
                    )),
                }
                None
            }
        }
    }

    fn exited_before_name(&mut self, pid: u32, start: &StartIdentity) -> Result<bool, RunnerError> {
        match self.operations.start(pid)? {
            None => return Ok(true),
            Some(current) if current != *start => {
                return Err(RunnerError::refused(
                    "process_identity_changed",
                    "identity changed before exit registration",
                ));
            }
            Some(_) => {}
        }
        if self.operations.register(pid)? {
            return Ok(false);
        }
        match self.operations.start(pid)? {
            Some(current) if current != *start => Err(RunnerError::refused(
                "process_identity_changed",
                "identity changed after registration ESRCH",
            )),
            _ => Ok(true),
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

#[path = "cleanup_kernel.rs"]
mod kernel;
pub(super) use kernel::Kernel;

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;
