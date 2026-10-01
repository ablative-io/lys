#![cfg(test)]

use super::{DialHub, Leave, Queued, withdraw_on_leave};
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn a_poisoned_leave_hook_closes_the_new_connection_and_refuses_it() -> Result<(), Box<dyn Error>> {
    let leave = Leave::default();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let held = leave
            .hook
            .lock()
            .expect("fixture lock poisoned before injection");
        assert!(held.is_none());
        panic!("leave hook state failure");
    }));
    assert!(poisoned.is_err());
    let closed = Arc::new(AtomicBool::new(false));
    let callback = Arc::clone(&closed);
    let refused = leave
        .hold(Box::new(move || callback.store(true, Ordering::SeqCst)))
        .err()
        .ok_or("poisoned leave hook admitted connection")?;
    assert_eq!(refused.name(), "runner_unreachable");
    assert!(refused.to_string().contains("leave hook"));
    assert!(closed.load(Ordering::SeqCst));
    assert!(leave.done().is_err());
    assert!(leave.leave().is_err());
    Ok(())
}

#[tokio::test]
async fn a_poisoned_dial_hub_cannot_recover_nonce_or_deliver_work() -> Result<(), Box<dyn Error>> {
    let hub = DialHub::default();
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let held = hub
            .machines
            .lock()
            .expect("fixture lock poisoned before injection");
        assert!(held.is_empty());
        panic!("dial state failure");
    }));
    assert!(poisoned.is_err());
    let nonce = hub
        .fresh("machine", "00112233445566778899aabbccddeeff")
        .err()
        .ok_or("poisoned dial hub admitted nonce")?;
    assert_eq!(nonce.name(), "runner_unreachable");
    assert!(nonce.to_string().contains("dial hub"));
    let next = hub
        .next("machine")
        .await
        .err()
        .ok_or("poisoned dial hub delivered work")?;
    assert_eq!(next.name(), "runner_unreachable");
    let reply = hub
        .reply("machine", "ticket", Ok(String::new()))
        .err()
        .ok_or("poisoned dial hub delivered reply")?;
    assert_eq!(reply.name(), "runner_unreachable");
    assert!(hub.withdraw("machine", "ticket").is_err());
    Ok(())
}

#[test]
fn a_poisoned_dial_hub_still_answers_the_caller_who_leaves() -> Result<(), Box<dyn Error>> {
    let hub = Arc::new(DialHub::default());
    let Queued {
        ticket,
        answer,
        cancellation,
    } = hub.queue("machine", lys_runner::Act::Status { session: None })?;
    let leave = Leave::default();
    leave.hold(withdraw_on_leave(
        Arc::clone(&hub),
        "machine".to_owned(),
        ticket,
        cancellation,
    ))?;
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let held = hub
            .machines
            .lock()
            .expect("fixture lock poisoned before injection");
        assert_eq!(held.len(), 1);
        panic!("dial state failure");
    }));
    assert!(poisoned.is_err());
    leave.leave()?;
    let refused = answer
        .recv()?
        .err()
        .ok_or("poisoned hub answered caller with success")?;
    assert_eq!(refused.name(), "runner_unreachable");
    assert!(refused.to_string().contains("dial hub"));
    Ok(())
}
