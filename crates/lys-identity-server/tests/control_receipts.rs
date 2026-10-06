#![cfg(test)]
//! Queued deliveries use current open aims and keep their occurrence identity.

use lys_identity_server::goals_state::{
    Change, Changed, Delivery, Fired, Goal, Held, Holder, HolderKind, Kind, Line, Marked, Remind,
    Sent, Standing,
};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

fn queued() -> Result<Held, Box<dyn Error>> {
    let mut held = Held::default();
    held.hold(Line::Set(Goal {
        id: "goal".to_owned(),
        holder: Holder {
            kind: HolderKind::Agent,
            id: "agent".to_owned(),
        },
        kind: Kind::Goal,
        words: "original words".to_owned(),
        deadline: None,
        active: true,
        evidence: None,
        judged_by: None,
        reminders: vec![Remind::Every { seconds: 1 }],
        responsible: "person".to_owned(),
        set_by: "person".to_owned(),
        at: 1,
    }))?;
    held.hold(Line::Fired(Fired {
        operation: "occurrence".to_owned(),
        goal: "goal".to_owned(),
        reminder: 0,
        due: 2,
        fired: 2,
        late: false,
        text: "Reminder from Lys. goal: original words.".to_owned(),
        sent: vec![Sent {
            session: "session".to_owned(),
            operation: "delivery".to_owned(),
            state: Delivery::Accepted,
            words: String::new(),
            at: 2,
        }],
        refused: None,
    }))?;
    Ok(held)
}

#[test]
fn a_goal_marked_met_before_dispatch_sends_no_reminder() -> TestResult {
    let mut held = queued()?;
    held.hold(Line::Marked(Marked {
        operation: "mark".to_owned(),
        goal: "goal".to_owned(),
        standing: Standing::Met,
        by: "person".to_owned(),
        words: "met".to_owned(),
        evidence: None,
        at: 3,
    }))?;
    assert!(held.unsettled()?.is_empty());
    Ok(())
}

#[test]
fn a_queued_old_version_sends_the_current_authorised_words_after_an_edit() -> TestResult {
    let mut held = queued()?;
    held.hold(Line::Changed(Changed {
        operation: "revision".to_owned(),
        goal: "goal".to_owned(),
        change: Change::Words {
            words: "current words".to_owned(),
        },
        by: "person".to_owned(),
        at: 3,
    }))?;
    let pending = held.unsettled()?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].0.operation, "delivery");
    assert!(pending[0].1.contains("current words"));
    assert!(!pending[0].1.contains("original words"));
    Ok(())
}

fn context_policy() -> Result<
    (
        lys_identity_server::budgets_state::Held,
        lys_identity_server::budgets_state::Standing,
        lys_runner::harness_control::ControlStatus,
    ),
    Box<dyn Error>,
> {
    use lys_identity_server::budgets_limits::{Limit, Limits};
    use lys_identity_server::budgets_state::{Act, Holder, HolderKind, Leaf, Measure};
    let mut held = lys_identity_server::budgets_state::Held::default();
    held.hold(Leaf::LimitsSet(Limits {
        holder: Holder {
            kind: HolderKind::Agent,
            id: "agent".to_owned(),
        },
        limits: vec![Limit {
            unit: Measure::ContextPercent,
            amount: 80.into(),
            period: None,
            act: Act::Compact,
            zone: None,
        }],
        warn_at: None,
        version: 1,
        by: "person".to_owned(),
        at: 1,
    }))?;
    held.hold(Leaf::Acted(lys_identity_server::budgets_crossing::Acted {
        operation: "crossing".to_owned(),
        stands: lys_identity_server::budgets_crossing::Stands::Confirmed,
        words: "harness_compacted".to_owned(),
        at_ms: 2000,
        ended: None,
    }))?;
    Ok((
        held,
        lys_identity_server::budgets_state::Standing {
            agent: "agent".to_owned(),
            person: Some("person".to_owned()),
            teams: std::collections::BTreeSet::new(),
        },
        lys_runner::harness_control::ControlStatus {
            generation: 1,
            boundary: Some("boundary".to_owned()),
            crossing: Some("crossing".to_owned()),
            queued: Vec::new(),
        },
    ))
}

fn measurement(
    held: &mut lys_identity_server::budgets_state::Held,
    at_ms: i64,
    figure: Option<u64>,
) -> TestResult {
    use lys_identity_server::budgets_state::{Leaf, Usage};
    held.hold(Leaf::Used(Usage {
        event: format!("measurement-{at_ms}"),
        agent: "agent".to_owned(),
        at_ms,
        session: Some("session".to_owned()),
        context_percent: figure,
        ..Usage::default()
    }))?;
    Ok(())
}

#[test]
fn a_fresh_below_threshold_report_after_confirmed_compaction_releases_input() -> TestResult {
    let (mut held, standing, status) = context_policy()?;
    measurement(&mut held, 2001, Some(79))?;
    assert_eq!(
        held.control_context(&standing, &status, "session", 2002)?,
        lys_runner::harness_control::ContextDecision::Released
    );
    Ok(())
}

#[test]
fn a_post_compaction_above_threshold_report_keeps_input_held_without_another_compaction()
-> TestResult {
    let (mut held, standing, status) = context_policy()?;
    measurement(&mut held, 2001, Some(80))?;
    assert!(matches!(
        held.control_context(&standing, &status, "session", 2002)?,
        lys_runner::harness_control::ContextDecision::Held { .. }
    ));
    Ok(())
}

#[test]
fn a_missing_post_compaction_report_keeps_input_held() -> TestResult {
    let (held, standing, status) = context_policy()?;
    assert!(matches!(
        held.control_context(&standing, &status, "session", 2002)?,
        lys_runner::harness_control::ContextDecision::Held { .. }
    ));
    Ok(())
}

#[test]
fn a_stale_below_threshold_report_cannot_release_the_compaction_hold() -> TestResult {
    let (mut held, standing, status) = context_policy()?;
    measurement(&mut held, 1999, Some(30))?;
    assert!(matches!(
        held.control_context(&standing, &status, "session", 2002)?,
        lys_runner::harness_control::ContextDecision::Held { .. }
    ));
    Ok(())
}

#[test]
fn a_new_missing_report_cannot_reuse_an_earlier_valid_percentage() -> TestResult {
    let (mut held, standing, status) = context_policy()?;
    measurement(&mut held, 2001, Some(30))?;
    measurement(&mut held, 2002, None)?;
    assert!(matches!(
        held.control_context(&standing, &status, "session", 2003)?,
        lys_runner::harness_control::ContextDecision::Held { .. }
    ));
    Ok(())
}

fn resend_line(held: &Held) -> Result<Line, Box<dyn Error>> {
    let mut fired = held.items[0].fired[0].clone();
    fired.operation = "resent-occurrence".to_owned();
    fired.sent[0].operation = "resent-delivery".to_owned();
    fired.sent[0].state = Delivery::Pending;
    fired.fired = 30;
    Ok(serde_json::from_value(
        serde_json::json!({ "line":"resent", "fired":fired,
        "prior":"delivery", "by":"person" }),
    )?)
}

fn uncertain() -> Result<Held, Box<dyn Error>> {
    let mut held = queued()?;
    held.hold(Line::Answered(lys_identity_server::goals_state::Answered {
        operation: "delivery".to_owned(),
        state: Delivery::Uncertain,
        words: "possible prior delivery".to_owned(),
        at: 3,
    }))?;
    Ok(held)
}

#[test]
fn an_explicit_resend_keeps_the_regular_timer_due_instant() -> TestResult {
    let mut held = uncertain()?;
    let next = held.next_due();
    held.hold(resend_line(&held)?)?;
    assert_eq!(held.next_due(), next);
    assert_eq!(held.items[0].fired[1].operation, "resent-occurrence");
    assert_eq!(held.items[0].fired[1].due, 2);
    assert!(held.events.is_empty());
    Ok(())
}

#[test]
fn a_resent_keeps_its_prior_delivery_label_across_snapshot_recovery() -> TestResult {
    let mut held = uncertain()?;
    held.hold(resend_line(&held)?)?;
    let recovered = Held::decode(&held.encode()?)?;
    let sealed = serde_json::to_value(&recovered)?;
    assert_eq!(sealed["resends"]["resent-occurrence"], "delivery");
    assert_eq!(
        recovered.pending_for_session("session")?[0].sent.operation,
        "resent-delivery"
    );
    Ok(())
}

#[test]
fn a_refused_resend_changes_neither_the_goal_nor_its_timer() -> TestResult {
    let mut held = uncertain()?;
    let mut wire = serde_json::to_value(resend_line(&held)?)?;
    wire["by"] = serde_json::json!("unrelated-person");
    let line = serde_json::from_value(wire)?;
    let before = held.encode()?;
    assert!(held.hold(line).is_err());
    assert_eq!(held.encode()?, before);
    Ok(())
}

#[test]
fn an_old_version_goals_snapshot_is_rebuilt_without_losing_goals_or_timers() -> TestResult {
    use lys_core::Ed25519Identity;
    use lys_identity_server::goals_store::{GoalStore, ORIGIN};
    use lys_log_store::{FileLeafStore, FrontierLog, Start};
    use std::sync::Arc;
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("goals");
    FileLeafStore::create(&path, ORIGIN)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let held = queued()?;
    log.append(&serde_json::to_vec(&Line::Set(held.items[0].goal.clone()))?)?;
    log.append(&serde_json::to_vec(&Line::Fired(
        held.items[0].fired[0].clone(),
    ))?)?;
    let mut old = serde_json::to_value(&held)?;
    old.as_object_mut()
        .ok_or("snapshot not an object")?
        .remove("resends");
    let old = serde_json::to_vec(&serde_json::json!({"format":"lys-goals-state/v1","held":old}))?;
    log.write_snapshot(lys_identity_server::goals_state::DOMAIN, &old, &key)?;
    drop(log);
    let store = GoalStore::open(&path, key)?;
    assert!(matches!(store.start(), Start::Rebuilt { .. }));
    assert_eq!(
        store.item("goal").ok_or("goal lost")?.timers,
        held.items[0].timers
    );
    Ok(())
}
