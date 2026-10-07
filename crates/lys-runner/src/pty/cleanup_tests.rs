use super::super::*;

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use std::io::{BufRead, BufReader};

    struct Harness(Spawned);
    impl Drop for Harness {
        fn drop(&mut self) {
            match self.0.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => {}
                Err(error) => eprintln!("cleanup status failed: {error}"),
            }
            if let Err(error) = self.0.child.kill() {
                eprintln!("cleanup signal failed: {error}");
            }
            if let Err(error) = self.0.child.wait() {
                eprintln!("cleanup reap failed: {error}");
            }
        }
    }

    #[test]
    fn a_group_probe_permission_refusal_still_ends_proved_members()
    -> Result<(), Box<dyn std::error::Error>> {
        let arguments = vec![
            "-c".to_owned(),
            "import signal\nprint('ready', flush=True)\nsignal.pause()".to_owned(),
        ];
        let mut harness = Harness(spawn(&Spawn {
            program: "/usr/bin/python3",
            arguments: &arguments,
            directory: "/",
            environment: &BTreeMap::new(),
            columns: 80,
            rows: 24,
        })?);
        let mut line = String::new();
        BufReader::new(&mut harness.0.reader).read_line(&mut line)?;
        assert_eq!(line.trim(), "ready");
        let leader = crate::peer::Leader {
            pid: harness.0.pid,
            start: crate::peer::start_identity(harness.0.pid)?,
        };
        let result = end_left_group_with(&leader, |_| Err(rustix::io::Errno::PERM))?;
        let Left::Ended {
            reason: Some(reason),
        } = result
        else {
            return Err(format!("proved group was not ended: {result:?}").into());
        };
        assert!(reason.contains("EPERM"), "{reason}");
        assert!(!harness.0.child.wait()?.success());
        Ok(())
    }
}

#[cfg(test)]
mod cycle_tests {
    use super::Left;
    use super::cleanup::{Operations, Plan};
    use crate::error::RunnerError;
    use crate::peer::{Leader, StartIdentity};
    use std::collections::{BTreeMap, VecDeque};
    use std::os::fd::BorrowedFd;

    #[derive(Default)]
    struct Kernel {
        listings: VecDeque<Vec<u32>>,
        current: BTreeMap<u32, StartIdentity>,
        actions: Vec<String>,
        unproved: Option<u32>,
        unregistered: Option<u32>,
        unsignalled: Option<u32>,
        unreadable: bool,
        cancelled: bool,
        reused: bool,
    }
    impl Operations for Kernel {
        fn start(&mut self, pid: u32) -> Result<Option<StartIdentity>, RunnerError> {
            Ok(self.current.get(&pid).cloned())
        }
        fn members(&mut self, _: u32) -> Result<Vec<u32>, RunnerError> {
            if self.unreadable {
                return Err(RunnerError::refused("process_group_unreadable", "denied"));
            }
            self.listings.pop_front().ok_or_else(|| {
                RunnerError::refused("fixture_listing_exhausted", "an unexpected extra listing")
            })
        }
        fn prove(&mut self, pid: u32, _: &StartIdentity, _: &Leader) -> Result<(), RunnerError> {
            self.actions.push(format!("prove:{pid}"));
            if self.unproved == Some(pid) {
                return Err(RunnerError::refused(
                    "process_session_mismatch",
                    "different session",
                ));
            }
            Ok(())
        }
        fn basename(&mut self, _: u32) -> Result<String, RunnerError> {
            Ok("harness".to_owned())
        }
        fn register(&mut self, pid: u32) -> Result<bool, RunnerError> {
            self.actions.push(format!("register:{pid}"));
            if self.unregistered == Some(pid) {
                return Err(RunnerError::refused(
                    "end_failed",
                    "registration EPERM errno 1",
                ));
            }
            Ok(true)
        }
        fn signal(&mut self, pid: u32) -> Result<(), RunnerError> {
            self.actions.push(format!("signal:{pid}"));
            if self.unsignalled == Some(pid) {
                return Err(RunnerError::refused("end_failed", "signal EPERM errno 1"));
            }
            Ok(())
        }
        fn wait(
            &mut self,
            pending: &BTreeMap<u32, StartIdentity>,
            _: Option<BorrowedFd<'_>>,
        ) -> Result<Vec<u32>, RunnerError> {
            self.actions.push("wait".to_owned());
            if self.cancelled {
                return Err(RunnerError::refused("end_failed", "cleanup cancelled"));
            }
            if self.reused {
                self.current
                    .insert(11, StartIdentity("new-start".to_owned()));
                self.unproved = Some(11);
            }
            Ok(pending.keys().copied().collect())
        }
    }
    fn plan(listings: Vec<Vec<u32>>) -> Plan<Kernel> {
        let kernel = Kernel {
            listings: listings.into(),
            current: [(10, "leader"), (11, "first"), (12, "late")]
                .into_iter()
                .map(|(pid, start)| (pid, StartIdentity(start.to_owned())))
                .collect(),
            ..Kernel::default()
        };
        Plan::prepare(
            Leader {
                pid: 10,
                start: StartIdentity("leader".to_owned()),
            },
            kernel,
            Err(rustix::io::Errno::PERM),
        )
    }
    #[test]
    fn exit_registration_precedes_signalling_and_success_waits_on_receipts() {
        let mut ending = plan(vec![vec![11], vec![]]);
        assert_eq!(
            ending.operations.actions,
            ["prove:11", "register:11", "signal:11"]
        );
        assert!(matches!(ending.wait(None), Left::Ended { .. }));
        assert_eq!(
            ending.operations.actions,
            ["prove:11", "register:11", "signal:11", "wait"]
        );
    }
    #[test]
    fn a_late_group_member_is_ended_in_a_second_cycle() {
        let mut ending = plan(vec![vec![11], vec![11, 12], vec![11, 12]]);
        assert!(matches!(ending.wait(None), Left::Ended { .. }));
        assert_eq!(
            ending.operations.actions,
            [
                "prove:11",
                "register:11",
                "signal:11",
                "wait",
                "prove:12",
                "register:12",
                "signal:12",
                "wait"
            ]
        );
    }
    #[test]
    fn an_unreaped_member_is_excluded_only_on_its_matching_exit_receipt() {
        let mut ending = plan(vec![vec![11], vec![11]]);
        assert!(matches!(ending.wait(None), Left::Ended { .. }));
        assert_eq!(
            ending
                .operations
                .actions
                .iter()
                .filter(|action| *action == "wait")
                .count(),
            1
        );
    }
    #[test]
    fn a_reused_pid_does_not_inherit_an_old_exit_receipt() {
        let mut ending = plan(vec![vec![11], vec![11]]);
        ending.operations.reused = true;
        let result = ending.wait(None);
        assert!(matches!(result, Left::Unended { .. }), "{result:?}");
        assert_eq!(
            ending
                .operations
                .actions
                .iter()
                .filter(|action| *action == "prove:11")
                .count(),
            2
        );
    }
    #[test]
    fn a_cancelled_exit_wait_names_every_unconfirmed_member() {
        let mut ending = plan(vec![vec![11, 12]]);
        ending.operations.cancelled = true;
        let Left::Unended { reason } = ending.wait(None) else {
            panic!("cancelled cleanup passed");
        };
        assert!(
            reason.contains("cancelled") && reason.contains("[11,12]"),
            "{reason}"
        );
    }
    #[test]
    fn one_unprovable_member_refuses_the_whole_cleanup() {
        let mut ending = plan(vec![vec![11], vec![11, 12]]);
        ending.operations.unproved = Some(12);
        let Left::Unended { reason } = ending.wait(None) else {
            panic!("unproved cleanup passed");
        };
        assert!(
            reason.contains("process 12 (harness)") && reason.contains("process_session_mismatch"),
            "{reason}"
        );
        assert!(
            !ending
                .operations
                .actions
                .iter()
                .any(|action| action == "signal:12")
        );
    }
    #[test]
    fn unreadable_registration_and_signal_refusals_do_not_pass() {
        for refusal in 0..3 {
            let mut ending = plan(vec![vec![11], vec![12]]);
            match refusal {
                0 => ending.operations.unreadable = true,
                1 => ending.operations.unregistered = Some(12),
                _ => ending.operations.unsignalled = Some(12),
            }
            assert!(matches!(ending.wait(None), Left::Unended { .. }));
        }
    }
}
