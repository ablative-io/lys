//! The shape of each destination a seat import writes (AGENTS-003 R4, R5),
//! read from a plan entry's `change` and checked before the plan binds it.
//! Every shape refuses a member it does not name; an agent id named in a
//! change is the seat's own agent, or the plan is refused.

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::budgets_limits::Limit;
use crate::budgets_state::{Holder, HolderKind};
use crate::error::ServerError;
use crate::error_seat_import::SeatImportError;
use crate::provisioning_store::Settings;
use crate::schedules_state::{
    Change as Altered, Changed, Item, Line as ScheduleLine, Recipient, Schedule,
};
use crate::schedules_store::SchedulesKept;
use crate::seat_import_plan::{BOUNDS, DestinationEntry, Refusal, record};
use crate::session::now;
use crate::variables_state::Scope;
use crate::words_state::{Layer, Setting, Slot};

/// A `profile_version` change: the next version of the agent's profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileChange {
    /// What the version sets.
    pub settings: Settings,
}

/// A `words_template` change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateChange {
    /// The template's name.
    pub name: String,
    /// Its text.
    pub text: String,
}

/// A `words_slot` change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotChange {
    /// The layer.
    pub layer: Layer,
    /// The slot.
    pub slot: Slot,
    /// What the layer says.
    pub setting: Setting,
}

/// A `variable` change: one variable of one scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariableChange {
    /// The Lys scope.
    pub scope: Scope,
    /// The variable.
    pub name: String,
    /// Its value.
    pub value: Value,
    /// Who set it at the source.
    pub author: String,
    /// When it expires, in seconds since the Unix epoch.
    #[serde(default)]
    pub expires_at: Option<u64>,
    /// The canonical scope it was read at, as provenance.
    #[serde(default)]
    pub source_scope: Option<String>,
    /// The seat it was read for, as provenance.
    #[serde(default)]
    pub seat: Option<String>,
}

/// A `budget_limits` change: every limit of one holder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitsChange {
    /// The holder.
    pub holder: Holder,
    /// Its limits.
    pub limits: Vec<Limit>,
    /// The warning percentage.
    pub warn_at: Option<Number>,
    /// A context policy, which no Lys owner holds yet: refused unless null.
    #[serde(default)]
    pub context_policy: Option<Value>,
    /// Where each level came from, as provenance.
    #[serde(default)]
    pub sources: Option<Value>,
}

/// A `schedule` change, kept paused with every source state it carries.
/// The schedule keeps its monitor's original `at`, which is never checked
/// against now; its spent occurrences are named by `occurrences_completed`
/// and `next_due`, and its delivery provenance by `spent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleChange {
    /// The schedule, its recipient the importing seat's agent.
    pub schedule: Schedule,
    /// `active` or `stopped` at the source.
    pub state: String,
    /// Why it was stopped at the source: `paused`, `failed` or `uncertain`.
    #[serde(default)]
    pub stopped_reason: Option<String>,
    /// What resuming a stopped schedule needs.
    #[serde(default)]
    pub resume: Option<String>,
    /// The next instant due at the source, in seconds since the Unix epoch.
    #[serde(default)]
    pub next_due: Option<u64>,
    /// The occurrences already spent at the source, never sent again.
    pub occurrences_completed: u64,
    /// The source template, as provenance.
    #[serde(default)]
    pub template: Option<Value>,
    /// The last delivery's provenance.
    #[serde(default)]
    pub spent: Option<Value>,
    /// Each seat's recipient binding.
    pub bindings: Vec<ScheduleBinding>,
    /// The source definition's own revision.
    #[serde(default)]
    pub source_revision: Option<Value>,
}

/// One seat's binding to a shared schedule, confirmed by that seat's own
/// import and enabled only after its move proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduleBinding {
    /// The legacy session it was sent to.
    pub source_session: String,
    /// The Lys seat it maps to.
    pub seat: String,
    /// Whether this import is that seat's.
    pub this_import: bool,
    /// Whether it delivers: an import enables none.
    pub enabled: bool,
    /// The last delivery to it, as provenance.
    #[serde(default)]
    pub last_sent: Option<Value>,
}

/// A destination's change, read in its kind's shape.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Change {
    /// A profile version.
    Profile(Box<ProfileChange>),
    /// A words template.
    Template(TemplateChange),
    /// A words slot.
    Slot(SlotChange),
    /// One variable.
    Variable(VariableChange),
    /// A holder's limits.
    Limits(LimitsChange),
    /// A schedule.
    Schedule(Box<ScheduleChange>),
    /// A rule kept in the receipt only.
    Transfer,
}

/// A destination as refusals and receipts name it.
pub(crate) fn member(destination: &DestinationEntry) -> String {
    format!("{} {}", destination.record_kind, destination.record_id)
}

fn shaped<T: serde::de::DeserializeOwned>(
    destination: &DestinationEntry,
) -> Result<T, SeatImportError> {
    serde_json::from_value(destination.change.clone()).map_err(|error| {
        SeatImportError::DestinationMalformed {
            record: member(destination),
            reason: error.to_string(),
        }
    })
}

/// `destination`'s change in its kind's shape, refused by name otherwise.
pub(crate) fn parsed(destination: &DestinationEntry) -> Result<Change, SeatImportError> {
    Ok(match destination.record_kind.as_str() {
        record::PROFILE => Change::Profile(Box::new(shaped(destination)?)),
        record::WORDS_TEMPLATE => Change::Template(shaped(destination)?),
        record::WORDS_SLOT => Change::Slot(shaped(destination)?),
        record::VARIABLE => Change::Variable(shaped(destination)?),
        record::BUDGET_LIMITS => Change::Limits(shaped(destination)?),
        record::SCHEDULE => Change::Schedule(Box::new(shaped(destination)?)),
        record::RULE_TRANSFER if destination.change.is_object() => Change::Transfer,
        record::RULE_TRANSFER => {
            return Err(SeatImportError::DestinationMalformed {
                record: member(destination),
                reason: "a rule transfer is an object".to_owned(),
            });
        }
        kind => {
            return Err(SeatImportError::DestinationUnsupported {
                record: member(destination),
                kind: kind.to_owned(),
            });
        }
    })
}

/// Refuse a destination whose change does not read as its kind, names an
/// agent other than the seat's own `agent`, enables a delivery or exceeds
/// a plan bound.
pub fn checked(destination: &DestinationEntry, agent: &str) -> Result<(), Refusal> {
    let at = member(destination);
    let refusal = |name: &str, detail: String| Refusal {
        name: name.to_owned(),
        member: at.clone(),
        detail,
    };
    let change = parsed(destination).map_err(|error| refusal(error.name(), error.to_string()))?;
    let foreign = |id: &str| {
        refusal(
            "import_destination_foreign",
            format!("names agent {id}, not the seat's agent {agent}"),
        )
    };
    match change {
        Change::Slot(SlotChange {
            layer: Layer::Agent { id },
            ..
        })
        | Change::Variable(VariableChange {
            scope: Scope::Agent { id },
            ..
        }) if id != agent => Err(foreign(&id)),
        Change::Variable(VariableChange {
            value: Value::Null, ..
        }) => Err(refusal(
            "import_destination_malformed",
            "a null value removes a variable; an import sets values".to_owned(),
        )),
        Change::Limits(limits)
            if limits.holder.kind == HolderKind::Agent && limits.holder.id != agent =>
        {
            Err(foreign(&limits.holder.id))
        }
        Change::Limits(LimitsChange {
            context_policy: Some(policy),
            ..
        }) if !policy.is_null() => Err(refusal(
            "import_member_unsupported",
            "context_policy has no Lys owner yet".to_owned(),
        )),
        Change::Schedule(change) => schedule_checked(&change, agent, &refusal, &foreign),
        Change::Profile(_)
        | Change::Template(_)
        | Change::Slot(_)
        | Change::Variable(_)
        | Change::Limits(_)
        | Change::Transfer => Ok(()),
    }
}

/// Refuse a schedule that enables a binding, names too many recipients or
/// sends to another agent.
fn schedule_checked(
    change: &ScheduleChange,
    agent: &str,
    refusal: &dyn Fn(&str, String) -> Refusal,
    foreign: &dyn Fn(&str) -> Refusal,
) -> Result<(), Refusal> {
    if change.bindings.iter().any(|binding| binding.enabled) {
        return Err(refusal(
            "import_destination_malformed",
            "a binding is enabled; an import enables no delivery".to_owned(),
        ));
    }
    let recipients = &change.schedule.recipients;
    if u64::try_from(recipients.len()).unwrap_or(u64::MAX) > BOUNDS.recipients_per_schedule {
        let detail = format!(
            "{} recipients; at most {}",
            recipients.len(),
            BOUNDS.recipients_per_schedule
        );
        return Err(refusal("import_bound_exceeded", detail));
    }
    let other = recipients.iter().find_map(|recipient| match recipient {
        Recipient::Agent { id } if id != agent => Some(id.as_str()),
        Recipient::Agent { .. } | Recipient::Session { .. } => None,
    });
    match other {
        Some(id) => Err(foreign(id)),
        None => Ok(()),
    }
}

/// The owner revision a destination is bound to: destinations sharing one
/// key share one revision counter, and are bound one after another.
pub fn revision_key(destination: &DestinationEntry) -> String {
    match parsed(destination) {
        Ok(Change::Profile(_)) => "profile".to_owned(),
        Ok(Change::Template(change)) => format!("template:{}", change.name),
        Ok(Change::Slot(change)) => {
            format!("slot:{}/{}", change.layer.key(), change.slot.name())
        }
        Ok(Change::Variable(change)) => format!("variables:{}", change.scope.key()),
        Ok(Change::Limits(change)) => {
            format!("limits:{:?}:{}", change.holder.kind, change.holder.id)
        }
        Ok(Change::Schedule(change)) => format!("schedule:{}", change.schedule.id),
        Ok(Change::Transfer) | Err(_) => format!("record:{}", member(destination)),
    }
}

/// `setting` as the words store keeps it: text trimmed.
pub(crate) fn stored(setting: &Setting) -> Setting {
    match setting {
        Setting::Text { text } => Setting::Text {
            text: text.trim().to_owned(),
        },
        other => other.clone(),
    }
}

/// What reading a step back found.
pub(crate) enum Readback {
    /// This step's write, at the revision it left.
    Ours(u64),
    /// Part of this step's write: the schedule set and not yet paused.
    Partial,
    /// Nothing of this step.
    Absent,
}

fn pause_operation(step: &str) -> String {
    format!("{step}.pause")
}

/// A schedule's revision: 0 while it is not held, then one for its set and
/// one for each change.
pub(crate) fn schedule_revision(item: Option<&Item>) -> u64 {
    item.map_or(0, |item| {
        1 + u64::try_from(item.changes.len()).unwrap_or(u64::MAX - 1)
    })
}

/// What a step reads back as, from whether its write is held.
pub(crate) fn ours(held: bool, revision: u64) -> Readback {
    if held {
        Readback::Ours(revision)
    } else {
        Readback::Absent
    }
}

/// What a schedule's owner holds of a step: the same definition bound to
/// `agent` and paused is this step's; one set and not yet paused, with no
/// change, is part of it.
pub(crate) fn scheduled(
    item: Option<&Item>,
    change: &ScheduleChange,
    agent: &str,
    expected: u64,
) -> Readback {
    let Some(item) = item else {
        return Readback::Absent;
    };
    let recipients = item.recipients().to_vec();
    let asked = Schedule {
        set_at: item.schedule.set_at,
        recipients: recipients.clone(),
        ..change.schedule.clone()
    };
    let same = asked
        == Schedule {
            recipients,
            ..item.schedule.clone()
        };
    let bound = item.recipients().contains(&Recipient::Agent {
        id: agent.to_owned(),
    });
    match (same && bound, item.paused()) {
        (true, true) => Readback::Ours(schedule_revision(Some(item))),
        (true, false) if expected == 0 && item.changes.is_empty() => Readback::Partial,
        _ => Readback::Absent,
    }
}

/// Keep `asked` paused, under one lock: set with its pause when it is not
/// held, paused when `partial`, or, when another seat's import holds it
/// paused, given `agent` as one more recipient. Its start is the source's
/// own and is never checked against now.
pub(crate) fn schedule(
    schedules: &SchedulesKept,
    step: &str,
    asked: Schedule,
    (agent, by): (&str, &str),
    partial: bool,
) -> Result<u64, ServerError> {
    let id = asked.id.clone();
    let changed = |operation: String, change: Altered| {
        ScheduleLine::Changed(Changed {
            operation,
            schedule: id.clone(),
            change,
            by: by.to_owned(),
            at: now(),
        })
    };
    schedules.kept.with(|log, unavailable| {
        let pause = changed(pause_operation(step), Altered::Paused { paused: true });
        match log.held().item(&id).cloned() {
            None => {
                log.append(ScheduleLine::Set(asked), unavailable)?;
                log.append(pause, unavailable)?;
            }
            Some(_) if partial => log.append(pause, unavailable)?,
            Some(item) if item.paused() => {
                let mut recipients = item.recipients().to_vec();
                recipients.push(Recipient::Agent {
                    id: agent.to_owned(),
                });
                let bind = changed(format!("{step}.bind"), Altered::Recipients { recipients });
                log.append(bind, unavailable)?;
            }
            Some(_) => {
                return Err(SeatImportError::ScheduleActive {
                    record: format!("{} {id}", record::SCHEDULE),
                }
                .into());
            }
        }
        Ok(schedule_revision(log.held().item(&id)))
    })
}
