#![cfg(test)]

//! AGENTS-001 R4: schedules. A once schedule fires once; an interval
//! schedule with max 3 stops after 3; stopped across two intervals and
//! started again, one send stands for both then the next slot follows; an
//! uncertain delivery stops the schedule and says why; a pause keeps the
//! count; every occurrence is kept with its rendered text and revisions
//! before any runner is asked; and the routes refuse by name.

use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use identity_contract::fake_issuer::Login;
use identity_contract::harness::{ADMINISTRATOR, GRANT_MODEL, Service};
use lys_core::Ed25519Identity;
use lys_identity::OperationId;
use lys_identity_server::dev_seed::seed_configured;
use lys_identity_server::schedules_state::{
    Change, Changed, Delivery, Recipient, Schedule, Source,
};
use lys_identity_server::schedules_store::{
    Deliver, Delivering, SchedulesKept, Undelivered, Worded, change, item, open, pass, set,
};
use lys_runner::operations::{Operation, OperationOutcome, OperationRequest, OperationState};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const AGENT: &str = "agent-00000000000000000000000000000002";
const BEA: &str = "bea-subject";

fn now() -> Result<u64, Box<dyn Error>> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

fn key(dir: &Path) -> Result<Arc<Ed25519Identity>, Box<dyn Error>> {
    Ok(Arc::new(Ed25519Identity::load_or_generate(
        &dir.join("schedules.key"),
    )?))
}

fn opened(dir: &Path) -> Result<SchedulesKept, Box<dyn Error>> {
    Ok(SchedulesKept::new(open(&dir.join("schedules"), key(dir)?)?))
}

fn schedule(id: &str, at: u64, interval: Option<u64>, max: Option<u64>) -> Schedule {
    Schedule {
        id: id.to_owned(),
        at,
        until: None,
        interval,
        max_occurrences: max,
        recipients: vec![Recipient::Agent {
            id: AGENT.to_owned(),
        }],
        source: Source::Text {
            text: "Stand-up in {{vars.room | the usual room}}.".to_owned(),
        },
        author: "person-1".to_owned(),
        set_at: at.saturating_sub(10),
    }
}

/// A runner as the operations record answers: an operation asked again
/// under its id is answered as it stands and never typed twice.
struct Runner {
    held: Mutex<BTreeMap<String, OperationOutcome>>,
    typed: AtomicUsize,
    texts: Mutex<Vec<String>>,
    lose_next: AtomicBool,
    answer: OperationState,
}

impl Runner {
    fn new(answer: OperationState) -> Self {
        Self {
            held: Mutex::default(),
            typed: AtomicUsize::new(0),
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
    fn sessions(&self, _recipient: &Recipient) -> Result<Vec<(String, Option<String>)>, String> {
        Ok(vec![("session-1".to_owned(), Some(AGENT.to_owned()))])
    }

    fn worded(
        &self,
        source: &Source,
        _agent: Option<&str>,
        _session: &str,
    ) -> Result<Worded, String> {
        let Source::Text { text } = source else {
            return Err("slot words need a server".to_owned());
        };
        let rendered = lys_runner::render::render(text, &lys_runner::render::Inputs::default());
        Ok(Worded {
            text: rendered.text,
            contributed: Vec::new(),
            missing: rendered.missing,
        })
    }

    fn operate(&self, operation: Operation) -> Delivering<'_> {
        Box::pin(async move {
            let OperationRequest::Reminder { text } = operation.request else {
                return Err(Undelivered::Refused("not a reminder".to_owned()));
            };
            let mut held = self.held.lock().expect("fixture lock poisoned");
            if let Some(outcome) = held.get(&operation.operation) {
                return Ok(outcome.clone());
            }
            self.typed.fetch_add(1, Ordering::SeqCst);
            self.texts
                .lock()
                .expect("fixture text lock poisoned")
                .push(text);
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
            held.insert(operation.operation, outcome.clone());
            if self.lose_next.swap(false, Ordering::SeqCst) {
                return Err(Undelivered::Unknown("the answer was lost".to_owned()));
            }
            Ok(outcome)
        })
    }
}

#[tokio::test]
async fn a_once_schedule_fires_once_with_its_text_rendered_and_then_stops() -> TestResult {
    let dir = tempfile::tempdir()?;
    let kept = opened(dir.path())?;
    let at = now()?;
    set(&kept, schedule("op-s1", at + 60, None, None))?;
    assert_eq!(kept.with(|log| Ok(log.held().next_due()))?, Some(at + 60));
    let runner = Runner::new(OperationState::Delivered);
    pass(&kept, &runner, at + 30).await?;
    assert_eq!(runner.typed(), 0, "not due yet");
    pass(&kept, &runner, at + 61).await?;
    pass(&kept, &runner, at + 120).await?;
    assert_eq!(runner.typed(), 1, "typed once");
    let item = item(&kept, "op-s1")?;
    assert_eq!(item.fired.len(), 1);
    let fired = &item.fired[0];
    assert_eq!(fired.occurrence, 1);
    assert_eq!(fired.due, at + 60);
    assert_eq!(fired.coalesced, 1);
    assert_eq!(fired.sent.len(), 1);
    assert_eq!(fired.sent[0].text, "Stand-up in the usual room.");
    assert_eq!(fired.sent[0].state, Delivery::Delivered);
    assert_eq!(item.next_due, None);
    assert_eq!(
        item.stopped.as_ref().map(|stopped| stopped.reason.as_str()),
        Some("finished")
    );
    Ok(())
}

#[tokio::test]
async fn an_interval_schedule_with_max_three_stops_after_three() -> TestResult {
    let dir = tempfile::tempdir()?;
    let kept = opened(dir.path())?;
    let at = now()?;
    set(&kept, schedule("op-s2", at + 60, Some(60), Some(3)))?;
    let runner = Runner::new(OperationState::Delivered);
    for step in 1..=5 {
        pass(&kept, &runner, at + 60 * step + 1).await?;
    }
    assert_eq!(runner.typed(), 3);
    let item = item(&kept, "op-s2")?;
    assert_eq!(item.fired.len(), 3);
    assert_eq!(item.fired[2].due, at + 180);
    assert_eq!(
        item.stopped.as_ref().map(|stopped| stopped.reason.as_str()),
        Some("finished")
    );
    Ok(())
}

#[tokio::test]
async fn intervals_missed_while_stopped_coalesce_to_one_send_then_the_next_slot() -> TestResult {
    let dir = tempfile::tempdir()?;
    let at = now()?;
    {
        let kept = opened(dir.path())?;
        set(&kept, schedule("op-s3", at + 60, Some(60), None))?;
    }
    // Stopped across two intervals: the first pass after the restart sends once.
    let kept = opened(dir.path())?;
    assert_eq!(kept.with(|log| Ok(log.held().next_due()))?, Some(at + 60));
    let runner = Runner::new(OperationState::Delivered);
    pass(&kept, &runner, at + 125).await?;
    assert_eq!(runner.typed(), 1, "one send for two due instants");
    let item = item(&kept, "op-s3")?;
    assert_eq!(item.fired.len(), 1);
    assert_eq!(item.fired[0].coalesced, 2);
    assert_eq!(item.fired[0].due, at + 120);
    assert_eq!(item.next_due, Some(at + 180), "then the next slot");
    pass(&kept, &runner, at + 181).await?;
    assert_eq!(runner.typed(), 2);
    Ok(())
}

#[tokio::test]
async fn an_uncertain_delivery_stops_the_schedule_and_says_why() -> TestResult {
    let dir = tempfile::tempdir()?;
    let kept = opened(dir.path())?;
    let at = now()?;
    set(&kept, schedule("op-s4", at + 60, Some(60), None))?;
    let runner = Runner::new(OperationState::Uncertain);
    pass(&kept, &runner, at + 61).await?;
    let item = item(&kept, "op-s4")?;
    assert_eq!(item.fired[0].sent[0].state, Delivery::Uncertain);
    let stopped = item.stopped.as_ref().expect("stopped");
    assert!(
        stopped.reason.starts_with("uncertain_delivery:"),
        "{}",
        stopped.reason
    );
    assert_eq!(item.next_due, None);
    pass(&kept, &runner, at + 121).await?;
    assert_eq!(runner.typed(), 1, "never sent again");
    let refused = change(
        &kept,
        Changed {
            operation: "op-c1".to_owned(),
            schedule: "op-s4".to_owned(),
            change: Change::Paused { paused: false },
            by: "person-1".to_owned(),
            at: at + 130,
        },
    );
    assert!(refused.is_err(), "{refused:?}");
    Ok(())
}

#[tokio::test]
async fn a_lost_answer_is_asked_again_under_the_same_id_and_never_typed_twice() -> TestResult {
    let dir = tempfile::tempdir()?;
    let kept = opened(dir.path())?;
    let at = now()?;
    set(&kept, schedule("op-s5", at + 60, None, None))?;
    let runner = Runner::new(OperationState::Delivered);
    runner.lose_next.store(true, Ordering::SeqCst);
    pass(&kept, &runner, at + 61).await?;
    let pending = item(&kept, "op-s5")?;
    assert_eq!(pending.fired[0].sent[0].state, Delivery::Pending);
    pass(&kept, &runner, at + 62).await?;
    assert_eq!(runner.typed(), 1, "the runner answered as it stood");
    assert_eq!(
        item(&kept, "op-s5")?.fired[0].sent[0].state,
        Delivery::Delivered
    );
    Ok(())
}

#[tokio::test]
async fn a_pause_keeps_the_count_and_a_resume_continues_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let kept = opened(dir.path())?;
    let at = now()?;
    set(&kept, schedule("op-s6", at + 60, Some(60), Some(2)))?;
    let runner = Runner::new(OperationState::Delivered);
    pass(&kept, &runner, at + 61).await?;
    let paused = |paused: bool, op: &str, when: u64| Changed {
        operation: op.to_owned(),
        schedule: "op-s6".to_owned(),
        change: Change::Paused { paused },
        by: "person-1".to_owned(),
        at: when,
    };
    change(&kept, paused(true, "op-p1", at + 70))?;
    assert_eq!(item(&kept, "op-s6")?.next_due, None);
    pass(&kept, &runner, at + 121).await?;
    assert_eq!(runner.typed(), 1, "paused");
    change(&kept, paused(false, "op-p2", at + 130))?;
    assert_eq!(item(&kept, "op-s6")?.fired.len(), 1, "the count is kept");
    pass(&kept, &runner, at + 181).await?;
    assert_eq!(runner.typed(), 2);
    assert_eq!(
        item(&kept, "op-s6")?
            .stopped
            .as_ref()
            .map(|s| s.reason.as_str()),
        Some("finished")
    );
    Ok(())
}

fn login(subject: &str) -> Login {
    Login {
        subject: subject.to_owned(),
        email: format!("{subject}@example.test"),
    }
}

struct Table {
    service: Service,
    beas_agent: String,
    adas_agent: String,
    ada: String,
    bea: String,
}

impl Table {
    async fn set() -> Result<Self, Box<dyn Error>> {
        let (service, seeded) =
            Service::start_with(|config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?))
                .await?;
        let ada = service.sign_in(login(ADMINISTRATOR)).await?;
        let bea = service.sign_in(login(BEA)).await?;
        Ok(Self {
            service,
            beas_agent: seeded.people[1].agents[0].id.to_string(),
            adas_agent: seeded.people[0].agents[0].id.to_string(),
            ada,
            bea,
        })
    }

    fn body(&self, agent: &str, at: u64) -> Result<Value, Box<dyn Error>> {
        Ok(json!({
            "operation": OperationId::generate()?.to_string(),
            "at": at,
            "interval": 3600,
            "max_occurrences": 2,
            "recipients": [{ "kind": "agent", "id": agent }],
            "source": { "kind": "text", "text": "Stand-up." },
        }))
    }
}

#[tokio::test]
async fn the_routes_set_read_change_and_stop_and_refuse_by_name() -> TestResult {
    let table = Table::set().await?;
    let at = now()? + 86_400;
    let body = table.body(&table.beas_agent, at)?;
    let (status, set) = table
        .service
        .post("/schedules", Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{set}");
    let id = set["schedule"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    assert_eq!(set["next_due"], at);
    assert_eq!(
        set["schedule"]["author"]
            .as_str()
            .map(|a| a.starts_with("person-")),
        Some(true)
    );

    // The same operation in the same words answers as it stands; in other words it is refused.
    let (status, again) = table
        .service
        .post("/schedules", Some(&table.bea), &body)
        .await?;
    assert_eq!(status, 200, "{again}");
    let mut other = body.clone();
    other["source"] = json!({ "kind": "text", "text": "Something else." });
    let (status, refused) = table
        .service
        .post("/schedules", Some(&table.bea), &other)
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "schedule_reused");

    let (status, read) = table
        .service
        .get(&format!("/schedules/{id}"), Some(&table.bea))
        .await?;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["schedule"]["id"], id);
    let (status, list) = table.service.get("/schedules", Some(&table.ada)).await?;
    assert_eq!(status, 200, "{list}");
    assert_eq!(list["schedules"].as_array().map(Vec::len), Some(1));

    let pause = json!({ "operation": OperationId::generate()?.to_string(), "change": { "field": "paused", "paused": true } });
    let (status, paused) = table
        .service
        .post(&format!("/schedules/{id}/change"), Some(&table.bea), &pause)
        .await?;
    assert_eq!(status, 200, "{paused}");
    assert_eq!(paused["next_due"], Value::Null);

    let (status, stopped) = table
        .service
        .post(
            &format!("/schedules/{id}/stop"),
            Some(&table.ada),
            &json!({ "words": "done" }),
        )
        .await?;
    assert_eq!(status, 200, "{stopped}");
    assert!(
        stopped["stopped"]["reason"]
            .as_str()
            .is_some_and(|r| r.starts_with("stopped_by:"))
    );
    let (status, refused) = table
        .service
        .post(&format!("/schedules/{id}/change"), Some(&table.bea), &json!({ "operation": OperationId::generate()?.to_string(), "change": { "field": "paused", "paused": false } }))
        .await?;
    assert_eq!(status, 409, "{refused}");
    assert_eq!(refused["refusal"], "schedule_stopped");

    // Refusals by name.
    let (status, refused) = table
        .service
        .get("/schedules/op-nobody", Some(&table.ada))
        .await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "schedule_unknown");
    let (status, refused) = table
        .service
        .post(
            "/schedules",
            Some(&table.ada),
            &table.body(&table.beas_agent, 1)?,
        )
        .await?;
    assert_eq!(status, 400, "{refused}");
    assert_eq!(refused["refusal"], "schedule_malformed");
    let (status, refused) = table
        .service
        .post(
            "/schedules",
            Some(&table.bea),
            &table.body(&table.adas_agent, at)?,
        )
        .await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "AgentNotVisible");
    let mut session = table.body(&table.beas_agent, at)?;
    session["recipients"] = json!([{ "kind": "session", "id": "session-nobody" }]);
    let (status, refused) = table
        .service
        .post("/schedules", Some(&table.ada), &session)
        .await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "SessionUnknown");
    let (status, refused) = table.service.get("/schedules", None).await?;
    assert_eq!(status, 401, "{refused}");
    assert_eq!(refused["refusal"], "NotSignedIn");

    // Bea does not see a schedule on Ada's agent.
    let (status, adas) = table
        .service
        .post(
            "/schedules",
            Some(&table.ada),
            &table.body(&table.adas_agent, at)?,
        )
        .await?;
    assert_eq!(status, 200, "{adas}");
    let adas_id = adas["schedule"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let (status, refused) = table
        .service
        .get(&format!("/schedules/{adas_id}"), Some(&table.bea))
        .await?;
    assert_eq!(status, 404, "{refused}");
    assert_eq!(refused["refusal"], "schedule_unknown");
    let (status, list) = table.service.get("/schedules", Some(&table.bea)).await?;
    assert_eq!(status, 200, "{list}");
    assert_eq!(list["schedules"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[tokio::test]
async fn without_a_schedules_directory_the_routes_answer_schedules_unavailable() -> TestResult {
    let (service, _) = Service::start_adjusted(
        GRANT_MODEL,
        None,
        None,
        None,
        |config| config.schedules_dir = None,
        |config| Ok(seed_configured(config, [ADMINISTRATOR, BEA])?),
    )
    .await?;
    let ada = service.sign_in(login(ADMINISTRATOR)).await?;
    let (status, answer) = service.get("/schedules", Some(&ada)).await?;
    assert_eq!(status, 503, "{answer}");
    assert_eq!(answer["refusal"], "schedules_unavailable");
    Ok(())
}
