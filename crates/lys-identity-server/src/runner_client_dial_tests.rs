#![cfg(test)]

//! A runner that dials in is asked only while a bridge of its is dialled
//! in: a request for one nobody holds is refused at once, never queued for
//! a bridge that may never come (14 Oct 2026: one such machine held the
//! survey of every runner open for good).

use super::{Leave, RunnerRecord, Runners};
use lys_core::Ed25519Identity;
use lys_runner::{Act, RunnerError};
use std::error::Error;
use std::sync::Arc;

const KEY: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

fn dialled() -> RunnerRecord {
    RunnerRecord::Dialled {
        key: KEY.to_owned(),
        runner: None,
    }
}

fn runners() -> Arc<Runners> {
    Arc::new(Runners::new(Arc::new(Ed25519Identity::ephemeral()), None))
}

#[test]
fn a_dialled_runner_nobody_holds_is_refused_at_once_and_nothing_waits() -> Result<(), Box<dyn Error>>
{
    let runners = runners();
    let refused = runners
        .ask(
            "machine",
            &dialled(),
            &Act::Liveness { session: None },
            &Leave::default(),
        )
        .err()
        .ok_or("a runner nobody holds answered a request")?;
    assert_eq!(refused.name(), "runner_unreachable");
    assert!(refused.to_string().contains("not dialled in"));
    assert!(
        runners
            .hub()
            .with("machine", |held| held.queue.is_empty())?
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_dialled_runner_with_a_bridge_waiting_is_asked_and_its_answer_passed_on()
-> Result<(), Box<dyn Error>> {
    let runners = runners();
    let waiting = Arc::clone(&runners);
    let bridge = tokio::spawn(async move { waiting.hub().next("machine").await });
    while !runners.hub().bridged("machine")? {
        tokio::task::yield_now().await;
    }
    let asking = Arc::clone(&runners);
    let asked = tokio::task::spawn_blocking(move || {
        asking.ask(
            "machine",
            &dialled(),
            &Act::Liveness { session: None },
            &Leave::default(),
        )
    });
    let (ticket, act) = bridge.await??;
    assert!(matches!(act, Act::Liveness { session: None }));
    runners.hub().reply(
        "machine",
        &ticket,
        Err(RunnerError::refused("bridge_said", "the bridge answered")),
    )?;
    let answered = asked
        .await?
        .err()
        .ok_or("the bridge's refusal was not passed on to the caller")?;
    assert_eq!(answered.name(), "bridge_said");
    Ok(())
}
