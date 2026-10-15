//! Departed callers can read the first completed reply, including after a duplicate post.

use super::DialHub;
use lys_runner::Act;
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[tokio::test]
async fn a_late_completion_is_retained_after_its_caller_leaves() -> TestResult {
    let hub = DialHub::default();
    hub.with("machine", |held| held.bridges = 1)?;
    let pending = hub.queue("machine", Act::Status { session: None })?;
    let (ticket, _) = hub.next("machine").await?;
    hub.withdraw("machine", &ticket)?;
    drop(pending);
    let error = hub
        .reply("machine", &ticket, Ok("finished".to_owned()))
        .err()
        .ok_or("the original caller had already left")?;
    assert_eq!(error.name(), "runner_reply_expired");
    assert_eq!(
        hub.completion("machine", &ticket)?.as_deref(),
        Some("finished"),
        "the completed reply was discarded after the caller left"
    );
    assert!(hub.acknowledge_completion("machine", &ticket)?);
    assert!(hub.completion("machine", &ticket)?.is_none());
    let duplicate = hub
        .reply("machine", &ticket, Ok("finished".to_owned()))
        .err()
        .ok_or("acknowledged reply admitted twice")?;
    assert_eq!(duplicate.name(), "runner_reply_duplicate");
    assert!(hub.completion("machine", &ticket)?.is_none());
    Ok(())
}

#[tokio::test]
async fn a_duplicate_post_preserves_the_first_completion_for_readback() -> TestResult {
    let hub = DialHub::default();
    hub.with("machine", |held| held.bridges = 1)?;
    let pending = hub.queue("machine", Act::Status { session: None })?;
    let (ticket, _) = hub.next("machine").await?;
    let first = lys_runner::protocol::reply_line(lys_runner::Answer::Liveness {
        sessions: Vec::new(),
    });
    hub.reply("machine", &ticket, Ok(first.clone()))?;
    assert!(
        hub.reply("machine", &ticket, Ok("second".to_owned()))
            .is_err()
    );
    assert_eq!(
        hub.completion("machine", &ticket)?.as_deref(),
        Some(first.as_str()),
        "the first completion was not readable"
    );
    assert_eq!(hub.completion("another-machine", &ticket)?, None);
    assert_eq!(pending.answer.recv()??, first);
    assert!(
        hub.completion("machine", &ticket)?.is_none(),
        "a received reply still consumes completion capacity"
    );
    let duplicate = hub
        .reply("machine", &ticket, Ok("second".to_owned()))
        .err()
        .ok_or("duplicate admitted after receipt")?;
    assert_eq!(duplicate.name(), "runner_reply_duplicate");
    assert!(hub.completion("machine", &ticket)?.is_none());
    Ok(())
}

#[tokio::test]
async fn unacknowledged_completions_refuse_more_work_instead_of_being_evicted() -> TestResult {
    let hub = DialHub::default();
    hub.with("machine", |held| {
        held.bridges = 1;
        for id in 0..super::FINISHED {
            held.completions
                .insert(id.to_string(), "finished".to_owned());
        }
    })?;
    let error = hub
        .queue("machine", Act::Status { session: None })
        .err()
        .ok_or("full completion storage silently accepted more work")?;
    assert_eq!(error.name(), "runner_completion_capacity");
    assert!(hub.acknowledge_completion("machine", "0")?);
    let queued = hub.queue("machine", Act::Status { session: None })?;
    hub.withdraw("machine", &queued.ticket)?;
    Ok(())
}

#[tokio::test]
async fn a_malformed_delivered_reply_remains_available_for_diagnosis() -> TestResult {
    let hub = DialHub::default();
    hub.with("machine", |held| held.bridges = 1)?;
    let pending = hub.queue("machine", Act::Status { session: None })?;
    let (ticket, _) = hub.next("machine").await?;
    hub.reply("machine", &ticket, Ok("malformed reply".to_owned()))?;
    let received = pending.answer.recv()??;
    assert!(lys_runner::protocol::read_reply(&received).is_err());
    assert_eq!(
        hub.completion("machine", &ticket)?.as_deref(),
        Some("malformed reply"),
        "the undecodable completion was acknowledged before the caller could read it"
    );
    Ok(())
}
