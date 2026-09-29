//! DIRECTORY-051 R6: the refusals read from a runner's feed are kept once
//! each, beside how far that feed was read. The same page read again keeps
//! nothing more, what was kept survives a restart, and an agent's refusals
//! read back newest first.

use std::error::Error;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity_server::budgets_store::BudgetStore;
use lys_runner::refusals::RefusalRecord;

type TestResult = Result<(), Box<dyn Error>>;

fn refusal(attempt: &str, agent: &str, at: u64) -> RefusalRecord {
    RefusalRecord {
        version: lys_runner::refusals::REFUSAL_VERSION,
        source: "runner-one".to_owned(),
        attempt: attempt.to_owned(),
        session: "session-1".to_owned(),
        agent: agent.to_owned(),
        at,
        tool: "Bash".to_owned(),
        target: "rm".to_owned(),
        policy_version: 1,
        rule: Some("no-shell".to_owned()),
        check: "policy_denied".to_owned(),
        grantable: false,
        permission: None,
        grantor: None,
        words: "Rule no-shell denies this call".to_owned(),
    }
}

#[test]
fn a_feed_page_read_twice_keeps_each_refusal_once_across_a_restart() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let logs = dir.path().join("budgets");
    let page = vec![
        refusal("a", "agent-scribe", 1),
        refusal("b", "agent-other", 2),
        refusal("c", "agent-scribe", 3),
    ];
    let mut store = BudgetStore::open(&logs, Arc::clone(&key))?;
    store.read_feed("runner-one", page.clone(), "cursor-3".to_owned())?;
    store.read_feed("runner-one", page, "cursor-3".to_owned())?;
    assert_eq!(store.held().refusals.records.len(), 3);
    drop(store);

    let again = BudgetStore::open(&logs, key)?;
    let held = &again.held().refusals;
    assert_eq!(
        held.records.len(),
        3,
        "kept once, and kept across the restart"
    );
    assert_eq!(
        held.cursors.get("runner-one").map(String::as_str),
        Some("cursor-3")
    );
    let attempts: Vec<String> = held
        .of_agent("agent-scribe")
        .into_iter()
        .map(|record| record.attempt)
        .collect();
    assert_eq!(attempts, ["c", "a"], "the agent's own, newest first");
    Ok(())
}
