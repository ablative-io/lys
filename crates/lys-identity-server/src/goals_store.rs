//! The goals as they are kept: a leaf store of their own, one leaf for each
//! item set, each judgement, each reminder fired, each runner answer to a
//! delivery and each event a reminder waits on. A leaf is stored whole or
//! not at all.
//!
//! An append that fails leaves its outcome unknown. It is settled by opening
//! the leaf store again and reading what it holds, before anything else is
//! read or written, so memory never runs ahead of or behind the leaves.
//!
//! A reminder is a timer folded from the leaves. A pass fires every timer
//! due: the firing, naming each session's runner operation id, is kept
//! before any runner is asked, and each answer is kept as it comes. A
//! delivery whose answer was lost is asked again under the same operation
//! id, which the runner answers as it stands and never types twice. A
//! reminder that fell due while the service was stopped fires once, at the
//! first pass, kept late with the instant it fell due and the instant it
//! fired.
//!
//! What the leaves fold to is sealed in the log's signed snapshot every
//! [`SNAPSHOT_EVERY`] leaves and at once after a rebuild, so a start reads the
//! snapshot and only the leaves after it.

use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};
use lys_runner::operations::{Operation, OperationOutcome, OperationRequest};
use sha2::{Digest, Sha256};
use tokio::sync::Notify;

mod control;
pub(crate) use control::{occurrence_text, text_with_words};

use crate::config::Config;
use crate::error::ServerError;
use crate::goals_state::{
    Answered, Changed, DOMAIN, Delivery, Due, Evented, Fired, GoalError, Held, Holder, Item, Line,
    Marked, Sent, Standing,
};
use crate::routes::{Say, hex};
use crate::session::now;

/// How the leaf store is opened again after an append whose outcome is not known.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;

/// The origin the goals' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/goals";

/// The goals, read from their leaf store and appended to it.
pub struct GoalStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    opened_at: u64,
    since_snapshot: u64,
    snapshot_failure: Option<String>,
    uncertain: bool,
}

/// A log opened and folded: the log, what it folds to, and how it started.
type Opened<S> = (FrontierLog<S>, Held, Start);

fn unavailable(what: impl std::fmt::Display) -> ServerError {
    GoalError::Unavailable {
        reason: what.to_string(),
    }
    .into()
}

impl GoalStore<FileLeafStore> {
    /// The goals in the directory `config` names, their snapshots signed by
    /// `key`, saying through `say` how the log started; none when it names no
    /// directory.
    pub fn configured(
        config: &Config,
        key: Arc<Ed25519Identity>,
        say: &Say,
    ) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.goals_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(dir, key)?;
        say(&format!(
            "goals log {}, holding {} goals",
            store.start(),
            store.held.items.len()
        ));
        Ok(Some(store))
    }

    /// The goals kept in the directory `dir`, which is created when it does
    /// not exist, their snapshots signed by `key`.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> GoalStore<S> {
    /// The goals kept in the leaf store `reopen` opens, their snapshots
    /// signed by `key`. A reminder due before now fell due while the
    /// service was stopped.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        let (log, held, start) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            log,
            held,
            start: start.clone(),
            opened_at: now(),
            since_snapshot: 0,
            snapshot_failure: None,
            uncertain: false,
        };
        store.after_start(&start);
        Ok(store)
    }

    /// How the log was started: from its snapshot, or from every leaf and
    /// the refusal that sent it there.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// When this store was opened, in seconds since the Unix epoch.
    pub fn opened_at(&self) -> u64 {
        self.opened_at
    }

    /// Why the last snapshot could not be written, while no later one was.
    pub fn snapshot_failure(&self) -> Option<&str> {
        self.snapshot_failure.as_deref()
    }

    fn after_start(&mut self, start: &Start) {
        match start {
            Start::Resumed { replayed, .. } => self.since_snapshot = *replayed,
            Start::Rebuilt { .. } => self.write_snapshot(),
        }
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot();
        }
    }

    fn write_snapshot(&mut self) {
        let written = self.held.encode().and_then(|state| {
            self.log
                .write_snapshot(DOMAIN, &state, &self.key)
                .map_err(|error| error.to_string())
        });
        match written {
            Ok(_) => {
                self.since_snapshot = 0;
                self.snapshot_failure = None;
            }
            Err(reason) => self.snapshot_failure = Some(reason),
        }
    }

    /// Resolve an append whose outcome is not known, by opening the leaf
    /// store again and reading what it holds. Until that succeeds nothing is
    /// answered from memory and nothing is appended.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, held, start) = opened(&self.reopen, &self.key)?;
            self.log = log;
            self.held = held;
            self.start = start.clone();
            self.uncertain = false;
            self.after_start(&start);
        }
        Ok(())
    }

    /// Append one line as one leaf. A failed append is settled by reading
    /// back: the line is kept only if the leaf store holds exactly it.
    fn append(&mut self, line: Line) -> Result<(), ServerError> {
        self.settle()?;
        let bytes = serde_json::to_vec(&line).map_err(unavailable)?;
        let index = self.log.len();
        let Err(failure) = self.log.append(&bytes) else {
            if let Err(reason) = self.held.hold(line) {
                self.uncertain = true;
                return Err(unavailable(reason));
            }
            self.since_snapshot += 1;
            if self.since_snapshot >= SNAPSHOT_EVERY.get() {
                self.write_snapshot();
            }
            return Ok(());
        };
        self.uncertain = true;
        self.settle()?;
        match self.log.leaf_bytes(index).map_err(unavailable)? {
            Some(held) if held == bytes => Ok(()),
            Some(_) => Err(unavailable(format!(
                "leaf {index} was written by another writer: {failure}"
            ))),
            None => Err(unavailable(failure)),
        }
    }

    /// The item `id`.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.held.item(id)
    }

    pub(crate) fn pending_for_session(
        &self,
        session: &str,
    ) -> Result<Vec<crate::goals_state::PendingReminder<'_>>, ServerError> {
        self.held.pending_for_session(session).map_err(unavailable)
    }

    pub(crate) fn pending_operation(
        &self,
        operation: &str,
    ) -> Result<Option<crate::goals_state::PendingReminder<'_>>, ServerError> {
        self.held.pending_operation(operation).map_err(unavailable)
    }

    /// Every item held on `holder`, in the order set.
    pub fn of_holder(&self, holder: &Holder) -> Vec<Item> {
        self.held.of_holder(holder).cloned().collect()
    }

    /// The earliest instant any timer falls due.
    pub fn next_due(&self) -> Option<u64> {
        self.held.next_due()
    }

    /// Keep `goal` as set. Sent again in the same words it answers the item
    /// as it stands; the same operation in other words is refused.
    pub fn set(&mut self, goal: crate::goals_state::Goal) -> Result<Item, ServerError> {
        self.settle()?;
        goal.check()?;
        if self.held.changed(&goal.id).is_some() {
            return Err(GoalError::Reused { operation: goal.id }.into());
        }
        if let Some(item) = self.held.item(&goal.id) {
            let same = crate::goals_state::Goal {
                set_by: item.goal.set_by.clone(),
                at: item.goal.at,
                ..goal.clone()
            };
            if same != item.goal {
                return Err(GoalError::Reused { operation: goal.id }.into());
            }
            return Ok(item.clone());
        }
        let id = goal.id.clone();
        self.append(Line::Set(goal))?;
        self.held
            .item(&id)
            .cloned()
            .ok_or(GoalError::Unknown.into())
    }

    /// Keep `marked`, closing its item and cancelling every pending
    /// reminder. Sent again in the same words it answers the item as it
    /// stands; the same operation in other words is refused, as is a mark
    /// of an item already closed and a met deliverable claiming no evidence.
    pub fn mark(&mut self, marked: Marked) -> Result<Item, ServerError> {
        self.settle()?;
        if let Some(kept) = self.held.marked(&marked.operation) {
            let same = kept.goal == marked.goal
                && kept.standing == marked.standing
                && kept.by == marked.by
                && kept.words == marked.words
                && kept.evidence == marked.evidence;
            if !same {
                return Err(GoalError::Reused {
                    operation: marked.operation,
                }
                .into());
            }
            return self
                .held
                .item(&marked.goal)
                .cloned()
                .ok_or(GoalError::Unknown.into());
        }
        let item = self.held.item(&marked.goal).ok_or(GoalError::Unknown)?;
        if item.standing != Standing::Open {
            return Err(GoalError::Closed {
                goal: marked.goal,
                standing: item.standing.name(),
            }
            .into());
        }
        let deliverable = item.goal.kind == crate::goals_state::Kind::Deliverable;
        let claimed = marked
            .evidence
            .as_deref()
            .is_some_and(|words| !words.trim().is_empty());
        if deliverable && marked.standing == Standing::Met && !claimed {
            return Err(GoalError::EvidenceMissing {
                why: "a deliverable is marked met with the evidence claimed, in the judge's words",
            }
            .into());
        }
        let id = marked.goal.clone();
        self.append(Line::Marked(marked))?;
        self.held
            .item(&id)
            .cloned()
            .ok_or(GoalError::Unknown.into())
    }

    /// Keep an activity or words change under its operation id, retaining the original set.
    pub fn change(&mut self, changed: Changed) -> Result<Item, ServerError> {
        self.settle()?;
        changed.change.check()?;
        if let Some(kept) = self.held.changed(&changed.operation) {
            if kept.goal != changed.goal || kept.change != changed.change || kept.by != changed.by {
                return Err(GoalError::Reused {
                    operation: changed.operation,
                }
                .into());
            }
            return self
                .held
                .item(&changed.goal)
                .cloned()
                .ok_or(GoalError::Unknown.into());
        }
        if self.held.kept(&changed.operation)
            || self.held.marked(&changed.operation).is_some()
            || self.held.item(&changed.operation).is_some()
        {
            return Err(GoalError::Reused {
                operation: changed.operation,
            }
            .into());
        }
        let item = self.held.item(&changed.goal).ok_or(GoalError::Unknown)?;
        if item.standing != Standing::Open {
            return Err(GoalError::Closed {
                goal: changed.goal,
                standing: item.standing.name(),
            }
            .into());
        }
        let id = changed.goal.clone();
        self.append(Line::Changed(changed))?;
        self.held
            .item(&id)
            .cloned()
            .ok_or(GoalError::Unknown.into())
    }

    /// Keep `evented` for the open items it concerns, making each reminder
    /// waiting on it due at its instant. Kept once under its operation.
    pub fn event(&mut self, evented: Evented) -> Result<(), ServerError> {
        self.settle()?;
        if self.held.kept(&evented.operation) {
            return Ok(());
        }
        for goal in &evented.goals {
            self.held.item(goal).ok_or(GoalError::Unknown)?;
        }
        self.append(Line::Evented(evented))
    }

    /// The firing of `due` at `fired`, delivered into `sessions`, or kept
    /// refused by the name `sessions` gives.
    fn firing(
        &self,
        due: &Due,
        fired: u64,
        sessions: Result<Vec<String>, String>,
    ) -> Option<Fired> {
        let item = self.held.item(&due.goal)?;
        let operation = op_id(&[
            "reminder",
            &due.goal,
            &due.reminder.to_string(),
            &due.due.to_string(),
        ]);
        let (sent, refused) = match sessions {
            Ok(sessions) if sessions.is_empty() => (
                Vec::new(),
                Some("no_live_session: the holder has no live session to remind".to_owned()),
            ),
            Ok(sessions) => (
                sessions
                    .into_iter()
                    .map(|session| Sent {
                        operation: op_id(&["delivery", &operation, &session]),
                        session,
                        state: Delivery::Pending,
                        words: "asked of the session's runner".to_owned(),
                        at: fired,
                    })
                    .collect(),
                None,
            ),
            Err(refused) => (Vec::new(), Some(refused)),
        };
        Some(Fired {
            text: text(item, fired),
            operation,
            goal: due.goal.clone(),
            reminder: due.reminder,
            due: due.due,
            fired,
            late: due.due < self.opened_at,
            sent,
            refused,
        })
    }

    /// Keep a runner's answer to a delivery, when it changes what is kept.
    pub fn answer(&mut self, answered: Answered) -> Result<(), ServerError> {
        self.settle()?;
        let changed = self
            .held
            .sent(&answered.operation)
            .is_some_and(|sent| sent.state.unsettled() && sent.state != answered.state);
        if changed {
            self.append(Line::Answered(answered))?;
        }
        Ok(())
    }
}

/// An operation id made from `parts`, the same each time.
fn op_id(parts: &[&str]) -> String {
    let digest = Sha256::digest(format!("lys-identity/goals/v1\n{}", parts.join("\n")));
    format!("op-{}", &hex(&digest)[..32])
}

/// The reminder's text: the item's words and the time left at `at`.
fn text(item: &Item, at: u64) -> String {
    text_with_words(item, item.words(), at)
}

fn span(seconds: u64) -> String {
    let (hours, minutes) = (seconds / 3600, seconds % 3600 / 60);
    match (hours, minutes) {
        (0, 0) => format!("{seconds} seconds"),
        (0, _) => format!("{minutes} minutes"),
        _ => format!("{hours} hours {minutes} minutes"),
    }
}

pub use crate::runner_operate::Undelivered;

/// A runner's answer to one delivery, or why there is none.
pub type Delivering<'a> =
    Pin<Box<dyn Future<Output = Result<OperationOutcome, Undelivered>> + Send + 'a>>;

/// How reminders reach the holder's live sessions: through each session's
/// runner, under the runner's operation ids.
pub trait Deliver: Send + Sync {
    /// The live sessions of `holder`, or why none can be reached, by name.
    fn sessions(&self, holder: &Holder) -> Result<Vec<String>, String>;
    /// Ask the session's runner for `operation`, answering how it stands.
    fn operate(&self, operation: Operation) -> Delivering<'_>;
}

/// The goals and the signal their timers wait on.
pub struct Goals {
    /// The store, one caller at a time.
    pub store: Mutex<GoalStore>,
    /// Signalled each time something a timer depends on is kept.
    pub changed: Arc<Notify>,
}

impl Goals {
    /// `store`, with its signal.
    pub fn new(store: GoalStore) -> Self {
        Self {
            store: Mutex::new(store),
            changed: Arc::new(Notify::new()),
        }
    }

    /// Run `act` on the store once it is settled.
    pub fn with<T>(
        &self,
        act: impl FnOnce(&mut GoalStore) -> Result<T, ServerError>,
    ) -> Result<T, ServerError> {
        let mut store = self.store.lock().map_err(|error| GoalError::Unavailable {
            reason: format!("the goals lock is poisoned: {error}"),
        })?;
        store.settle()?;
        act(&mut store)
    }
}

/// One pass at `at`: every delivery still unsettled is asked again under its
/// own operation id, and every reminder due fires once.
pub async fn remind(goals: &Goals, deliver: &dyn Deliver, at: u64) -> Result<(), ServerError> {
    let asks = goals.with(|store| {
        let mut asks = Vec::new();
        let pending = store.held.pending().map_err(unavailable)?.into_iter()
            .map(|pending| (pending.sent.clone(), pending.item.goal.holder.clone(),
                pending.active && pending.item.standing == Standing::Open,
                text_with_words(pending.item, pending.words, at)))
            .collect::<Vec<_>>();
        for (sent, holder, permitted, text) in pending {
            if !permitted {
                store.answer(Answered { operation: sent.operation, state: Delivery::Refused,
                    words: "goal_closed_or_inactive: queued words are no longer authorised".to_owned(), at })?;
                continue;
            }
            if holder.kind == crate::goals_state::HolderKind::Team {
                let sessions = deliver.sessions(&holder).map_err(unavailable)?;
                if !sessions.contains(&sent.session) {
                    store.answer(Answered { operation: sent.operation, state: Delivery::Refused,
                        words: format!("team_membership_held: session `{}` is not a current admitted recipient of team `{}`", sent.session, holder.id), at })?;
                    continue;
                }
            }
            asks.push((sent, text));
        }
        for due in store.held.due(at) {
            let holder = match store.held.item(&due.goal) {
                Some(item) => item.goal.holder.clone(),
                None => continue,
            };
            let Some(fired) = store.firing(&due, at, deliver.sessions(&holder)) else {
                continue;
            };
            asks.extend(
                fired
                    .sent
                    .iter()
                    .map(|sent| (sent.clone(), fired.text.clone())),
            );
            store.append(Line::Fired(fired))?;
        }
        Ok(asks)
    })?;
    for (sent, text) in asks {
        let operation = Operation {
            operation: sent.operation.clone(),
            session: sent.session,
            request: OperationRequest::Reminder { text },
        };
        let answered = match deliver.operate(operation).await {
            Ok(outcome) => Answered::from_runner(&outcome, now()),
            Err(Undelivered::Refused(words)) => Answered {
                operation: sent.operation,
                state: Delivery::Refused,
                words,
                at: now(),
            },
            Err(Undelivered::Unknown(_)) => continue,
        };
        goals.with(|store| store.answer(answered))?;
    }
    Ok(())
}

/// Open the log from its snapshot, or from every leaf when the snapshot or
/// its state is refused, and fold what the start hands back.
fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, ServerError> {
    let store = reopen().map_err(unavailable)?;
    let started =
        open_with_snapshot(store, DOMAIN, &key.public_key_bytes()).map_err(unavailable)?;
    let mut held = match started.state.as_deref().map(Held::decode) {
        None => Held::default(),
        Some(Ok(held)) => held,
        Some(Err(reason)) => return rebuilt(reopen, reason),
    };
    held.fold(&started.tail).map_err(unavailable)?;
    Ok((started.log, held, started.start))
}

/// Open the log from every leaf, because the snapshot's state was refused
/// for `reason`.
fn rebuilt<S: LeafStore>(reopen: &Reopen<S>, reason: String) -> Result<Opened<S>, ServerError> {
    let (log, tail) = FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
    let mut held = Held::default();
    held.fold(&tail).map_err(unavailable)?;
    let replayed = log.len();
    let start = Start::Rebuilt {
        refusal: SnapshotRefusal::StateUnreadable { reason },
        replayed,
    };
    Ok((log, held, start))
}

#[cfg(test)]
#[path = "goals_store_poison_tests.rs"]
mod poison_tests;
