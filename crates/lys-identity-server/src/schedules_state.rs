//! The schedules (AGENTS-001 R4): a message sent at an instant, once or at
//! an interval anchored to it, to agents or sessions, folded from the
//! schedules log's leaves. Each occurrence is kept before any runner is
//! asked, naming each recipient's delivery under its own operation id, so
//! nothing fires twice for one occurrence and a restart keeps every
//! schedule where it stood. Intervals missed while Lys was down coalesce
//! to one send; an uncertain delivery stops the schedule and says so.

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::agents_log::Folded;
use crate::words_state::{Contributed, Slot};

/// The shortest interval, in seconds.
pub const INTERVAL_MIN: u64 = 60;
/// The longest interval, in seconds: 366 days.
pub const INTERVAL_MAX: u64 = 366 * 24 * 3600;

/// Everything the schedules refuse, each by name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SchedulesError {
    /// The schedules are not configured, or their log could not be read or written.
    #[error("schedules_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// No schedule by that id is visible to the caller.
    #[error("schedule_unknown: no schedule by that id is visible to the caller")]
    Unknown,
    /// The operation id already names a schedule in other words.
    #[error("schedule_reused: operation `{operation}` already names a schedule in other words")]
    Reused {
        /// The operation id.
        operation: String,
    },
    /// The schedule has stopped and takes no other change.
    #[error("schedule_stopped: schedule `{schedule}` stopped: {reason}")]
    Stopped {
        /// The schedule.
        schedule: String,
        /// Why it stopped.
        reason: String,
    },
    /// A schedule that cannot be kept as asked.
    #[error("schedule_malformed: {reason}")]
    Malformed {
        /// Why.
        reason: String,
    },
}

impl SchedulesError {
    /// How the refusal is answered over HTTP.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Unknown => StatusCode::NOT_FOUND,
            Self::Reused { .. } | Self::Stopped { .. } => StatusCode::CONFLICT,
            Self::Malformed { .. } => StatusCode::BAD_REQUEST,
        }
    }

    /// The stable refusal name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "schedules_unavailable",
            Self::Unknown => "schedule_unknown",
            Self::Reused { .. } => "schedule_reused",
            Self::Stopped { .. } => "schedule_stopped",
            Self::Malformed { .. } => "schedule_malformed",
        }
    }
}

/// Who a schedule sends to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleRecipient)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Recipient {
    /// An agent: every live session of it.
    Agent {
        /// The agent.
        id: String,
    },
    /// One session.
    Session {
        /// The session.
        id: String,
    },
}

/// What a schedule sends: its own text, or a slot's words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleSource)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Source {
    /// Its own text, with placeholders rendered at delivery.
    Text {
        /// The text.
        text: String,
    },
    /// A slot's words, resolved for the recipient at delivery.
    Slot {
        /// The slot.
        slot: Slot,
    },
}

/// A schedule as set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleSet)]
#[serde(deny_unknown_fields)]
pub struct Schedule {
    /// The operation id it was set under, which names it.
    pub id: String,
    /// The first instant, in seconds since the Unix epoch.
    pub at: u64,
    /// The instant before which every occurrence falls, exclusive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<u64>,
    /// Seconds between occurrences, anchored to `at`; none for once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval: Option<u64>,
    /// The most occurrences; none for no limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_occurrences: Option<u64>,
    /// Who it sends to.
    pub recipients: Vec<Recipient>,
    /// What it sends.
    pub source: Source,
    /// Who set it.
    pub author: String,
    /// When it was set, in seconds since the Unix epoch.
    pub set_at: u64,
}

impl Schedule {
    /// Refuse a schedule that cannot be kept as asked.
    pub fn check(&self, now: u64) -> Result<(), SchedulesError> {
        let refuse = |reason: String| Err(SchedulesError::Malformed { reason });
        if self.at <= now {
            return refuse("at has passed: a schedule starts at an instant after now".to_owned());
        }
        if let Some(until) = self.until
            && until <= self.at
        {
            return refuse("until is at or before at".to_owned());
        }
        if let Some(interval) = self.interval
            && !(INTERVAL_MIN..=INTERVAL_MAX).contains(&interval)
        {
            return refuse(format!(
                "interval is {interval} seconds; it is at least {INTERVAL_MIN} and at most {INTERVAL_MAX}"
            ));
        }
        if self.max_occurrences == Some(0) {
            return refuse("max_occurrences is 0: a schedule sends at least once".to_owned());
        }
        if self.recipients.is_empty() {
            return refuse("recipients is empty".to_owned());
        }
        if let Source::Text { text } = &self.source {
            crate::words_state::checked_text(text).map_err(|refused| SchedulesError::Malformed {
                reason: refused.to_string(),
            })?;
        }
        Ok(())
    }
}

/// Where one recipient's delivery of an occurrence stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleDelivery)]
#[serde(rename_all = "snake_case")]
pub enum Delivery {
    /// Asked, with no answer kept yet: asked again under the same id.
    Pending,
    /// Accepted by the runner, waiting for the session's turn boundary.
    Accepted,
    /// Delivered into the session.
    Delivered,
    /// Whether it reached the session cannot be known; never sent again.
    Uncertain,
    /// Not delivered, by name; a temporary refusal counts as attempted.
    Refused,
}

impl Delivery {
    /// Whether it is still asked of the runner.
    pub fn unsettled(self) -> bool {
        matches!(self, Self::Pending | Self::Accepted)
    }
}

/// One session's delivery of an occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleSent)]
#[serde(deny_unknown_fields)]
pub struct Sent {
    /// The recipient it is for.
    pub recipient: Recipient,
    /// The session.
    pub session: String,
    /// The runner operation id it is asked under.
    pub operation: String,
    /// The text rendered for it, at the moment the occurrence was kept.
    pub text: String,
    /// Every revision that contributed to the text.
    pub contributed: Vec<Contributed>,
    /// Each placeholder that rendered empty for want of a key.
    pub missing: Vec<String>,
    /// Where it stands.
    pub state: Delivery,
    /// The runner's words, or why it stands as it does.
    pub words: String,
    /// When it came to stand there, in seconds since the Unix epoch.
    pub at: u64,
}

/// An occurrence fired.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleFired)]
#[serde(deny_unknown_fields)]
pub struct Fired {
    /// The operation id naming this occurrence.
    pub operation: String,
    /// The schedule.
    pub schedule: String,
    /// The occurrence, counting from 1.
    pub occurrence: u64,
    /// When it fell due, in seconds since the Unix epoch.
    pub due: u64,
    /// When it fired.
    pub fired: u64,
    /// How many due instants it stands for: 1, or more when intervals
    /// missed while Lys was down coalesced into it.
    pub coalesced: u64,
    /// Each session it is delivered into.
    pub sent: Vec<Sent>,
    /// Why no session was asked, by name, when none was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

/// A runner's answer to one delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleAnswered)]
#[serde(deny_unknown_fields)]
pub struct Answered {
    /// The runner operation id.
    pub operation: String,
    /// Where the delivery stands.
    pub state: Delivery,
    /// The runner's words.
    pub words: String,
    /// When it was kept, in seconds since the Unix epoch.
    pub at: u64,
}

/// A change to a schedule kept apart from how it was set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleChange)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    /// Pause or resume without losing the count.
    Paused {
        /// Whether it is paused.
        paused: bool,
    },
    /// New recipients.
    Recipients {
        /// The recipients.
        recipients: Vec<Recipient>,
    },
    /// A new source.
    Source {
        /// The source.
        source: Source,
    },
    /// A new end.
    Until {
        /// The instant, or none for no end.
        until: Option<u64>,
    },
    /// A new limit.
    MaxOccurrences {
        /// The most occurrences, or none for no limit.
        max_occurrences: Option<u64>,
    },
}

/// One kept change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleChanged)]
#[serde(deny_unknown_fields)]
pub struct Changed {
    /// The operation id naming the change.
    pub operation: String,
    /// The schedule.
    pub schedule: String,
    /// The change.
    pub change: Change,
    /// Who changed it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// A schedule stopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleStopped)]
#[serde(deny_unknown_fields)]
pub struct Stopped {
    /// The schedule.
    pub schedule: String,
    /// Why: `finished`, `until_reached`, `uncertain_delivery`,
    /// `terminal_failure` or `stopped_by` and who.
    pub reason: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// One leaf of the schedules log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum Line {
    /// A schedule set.
    Set(Schedule),
    /// A schedule changed.
    Changed(Changed),
    /// An occurrence fired.
    Fired(Fired),
    /// A runner's answer to a delivery.
    Answered(Answered),
    /// A schedule stopped.
    Stopped(Stopped),
}

/// A schedule as its log folds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = ScheduleItem)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// The schedule as set.
    pub schedule: Schedule,
    /// Changes kept after the set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<Changed>,
    /// Every occurrence fired, in order.
    pub fired: Vec<Fired>,
    /// Why and when it stopped, once it has.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped: Option<Stopped>,
    /// The next due instant, in seconds since the Unix epoch; null when
    /// paused, stopped, or fired for the last time.
    pub next_due: Option<u64>,
}

impl Item {
    /// Whether reminders are paused.
    pub fn paused(&self) -> bool {
        self.changes
            .iter()
            .rev()
            .find_map(|changed| match changed.change {
                Change::Paused { paused } => Some(paused),
                _ => None,
            })
            .unwrap_or(false)
    }

    /// The current recipients.
    pub fn recipients(&self) -> &[Recipient] {
        self.changes
            .iter()
            .rev()
            .find_map(|changed| match &changed.change {
                Change::Recipients { recipients } => Some(recipients.as_slice()),
                _ => None,
            })
            .unwrap_or(&self.schedule.recipients)
    }

    /// The current source.
    pub fn source(&self) -> &Source {
        self.changes
            .iter()
            .rev()
            .find_map(|changed| match &changed.change {
                Change::Source { source } => Some(source),
                _ => None,
            })
            .unwrap_or(&self.schedule.source)
    }

    /// The current end.
    pub fn until(&self) -> Option<u64> {
        self.changes
            .iter()
            .rev()
            .find_map(|changed| match changed.change {
                Change::Until { until } => Some(until),
                _ => None,
            })
            .unwrap_or(self.schedule.until)
    }

    /// The current limit.
    pub fn max_occurrences(&self) -> Option<u64> {
        self.changes
            .iter()
            .rev()
            .find_map(|changed| match changed.change {
                Change::MaxOccurrences { max_occurrences } => Some(max_occurrences),
                _ => None,
            })
            .unwrap_or(self.schedule.max_occurrences)
    }

    /// The due instants the schedule stands for: the first, then every
    /// interval after it, under `until` and the limit.
    fn due_after(&self, last: Option<u64>) -> Option<u64> {
        let schedule = &self.schedule;
        let fired = u64::try_from(self.fired.len()).unwrap_or(u64::MAX);
        if self.max_occurrences().is_some_and(|max| fired >= max) {
            return None;
        }
        let next = match (last, schedule.interval) {
            (None, _) => schedule.at,
            (Some(_), None) => return None,
            (Some(last), Some(interval)) => {
                let steps = last.saturating_sub(schedule.at) / interval + 1;
                schedule.at.saturating_add(steps.saturating_mul(interval))
            }
        };
        Some(next).filter(|next| self.until().is_none_or(|until| *next < until))
    }

    fn recompute(&mut self) {
        self.next_due = if self.stopped.is_some() || self.paused() {
            None
        } else {
            self.due_after(self.fired.last().map(|fired| fired.due))
        };
    }

    /// Whether every delivery of every occurrence is settled.
    pub fn settled(&self) -> bool {
        self.fired
            .iter()
            .all(|fired| fired.sent.iter().all(|sent| !sent.state.unsettled()))
    }
}

/// The schedules as their log folds them, in the order set.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Schedules {
    /// The schedules.
    pub items: Vec<Item>,
}

impl Folded for Schedules {
    type Line = Line;
    const DOMAIN: &'static str = "lys/identity/schedules-state/v1";
    const FORMAT: &'static str = "lys-schedules-state/v1";
    const KIND: &'static str = "schedules";

    fn hold(&mut self, line: Line) -> Result<(), String> {
        match line {
            Line::Set(schedule) => {
                if self.item(&schedule.id).is_some() {
                    return Err(format!("operation `{}` already names a schedule", schedule.id));
                }
                let mut item = Item {
                    schedule,
                    changes: Vec::new(),
                    fired: Vec::new(),
                    stopped: None,
                    next_due: None,
                };
                item.recompute();
                self.items.push(item);
            }
            Line::Changed(changed) => {
                let item = self.item_mut(&changed.schedule)?;
                item.changes.push(changed);
                item.recompute();
            }
            Line::Fired(fired) => {
                if self.firing(&fired.operation).is_some() {
                    return Err(format!(
                        "operation `{}` already names an occurrence",
                        fired.operation
                    ));
                }
                let item = self.item_mut(&fired.schedule)?;
                item.fired.push(fired);
                item.recompute();
            }
            Line::Answered(answered) => {
                let sent = self
                    .sent_mut(&answered.operation)
                    .ok_or_else(|| format!("no delivery `{}` is held", answered.operation))?;
                sent.state = answered.state;
                sent.words = answered.words;
                sent.at = answered.at;
            }
            Line::Stopped(stopped) => {
                let item = self.item_mut(&stopped.schedule)?;
                item.stopped = Some(stopped);
                item.recompute();
            }
        }
        Ok(())
    }
}

impl Schedules {
    /// The schedule `id`.
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|item| item.schedule.id == id)
    }

    fn item_mut(&mut self, id: &str) -> Result<&mut Item, String> {
        self.items
            .iter_mut()
            .find(|item| item.schedule.id == id)
            .ok_or_else(|| format!("no schedule `{id}` is held"))
    }

    /// The occurrence asked under `operation`.
    pub fn firing(&self, operation: &str) -> Option<&Fired> {
        self.items
            .iter()
            .flat_map(|item| item.fired.iter())
            .find(|fired| fired.operation == operation)
    }

    /// The delivery asked under `operation`, and its schedule.
    pub fn sent(&self, operation: &str) -> Option<(&Item, &Sent)> {
        self.items.iter().find_map(|item| {
            item.fired
                .iter()
                .flat_map(|fired| fired.sent.iter())
                .find(|sent| sent.operation == operation)
                .map(|sent| (item, sent))
        })
    }

    fn sent_mut(&mut self, operation: &str) -> Option<&mut Sent> {
        self.items
            .iter_mut()
            .flat_map(|item| item.fired.iter_mut())
            .flat_map(|fired| fired.sent.iter_mut())
            .find(|sent| sent.operation == operation)
    }

    /// Every schedule due at `now`, with the due instant and how many due
    /// instants coalesce into it.
    pub fn due(&self, now: u64) -> Vec<(String, u64, u64)> {
        self.items
            .iter()
            .filter_map(|item| {
                let first = item.next_due.filter(|due| *due <= now)?;
                let coalesced = match item.schedule.interval {
                    Some(interval) if interval > 0 => (now - first) / interval + 1,
                    _ => 1,
                };
                let last = first.saturating_add((coalesced - 1).saturating_mul(
                    item.schedule.interval.unwrap_or(0),
                ));
                let last = match item.until() {
                    Some(until) if last >= until => first,
                    _ => last,
                };
                Some((item.schedule.id.clone(), last, coalesced))
            })
            .collect()
    }

    /// The earliest instant any schedule falls due.
    pub fn next_due(&self) -> Option<u64> {
        self.items.iter().filter_map(|item| item.next_due).min()
    }

    /// Every delivery still asked of a runner.
    pub fn pending(&self) -> Vec<(String, Sent)> {
        self.items
            .iter()
            .flat_map(|item| {
                item.fired.iter().flat_map(move |fired| {
                    fired
                        .sent
                        .iter()
                        .filter(|sent| sent.state.unsettled())
                        .map(move |sent| (item.schedule.id.clone(), sent.clone()))
                })
            })
            .collect()
    }
}
