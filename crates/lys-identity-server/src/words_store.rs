//! The words as they are kept (AGENTS-001 R1): a leaf store of their own,
//! one leaf for each save of a slot at a layer and each save of a template,
//! folded through the agents log engine and sealed in its signed snapshot.

use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_log_store::FileLeafStore;

use crate::agents_log::{Kept, RecordLog};
use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;
use crate::session::now;
use crate::words_state::{
    Layer, Line, Resolved, Setting, Slot, Words, WordsError, checked_name, checked_text,
};

/// The origin the words' leaf store is created with.
pub const ORIGIN: &str = "lys/identity/words";

/// The words' store refusal.
fn unavailable(reason: String) -> ServerError {
    WordsError::Unavailable { reason }.into()
}

/// The words, read from their leaf store and appended to it.
pub type WordStore = RecordLog<Words, FileLeafStore>;

/// The words behind one lock.
pub type WordsKept = Kept<Words>;

/// The words in the directory `config` names, their snapshots signed by
/// `key`, saying through `say` how the log started; none when it names no
/// directory.
pub fn configured(
    config: &Config,
    key: Arc<Ed25519Identity>,
    say: &Say,
) -> Result<Option<WordsKept>, ServerError> {
    let Some(dir) = config.words_dir.as_deref() else {
        return Ok(None);
    };
    let store = open(dir, key)?;
    say(&format!(
        "words log {}, holding {} layers and {} templates",
        store.start(),
        store.held().layers.len(),
        store.held().templates.len()
    ));
    Ok(Some(Kept::new(store, unavailable)))
}

/// The words kept in the directory `dir`, created when it does not exist.
pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<WordStore, ServerError> {
    RecordLog::open(dir, ORIGIN, key, &unavailable)
}

/// A save of a slot at a layer: the setting, and the revision the caller read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Save {
    /// The layer.
    pub layer: Layer,
    /// The slot.
    pub slot: Slot,
    /// The setting.
    pub setting: Setting,
    /// The revision the caller read; 0 for a slot never saved.
    pub revision: u64,
    /// Who saves it.
    pub by: String,
}

/// Keep `save`, refusing a stale revision, malformed text or a link to no
/// template, and answer the revision it made.
pub fn set(words: &WordsKept, save: Save) -> Result<u64, ServerError> {
    words.with(|log, unavailable| {
        let held = log.held().revision(&save.layer, save.slot);
        if held != save.revision {
            return Err(WordsError::Stale {
                what: format!("{}/{}", save.layer.name(), save.slot.name()),
                held,
                given: save.revision,
            }
            .into());
        }
        let setting = match save.setting {
            Setting::Text { text } => Setting::Text {
                text: checked_text(&text)?,
            },
            Setting::Template { name } => {
                let name = checked_name(&name)?;
                if !log.held().templates.contains_key(&name) {
                    return Err(WordsError::TemplateUnknown { name }.into());
                }
                Setting::Template { name }
            }
            Setting::Inherit => Setting::Inherit,
        };
        let revision = held + 1;
        log.append(
            Line::Set {
                layer: save.layer,
                slot: save.slot,
                setting,
                revision,
                by: save.by,
                at: now(),
            },
            unavailable,
        )?;
        Ok(revision)
    })
}

/// Keep the template `name` as `text`, refusing a stale revision, and
/// answer the revision it made.
pub fn template(
    words: &WordsKept,
    name: &str,
    text: &str,
    revision: u64,
    by: &str,
) -> Result<u64, ServerError> {
    let name = checked_name(name)?;
    let text = checked_text(text)?;
    words.with(|log, unavailable| {
        let held = log.held().template_revision(&name);
        if held != revision {
            return Err(WordsError::Stale {
                what: format!("template {name}"),
                held,
                given: revision,
            }
            .into());
        }
        let made = held + 1;
        log.append(
            Line::Template {
                name: name.clone(),
                text: text.clone(),
                revision: made,
                by: by.to_owned(),
                at: now(),
            },
            unavailable,
        )?;
        Ok(made)
    })
}

/// Resolve `slot` for `session` of `agent` from the words as held.
pub fn resolve(
    words: &WordsKept,
    slot: Slot,
    agent: Option<&str>,
    session: Option<&str>,
) -> Result<Resolved, ServerError> {
    words.with(|log, _| Ok(log.held().resolve(slot, agent, session)))
}

/// The words as held, whole.
pub fn held(words: &WordsKept) -> Result<Words, ServerError> {
    words.with(|log, _| Ok(log.held().clone()))
}
