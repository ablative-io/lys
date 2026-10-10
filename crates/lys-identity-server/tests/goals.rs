#![cfg(test)]

//! Goals, expectations and deliverables: a due reminder is delivered once
//! under the runner's operation id; a restart keeps each timer's due
//! instant, and one that fell due while the service was stopped fires once,
//! late, keeping the instant it fell due and the instant it fired; a crash
//! after the runner accepted a reminder does not type it again, and an
//! uncertain one is never shown delivered; marking an item met cancels its
//! reminders; a deliverable without evidence is refused `evidence_missing`;
//! the agent an item judges is refused `not_your_judgement`; the log starts
//! from its signed snapshot; and the routes keep the holder's visibility.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_core::attestation::sign_attestation;
use lys_core::ca::create_certificate_request;
use lys_identity::OperationId;
use lys_identity_server::agent_signature::{HEADER, payload};
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::error::ServerError;
use lys_identity_server::error_agents::AgentsError;
use lys_identity_server::goals_state::{
    Delivery, Event, Evented, EvidenceKind, Goal, GoalError, Holder, HolderKind, Kind, Marked,
    Remind, Standing,
};
use lys_identity_server::goals_store::{
    Deliver, Delivering, GoalStore, Goals, Undelivered, remind,
};
use lys_log_store::Start;
use lys_runner::operations::{Operation, OperationOutcome, OperationRequest, OperationState};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const AGENT: &str = "agent-00000000000000000000000000000002";
const PERSON: &str = "person-00000000000000000000000000000001";

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("goals.key"),
    )?))
}

fn now() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn goal(id: &str, kind: Kind, at: u64, deadline: u64, reminders: Vec<Remind>) -> Goal {
    Goal {
        id: id.to_owned(),
        holder: Holder {
            kind: HolderKind::Agent,
            id: AGENT.to_owned(),
        },
        kind,
        words: "the sign-in page is live".to_owned(),
        deadline: Some(deadline),
        active: true,
        evidence: (kind == Kind::Deliverable).then_some(EvidenceKind::Commit),
        judged_by: None,
        reminders,
        responsible: PERSON.to_owned(),
        set_by: PERSON.to_owned(),
        at,
    }
}

fn marked(operation: &str, goal: &str, standing: Standing, evidence: Option<&str>) -> Marked {
    Marked {
        operation: operation.to_owned(),
        goal: goal.to_owned(),
        standing,
        by: PERSON.to_owned(),
        words: "seen live".to_owned(),
        evidence: evidence.map(str::to_owned),
        at: 1,
    }
}

/// A runner as the operations record answers: an operation asked again
/// under its id is answered as it stands and never typed twice.
struct Runner {
    sessions: Vec<String>,
    held: Mutex<BTreeMap<String, (String, OperationOutcome)>>,
    typed: AtomicUsize,
    asked: AtomicUsize,
    texts: Mutex<Vec<String>>,
    lose_next: AtomicBool,
    answer: OperationState,
}

impl Runner {
    fn new(answer: OperationState) -> Self {
        Self {
            sessions: vec!["session-1".to_owned()],
            held: Mutex::default(),
            typed: AtomicUsize::new(0),
            asked: AtomicUsize::new(0),
            texts: Mutex::default(),
            lose_next: AtomicBool::new(false),
            answer,
        }
    }

    fn typed(&self) -> usize {
        self.typed.load(Ordering::SeqCst)
    }
}

impl Deliver for Runner {
    fn sessions(&self, _holder: &Holder) -> Result<Vec<String>, String> {
        Ok(self.sessions.clone())
    }

    fn operate(&self, operation: Operation) -> Delivering<'_> {
        Box::pin(async move {
            self.asked.fetch_add(1, Ordering::SeqCst);
            let OperationRequest::Reminder { text } = operation.request else {
                return Err(Undelivered::Refused("not a reminder".to_owned()));
            };
            self.texts
                .lock()
                .expect("fixture text lock poisoned")
                .push(text.clone());
            let mut held = self.held.lock().expect("fixture lock poisoned");
            if let Some((kept, outcome)) = held.get(&operation.operation) {
                if *kept != text {
                    return Err(Undelivered::Refused("operation_reused".to_owned()));
                }
                return Ok(outcome.clone());
            }
            self.typed.fetch_add(1, Ordering::SeqCst);
            let outcome = OperationOutcome {
                operation: operation.operation.clone(),
                session: operation.session,
                request: "reminder".to_owned(),
                state: self.answer,
                at: 0,
                words: "typed into the session".to_owned(),
                text: None,
                ended: None,
            };
            held.insert(operation.operation, (text, outcome.clone()));
            if self.lose_next.swap(false, Ordering::SeqCst) {
                return Err(Undelivered::Unknown("the answer was lost".to_owned()));
            }
            Ok(outcome)
        })
    }
}

fn opened(dir: &Path) -> Result<Goals, Box<dyn Error>> {
    Ok(Goals::new(GoalStore::open(&dir.join("goals"), key(dir)?)?))
}

fn item(goals: &Goals, id: &str) -> Result<lys_identity_server::goals_state::Item, Box<dyn Error>> {
    Ok(goals.with(|store| {
        store
            .item(id)
            .cloned()
            .ok_or_else(|| GoalError::Unknown.into())
    })?)
}

#[tokio::test]
async fn a_due_reminder_is_delivered_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    let goals = opened(dir.path())?;
    let at = now()?;
    let before = vec![Remind::Before { seconds: 7200 }];
    goals.with(|store| store.set(goal("op-g1", Kind::Goal, at, at + 3600, before)))?;
    let runner = Runner::new(OperationState::Delivered);
    remind(&goals, &runner, at).await?;
    remind(&goals, &runner, at + 10).await?;
    assert_eq!(runner.typed(), 1, "typed once");
    let item = item(&goals, "op-g1")?;
    assert_eq!(item.fired.len(), 1, "{item:?}");
    let fired = &item.fired[0];
    assert!(!fired.late);
    assert!(
        fired.text.contains("the sign-in page is live"),
        "{}",
        fired.text
    );
    assert!(fired.text.contains("left"), "{}", fired.text);
    assert_eq!(fired.sent[0].state, Delivery::Delivered);
    assert_eq!(item.timers[0].next_due, None, "fired for the last time");
    Ok(())
}

#[tokio::test]
async fn a_restart_keeps_the_reminders_due_instant() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = now()?;
    let deadline = at + 86_400;
    let goals = opened(dir.path())?;
    let reminders = vec![Remind::Before { seconds: 3600 }];
    goals.with(|store| store.set(goal("op-g1", Kind::Goal, at, deadline, reminders)))?;
    drop(goals);

    let goals = opened(dir.path())?;
    let item = item(&goals, "op-g1")?;
    assert_eq!(item.timers[0].next_due, Some(deadline - 3600));
    assert_eq!(
        goals.with(|store| Ok(store.next_due()))?,
        Some(deadline - 3600)
    );
    Ok(())
}

#[tokio::test]
async fn marking_a_goal_met_cancels_its_pending_reminders() -> TestResult {
    let dir = tempfile::tempdir()?;
    let goals = opened(dir.path())?;
    let at = now()?;
    let every = vec![
        Remind::Every { seconds: 600 },
        Remind::Before { seconds: 60 },
    ];
    goals.with(|store| store.set(goal("op-g1", Kind::Goal, at, at + 7200, every)))?;
    assert_eq!(goals.with(|store| Ok(store.next_due()))?, Some(at + 600));
    let item = goals.with(|store| store.mark(marked("op-m1", "op-g1", Standing::Met, None)))?;
    assert_eq!(item.standing, Standing::Met);
    assert!(item.timers.iter().all(|timer| timer.next_due.is_none()));
    let runner = Runner::new(OperationState::Delivered);
    remind(&goals, &runner, at + 7200).await?;
    assert_eq!(runner.typed(), 0);
    assert!(self::item(&goals, "op-g1")?.fired.is_empty());
    let again = goals.with(|store| store.mark(marked("op-m2", "op-g1", Standing::Dropped, None)));
    assert!(
        matches!(
            again,
            Err(ServerError::Agents(AgentsError::Goal(
                GoalError::Closed { .. }
            )))
        ),
        "{again:?}"
    );
    Ok(())
}

#[test]
fn a_deliverable_without_named_evidence_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let goals = opened(dir.path())?;
    let mut unnamed = goal("op-d0", Kind::Deliverable, 1, 99, Vec::new());
    unnamed.evidence = None;
    let refused = goals.with(|store| store.set(unnamed));
    assert!(
        matches!(
            refused,
            Err(ServerError::Agents(AgentsError::Goal(
                GoalError::EvidenceMissing { .. }
            )))
        ),
        "{refused:?}"
    );
    goals.with(|store| store.set(goal("op-d1", Kind::Deliverable, 1, 99, Vec::new())))?;
    let claimless = goals.with(|store| store.mark(marked("op-m1", "op-d1", Standing::Met, None)));
    assert!(
        matches!(
            claimless,
            Err(ServerError::Agents(AgentsError::Goal(
                GoalError::EvidenceMissing { .. }
            )))
        ),
        "{claimless:?}"
    );
    let met = goals.with(|store| {
        store.mark(marked(
            "op-m2",
            "op-d1",
            Standing::Met,
            Some("landed as 5cac25e7"),
        ))
    })?;
    let judged = met.marked.ok_or("no mark kept")?;
    assert_eq!(judged.evidence.as_deref(), Some("landed as 5cac25e7"));
    assert_eq!(judged.by, PERSON, "the claim names who made it");
    Ok(())
}

#[tokio::test]
async fn a_reminder_due_while_stopped_fires_once_late_with_both_instants() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = now()? - 1000;
    let goals = opened(dir.path())?;
    let every = vec![Remind::Every { seconds: 100 }];
    goals.with(|store| store.set(goal("op-g1", Kind::Expectation, at, at + 2000, every)))?;
    drop(goals);

    let goals = opened(dir.path())?;
    let started = goals.with(|store| Ok(store.opened_at()))?;
    let runner = Runner::new(OperationState::Delivered);
    remind(&goals, &runner, started).await?;
    remind(&goals, &runner, started).await?;
    drop(goals);

    let goals = opened(dir.path())?;
    let item = item(&goals, "op-g1")?;
    assert_eq!(item.fired.len(), 1, "fired once: {:?}", item.fired);
    let fired = &item.fired[0];
    assert!(fired.late);
    assert_eq!(fired.due, at + 100, "the instant it first fell due");
    assert_eq!(fired.fired, started, "the instant it fired");
    assert!(item.timers[0].next_due.is_some_and(|next| next > started));
    assert_eq!(runner.typed(), 1);
    Ok(())
}

#[tokio::test]
async fn a_crash_after_acceptance_does_not_type_the_reminder_twice() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = now()?;
    let runner = Runner::new(OperationState::Delivered);
    runner.lose_next.store(true, Ordering::SeqCst);
    let goals = opened(dir.path())?;
    let before = vec![Remind::Before { seconds: 7200 }];
    goals.with(|store| store.set(goal("op-g1", Kind::Goal, at, at + 3600, before)))?;
    remind(&goals, &runner, at).await?;
    let asked = item(&goals, "op-g1")?.fired[0].sent[0].clone();
    assert_eq!(asked.state, Delivery::Pending, "no answer was kept");
    let first_words = item(&goals, "op-g1")?.fired[0].text.clone();
    let retry_at = at + 61;
    let later = tempfile::tempdir()?;
    let later_goals = opened(later.path())?;
    later_goals.with(|store| {
        store.set(goal(
            "op-g1",
            Kind::Goal,
            at,
            at + 3600,
            vec![Remind::Before { seconds: 7200 }],
        ))
    })?;
    remind(
        &later_goals,
        &Runner::new(OperationState::Delivered),
        retry_at,
    )
    .await?;
    let rebuilt_words = item(&later_goals, "op-g1")?.fired[0].text.clone();
    assert_ne!(
        first_words, rebuilt_words,
        "the retry crosses a time-left boundary"
    );
    drop(goals);

    let goals = opened(dir.path())?;
    remind(&goals, &runner, retry_at).await?;
    assert_eq!(
        runner.asked.load(Ordering::SeqCst),
        2,
        "the pending operation is re-asked"
    );
    assert_eq!(
        *runner.texts.lock().expect("fixture text lock poisoned"),
        vec![first_words.clone(), first_words],
        "both asks carry identical words"
    );
    assert_eq!(runner.typed(), 1, "asked again under its id, typed once");
    let sent = item(&goals, "op-g1")?.fired[0].sent[0].clone();
    assert_eq!(sent.operation, asked.operation);
    assert_eq!(sent.state, Delivery::Delivered);
    Ok(())
}

#[tokio::test]
async fn an_uncertain_reminder_is_not_labelled_delivered() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = now()?;
    let goals = opened(dir.path())?;
    let before = vec![Remind::Before { seconds: 7200 }];
    goals.with(|store| store.set(goal("op-g1", Kind::Goal, at, at + 3600, before)))?;
    let runner = Runner::new(OperationState::Uncertain);
    remind(&goals, &runner, at).await?;
    remind(&goals, &runner, at + 1).await?;
    let item = item(&goals, "op-g1")?;
    let sent = &item.fired[0].sent[0];
    assert_eq!(sent.state, Delivery::Uncertain);
    let shown = serde_json::to_value(&item)?;
    assert_eq!(
        shown["fired"][0]["sent"][0]["state"], "uncertain",
        "{shown}"
    );
    assert_eq!(runner.typed(), 1, "never typed again");
    Ok(())
}

#[tokio::test]
async fn an_event_makes_its_reminder_due() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = now()?;
    let goals = opened(dir.path())?;
    let on = vec![Remind::On {
        event: Event::Compaction,
    }];
    goals.with(|store| store.set(goal("op-g1", Kind::Goal, at, at + 3600, on)))?;
    assert_eq!(goals.with(|store| Ok(store.next_due()))?, None);
    let compacted = Evented {
        operation: "op-e1".to_owned(),
        event: Event::Compaction,
        goals: vec!["op-g1".to_owned()],
        at: at + 5,
    };
    goals.with(|store| store.event(compacted.clone()))?;
    goals.with(|store| store.event(compacted))?;
    let runner = Runner::new(OperationState::Delivered);
    remind(&goals, &runner, at + 6).await?;
    let item = item(&goals, "op-g1")?;
    assert_eq!(item.fired.len(), 1);
    assert_eq!(item.fired[0].due, at + 5);
    assert_eq!(item.timers[0].next_due, None, "waits for the next event");
    Ok(())
}

#[tokio::test]
async fn a_restart_reads_only_the_leaves_after_the_snapshot() -> TestResult {
    let dir = tempfile::tempdir()?;
    drop(opened(dir.path())?);
    let at = now()?;
    let goals = opened(dir.path())?;
    let before = vec![Remind::Before { seconds: 7200 }];
    goals.with(|store| store.set(goal("op-g1", Kind::Goal, at, at + 3600, before)))?;
    remind(&goals, &Runner::new(OperationState::Delivered), at).await?;
    goals.with(|store| store.mark(marked("op-m1", "op-g1", Standing::Missed, None)))?;
    let kept = item(&goals, "op-g1")?;
    drop(goals);

    let goals = opened(dir.path())?;
    let start = goals.with(|store| Ok(store.start().clone()))?;
    assert_eq!(
        start,
        Start::Resumed {
            size: 0,
            replayed: 4
        }
    );
    assert_eq!(item(&goals, "op-g1")?, kept);
    let reused = goals.with(|store| {
        store.set(Goal {
            words: "other words".to_owned(),
            ..goal("op-g1", Kind::Goal, at, at + 3600, Vec::new())
        })
    });
    assert!(
        matches!(
            reused,
            Err(ServerError::Agents(AgentsError::Goal(
                GoalError::Reused { .. }
            )))
        ),
        "{reused:?}"
    );
    Ok(())
}

const BEA: &str = "bea-subject";

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: "shared@example.test".to_owned(),
    }
}

fn operation() -> Result<String, Box<dyn Error>> {
    Ok(OperationId::generate()?.to_string())
}

fn refused(answer: &(u16, Value), status: u16, name: &str) {
    assert_eq!(answer.0, status, "{}", answer.1);
    assert_eq!(answer.1["refusal"], name, "{}", answer.1);
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|byte| {
            [
                DIGITS[usize::from(byte >> 4)],
                DIGITS[usize::from(byte & 0x0f)],
            ]
        })
        .map(char::from)
        .collect()
}

/// Bea's agent with a certificate over `key`, Ada's agent, and the cookies
/// of Ada (administrator) and Bea.
struct Table {
    service: Service,
    agent: String,
    adas_agent: String,
    ada: String,
    bea: String,
    key: Arc<Ed25519Identity>,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        let agent = seeded.people[1].agents[0].id.to_string();
        let key = Arc::new(Ed25519Identity::load_or_generate(
            &service.dir.path().join("agent.key"),
        )?);
        let issue = json!({
            "operation": operation()?,
            "request": STANDARD.encode(create_certificate_request(&key, &agent)?),
        });
        let (status, answer) = service
            .post(&format!("/agents/{agent}/certificates"), Some(&bea), &issue)
            .await?;
        assert_eq!(status, 200, "{answer}");
        Ok(Self {
            service,
            agent,
            adas_agent: seeded.people[0].agents[0].id.to_string(),
            ada,
            bea,
            key,
        })
    }

    async fn set_goal(&self, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/agents/{}/goals", self.agent);
        self.service.post(&path, Some(&self.bea), body).await
    }

    /// The agent's own signed mark of `goal`.
    async fn agent_marks(&self, goal: &str, body: &Value) -> Result<(u16, Value), Box<dyn Error>> {
        self.agent_post(goal, "mark", 7, body).await
    }

    async fn agent_post(
        &self,
        goal: &str,
        action: &str,
        nonce: u8,
        body: &Value,
    ) -> Result<(u16, Value), Box<dyn Error>> {
        let path = format!("/goals/{goal}/{action}");
        let bytes = body.to_string().into_bytes();
        let signed_at = u64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())?;
        let nonce = hex(&[nonce; 16]);
        let signed = payload("POST", &path, &bytes, signed_at, &nonce);
        let cose = sign_attestation(&signed, &self.key).to_cose_bytes();
        let header = format!("{} {signed_at} {nonce} {}", self.agent, hex(&cose));
        self.service
            .post_signed(&path, (HEADER, &header), bytes)
            .await
    }
}

fn deliverable(deadline: u64) -> Result<Value, Box<dyn Error>> {
    Ok(json!({
        "operation": operation()?, "kind": "deliverable", "words": "the sign-in page",
        "deadline": deadline, "evidence": "commit",
        "reminders": [{ "when": "before", "seconds": 999_999 }, { "when": "every", "seconds": 3600 }],
    }))
}

#[tokio::test]
async fn a_goal_set_through_its_route_is_reminded_and_judged_by_its_person() -> TestResult {
    let table = Table::set().await?;
    let deadline = now()? + 86_400;
    let body = deliverable(deadline)?;
    let (status, set) = table.set_goal(&body).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["standing"], "open");
    assert_eq!(
        set["fired"][0]["refused"]
            .as_str()
            .map(|words| words.starts_with("no_live_session")),
        Some(true),
        "{set}"
    );
    assert!(set["timers"][1]["next_due"].is_u64(), "{set}");
    let (status, again) = table.set_goal(&body).await?;
    assert_eq!(status, 200, "{again}");
    assert_eq!(
        again["goal"], set["goal"],
        "the same act answers what was kept"
    );
    let mut changed = body.clone();
    changed["words"] = json!("another page");
    refused(&table.set_goal(&changed).await?, 409, "goal_reused");
    let mut unnamed = deliverable(deadline)?;
    unnamed["evidence"] = Value::Null;
    refused(&table.set_goal(&unnamed).await?, 400, "evidence_missing");

    let id = set["goal"]["id"].as_str().ok_or("no id")?;
    let mark = |evidence: Value| -> Result<Value, Box<dyn Error>> {
        Ok(
            json!({ "operation": operation()?, "standing": "met", "words": "seen live", "evidence": evidence }),
        )
    };
    let path = format!("/goals/{id}/mark");
    refused(
        &table.agent_marks(id, &mark(json!("landed"))?).await?,
        403,
        "not_your_judgement",
    );
    refused(
        &table
            .service
            .post(&path, Some(&table.ada), &mark(json!("landed"))?)
            .await?,
        403,
        "not_permitted",
    );
    refused(
        &table
            .service
            .post(&path, Some(&table.bea), &mark(Value::Null)?)
            .await?,
        400,
        "evidence_missing",
    );
    let (status, met) = table
        .service
        .post(&path, Some(&table.bea), &mark(json!("landed as 5cac25e7"))?)
        .await?;
    assert_eq!(status, 200, "{met}");
    assert_eq!(met["standing"], "met");
    assert_eq!(met["marked"]["evidence"], "landed as 5cac25e7");
    assert!(
        met["timers"]
            .as_array()
            .ok_or("no timers")?
            .iter()
            .all(|timer| timer["next_due"].is_null()),
        "{met}"
    );
    refused(
        &table
            .service
            .post(&path, Some(&table.bea), &mark(json!("again"))?)
            .await?,
        409,
        "goal_closed",
    );
    refused(
        &table
            .service
            .post("/goals/op-none/mark", Some(&table.bea), &mark(json!("x"))?)
            .await?,
        404,
        "goal_unknown",
    );

    let (status, listed) = table
        .service
        .get(&format!("/agents/{}/goals", table.agent), Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{listed}");
    assert_eq!(listed["goals"][0]["standing"], "met");
    Ok(())
}

#[tokio::test]
async fn goals_keep_their_holders_visibility() -> TestResult {
    let table = Table::set().await?;
    let adas = format!("/agents/{}/goals", table.adas_agent);
    refused(
        &table.service.get(&adas, Some(&table.bea)).await?,
        404,
        "AgentNotVisible",
    );
    let (status, team) = table
        .service
        .post(
            "/teams",
            Some(&table.ada),
            &json!({ "operation": operation()?, "name": "screens" }),
        )
        .await?;
    assert_eq!(status, 200, "{team}");
    let goals = format!("/teams/{}/goals", team["id"].as_str().ok_or("no team id")?);
    let body = json!({
        "operation": operation()?, "kind": "expectation", "words": "every screen reviewed",
        "deadline": now()? + 3600,
    });
    refused(
        &table.service.post(&goals, Some(&table.bea), &body).await?,
        403,
        "NotAdmitted",
    );
    refused(
        &table.service.get(&goals, Some(&table.bea)).await?,
        403,
        "NotAdmitted",
    );
    let (status, set) = table.service.post(&goals, Some(&table.ada), &body).await?;
    assert_eq!(status, 200, "{set}");
    assert_eq!(set["goal"]["holder"]["kind"], "team");
    refused(
        &table
            .service
            .post("/teams/op-none/goals", Some(&table.ada), &body)
            .await?,
        404,
        "TeamUnknown",
    );
    Ok(())
}

#[tokio::test]
async fn without_a_goals_directory_the_routes_say_so() -> TestResult {
    let (service, seeded) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.goals_dir = None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    )
    .await?;
    let bea = service.sign_in(login(BEA)).await?;
    let path = format!("/agents/{}/goals", seeded.people[1].agents[0].id);
    refused(
        &service.get(&path, Some(&bea)).await?,
        503,
        "goals_unavailable",
    );
    Ok(())
}

async fn forged_mark_is_refused(content_type: &str, foreign_origin: bool) -> TestResult {
    let table = Table::set().await?;
    let goal = deliverable(now()? + 86_400)?;
    let (status, set) = table.set_goal(&goal).await?;
    assert_eq!(status, 200, "{set}");
    let id = set["goal"]["id"].as_str().ok_or("no goal id")?;
    let path = format!("/goals/{id}/mark");
    let body = json!({ "operation": operation()?, "standing": "met", "words": "forged completion", "evidence": "a=b" });
    let client = reqwest::Client::new();
    let mut request = client
        .post(format!("{}{path}", table.service.base))
        .header("cookie", &table.bea)
        .header("content-type", content_type)
        .body(body.to_string());
    if foreign_origin {
        request = request.header("origin", "http://127.0.0.1:1");
    }
    let response = request.send().await?;
    assert_eq!(response.status(), 400);
    let answer: Value = serde_json::from_str(&response.text().await?)?;
    assert_eq!(answer["refusal"], "RequestMalformed", "{answer}");
    let read = table
        .service
        .get(&format!("/agents/{}/goals", table.agent), Some(&table.bea))
        .await?;
    assert_eq!(read.0, 200, "{}", read.1);
    assert!(!read.1.to_string().contains("forged completion"));
    let response = client
        .post(format!("{}{path}", table.service.base))
        .header("cookie", &table.bea)
        .header("content-type", "application/json")
        .header("origin", &table.service.base)
        .body(body.to_string())
        .send()
        .await?;
    assert_eq!(response.status(), 200, "{}", response.text().await?);
    Ok(())
}

#[tokio::test]
async fn a_plain_text_form_cannot_mark_a_goal_with_the_owners_cookie() -> TestResult {
    forged_mark_is_refused("text/plain", false).await
}

#[tokio::test]
async fn a_json_request_from_another_origin_cannot_mark_a_goal() -> TestResult {
    forged_mark_is_refused("application/json", true).await
}

#[tokio::test]
async fn a_pending_team_reminder_is_not_replayed_to_a_recipient_now_held() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = now()?;
    let mut runner = Runner::new(OperationState::Delivered);
    runner.lose_next.store(true, Ordering::SeqCst);
    let goals = opened(dir.path())?;
    let mut wanted = goal(
        "op-team-replay",
        Kind::Goal,
        at,
        at + 3600,
        vec![Remind::Before { seconds: 7200 }],
    );
    wanted.holder = Holder {
        kind: HolderKind::Team,
        id: "op-team".to_owned(),
    };
    goals.with(|store| store.set(wanted))?;
    remind(&goals, &runner, at).await?;
    assert_eq!(runner.asked.load(Ordering::SeqCst), 1);
    assert_eq!(
        item(&goals, "op-team-replay")?.fired[0].sent[0].state,
        Delivery::Pending
    );
    drop(goals);
    runner.sessions.clear();
    let goals = opened(dir.path())?;
    remind(&goals, &runner, at + 1).await?;
    assert_eq!(
        runner.asked.load(Ordering::SeqCst),
        1,
        "the replay must not reach the old recipient's runner"
    );
    let sent = item(&goals, "op-team-replay")?.fired[0].sent[0].clone();
    assert_eq!(sent.state, Delivery::Refused);
    assert!(
        sent.words.contains("team_membership_held"),
        "{}",
        sent.words
    );
    Ok(())
}

#[tokio::test]
async fn standing_aim_edits_use_the_existing_mark_judgement_authority() -> TestResult {
    let table = Table::set().await?;
    let body =
        json!({"operation":operation()?, "kind":"goal", "words":"keep the directory available"});
    let (status, item) = table.set_goal(&body).await?;
    assert_eq!(status, 200, "{item}");
    let id = item["goal"]["id"].as_str().ok_or("goal id absent")?;
    for (action, field, value, nonce) in [
        ("active", "active", json!(false), 1),
        ("words", "words", json!("keep the service available"), 2),
    ] {
        let mut body = json!({"operation":operation()?});
        body[field] = value;
        refused(
            &table.agent_post(id, action, nonce, &body).await?,
            403,
            "not_your_judgement",
        );
        let path = format!("/goals/{id}/{action}");
        refused(
            &table.service.post(&path, Some(&table.ada), &body).await?,
            403,
            "not_permitted",
        );
        let (status, answer) = table.service.post(&path, Some(&table.bea), &body).await?;
        assert_eq!(status, 200, "{answer}");
        assert_eq!(answer["goal"][field], body[field]);
    }
    Ok(())
}
