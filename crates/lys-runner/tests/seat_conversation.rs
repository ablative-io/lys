//! Fresh owner plans wait for proved threads and retain the resulting binding across reopen.

use lys_runner::peer::{Leader, StartIdentity};
use lys_runner::seat_owner::process::{OwnerState, establish};
use lys_runner::seat_owner::store::OwnerStore;
use std::error::Error;
use std::path::PathBuf;

use lys_runner::harness_control::{ManagedLaunch, Transport};
use lys_runner::seat_owner::protocol::OwnerBinding;
use lys_runner::seat_owner::spawn::OwnerPlan;
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;
const THREAD: &str = "019c6e27-e55b-73d1-87d8-4e01f1f75043";

fn plan() -> Result<OwnerPlan, serde_json::Error> {
    let managed: ManagedLaunch = serde_json::from_value(json!({
        "launch":{"session":"fresh","program":"codex","arguments":[],
            "directory":"/","environment":{},"columns":80,"rows":24},
        "transport":{"mode":"codex"},"conversation":"","requires_controls":true
    }))?;
    Ok(OwnerPlan::new(
        OwnerBinding {
            seat: "seat".to_owned(),
            session: "fresh".to_owned(),
            conversation: String::new(),
            generation: 1,
        },
        managed,
        None,
    ))
}

#[test]
fn a_fresh_codex_owner_can_wait_for_its_proved_thread() -> TestResult {
    let plan = plan()?;
    plan.validate()?;
    let mut claude = plan.clone();
    claude.managed.transport = Transport::Claude;
    assert!(claude.validate().is_err());
    Ok(())
}

#[test]
fn the_proved_owner_conversation_is_durable_and_cannot_be_replaced() -> TestResult {
    let directory = tempfile::tempdir()?;
    let plan = plan()?;
    let own = Leader {
        pid: 42,
        start: StartIdentity("proved-process".to_owned()),
    };
    let mut store = OwnerStore::open(directory.path())?;
    establish(&mut store, &plan, &own)?;
    let owner = OwnerState::new(
        store,
        plan.binding.clone(),
        own,
        "runner".to_owned(),
        PathBuf::from("socket"),
    );
    assert!(owner.view()?.binding.conversation.is_empty());
    owner.bind_conversation("fresh", THREAD)?;
    assert_eq!(owner.view()?.binding.conversation, THREAD);
    owner.bind_conversation("fresh", THREAD)?;
    assert!(owner.bind_conversation("fresh", "different").is_err());
    assert!(owner.bind_conversation("another-session", THREAD).is_err());
    drop(owner);
    let reopened = OwnerStore::open(directory.path())?;
    assert_eq!(
        reopened.owner("fresh").ok_or("owner missing")?.conversation,
        THREAD
    );
    Ok(())
}

#[test]
fn a_claude_resume_keeps_the_session_and_old_managed_plans_decode() -> TestResult {
    let arguments = lys_runner::harness_control::claude::resume_arguments(&[], THREAD)?;
    assert!(
        arguments
            .windows(2)
            .any(|pair| pair == ["--resume", THREAD])
    );
    assert!(!arguments.iter().any(|argument| argument == "--session-id"));
    assert!(!plan()?.managed.resume);
    assert!(
        serde_json::to_value(plan()?.managed)?
            .get("resume")
            .is_none()
    );
    for transport in [Transport::Claude, Transport::Codex] {
        transport.validate_resume(THREAD)?;
        for invalid in ["", " padded ", "../thread", "not-a-uuid"] {
            assert!(transport.validate_resume(invalid).is_err());
        }
    }
    let mut resumed = plan()?;
    resumed.managed.resume = true;
    assert!(resumed.validate().is_err());
    Ok(())
}
