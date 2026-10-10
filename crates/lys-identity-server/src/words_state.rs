//! The words a seat is sent (AGENTS-001 R1): five message slots at four
//! layers, each layer its own text, a link to a named template, or empty
//! meaning inherit, folded from the words log's leaves. A slot resolves most
//! specific first: session, then agent, then workspace, then the built-in
//! wording in code. Every save carries the revision it read and is refused
//! by name on a stale one, so nobody overwrites words blind.

use std::collections::BTreeMap;

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::agents_log::Folded;

/// Everything the words refuse, each by name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WordsError {
    /// The words are not configured, or their log could not be read or written.
    #[error("words_unavailable: {reason}")]
    Unavailable {
        /// Why.
        reason: String,
    },
    /// The save carried a revision that is not the one held.
    #[error(
        "words_stale: `{what}` is at revision {held}, not {given}: read it again and save from what you read"
    )]
    Stale {
        /// The slot and layer, or the template, saved.
        what: String,
        /// The revision held.
        held: u64,
        /// The revision the save carried.
        given: u64,
    },
    /// A link names a template the words do not hold.
    #[error("words_template_unknown: no template is named `{name}`")]
    TemplateUnknown {
        /// The name.
        name: String,
    },
    /// A slot, layer or template name the words do not know.
    #[error("words_malformed: {reason}")]
    Malformed {
        /// Why.
        reason: String,
    },
}

impl WordsError {
    /// How the refusal is answered over HTTP.
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Self::Unavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::Stale { .. } => StatusCode::CONFLICT,
            Self::TemplateUnknown { .. } => StatusCode::NOT_FOUND,
            Self::Malformed { .. } => StatusCode::BAD_REQUEST,
        }
    }

    /// The stable refusal name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unavailable { .. } => "words_unavailable",
            Self::Stale { .. } => "words_stale",
            Self::TemplateUnknown { .. } => "words_template_unknown",
            Self::Malformed { .. } => "words_malformed",
        }
    }
}

/// The five message slots.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[schema(as = WordsSlot)]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    /// The context warning, typed when a session's context budget warns.
    ContextWarning,
    /// The preparation, typed before a compaction is asked.
    Preparation,
    /// The compaction command itself.
    Compaction,
    /// The wake-up, typed when an agent is woken with a message.
    WakeUp,
    /// A scheduled reminder's words, when its schedule names the slot.
    ScheduledReminder,
}

impl Slot {
    /// Every slot, in the order the screen shows them.
    pub const ALL: [Self; 5] = [
        Self::ContextWarning,
        Self::Preparation,
        Self::Compaction,
        Self::WakeUp,
        Self::ScheduledReminder,
    ];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::ContextWarning => "context_warning",
            Self::Preparation => "preparation",
            Self::Compaction => "compaction",
            Self::WakeUp => "wake_up",
            Self::ScheduledReminder => "scheduled_reminder",
        }
    }

    /// The slot named `name`.
    pub fn named(name: &str) -> Result<Self, WordsError> {
        Self::ALL
            .into_iter()
            .find(|slot| slot.name() == name)
            .ok_or_else(|| WordsError::Malformed {
                reason: format!(
                    "`{name}` is not a slot: context_warning, preparation, compaction, wake_up or scheduled_reminder"
                ),
            })
    }

    /// The built-in wording in code, the last layer a slot resolves to;
    /// none for the compaction command, which the profile names when no
    /// layer sets it.
    pub fn built_in(self) -> Option<&'static str> {
        match self {
            Self::ContextWarning => Some(
                "[Lys context watch] Context is at {{context_percent}} percent of the window. Write your checkpoint and set your variables now; compaction follows at the next turn boundary. {{goals}}",
            ),
            Self::Preparation => Some(
                "[Lys context watch] Compaction is next. Finish the line you are on and write what the next window needs to know.",
            ),
            Self::Compaction => None,
            Self::WakeUp => Some("[Lys] {{message}}"),
            Self::ScheduledReminder => Some("Reminder from Lys. {{text}} {{time_left}}"),
        }
    }
}

/// Where words are set: the workspace, one agent or one session.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, utoipa::ToSchema,
)]
#[schema(as = WordsLayer)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Layer {
    /// The whole install.
    Workspace,
    /// One agent.
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

impl Layer {
    /// Its name, for refusals and receipts.
    pub fn name(&self) -> String {
        match self {
            Self::Workspace => "workspace".to_owned(),
            Self::Agent { id } => format!("agent {id}"),
            Self::Session { id } => format!("session {id}"),
        }
    }

    /// Its key in the fold: `workspace`, `agent:<id>` or `session:<id>`, a
    /// string so the fold serialises as JSON.
    pub fn key(&self) -> String {
        match self {
            Self::Workspace => "workspace".to_owned(),
            Self::Agent { id } => format!("agent:{id}"),
            Self::Session { id } => format!("session:{id}"),
        }
    }
}

/// What a layer says for a slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsSetting)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Setting {
    /// Its own text.
    Text {
        /// The text, with placeholders rendered at delivery.
        text: String,
    },
    /// A link to a named template.
    Template {
        /// The template's name.
        name: String,
    },
    /// Nothing: the next layer is asked.
    Inherit,
}

/// A slot's words at one layer, as held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsHeld)]
#[serde(deny_unknown_fields)]
pub struct Held {
    /// The setting.
    pub setting: Setting,
    /// Its revision, the count of saves of this slot at this layer.
    pub revision: u64,
    /// Who saved it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// A named template, as held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsTemplate)]
#[serde(deny_unknown_fields)]
pub struct Template {
    /// The text, with placeholders rendered at delivery.
    pub text: String,
    /// Its revision, the count of saves of this template.
    pub revision: u64,
    /// Who saved it.
    pub by: String,
    /// When, in seconds since the Unix epoch.
    pub at: u64,
}

/// One leaf of the words log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "line", rename_all = "snake_case")]
pub enum Line {
    /// A slot set at a layer.
    Set {
        /// The layer.
        layer: Layer,
        /// The slot.
        slot: Slot,
        /// The setting.
        setting: Setting,
        /// The revision this save makes: the held revision plus one.
        revision: u64,
        /// Who saved it.
        by: String,
        /// When, in seconds since the Unix epoch.
        at: u64,
    },
    /// A template saved.
    Template {
        /// The name.
        name: String,
        /// The text.
        text: String,
        /// The revision this save makes: the held revision plus one.
        revision: u64,
        /// Who saved it.
        by: String,
        /// When, in seconds since the Unix epoch.
        at: u64,
    },
}

/// A revision that contributed to a resolution or a rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsRevision)]
#[serde(deny_unknown_fields)]
pub struct Contributed {
    /// What: `words`, `template`, `variables` or `goals`.
    pub kind: String,
    /// Which: the layer and slot, the template name, the scope or the holder.
    pub key: String,
    /// The revision used.
    pub revision: u64,
}

/// A slot resolved for a session or an agent: the text, where it came
/// from, and every revision that contributed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = WordsResolved)]
#[serde(deny_unknown_fields)]
pub struct Resolved {
    /// The slot.
    pub slot: Slot,
    /// The text before rendering; none when no layer and no built-in sets it.
    pub text: Option<String>,
    /// The layer the text came from: `session`, `agent`, `workspace`,
    /// `built_in` or `profile` (the compaction command, named by the profile).
    pub source: String,
    /// Every revision that contributed, outermost first.
    pub contributed: Vec<Contributed>,
}

/// The words as their log folds them.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Words {
    /// Each layer's slots, by the layer's key then slot.
    pub layers: BTreeMap<String, BTreeMap<Slot, Held>>,
    /// The templates, by name.
    pub templates: BTreeMap<String, Template>,
}

impl Folded for Words {
    type Line = Line;
    const DOMAIN: &'static str = "lys/identity/words-state/v1";
    const FORMAT: &'static str = "lys-words-state/v1";
    const KIND: &'static str = "words";

    fn hold(&mut self, line: Line) -> Result<(), String> {
        match line {
            Line::Set {
                layer,
                slot,
                setting,
                revision,
                by,
                at,
            } => {
                let held = self.layers.entry(layer.key()).or_default();
                let current = held.get(&slot).map_or(0, |held| held.revision);
                if revision != current + 1 {
                    return Err(format!(
                        "slot {} is at revision {current}, so the next save is {}, not {revision}",
                        slot.name(),
                        current + 1
                    ));
                }
                held.insert(
                    slot,
                    Held {
                        setting,
                        revision,
                        by,
                        at,
                    },
                );
            }
            Line::Template {
                name,
                text,
                revision,
                by,
                at,
            } => {
                let current = self.templates.get(&name).map_or(0, |held| held.revision);
                if revision != current + 1 {
                    return Err(format!(
                        "template `{name}` is at revision {current}, so the next save is {}, not {revision}",
                        current + 1
                    ));
                }
                self.templates.insert(
                    name,
                    Template {
                        text,
                        revision,
                        by,
                        at,
                    },
                );
            }
        }
        Ok(())
    }
}

impl Words {
    /// The revision of `slot` at `layer`: 0 when never saved.
    pub fn revision(&self, layer: &Layer, slot: Slot) -> u64 {
        self.layers
            .get(&layer.key())
            .and_then(|held| held.get(&slot))
            .map_or(0, |held| held.revision)
    }

    /// The revision of the template `name`: 0 when never saved.
    pub fn template_revision(&self, name: &str) -> u64 {
        self.templates.get(name).map_or(0, |held| held.revision)
    }

    /// Resolve `slot` for `session` of `agent`, most specific layer first;
    /// a layer that inherits is passed over, and a link is followed to its
    /// template, both revisions kept.
    pub fn resolve(&self, slot: Slot, agent: Option<&str>, session: Option<&str>) -> Resolved {
        let mut contributed = Vec::new();
        let layers = session
            .map(|id| Layer::Session { id: id.to_owned() })
            .into_iter()
            .chain(agent.map(|id| Layer::Agent { id: id.to_owned() }))
            .chain(std::iter::once(Layer::Workspace));
        for layer in layers {
            let Some(held) = self
                .layers
                .get(&layer.key())
                .and_then(|held| held.get(&slot))
            else {
                continue;
            };
            contributed.push(Contributed {
                kind: "words".to_owned(),
                key: format!("{}/{}", layer.name(), slot.name()),
                revision: held.revision,
            });
            match &held.setting {
                Setting::Inherit => {}
                Setting::Text { text } => {
                    return Resolved {
                        slot,
                        text: Some(text.clone()),
                        source: source_name(&layer),
                        contributed,
                    };
                }
                Setting::Template { name } => {
                    if let Some(template) = self.templates.get(name) {
                        contributed.push(Contributed {
                            kind: "template".to_owned(),
                            key: name.clone(),
                            revision: template.revision,
                        });
                        return Resolved {
                            slot,
                            text: Some(template.text.clone()),
                            source: source_name(&layer),
                            contributed,
                        };
                    }
                    // A link to a template since removed cannot happen: templates
                    // are never removed; an unknown name is refused at save.
                }
            }
        }
        match slot.built_in() {
            Some(text) => Resolved {
                slot,
                text: Some(text.to_owned()),
                source: "built_in".to_owned(),
                contributed,
            },
            None => Resolved {
                slot,
                text: None,
                source: "profile".to_owned(),
                contributed,
            },
        }
    }
}

fn source_name(layer: &Layer) -> String {
    match layer {
        Layer::Workspace => "workspace".to_owned(),
        Layer::Agent { .. } => "agent".to_owned(),
        Layer::Session { .. } => "session".to_owned(),
    }
}

/// Refuse text that cannot be typed as words: empty, or holding a control
/// character other than a newline (a managed session takes a message of
/// several lines; a terminal session is given one line by its runner).
pub fn checked_text(text: &str) -> Result<String, WordsError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(WordsError::Malformed {
            reason: "words are empty".to_owned(),
        });
    }
    if text
        .chars()
        .any(|character| character.is_control() && character != '\n')
    {
        return Err(WordsError::Malformed {
            reason: "words hold a control character other than a newline".to_owned(),
        });
    }
    Ok(text.to_owned())
}

/// Refuse a template name that is not lowercase letters, digits, `-` or
/// `_`, 1 to 64 long.
pub fn checked_name(name: &str) -> Result<String, WordsError> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        });
    if ok {
        Ok(name.to_owned())
    } else {
        Err(WordsError::Malformed {
            reason: format!(
                "`{name}` is not a template name: lowercase letters, digits, - and _, 1 to 64 long"
            ),
        })
    }
}
