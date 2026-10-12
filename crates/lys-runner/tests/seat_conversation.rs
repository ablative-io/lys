//! Fresh owner plans wait for proved threads and retain the resulting binding across reopen.

use std::error::Error;

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
    Ok(OwnerPlan::new(OwnerBinding {
        seat:"seat".to_owned(), session:"fresh".to_owned(),
        conversation:String::new(), generation:1,
    }, managed, None))
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
