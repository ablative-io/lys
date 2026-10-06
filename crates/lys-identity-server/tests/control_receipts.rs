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
            phase: lys_runner::harness_control::ControlPhase::Idle,
            active: None,
            context: None,
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

#[tokio::test]
async fn control_routes_require_a_person_before_resolving_a_session_or_goal() -> TestResult {
    use identity_contract::harness::Service;
    let service = Service::start().await?;
    let client = reqwest::Client::new();
    for path in [
        "/runtime/sessions/unknown/controls",
        "/runtime/sessions/unknown/control-receipts",
    ] {
        let response = client.get(format!("{}{path}", service.base)).send().await?;
        assert_eq!(response.status(), 401, "{path}");
        let answer: serde_json::Value = response.json().await?;
        assert_eq!(answer["refusal"], "NotSignedIn");
    }
    for (path, body) in [
        (
            "/runtime/sessions/unknown/control-receipts/delivery/reconcile",
            serde_json::json!({"operation":"decision","decision":"not_seen"}),
        ),
        (
            "/goals/unknown/resend",
            serde_json::json!({"operation":"occurrence","prior":"delivery","session":"unknown"}),
        ),
    ] {
        let response = client
            .post(format!("{}{path}", service.base))
            .json(&body)
            .send()
            .await?;
        assert_eq!(response.status(), 401, "{path}");
        let answer: serde_json::Value = response.json().await?;
        assert_eq!(answer["refusal"], "NotSignedIn");
    }
    Ok(())
}

#[test]
fn a_refused_resend_keeps_the_actual_leaf_count_and_next_timer() -> TestResult {
    use lys_core::Ed25519Identity;
    use lys_identity_server::goals_store::{GoalStore, ORIGIN};
    use lys_log_store::{FileLeafStore, FrontierLog};
    use std::sync::Arc;
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("goals");
    FileLeafStore::create(&path, ORIGIN)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let held = uncertain()?;
    log.append(&serde_json::to_vec(&Line::Set(held.items[0].goal.clone()))?)?;
    log.append(&serde_json::to_vec(&Line::Fired(
        held.items[0].fired[0].clone(),
    ))?)?;
    drop(log);
    let mut store = GoalStore::open(&path, key)?;
    let next = store.next_due();
    let (log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let before = log.len();
    drop(log);
    let mut line = serde_json::to_value(resend_line(&held)?)?;
    line["by"] = serde_json::json!("unrelated-person");
    let Line::Resent(resent) = serde_json::from_value(line)? else {
        return Err("resend fixture has another shape".into());
    };
    assert!(store.resend(resent).is_err());
    assert_eq!(store.next_due(), next);
    let (log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    assert_eq!(log.len(), before);
    Ok(())
}

#[test]
fn a_saved_resend_retry_keeps_its_occurrence_delivery_and_regular_timer() -> TestResult {
    use lys_core::Ed25519Identity;
    use lys_identity_server::goals_store::{GoalStore, ORIGIN};
    use lys_log_store::{FileLeafStore, FrontierLog};
    use std::sync::Arc;
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let path = dir.path().join("goals");
    FileLeafStore::create(&path, ORIGIN)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let held = uncertain()?;
    log.append(&serde_json::to_vec(&Line::Set(held.items[0].goal.clone()))?)?;
    log.append(&serde_json::to_vec(&Line::Fired(
        held.items[0].fired[0].clone(),
    ))?)?;
    drop(log);
    let mut store = GoalStore::open(&path, Arc::clone(&key))?;
    let next = store.next_due();
    let (asked, source) = store.prepare_resend(
        "goal",
        "delivery",
        "explicit-occurrence",
        "new-session",
        "person",
        30,
    )?;
    assert_eq!(source, "session");
    assert_eq!(asked.fired.due, 2);
    assert!(
        asked
            .fired
            .text
            .contains("Possible prior delivery of operation delivery")
    );
    assert!(asked.fired.text.contains("original words"));
    assert_ne!(asked.fired.sent[0].operation, "delivery");
    let fired = store.resend(asked)?;
    assert_eq!(store.next_due(), next);
    store.answer(lys_identity_server::goals_state::Answered {
        operation: fired.sent[0].operation.clone(),
        state: Delivery::Delivered,
        words: "matching admission".to_owned(),
        at: 31,
    })?;
    let (log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let before = log.len();
    drop(log);
    let (retry, _) = store.prepare_resend(
        "goal",
        "delivery",
        "explicit-occurrence",
        "new-session",
        "person",
        99,
    )?;
    assert_eq!(retry.fired.fired, 30);
    assert_eq!(retry.fired.sent[0].operation, fired.sent[0].operation);
    assert_eq!(store.resend(retry)?.sent[0].state, Delivery::Delivered);
    assert_eq!(store.next_due(), next);
    assert!(
        store
            .prepare_resend(
                "goal",
                "delivery",
                "explicit-occurrence",
                "another-session",
                "person",
                99
            )
            .is_err()
    );
    let (log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    assert_eq!(log.len(), before);
    drop(log);
    drop(store);
    let store = GoalStore::open(&path, key)?;
    let (reopened, _) = store.prepare_resend(
        "goal",
        "delivery",
        "explicit-occurrence",
        "new-session",
        "person",
        100,
    )?;
    assert_eq!(reopened.fired.sent[0].operation, fired.sent[0].operation);
    assert_eq!(reopened.fired.sent[0].state, Delivery::Delivered);
    assert_eq!(store.next_due(), next);
    Ok(())
}

#[tokio::test]
async fn a_foreign_person_cannot_read_control_evidence_or_resend_saved_goal_words() -> TestResult {
    use identity_contract::fake_issuer::Login;
    use identity_contract::harness::Service;
    use lys_identity::OperationId;
    use lys_identity_server::dev_seed::seed_configured;
    use lys_identity_server::runtime_state::{Report, Reported};
    use lys_identity_server::runtime_store::RuntimeStore;
    use std::sync::Arc;
    let session = OperationId::generate()?.to_string();
    let kept_session = session.clone();
    let (service, seeded) = Service::start_with(move |config| {
        let seeded = seed_configured(
            config,
            [identity_contract::harness::ADMINISTRATOR, "foreign-subject"],
        )?;
        let key = Arc::new(lys_identity::signer::load_service_key(
            &config.event_key_file,
        )?);
        let mut runtime = RuntimeStore::open(
            config.runtime_dir.as_deref().ok_or("runtime dir absent")?,
            key,
        )?;
        runtime.report(Report {
            operation: OperationId::generate()?.to_string(),
            session: kept_session,
            agent: Some(seeded.people[0].agents[0].id.to_string()),
            machine: "private-machine".to_owned(),
            state: Reported::Starting,
            what: String::new(),
            confirmation: String::new(),
            reported_by: seeded.people[0].id.to_string(),
            at: 1,
            launch: None,
        })?;
        Ok(seeded)
    })
    .await?;
    let owner = service
        .sign_in(Login {
            subject: identity_contract::harness::ADMINISTRATOR.to_owned(),
            email: "owner@example.test".to_owned(),
        })
        .await?;
    let foreign = service
        .sign_in(Login {
            subject: "foreign-subject".to_owned(),
            email: "foreign@example.test".to_owned(),
        })
        .await?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let (status, item) = service.post(&format!("/agents/{agent}/goals"), Some(&owner), &serde_json::json!({"operation":OperationId::generate()?.to_string(),"kind":"goal","words":"private saved goal words","reminders":[]})).await?;
    assert_eq!(status, 200, "{item}");
    let goal = item["goal"]["id"].as_str().ok_or("goal id absent")?;
    let unknown = OperationId::generate()?.to_string();
    for suffix in ["controls", "control-receipts"] {
        let actual = service
            .get(
                &format!("/runtime/sessions/{session}/{suffix}"),
                Some(&foreign),
            )
            .await?;
        let absent = service
            .get(
                &format!("/runtime/sessions/{unknown}/{suffix}"),
                Some(&foreign),
            )
            .await?;
        assert_eq!(actual, absent);
        assert_eq!(actual.0, 404, "{actual:?}");
        assert_eq!(actual.1["refusal"], "RuntimeSessionUnknown");
        assert!(!actual.1.to_string().contains("private-machine"));
    }
    let body = serde_json::json!({"operation":OperationId::generate()?.to_string(),"prior":"delivery","session":session});
    let (status, answer) = service
        .post(&format!("/goals/{goal}/resend"), Some(&foreign), &body)
        .await?;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["refusal"], "goal_unknown");
    assert!(!answer.to_string().contains("private saved goal words"));
    for prior in ["delivery", "missing-delivery"] {
        let actual = service
            .get(&format!("/goals/{goal}/resends/{prior}"), Some(&foreign))
            .await?;
        let absent = service
            .get(
                &format!("/goals/missing-goal/resends/{prior}"),
                Some(&foreign),
            )
            .await?;
        assert_eq!(actual, absent);
        assert_eq!(actual.0, 404, "{actual:?}");
        assert_eq!(actual.1["refusal"], "goal_unknown");
        assert!(!actual.1.to_string().contains("delivery"));
    }
    Ok(())
}

#[tokio::test]
async fn control_session_pages_keep_stopped_ids_and_refuse_foreign_visibility_and_cursors()
-> TestResult {
    use identity_contract::fake_issuer::Login;
    use identity_contract::harness::Service;
    use lys_identity::OperationId;
    use lys_identity_server::dev_seed::seed_configured;
    use lys_identity_server::runtime_state::{Report, Reported};
    use lys_identity_server::runtime_store::RuntimeStore;
    use std::sync::Arc;
    let (service, (seeded, expected, foreign_session)) = Service::start_with(|config| {
        let seeded = seed_configured(
            config,
            [identity_contract::harness::ADMINISTRATOR, "page-foreign"],
        )?;
        let key = Arc::new(lys_identity::signer::load_service_key(
            &config.event_key_file,
        )?);
        let path = config.runtime_dir.as_deref().ok_or("runtime dir absent")?;
        let mut runtime = RuntimeStore::open(path, Arc::clone(&key))?;
        let ids: Vec<String> = (0..514)
            .map(|_| OperationId::generate().map(|id| id.to_string()))
            .collect::<Result<_, _>>()?;
        let foreign = ids[0].clone();
        let reports = ids
            .iter()
            .enumerate()
            .map(|(number, session)| {
                let person = &seeded.people[usize::from(number == 0)];
                Report {
                    operation: format!("start-{number}"),
                    session: session.clone(),
                    agent: Some(person.agents[0].id.to_string()),
                    machine: "private-page-machine".to_owned(),
                    state: Reported::Starting,
                    what: "private report words".to_owned(),
                    confirmation: String::new(),
                    reported_by: person.id.to_string(),
                    at: 1,
                    launch: None,
                }
            })
            .collect();
        runtime.report_all(reports)?;
        runtime.report(Report {
            operation: "stop-first".to_owned(),
            session: ids[1].clone(),
            agent: Some(seeded.people[0].agents[0].id.to_string()),
            machine: "private-page-machine".to_owned(),
            state: Reported::Stopped,
            what: String::new(),
            confirmation: "exit".to_owned(),
            reported_by: seeded.people[0].id.to_string(),
            at: 2,
            launch: None,
        })?;
        drop(runtime);
        let reopened = RuntimeStore::open(path, key)?;
        assert!(
            reopened
                .session(&ids[1])
                .ok_or("stopped session absent")?
                .stopped()
        );
        Ok((seeded, ids[1..].to_vec(), foreign))
    })
    .await?;
    let owner = service
        .sign_in(Login {
            subject: identity_contract::harness::ADMINISTRATOR.to_owned(),
            email: "page-owner@example.test".to_owned(),
        })
        .await?;
    let foreign = service
        .sign_in(Login {
            subject: "page-foreign".to_owned(),
            email: "page-foreign@example.test".to_owned(),
        })
        .await?;
    let agent = seeded.people[0].agents[0].id.to_string();
    let path = format!("/agents/{agent}/control-sessions");
    let (status, denied) = service.get(&path, Some(&foreign)).await?;
    assert_eq!(status, 404, "{denied}");
    assert_eq!(denied["refusal"], "AgentNotVisible");
    let mut received = Vec::new();
    let mut after = None;
    for count in [256, 256, 1] {
        let url = after
            .as_ref()
            .map_or_else(|| path.clone(), |after| format!("{path}?after={after}"));
        let (status, page) = service.get(&url, Some(&owner)).await?;
        assert_eq!(status, 200, "{page}");
        assert_eq!(page["agent"], agent);
        let sessions = page["sessions"].as_array().ok_or("session ids absent")?;
        assert_eq!(sessions.len(), count);
        for session in sessions {
            received.push(
                session
                    .as_str()
                    .ok_or("session id is not a string")?
                    .to_owned(),
            );
        }
        after = page["after"].as_str().map(str::to_owned);
        assert_eq!(after.is_some(), count == 256);
        assert!(!page.to_string().contains("private-page-machine"));
        assert!(!page.to_string().contains("private report words"));
    }
    assert_eq!(received, expected);
    for (cursor, reason) in [
        (foreign_session, "control_sessions_cursor_foreign"),
        (
            OperationId::generate()?.to_string(),
            "control_sessions_cursor_unknown",
        ),
        (String::new(), "control_sessions_cursor_unknown"),
    ] {
        let (status, answer) = service
            .get(&format!("{path}?after={cursor}"), Some(&owner))
            .await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], "RequestMalformed");
        assert!(
            answer["reason"]
                .as_str()
                .ok_or("cursor reason absent")?
                .contains(reason),
            "{answer}"
        );
    }
    Ok(())
}

#[test]
fn existing_runtime_session_answers_keep_their_wire_bytes() -> TestResult {
    use lys_identity_server::runtime_api::{SessionView, SessionsView, StopView};
    let answer = SessionsView {
        sessions: vec![SessionView {
            session: "session".to_owned(),
            agent: Some("agent".to_owned()),
            machine: "machine".to_owned(),
            machine_name: None,
            runtime: None,
            shown: "stopped",
            last_reported: "stopped",
            first_report_at: 1,
            last_report_at: 2,
            what: "reported words".to_owned(),
            stopped: Some(StopView {
                at: 2,
                confirmation: "exit".to_owned(),
            }),
            stop_asked_at: Some(1),
            reported_by: "person".to_owned(),
        }],
    };
    assert_eq!(serde_json::to_vec(&answer)?, br#"{"sessions":[{"session":"session","agent":"agent","machine":"machine","machine_name":null,"runtime":null,"shown":"stopped","last_reported":"stopped","first_report_at":1,"last_report_at":2,"what":"reported words","stopped":{"at":2,"confirmation":"exit"},"stop_asked_at":1,"reported_by":"person"}]}"#);
    Ok(())
}

struct FailedResendDecision {
    dir: tempfile::TempDir,
    key: std::sync::Arc<lys_core::Ed25519Identity>,
    goals: lys_identity_server::goals_store::GoalStore,
    runner: std::sync::Arc<lys_runner::Sessions>,
}

fn decision_answer(
    runner: &std::sync::Arc<lys_runner::Sessions>,
    key: &lys_core::Ed25519Identity,
) -> Result<lys_runner::Answer, Box<dyn Error>> {
    let greeting = lys_runner::protocol::Greeting::fresh("21");
    let act = lys_runner::Act::ReconcileControl {
        operation: "delivery".to_owned(),
        decision: lys_runner::operations::Reconciled {
            operation: "kept-resend".to_owned(),
            by: "person".to_owned(),
            at: u64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis(),
            )?,
            decision: lys_runner::operations::Reconciliation::Resent {
                occurrence: "kept-resend".to_owned(),
            },
        },
    };
    let line = lys_runner::protocol::sign_request(key, &greeting, &act)?;
    Ok(lys_runner::socket::dispatch(
        runner,
        &key.public_key_bytes(),
        &greeting,
        &line,
        &std::sync::atomic::AtomicBool::new(false),
    ))
}

fn failed_resend_decision() -> Result<FailedResendDecision, Box<dyn Error>> {
    use lys_identity_server::goals_store::{GoalStore, ORIGIN};
    use lys_log_store::{FileLeafStore, FrontierLog};
    let dir = tempfile::tempdir()?;
    let key = std::sync::Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &dir.path().join("key"),
    )?);
    let path = dir.path().join("goals");
    FileLeafStore::create(&path, ORIGIN)?;
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let held = uncertain()?;
    for line in [
        Line::Set(held.items[0].goal.clone()),
        Line::Fired(held.items[0].fired[0].clone()),
    ] {
        log.append(&serde_json::to_vec(&line)?)?;
    }
    drop(log);
    let mut goals = GoalStore::open(&path, std::sync::Arc::clone(&key))?;
    let (resent, _) = goals.prepare_resend(
        "goal",
        "delivery",
        "kept-resend",
        "new-session",
        "person",
        30,
    )?;
    goals.resend(resent)?;
    let runner_path = dir.path().join("runner");
    std::fs::create_dir(&runner_path)?;
    let legacy = serde_json::json!({"operation":"delivery","session":"session","request":"reminder","state":"uncertain","at":0,"words":"possible prior delivery","text":null,"ended":null});
    let mut bytes = serde_json::to_vec(&legacy)?;
    bytes.push(b'\n');
    std::fs::write(runner_path.join("operations.jsonl"), bytes)?;
    let runner = lys_runner::Sessions::open(&runner_path, 4096)?;
    let journal = runner_path.join("operations.v2.journal");
    let saved = runner_path.join("saved-journal");
    std::fs::rename(&journal, &saved)?;
    std::fs::create_dir(&journal)?;
    let answer = decision_answer(&runner, &key)?;
    assert!(
        matches!(answer, lys_runner::Answer::Refused { .. }),
        "{answer:?}"
    );
    drop(runner);
    std::fs::remove_dir(&journal)?;
    std::fs::rename(saved, journal)?;
    let runner = lys_runner::Sessions::open(&runner_path, 4096)?;
    assert!(runner.control_receipt("delivery")?.reconciled.is_none());
    Ok(FailedResendDecision {
        dir,
        key,
        goals,
        runner,
    })
}

fn refuse_second_resend(goals: &lys_identity_server::goals_store::GoalStore) -> TestResult {
    let error = goals
        .prepare_resend(
            "goal",
            "delivery",
            "second-resend",
            "new-session",
            "person",
            31,
        )
        .err()
        .ok_or("a second occurrence was admitted for the same uncertain prior")?;
    assert!(
        matches!(&error, lys_identity_server::error::ServerError::Runner {refusal,..} if refusal == "goal_prior_already_resent"),
        "{error}"
    );
    assert!(error.to_string().contains("kept-resend"), "{error}");
    let item = goals.item("goal").ok_or("goal absent")?;
    assert_eq!(
        item.fired
            .iter()
            .flat_map(|fired| &fired.sent)
            .filter(|sent| sent.state == Delivery::Pending)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn a_new_resend_id_after_a_failed_decision_write_is_refused_with_one_pending_delivery() -> TestResult
{
    let fixture = failed_resend_decision()?;
    let kept = fixture
        .goals
        .resend_lookup("goal", "delivery")?
        .ok_or("kept occurrence absent")?;
    assert_eq!(
        (&*kept.operation, &*kept.sent[0].session),
        ("kept-resend", "new-session")
    );
    refuse_second_resend(&fixture.goals)
}

#[test]
fn a_new_resend_id_after_a_failed_decision_write_is_refused_after_reopen() -> TestResult {
    let fixture = failed_resend_decision()?;
    let path = fixture.dir.path().join("goals");
    let key = std::sync::Arc::clone(&fixture.key);
    drop(fixture.goals);
    let reopened = lys_identity_server::goals_store::GoalStore::open(&path, key)?;
    let kept = reopened
        .resend_lookup("goal", "delivery")?
        .ok_or("kept occurrence absent after reopen")?;
    assert_eq!(
        (&*kept.operation, &*kept.sent[0].session),
        ("kept-resend", "new-session")
    );
    refuse_second_resend(&reopened)
}

#[test]
fn the_same_resend_id_repairs_the_missing_decision_without_another_delivery() -> TestResult {
    use lys_log_store::{FileLeafStore, FrontierLog};
    let mut fixture = failed_resend_decision()?;
    let path = fixture.dir.path().join("goals");
    let (log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let before = log.len();
    drop(log);
    let (retry, _) = fixture.goals.prepare_resend(
        "goal",
        "delivery",
        "kept-resend",
        "new-session",
        "person",
        99,
    )?;
    let fired = fixture.goals.resend(retry)?;
    assert_eq!(fired.operation, "kept-resend");
    assert_eq!(fired.sent[0].state, Delivery::Pending);
    let answer = decision_answer(&fixture.runner, &fixture.key)?;
    assert!(
        matches!(answer, lys_runner::Answer::ControlReceipt { .. }),
        "{answer:?}"
    );
    let receipt = fixture.runner.control_receipt("delivery")?;
    let reconciled = receipt.reconciled.ok_or("person decision absent")?;
    assert_eq!(reconciled.operation, "kept-resend");
    assert_eq!(
        reconciled.decision,
        lys_runner::operations::Reconciliation::Resent {
            occurrence: "kept-resend".to_owned()
        }
    );
    assert_eq!(
        receipt.state,
        lys_runner::operations::OperationState::Uncertain
    );
    assert!(!receipt.admitted);
    let (log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    assert_eq!(log.len(), before);
    assert_eq!(
        fixture
            .goals
            .item("goal")
            .ok_or("goal absent")?
            .fired
            .iter()
            .flat_map(|fired| &fired.sent)
            .filter(|sent| sent.state == Delivery::Pending)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn control_decision_audit_measures_ordinary_and_checkpoint_sync_counts() -> TestResult {
    use lys_identity_server::runner_acts::{ActStore, RunnerAct};
    let dir = tempfile::tempdir()?;
    let key = std::sync::Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &dir.path().join("key"),
    )?);
    let mut audit = ActStore::open(&dir.path().join("audit"), key)?;
    let mut costs = std::collections::BTreeMap::new();
    for number in 0..lys_identity::SNAPSHOT_EVERY.get() {
        let before = lys_log_store::flush_count();
        audit.keep(RunnerAct {
            act: "reconcile_control".to_owned(),
            caller: "person".to_owned(),
            session: "session".to_owned(),
            agent: "agent".to_owned(),
            machine: "machine".to_owned(),
            at: number,
            text: None,
            keys: Vec::new(),
            outcome: "control_receipt".to_owned(),
        })?;
        let count = lys_log_store::flush_count() - before;
        *costs.entry(count).or_insert(0_usize) += 1;
    }
    println!(
        "control decision audit real sync counts: {costs:?}; interval {}",
        lys_identity::SNAPSHOT_EVERY
    );
    assert_eq!(
        costs.get(&1),
        Some(&(usize::try_from(lys_identity::SNAPSHOT_EVERY.get())? - 1))
    );
    assert_eq!(costs.get(&3), Some(&1));
    assert_eq!(costs.len(), 2);
    Ok(())
}

#[test]
fn a_seeded_control_audit_keeps_all_leaves_and_snapshots_at_the_existing_count() -> TestResult {
    use lys_identity_server::runner_acts::{ActStore, RunnerAct};
    use lys_log_store::{FileLeafStore, FrontierLog, Start};
    let dir = tempfile::tempdir()?;
    let key = std::sync::Arc::new(lys_core::Ed25519Identity::load_or_generate(
        &dir.path().join("key"),
    )?);
    let path = dir.path().join("audit");
    let act = |at| RunnerAct {
        act: "reconcile_control".to_owned(),
        caller: "person".to_owned(),
        session: "session".to_owned(),
        agent: "agent".to_owned(),
        machine: "machine".to_owned(),
        at,
        text: None,
        keys: Vec::new(),
        outcome: "control_receipt".to_owned(),
    };
    let mut audit = ActStore::open(&path, std::sync::Arc::clone(&key))?;
    let before = lys_log_store::flush_count();
    audit.keep(act(0))?;
    assert_eq!(lys_log_store::flush_count() - before, 1);
    drop(audit);
    let (mut log, _) = FrontierLog::open(FileLeafStore::open(&path)?)?;
    let count = lys_identity::SNAPSHOT_EVERY.get();
    let leaves: Vec<_> = (1..count - 1)
        .map(|number| serde_json::to_vec(&act(number)))
        .collect::<Result<_, _>>()?;
    let borrowed: Vec<_> = leaves.iter().map(Vec::as_slice).collect();
    let before = lys_log_store::flush_count();
    log.append_batch(&borrowed)?;
    assert_eq!(lys_log_store::flush_count() - before, 1);
    assert_eq!(log.len(), count - 1);
    drop(log);
    let mut audit = ActStore::open(&path, std::sync::Arc::clone(&key))?;
    assert!(
        matches!(audit.start(),Start::Resumed {size:0,replayed} if *replayed == count-1),
        "{:?}",
        audit.start()
    );
    let before = lys_log_store::flush_count();
    audit.keep(act(count - 1))?;
    assert_eq!(lys_log_store::flush_count() - before, 3);
    assert_eq!(audit.len(), count);
    drop(audit);
    let audit = ActStore::open(&path, key)?;
    assert_eq!(audit.len(), count);
    assert!(
        matches!(audit.start(),Start::Resumed {size,replayed:0} if *size==count),
        "{:?}",
        audit.start()
    );
    for number in 0..count {
        assert_eq!(
            audit.receipt(number)?.ok_or("audit leaf absent")?.act,
            act(number)
        );
    }
    Ok(())
}

#[tokio::test]
async fn resend_lookup_names_the_kept_occurrence_and_refuses_unknown_or_certain_priors()
-> TestResult {
    use identity_contract::{fake_issuer::Login, harness::Service};
    use lys_identity_server::{dev_seed::seed_configured, goals_store::GoalStore};
    use lys_log_store::{FileLeafStore, FrontierLog};
    use std::sync::Arc;
    let service = Service::start_with(|config| {
        let seeded = seed_configured(
            config,
            [identity_contract::harness::ADMINISTRATOR, "second-subject"],
        )?;
        let key = Arc::new(lys_identity::signer::load_service_key(
            &config.event_key_file,
        )?);
        let path = config.goals_dir.as_deref().ok_or("goals dir absent")?;
        drop(GoalStore::open(path, Arc::clone(&key))?);
        let mut original = uncertain()?.items.remove(0);
        original.goal.holder.id = seeded.people[0].agents[0].id.to_string();
        original.goal.responsible = seeded.people[0].id.to_string();
        original.goal.set_by = original.goal.responsible.clone();
        original.goal.reminders = vec![Remind::On {
            event: lys_identity_server::goals_state::Event::Compaction,
        }];
        let mut certain = original.fired[0].clone();
        certain.operation = "certain-occurrence".to_owned();
        certain.sent[0].operation = "certain-delivery".to_owned();
        certain.sent[0].state = Delivery::Delivered;
        let mut no_resend = original.fired[0].clone();
        no_resend.operation = "unresent-occurrence".to_owned();
        no_resend.sent[0].operation = "unresent-delivery".to_owned();
        let leaves = [
            Line::Set(original.goal),
            Line::Fired(original.fired.remove(0)),
            Line::Fired(certain),
            Line::Fired(no_resend),
        ];
        let (mut log, _) = FrontierLog::open(FileLeafStore::open(path)?)?;
        let bytes = leaves
            .iter()
            .map(serde_json::to_vec)
            .collect::<Result<Vec<_>, _>>()?;
        log.append_batch(&bytes.iter().map(Vec::as_slice).collect::<Vec<_>>())?;
        drop(log);
        let store = GoalStore::open(path, Arc::clone(&key))?;
        assert!(store.resend_lookup("goal", "unresent-delivery")?.is_none());
        drop(store);
        let mut store = GoalStore::open(path, Arc::clone(&key))?;
        assert!(store.resend_lookup("goal", "unresent-delivery")?.is_none());
        let (mut resend, _) = store.prepare_resend(
            "goal",
            "delivery",
            "kept-resend",
            "new-session",
            &seeded.people[0].id.to_string(),
            30,
        )?;
        resend.fired.sent[0].state = Delivery::Pending;
        store.resend(resend)?;
        drop(store);
        drop(GoalStore::open(path, key)?);
        Ok(())
    })
    .await?
    .0;
    let person = service
        .sign_in(Login {
            subject: identity_contract::harness::ADMINISTRATOR.to_owned(),
            email: "owner@example.test".to_owned(),
        })
        .await?;
    for (prior, occurrence) in [
        (
            "delivery",
            serde_json::json!({"operation":"kept-resend", "session":"new-session"}),
        ),
        ("unresent-delivery", serde_json::Value::Null),
    ] {
        let (status, answer) = service
            .get(&format!("/goals/goal/resends/{prior}"), Some(&person))
            .await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(
            answer,
            serde_json::json!({"goal":"goal", "prior":prior, "occurrence":occurrence})
        );
    }
    for (prior, refusal) in [
        ("missing-delivery", "goal_prior_unknown"),
        ("certain-delivery", "control_not_uncertain"),
    ] {
        let (status, answer) = service
            .get(&format!("/goals/goal/resends/{prior}"), Some(&person))
            .await?;
        assert_eq!(status, 409, "{answer}");
        assert_eq!(answer["refusal"], refusal);
        assert!(answer.get("occurrence").is_none());
    }
    Ok(())
}
