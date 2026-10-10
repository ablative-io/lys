//! The schedules as they are kept (AGENTS-001 R4): a leaf store of their
//! own through the agents log engine, and the pass that fires each
//! occurrence due. An occurrence is kept, with each recipient's rendered
//! text and the operation id its delivery is asked under, before any runner
//! is asked; each answer is kept as it comes; a delivery whose answer was
//! lost is asked again under the same operation id, which the runner
//! answers as it stands and never types twice. An uncertain delivery stops
//! the schedule, kept with its reason.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_log_store::FileLeafStore;
use lys_runner::operations::{Operation, OperationOutcome, OperationRequest, OperationState};
use sha2::{Digest, Sha256};
use tokio::sync::Notify;

use crate::agents_log::{Kept, RecordLog};
use crate::config::Config;
use crate::error::ServerError;
use crate::routes::{Say, hex};
use crate::schedules_state::{
    Answered, Change, Changed, Delivery, Fired, Item, Line, Recipient, Schedule, Schedules,
    SchedulesError, Sent, Source, Stopped,
};
use crate::session::now;
use crate::words_state::Contributed;

pub use crate::runner_operate::Undelivered;

/// The origin the schedules' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/schedules";

fn unavailable(reason: String) -> ServerError {
    SchedulesError::Unavailable { reason }.into()
}

/// The schedules, read from their leaf store and appended to it.
pub type ScheduleStore = RecordLog<Schedules, FileLeafStore>;

/// The schedules behind one lock, and the signal their timers wait on.
pub struct SchedulesKept {
    /// The log, one caller at a time.
    pub kept: Kept<Schedules>,
    /// Signalled each time something a timer depends on is kept.
    pub changed: Arc<Notify>,
}

impl SchedulesKept {
    /// `store`, with its signal.
    pub fn new(store: ScheduleStore) -> Self {
        Self {
            kept: Kept::new(store, unavailable),
            changed: Arc::new(Notify::new()),
        }
    }

    /// Run `act` on the settled log.
    pub fn with<T>(
        &self,
        act: impl FnOnce(&mut ScheduleStore) -> Result<T, ServerError>,
    ) -> Result<T, ServerError> {
        self.kept.with(|log, _| act(log))
    }

    fn append(&self, line: Line) -> Result<(), ServerError> {
        self.kept.with(|log, unavailable| log.append(line, unavailable))
    }
}

/// The schedules in the directory `config` names, their snapshots signed by
/// `key`, saying through `say` how the log started; none when it names no
/// directory.
pub fn configured(
    config: &Config,
    key: Arc<Ed25519Identity>,
    say: &Say,
) -> Result<Option<SchedulesKept>, ServerError> {
    let Some(dir) = config.schedules_dir.as_deref() else {
        return Ok(None);
    };
    let store = open(dir, key)?;
    say(&format!(
        "schedules log {}, holding {} schedules",
        store.start(),
        store.held().items.len()
    ));
    Ok(Some(SchedulesKept::new(store)))
}

/// The schedules kept in the directory `dir`, created when it does not exist.
pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<ScheduleStore, ServerError> {
    RecordLog::open(dir, ORIGIN, key, &unavailable)
}

/// Keep `schedule` as set. Sent again in the same words it answers the
/// schedule as it stands; the same operation in other words is refused.
pub fn set(schedules: &SchedulesKept, schedule: Schedule) -> Result<Item, ServerError> {
    schedule.check(now())?;
    let kept = schedules.with(|log| Ok(log.held().item(&schedule.id).cloned()))?;
    if let Some(item) = kept {
        let same = Schedule {
            set_at: item.schedule.set_at,
            ..schedule.clone()
        };
        if same != item.schedule {
            return Err(SchedulesError::Reused {
                operation: schedule.id,
            }
            .into());
        }
        return Ok(item);
    }
    let id = schedule.id.clone();
    schedules.append(Line::Set(schedule))?;
    schedules.changed.notify_one();
    item(schedules, &id)
}

/// The schedule `id` as it stands.
pub fn item(schedules: &SchedulesKept, id: &str) -> Result<Item, ServerError> {
    schedules.with(|log| {
        log.held()
            .item(id)
            .cloned()
            .ok_or_else(|| SchedulesError::Unknown.into())
    })
}

/// Every schedule, in the order set.
pub fn items(schedules: &SchedulesKept) -> Result<Vec<Item>, ServerError> {
    schedules.with(|log| Ok(log.held().items.clone()))
}

/// Keep `changed`, refusing a stopped schedule; the same operation sent
/// again in the same words answers the schedule as it stands.
pub fn change(schedules: &SchedulesKept, changed: Changed) -> Result<Item, ServerError> {
    let held = item(schedules, &changed.schedule)?;
    if let Some(kept) = held
        .changes
        .iter()
        .find(|kept| kept.operation == changed.operation)
    {
        if kept.change != changed.change || kept.by != changed.by {
            return Err(SchedulesError::Reused {
                operation: changed.operation,
            }
            .into());
        }
        return Ok(held);
    }
    if let Some(stopped) = &held.stopped {
        return Err(SchedulesError::Stopped {
            schedule: changed.schedule,
            reason: stopped.reason.clone(),
        }
        .into());
    }
    match &changed.change {
        Change::Recipients { recipients } if recipients.is_empty() => {
            return Err(SchedulesError::Malformed {
                reason: "recipients is empty".to_owned(),
            }
            .into());
        }
        Change::Source {
            source: Source::Text { text },
        } => {
            crate::words_state::checked_text(text).map_err(|refused| SchedulesError::Malformed {
                reason: refused.to_string(),
            })?;
        }
        Change::MaxOccurrences {
            max_occurrences: Some(0),
        } => {
            return Err(SchedulesError::Malformed {
                reason: "max_occurrences is 0".to_owned(),
            }
            .into());
        }
        _ => {}
    }
    let id = changed.schedule.clone();
    schedules.append(Line::Changed(changed))?;
    schedules.changed.notify_one();
    item(schedules, &id)
}

/// Stop the schedule `id` for `reason`; a schedule already stopped answers
/// as it stands.
pub fn stop(schedules: &SchedulesKept, id: &str, reason: String) -> Result<Item, ServerError> {
    let held = item(schedules, id)?;
    if held.stopped.is_some() {
        return Ok(held);
    }
    schedules.append(Line::Stopped(Stopped {
        schedule: id.to_owned(),
        reason,
        at: now(),
    }))?;
    schedules.changed.notify_one();
    item(schedules, id)
}

/// A runner's answer to one delivery, or why there is none.
pub type Delivering<'a> =
    Pin<Box<dyn Future<Output = Result<OperationOutcome, Undelivered>> + Send + 'a>>;

/// What a rendering for one recipient produced.
pub struct Worded {
    /// The text.
    pub text: String,
    /// Every revision that contributed.
    pub contributed: Vec<Contributed>,
    /// Each placeholder that rendered empty.
    pub missing: Vec<String>,
}

/// How occurrences reach recipients: through each session's runner, under
/// the runner's operation ids, with the words rendered for that session.
pub trait Deliver: Send + Sync {
    /// The live sessions of `recipient`, each with its agent; or why none
    /// can be reached, by name.
    fn sessions(&self, recipient: &Recipient) -> Result<Vec<(String, Option<String>)>, String>;
    /// `source` rendered for `session` of `agent` now.
    fn worded(
        &self,
        source: &Source,
        agent: Option<&str>,
        session: &str,
    ) -> Result<Worded, String>;
    /// Ask the session's runner for `operation`, answering how it stands.
    fn operate(&self, operation: Operation) -> Delivering<'_>;
}

/// An operation id made from `parts`, the same each time.
fn op_id(parts: &[&str]) -> String {
    let digest = Sha256::digest(format!("lys-identity/schedules/v1\n{}", parts.join("\n")));
    format!("op-{}", &hex(&digest)[..32])
}

/// One pass at `at`: every delivery still unsettled is asked again under
/// its own operation id, and every occurrence due fires once, kept before
/// any runner is asked. Answers the next due instant.
pub async fn pass(
    schedules: &SchedulesKept,
    deliver: &dyn Deliver,
    at: u64,
) -> Result<Option<u64>, ServerError> {
    // Unsettled deliveries first: asked again, never typed twice.
    let pending = schedules.with(|log| Ok(log.held().pending()))?;
    for (schedule, sent) in pending {
        let operation = Operation {
            operation: sent.operation.clone(),
            session: sent.session.clone(),
            request: OperationRequest::Reminder {
                text: sent.text.clone(),
            },
        };
        answer(schedules, &schedule, &sent.operation, deliver.operate(operation).await).await?;
    }
    // Then every occurrence due.
    let due = schedules.with(|log| Ok(log.held().due(at)))?;
    for (id, due_at, coalesced) in due {
        let held = item(schedules, &id)?;
        if held.stopped.is_some() || held.paused() {
            continue;
        }
        let occurrence = u64::try_from(held.fired.len()).unwrap_or(u64::MAX).saturating_add(1);
        let operation = op_id(&[&id, &occurrence.to_string(), &due_at.to_string()]);
        let fired = fire(&held, deliver, (operation, occurrence, due_at, coalesced), at);
        let asks: Vec<(String, Operation)> = fired
            .sent
            .iter()
            .map(|sent| {
                (
                    sent.operation.clone(),
                    Operation {
                        operation: sent.operation.clone(),
                        session: sent.session.clone(),
                        request: OperationRequest::Reminder {
                            text: sent.text.clone(),
                        },
                    },
                )
            })
            .collect();
        schedules.append(Line::Fired(fired))?;
        for (operation, ask) in asks {
            answer(schedules, &id, &operation, deliver.operate(ask).await).await?;
        }
        let after = item(schedules, &id)?;
        if after.stopped.is_none() && after.next_due.is_none() && after.settled() {
            let reason = if after.max_occurrences().is_some_and(|max| {
                u64::try_from(after.fired.len()).unwrap_or(u64::MAX) >= max
            }) || after.schedule.interval.is_none()
            {
                "finished"
            } else {
                "until_reached"
            };
            stop(schedules, &id, reason.to_owned())?;
        }
    }
    schedules.with(|log| Ok(log.held().next_due()))
}

/// The occurrence of `item` due at `due`, each recipient's sessions named
/// and worded, before any runner is asked.
fn fire(
    item: &Item,
    deliver: &dyn Deliver,
    (operation, occurrence, due, coalesced): (String, u64, u64, u64),
    at: u64,
) -> Fired {
    let mut sent = Vec::new();
    let mut refusals = Vec::new();
    for recipient in item.recipients() {
        match deliver.sessions(recipient) {
            Ok(sessions) if sessions.is_empty() => {
                refusals.push(format!("no_live_session: {}", recipient_name(recipient)));
            }
            Ok(sessions) => {
                for (session, agent) in sessions {
                    let worded = match deliver.worded(item.source(), agent.as_deref(), &session) {
                        Ok(worded) => worded,
                        Err(refused) => {
                            refusals.push(format!("words_unrendered: {session}: {refused}"));
                            continue;
                        }
                    };
                    sent.push(Sent {
                        recipient: recipient.clone(),
                        operation: op_id(&["delivery", &operation, &session]),
                        session,
                        text: worded.text,
                        contributed: worded.contributed,
                        missing: worded.missing,
                        state: Delivery::Pending,
                        words: "asked of the session's runner".to_owned(),
                        at,
                    });
                }
            }
            Err(refused) => refusals.push(refused),
        }
    }
    Fired {
        operation,
        schedule: item.schedule.id.clone(),
        occurrence,
        due,
        fired: at,
        coalesced,
        sent,
        refused: (!refusals.is_empty()).then(|| refusals.join("; ")),
    }
}

fn recipient_name(recipient: &Recipient) -> String {
    match recipient {
        Recipient::Agent { id } => format!("agent {id}"),
        Recipient::Session { id } => format!("session {id}"),
    }
}

/// Keep what the runner answered for `operation` of `schedule`, and stop
/// the schedule on an uncertain delivery or a terminal refusal.
async fn answer(
    schedules: &SchedulesKept,
    schedule: &str,
    operation: &str,
    answered: Result<OperationOutcome, Undelivered>,
) -> Result<(), ServerError> {
    let at = now();
    let (state, words, terminal) = match answered {
        Ok(outcome) => (
            match outcome.state {
                OperationState::Accepted | OperationState::Delivering => Delivery::Accepted,
                OperationState::Delivered | OperationState::Confirmed => Delivery::Delivered,
                OperationState::Uncertain => Delivery::Uncertain,
                OperationState::Refused => Delivery::Refused,
            },
            outcome.words,
            false,
        ),
        Err(Undelivered::Refused(words)) => (Delivery::Refused, words, true),
        Err(Undelivered::Unknown(words)) => (Delivery::Pending, words, false),
    };
    let changed = schedules.with(|log| {
        Ok(log
            .held()
            .sent(operation)
            .is_some_and(|(_, sent)| sent.state.unsettled() && (sent.state != state || sent.words != words)))
    })?;
    if changed {
        schedules.append(Line::Answered(Answered {
            operation: operation.to_owned(),
            state,
            words: words.clone(),
            at,
        }))?;
    }
    if state == Delivery::Uncertain {
        stop(
            schedules,
            schedule,
            format!("uncertain_delivery: operation {operation}: {words}"),
        )?;
    } else if terminal && words.starts_with("terminal:") {
        stop(schedules, schedule, format!("terminal_failure: {words}"))?;
    }
    Ok(())
}
