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

#[test]
fn an_old_delivered_ticket_does_not_claim_the_disconnected_runner_is_still_present()
-> Result<(), Box<dyn Error>> {
    let runners = runners();
    let (reply, _answer) = std::sync::mpsc::channel();
    runners.hub().with("machine", |held| {
        held.delivered.insert("old-ticket".to_owned(), reply);
    })?;
    assert!(
        !runners.hub().bridged("machine")?,
        "a delivered ticket outlived its bridge and would leave the next start waiting forever"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_runner_between_two_handoffs_remains_connected_and_serves_both_requests()
-> Result<(), Box<dyn Error>> {
    let runners = runners();
    let waiting = Arc::clone(&runners);
    let bridge = tokio::spawn(async move { waiting.hub().next("machine").await });
    while !runners.hub().bridged("machine")? {
        tokio::task::yield_now().await;
    }
    let first = runners
        .hub()
        .queue("machine", Act::Liveness { session: None })?;
    let (ticket, _) = bridge.await??;
    runners
        .hub()
        .reply("machine", &ticket, Ok("first".to_owned()))?;
    assert_eq!(first.answer.recv()??, "first");
    let second = runners
        .hub()
        .queue("machine", Act::Liveness { session: None })?;
    let (ticket, _) = runners.hub().next("machine").await?;
    runners
        .hub()
        .reply("machine", &ticket, Ok("second".to_owned()))?;
    assert_eq!(second.answer.recv()??, "second");
    Ok(())
}

#[test]
fn retiring_an_epoch_clears_its_nonce_memory_without_accepting_its_signed_dials()
-> Result<(), Box<dyn Error>> {
    let runners = runners();
    let hub = runners.hub();
    hub.with("machine", |held| {
        held.nonces.insert("retired".to_owned());
    })?;
    let rejected = hub
        .fresh_in("machine", "retired", "retired-epoch")
        .err()
        .ok_or("retired signature accepted")?;
    assert_eq!(rejected.name(), "runner_dial_stale");
    assert!(hub.fresh("machine", "current")?);
    assert_eq!(hub.with("machine", |held| held.nonces.len())?, 1);
    assert!(!hub.fresh("machine", "current")?);
    Ok(())
}
