//! Standing aims keep their words, optional deadlines and reversible reminder activity.

use std::error::Error;
use std::sync::{Arc, Mutex, PoisonError};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::goals_state::{Changed, Held, Line};
use lys_identity_server::goals_store::{
    Deliver, Delivering, GoalStore, Goals, ORIGIN, Undelivered, remind,
};
use lys_identity_server::goals_views::ItemView;
use lys_log_store::{FileLeafStore, LeafStore, Log};
use lys_runner::operations::{Operation, OperationOutcome, OperationRequest, OperationState};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

struct Table {
    service: Service,
    agent: String,
    cookie: String,
}

impl Table {
    async fn start() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) = Service::start_with(|config| {
            Ok(seed_configured(config, [ADMINISTRATOR, "other-subject"])?)
        })
        .await?;
        let cookie = service
            .sign_in(Login {
                subject: ADMINISTRATOR.to_owned(),
                email: "operator@example.test".to_owned(),
            })
            .await?;
        Ok(Self {
            service,
            agent: seeded.people[0].agents[0].id.to_string(),
            cookie,
        })
    }

    async fn post(&self, route: &str, body: &Value) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self.service.post(route, Some(&self.cookie), body).await?;
        assert_eq!(status, 200, "{route}: {answer}");
        Ok(answer)
    }

    async fn set(&self) -> Result<Value, Box<dyn Error>> {
        let mut body = set_body()?;
        body["deadline"] = json!(4_000_000_000_u64);
        self.post(&format!("/agents/{}/goals", self.agent), &body)
            .await
    }

    async fn listed(&self) -> Result<Value, Box<dyn Error>> {
        let (status, answer) = self
            .service
            .get(&format!("/agents/{}/goals", self.agent), Some(&self.cookie))
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(answer)
    }
}

fn set_body() -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": OperationId::generate()?.to_string(),
        "kind": "goal", "words": "keep the service available", "reminders": [],
    }))
}

fn goal_id(item: &Value) -> Result<&str, Box<dyn Error>> {
    item["goal"]["id"]
        .as_str()
        .ok_or_else(|| "goal id absent".into())
}

#[tokio::test]
async fn an_agent_goal_without_a_deadline_round_trips_as_null() -> TestResult {
    let table = Table::start().await?;
    let item = table
        .post(&format!("/agents/{}/goals", table.agent), &set_body()?)
        .await?;
    assert!(
        item["goal"]
            .as_object()
            .is_some_and(|goal| goal.contains_key("deadline"))
    );
    assert_eq!(item["goal"]["deadline"], Value::Null);
    assert_eq!(item["goal"]["active"], true);
    assert_eq!(table.listed().await?["goals"][0], item);
    Ok(())
}

#[tokio::test]
async fn an_explicit_null_deadline_round_trips() -> TestResult {
    let table = Table::start().await?;
    let mut body = set_body()?;
    body["deadline"] = Value::Null;
    let item = table
        .post(&format!("/agents/{}/goals", table.agent), &body)
        .await?;
    assert_eq!(table.listed().await?["goals"][0], item);
    assert_eq!(item["goal"]["deadline"], Value::Null);
    Ok(())
}

#[tokio::test]
async fn a_team_goal_without_a_deadline_round_trips_as_null() -> TestResult {
    let table = Table::start().await?;
    let team = table
        .post(
            "/teams",
            &json!({
                "operation": OperationId::generate()?.to_string(), "name": "Service team",
            }),
        )
        .await?;
    let id = team["id"].as_str().ok_or("team id absent")?;
    let route = format!("/teams/{id}/goals");
    let item = table.post(&route, &set_body()?).await?;
    assert_eq!(item["goal"]["deadline"], Value::Null);
    assert_eq!(item["goal"]["active"], true);
    let (status, listed) = table.service.get(&route, Some(&table.cookie)).await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["goals"][0], item);
    Ok(())
}

#[tokio::test]
async fn a_goal_switches_inactive_then_active_without_a_judgement() -> TestResult {
    let table = Table::start().await?;
    let item = table.set().await?;
    let route = format!("/goals/{}/active", goal_id(&item)?);
    for active in [false, true] {
        let answer = table
            .post(
                &route,
                &json!({
                    "operation": OperationId::generate()?.to_string(), "active": active,
                }),
            )
            .await?;
        assert_eq!(answer["goal"]["active"], active);
        assert_eq!(answer["standing"], "open");
        assert_eq!(answer["marked"], Value::Null);
        assert_eq!(answer["goal"]["deadline"], item["goal"]["deadline"]);
        assert_eq!(table.listed().await?["goals"][0], answer);
    }
    Ok(())
}

#[tokio::test]
async fn field_edits_preserve_every_other_goal_field_and_survive_restart() -> TestResult {
    let mut table = Table::start().await?;
    let item = table.set().await?;
    let other = table.set().await?;
    let mut expected = table.listed().await?;
    assert_eq!(expected["goals"], json!([item, other]));
    let route = format!("/goals/{}", goal_id(&item)?);
    expected["goals"][0]["goal"]["words"] = json!("keep every record available");
    let reworded = table
        .post(
            &format!("{route}/words"),
            &json!({"operation": OperationId::generate()?.to_string(), "words": "keep every record available"}),
        )
        .await?;
    assert_eq!(reworded, expected["goals"][0]);
    assert_eq!(table.listed().await?, expected);
    for active in [false, true] {
        expected["goals"][0]["goal"]["active"] = json!(active);
        let switched = table
            .post(
                &format!("{route}/active"),
                &json!({"operation": OperationId::generate()?.to_string(), "active": active}),
            )
            .await?;
        assert_eq!(switched, expected["goals"][0]);
        assert_eq!(table.listed().await?, expected);
    }
    table.service.restart().await?;
    assert_eq!(table.listed().await?, expected);
    Ok(())
}

#[tokio::test]
async fn rewording_round_trips_and_replaying_the_set_keeps_the_current_words() -> TestResult {
    let table = Table::start().await?;
    let mut set = set_body()?;
    set["deadline"] = json!(4_000_000_000_u64);
    let route = format!("/agents/{}/goals", table.agent);
    let item = table.post(&route, &set).await?;
    let edit = json!({
        "operation": OperationId::generate()?.to_string(), "words": "  keep the directory available  ",
    });
    let edit_route = format!("/goals/{}/words", goal_id(&item)?);
    let answer = table.post(&edit_route, &edit).await?;
    assert_eq!(answer["goal"]["words"], "keep the directory available");
    assert_eq!(answer["goal"]["deadline"], item["goal"]["deadline"]);
    assert_eq!(table.listed().await?["goals"][0], answer);
    assert_eq!(table.post(&edit_route, &edit).await?, answer);
    assert_eq!(table.post(&route, &set).await?, answer);
    Ok(())
}

#[tokio::test]
async fn an_edit_operation_cannot_be_reused_for_a_different_edit() -> TestResult {
    let table = Table::start().await?;
    let item = table.set().await?;
    let id = goal_id(&item)?;
    let operation = OperationId::generate()?.to_string();
    let route = format!("/goals/{id}/active");
    let body = json!({"operation": operation, "active": false});
    let changed = table.post(&route, &body).await?;
    assert_eq!(table.post(&route, &body).await?, changed);
    for (route, body) in [
        (route, json!({"operation": operation, "active": true})),
        (
            format!("/goals/{id}/words"),
            json!({"operation": operation, "words": "other words"}),
        ),
    ] {
        let (status, answer) = table
            .service
            .post(&route, Some(&table.cookie), &body)
            .await?;
        assert_eq!(status, 409, "{answer}");
        assert_eq!(answer["refusal"], "goal_reused");
    }
    assert_eq!(table.listed().await?["goals"][0], changed);
    Ok(())
}

#[tokio::test]
async fn rewording_validates_length_and_reminder_safe_text_before_writing() -> TestResult {
    let table = Table::start().await?;
    let item = table.set().await?;
    let route = format!("/goals/{}/words", goal_id(&item)?);
    for words in [
        String::new(),
        " ".to_owned(),
        "a".repeat(501),
        "two\nlines".to_owned(),
    ] {
        let (status, answer) = table
            .service
            .post(
                &route,
                Some(&table.cookie),
                &json!({
                    "operation": OperationId::generate()?.to_string(), "words": words,
                }),
            )
            .await?;
        assert_eq!(status, 400, "{answer}");
        assert_eq!(answer["refusal"], "RequestMalformed");
    }
    assert_eq!(
        table.listed().await?["goals"][0]["goal"]["words"],
        item["goal"]["words"]
    );
    let valid = table
        .post(
            &route,
            &json!({
                "operation": OperationId::generate()?.to_string(), "words": "a".repeat(500),
            }),
        )
        .await?;
    assert_eq!(
        valid["goal"]["words"].as_str().ok_or("words absent")?.len(),
        500
    );
    Ok(())
}

#[tokio::test]
async fn before_without_a_deadline_is_refused_by_name() -> TestResult {
    let table = Table::start().await?;
    let mut body = set_body()?;
    body["reminders"] = json!([{"when": "before", "seconds": 60}]);
    let (status, answer) = table
        .service
        .post(
            &format!("/agents/{}/goals", table.agent),
            Some(&table.cookie),
            &body,
        )
        .await?;
    assert_eq!(status, 400, "{answer}");
    assert_eq!(answer["refusal"], "reminder_needs_deadline");
    assert!(
        answer["reason"].as_str().is_some_and(
            |reason| reason.contains("a reminder before the deadline needs a deadline")
        )
    );
    assert_eq!(table.listed().await?["goals"], json!([]));
    Ok(())
}

#[tokio::test]
async fn the_tree_uses_current_goal_words_and_omits_inactive_aims() -> TestResult {
    let table = Table::start().await?;
    let team = table
        .post(
            "/teams",
            &json!({
                "operation": OperationId::generate()?.to_string(), "name": "Service team",
            }),
        )
        .await?;
    let team = team["id"].as_str().ok_or("team id absent")?;
    table
        .post(
            &format!("/teams/{team}/members"),
            &json!({
                "operation": OperationId::generate()?.to_string(), "member": table.agent,
            }),
        )
        .await?;
    let item = table.set().await?;
    let goal = goal_id(&item)?;
    let (status, tree) = table.service.get("/tree", Some(&table.cookie)).await?;
    assert_eq!(status, 200, "{tree}");
    assert_eq!(
        tree["teams"][0]["members"][0]["goals"],
        json!(["keep the service available"])
    );
    table.post(&format!("/goals/{goal}/words"), &json!({
        "operation": OperationId::generate()?.to_string(), "words": "keep the directory available",
    })).await?;
    let (status, tree) = table.service.get("/tree", Some(&table.cookie)).await?;
    assert_eq!(status, 200, "{tree}");
    assert_eq!(
        tree["teams"][0]["members"][0]["goals"],
        json!(["keep the directory available"])
    );
    table
        .post(
            &format!("/goals/{goal}/active"),
            &json!({
                "operation": OperationId::generate()?.to_string(), "active": false,
            }),
        )
        .await?;
    let (status, tree) = table.service.get("/tree", Some(&table.cookie)).await?;
    assert_eq!(status, 200, "{tree}");
    assert_eq!(tree["teams"][0]["members"][0]["goals"], json!([]));
    Ok(())
}

fn held_goal(deadline: Option<u64>) -> Result<Held, Box<dyn Error>> {
    let mut goal = json!({
        "line": "set", "id": "goal-1", "holder": {"kind": "agent", "id": "agent-1"},
        "kind": "goal", "words": "keep the service available", "evidence": null,
        "judged_by": null, "reminders": [{"when": "every", "seconds": 60}],
        "responsible": "person-1", "set_by": "person-1", "at": 1000,
    });
    if let Some(deadline) = deadline {
        goal["deadline"] = json!(deadline);
    }
    let mut held = Held::default();
    held.hold(serde_json::from_value(goal)?)?;
    Ok(held)
}

fn activity(operation: &str, active: bool) -> Result<Line, Box<dyn Error>> {
    Ok(serde_json::from_value(json!({
        "line": "changed", "operation": operation, "goal": "goal-1",
        "change": {"field": "active", "active": active}, "by": "person-1", "at": 1030,
    }))?)
}

#[test]
fn an_inactive_goal_is_never_due_and_resumes_its_existing_timer() -> TestResult {
    let mut held = held_goal(Some(4000))?;
    assert_eq!(held.next_due(), Some(1060));
    held.hold(activity("inactive", false)?)?;
    assert!(held.due(3000).is_empty());
    assert_eq!(held.next_due(), None);
    let snapshot = held.encode()?;
    let mut held = Held::decode(&snapshot)?;
    assert_eq!(held.next_due(), None);
    held.hold(activity("active", true)?)?;
    assert_eq!(held.next_due(), Some(1060));
    assert_eq!(held.due(3000)[0].due, 1060);
    Ok(())
}

#[test]
fn every_without_a_deadline_keeps_becoming_due_until_closed() -> TestResult {
    let mut held = held_goal(None)?;
    assert_eq!(held.next_due(), Some(1060));
    held.hold(serde_json::from_value(json!({
        "line": "fired", "operation": "firing-1", "goal": "goal-1", "reminder": 0,
        "due": 1060, "fired": 5000, "late": false, "text": "reminder", "sent": [], "refused": null,
    }))?)?;
    assert_eq!(held.next_due(), Some(5020));
    held.hold(serde_json::from_value(json!({
        "line": "marked", "operation": "mark-1", "goal": "goal-1", "standing": "dropped",
        "by": "person-1", "words": "closed", "evidence": null, "at": 5010,
    }))?)?;
    assert_eq!(held.next_due(), None);
    assert!(held.due(u64::MAX).is_empty());
    Ok(())
}

#[test]
fn inactivity_suspends_unsettled_deliveries_without_losing_them() -> TestResult {
    let mut held = held_goal(Some(4000))?;
    held.hold(serde_json::from_value(json!({
        "line": "fired", "operation": "firing-1", "goal": "goal-1", "reminder": 0,
        "due": 1060, "fired": 1060, "late": false, "text": "reminder", "refused": null,
        "sent": [{"session": "session-1", "operation": "delivery-1", "state": "accepted", "words": "accepted", "at": 1060}],
    }))?)?;
    assert_eq!(held.unsettled().len(), 1);
    held.hold(activity("inactive", false)?)?;
    assert!(held.unsettled().is_empty());
    let mut held = Held::decode(&held.encode()?)?;
    held.hold(activity("active", true)?)?;
    assert_eq!(held.unsettled()[0].0.operation, "delivery-1");
    Ok(())
}

#[tokio::test]
async fn an_old_goal_reads_active_and_keeps_its_original_leaf_bytes_after_edits() -> TestResult {
    let (service, (agent, goal, dir, original, key_file)) = Service::start_with(|config| {
        let seeded = seed_configured(config, [ADMINISTRATOR, "other-subject"])?;
        let agent = seeded.people[0].agents[0].id.to_string();
        let owner = seeded.people[0].id.to_string();
        let goal = OperationId::generate()?.to_string();
        let original = serde_json::to_vec(&json!({
            "line": "set", "id": goal, "holder": {"kind": "agent", "id": agent},
            "kind": "goal", "words": "keep the service available", "deadline": 4_000_000_000_u64,
            "evidence": null, "judged_by": null, "reminders": [],
            "responsible": owner, "set_by": owner, "at": 1000,
        }))?;
        let dir = config.goals_dir.clone().ok_or("goals directory absent")?;
        let mut log = Log::open(FileLeafStore::create(&dir, ORIGIN)?)?;
        log.append(&original)?;
        Ok((agent, goal, dir, original, config.event_key_file.clone()))
    })
    .await?;
    let cookie = service
        .sign_in(Login {
            subject: ADMINISTRATOR.to_owned(),
            email: "operator@example.test".to_owned(),
        })
        .await?;
    let (status, listed) = service
        .get(&format!("/agents/{agent}/goals"), Some(&cookie))
        .await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["goals"][0]["goal"]["active"], true);
    assert_eq!(listed["goals"][0]["goal"]["deadline"], 4_000_000_000_u64);
    for (field, value) in [
        ("active", json!(false)),
        ("words", json!("keep the directory available")),
    ] {
        let mut body = json!({"operation": OperationId::generate()?.to_string()});
        body[field] = value;
        let (status, answer) = service
            .post(&format!("/goals/{goal}/{field}"), Some(&cookie), &body)
            .await?;
        assert_eq!(status, 200, "{answer}");
    }
    assert_eq!(
        FileLeafStore::open(&dir)?
            .leaf(0)?
            .ok_or("old leaf absent")?,
        original
    );
    let reopened = GoalStore::open(&dir, Arc::new(Ed25519Identity::load(&key_file)?))?;
    let item = ItemView::from(
        reopened
            .item(&goal)
            .ok_or("goal absent after reopen")?
            .clone(),
    );
    let answer = serde_json::to_value(item)?;
    assert_eq!(answer["goal"]["active"], false);
    assert_eq!(answer["goal"]["words"], "keep the directory available");

    Ok(())
}

#[test]
fn an_old_snapshot_keeps_its_bytes_when_migrated_in_memory() -> TestResult {
    let held = held_goal(Some(4000))?;
    let old = held.encode()?;
    assert!(!std::str::from_utf8(&old)?.contains("\"active\""));
    assert!(!std::str::from_utf8(&old)?.contains("\"changes\""));
    let decoded = Held::decode(&old)?;
    assert_eq!(decoded.encode()?, old);
    assert_eq!(decoded.next_due(), Some(1060));
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut store = GoalStore::open(&dir.path().join("goals"), Arc::clone(&key))?;
    store.set(decoded.items[0].goal.clone())?;
    drop(store);
    let store = GoalStore::open(&dir.path().join("goals"), key)?;
    assert_eq!(
        store.item("goal-1").ok_or("goal absent")?.goal.deadline,
        decoded.items[0].goal.deadline
    );
    Ok(())
}

struct DeliveredText(Mutex<Vec<String>>);

impl Deliver for DeliveredText {
    fn sessions(
        &self,
        holder: &lys_identity_server::goals_state::Holder,
    ) -> Result<Vec<String>, String> {
        assert_eq!(
            holder.kind,
            lys_identity_server::goals_state::HolderKind::Agent
        );
        assert_eq!(holder.id, "agent-1");
        Ok(vec!["session-1".to_owned()])
    }

    fn operate(&self, operation: Operation) -> Delivering<'_> {
        Box::pin(async move {
            let OperationRequest::Reminder { text } = operation.request else {
                return Err(Undelivered::Refused("expected a reminder".to_owned()));
            };
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(text);
            Ok(OperationOutcome {
                operation: operation.operation,
                session: operation.session,
                request: "reminder".to_owned(),
                state: OperationState::Delivered,
                at: 0,
                words: "delivered".to_owned(),
                text: None,
                ended: None,
            })
        })
    }
}

#[tokio::test]
async fn deadline_free_reminders_use_the_current_words_without_deadline_phrases() -> TestResult {
    let dir = tempfile::tempdir()?;
    let key = Arc::new(Ed25519Identity::load_or_generate(&dir.path().join("key"))?);
    let mut store = GoalStore::open(&dir.path().join("goals"), key)?;
    store.set(held_goal(None)?.items[0].goal.clone())?;
    let changed: Changed = serde_json::from_value(json!({
        "operation":"edit-1", "goal":"goal-1", "change":{"field":"words", "words":"keep the directory available"},
        "by":"person-1", "at":1030,
    }))?;
    store.change(changed)?;
    let goals = Goals::new(store);
    let delivered = DeliveredText(Mutex::default());
    remind(&goals, &delivered, 1060).await?;
    assert_eq!(
        *delivered.0.lock().unwrap_or_else(PoisonError::into_inner),
        ["Reminder from Lys. Goal: keep the directory available."]
    );
    Ok(())
}
