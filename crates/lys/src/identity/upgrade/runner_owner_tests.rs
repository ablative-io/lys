//! Kernel-proved retired owners do not block replacement of the main runner.

use super::*;
use lys_runner::seat_owner::recovery::{Found, Unreachable};

fn found(unreachable: Option<Unreachable>) -> Found {
    use lys_runner::peer::{Leader, StartIdentity};
    use lys_runner::seat_owner::protocol::{OwnerBinding, OwnerEndpoint};
    Found {
        seat: OwnedSeat {
            binding: OwnerBinding {
                seat: "retired".into(),
                session: "retired-session".into(),
                conversation: "old-conversation".into(),
                generation: 1,
            },
            endpoint: OwnerEndpoint {
                dir: "owners/retired-session".into(),
                socket: "owners/retired.sock".into(),
                runner: "retired-runner".into(),
                owner: Leader {
                    pid: 2,
                    start: StartIdentity("old-start".into()),
                },
                build: "0.3.1".into(),
            },
            established_at: 1,
        },
        unreachable,
    }
}

#[test]
fn kernel_proved_exited_owners_do_not_block_preserving_the_live_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let mut live = found(None).seat;
    live.binding.seat = "live".into();
    live.binding.session = "live-session".into();
    live.endpoint.owner.start.0 = "live-start".into();
    for reason in [
        Unreachable::Exited,
        Unreachable::PidReused {
            found: "new-start".into(),
        },
    ] {
        let answer = owners_captured(
            Ok(Answer::Owned {
                owners: vec![live.clone()],
                unreachable: vec![found(Some(reason))],
            }),
            Path::new("runner.sock"),
        )?;
        assert_eq!(answer, vec![live.clone()]);
    }
    Ok(())
}

#[test]
fn an_owner_with_unproved_liveness_still_refuses_the_upgrade() {
    for reason in [
        None,
        Some(Unreachable::Unproved {
            reason: "kernel unavailable".into(),
        }),
    ] {
        let answer = owners_captured(
            Ok(Answer::Owned {
                owners: Vec::new(),
                unreachable: vec![found(reason)],
            }),
            Path::new("runner.sock"),
        );
        assert!(answer.is_err_and(|error| {
            error
                .to_string()
                .contains("runner_owned_capture_unavailable")
        }));
    }
}

#[test]
fn a_capture_accepted_by_the_runner_requires_cancelling_its_fence()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, fenced) = captured(
        Ok(Answer::UpgradeSessions {
            sessions: Vec::new(),
        }),
        true,
        Path::new("runner.sock"),
    )?;
    assert!(fenced);
    Ok(())
}
