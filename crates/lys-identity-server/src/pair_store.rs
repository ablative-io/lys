//! The pairings waiting on a person's decision (AGENTS-020 R1): each the
//! code the agent says, the wait id it holds its answer on, the name it
//! proposed, its PKCS#10 request, and, once a person decides, the
//! decision. Kept whole in `pairings.json` beside the directory log and
//! written whole, fsynced and renamed at every change, as the foundings
//! are: one durable operation per ask, decision or answer, never leaf by
//! leaf. No expiry and no timer (Tom, 15 October): a pairing ends when a
//! person decides it and the agent takes the answer, never by the clock.
//!
//! The held answer is the store's own event: every change bumps a watch
//! channel, and a wait that finds no decision yet subscribes and is woken
//! by the decision that names it.

use std::collections::BTreeMap;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use crate::error::ServerError;

/// The file's format name.
pub const FORMAT: &str = "lys-pairings/1";

/// One approved app the agent may reach, as `/apps` answers it
/// (DIRECTORY-114).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = PairReach)]
#[serde(deny_unknown_fields)]
pub struct Reach {
    /// The app's id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its address.
    pub address: String,
}

/// What an approved pairing answers the waiting agent (AGENTS-020 R3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[schema(as = PairAnswer)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The agent's id.
    pub agent: String,
    /// The agent's handle.
    pub handle: String,
    /// The certificate over the agent's key, as a `CERTIFICATE` PEM.
    pub certificate_pem: String,
    /// The serial the certificate was entered under.
    pub serial: String,
    /// The Lys API address.
    pub lys: String,
    /// The responsible person's handle.
    pub person: String,
    /// The organisation's approved apps.
    pub apps: Vec<Reach>,
}

/// A person's decision on a pairing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Decision {
    /// Approved: the answer for the agent.
    Approved {
        /// What the agent is told.
        answer: Box<Answer>,
    },
    /// Declined: nothing of the key is kept.
    Declined,
}

/// One pairing, from the ask to the answer the agent says it has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pairing {
    /// The four words the agent says.
    pub code: String,
    /// The id the agent holds its answer on.
    pub wait: String,
    /// The name the agent proposed.
    pub name: String,
    /// Its PKCS#10 request, DER in standard base64; dropped at a decline.
    pub request: String,
    /// When it asked, seconds since the Unix epoch.
    pub asked_at: u64,
    /// The person's decision, once made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    /// By code.
    pairings: BTreeMap<String, Pairing>,
}

/// The pairings, held in memory and written whole at every change.
pub struct PairStore {
    file: PathBuf,
    stored: Stored,
    changed: watch::Sender<u64>,
}

fn unusable(file: &Path, error: &dyn std::fmt::Display) -> ServerError {
    ServerError::DirectoryUnavailable {
        reason: format!("the pairings at {} cannot be used: {error}", file.display()),
    }
}

/// A refusal by name.
pub(crate) fn refused(name: &str, words: impl Into<String>) -> ServerError {
    ServerError::Runner {
        refusal: name.to_owned(),
        words: words.into(),
    }
}

impl PairStore {
    /// The pairings kept beside the directory log at `log_dir`, as
    /// `pairings.json` beside `foundings.json`: no configuration key names
    /// them.
    ///
    /// # Errors
    /// As [`PairStore::open`].
    pub fn beside(log_dir: &Path) -> Result<Self, ServerError> {
        Self::open(log_dir.with_file_name("pairings.json"))
    }

    /// Open `file`; absent, there are no pairings yet and the file is made
    /// at the first.
    ///
    /// # Errors
    /// Names a file that cannot be read or is not a pairings file.
    pub fn open(file: PathBuf) -> Result<Self, ServerError> {
        let stored = match std::fs::read(&file) {
            Ok(bytes) => {
                let stored: Stored =
                    serde_json::from_slice(&bytes).map_err(|error| unusable(&file, &error))?;
                if stored.format != FORMAT {
                    return Err(unusable(
                        &file,
                        &format!("its format is {}, not {FORMAT}", stored.format),
                    ));
                }
                stored
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Stored {
                format: FORMAT.to_owned(),
                pairings: BTreeMap::new(),
            },
            Err(error) => return Err(unusable(&file, &error)),
        };
        Ok(Self {
            file,
            stored,
            changed: watch::channel(0).0,
        })
    }

    /// Keep a new pairing for `name` over `request`: a code no undecided
    /// pairing holds and a fresh wait id, durably.
    ///
    /// # Errors
    /// Names the operating system's refusal to give random bytes, or a write
    /// that did not reach the disk; the memory is then as the file is.
    pub fn ask(&mut self, name: String, request: String) -> Result<(String, String), ServerError> {
        let code = loop {
            let mut bytes = [0_u8; 4];
            OsRng.try_fill_bytes(&mut bytes).map_err(|error| {
                ServerError::DirectoryUnavailable {
                    reason: format!(
                        "the operating system gave no random bytes for a pairing code: {error}"
                    ),
                }
            })?;
            let code = lys_core::pairing_words::code_from(bytes);
            if !self.stored.pairings.contains_key(&code) {
                break code;
            }
        };
        let wait = crate::foundings_store::new_token()?;
        let pairing = Pairing {
            code: code.clone(),
            wait: wait.clone(),
            name,
            request,
            asked_at: crate::session::now(),
            decision: None,
        };
        self.stored.pairings.insert(code.clone(), pairing);
        if let Err(error) = self.save() {
            self.stored.pairings.remove(&code);
            return Err(error);
        }
        self.bump();
        Ok((code, wait))
    }

    /// The undecided pairing `code` names.
    #[must_use]
    pub fn by_code(&self, code: &str) -> Option<&Pairing> {
        self.stored
            .pairings
            .get(code)
            .filter(|pairing| pairing.decision.is_none())
    }

    /// Every undecided pairing, oldest first.
    #[must_use]
    pub fn undecided(&self) -> Vec<&Pairing> {
        let mut waiting: Vec<&Pairing> = self
            .stored
            .pairings
            .values()
            .filter(|pairing| pairing.decision.is_none())
            .collect();
        waiting.sort_by_key(|pairing| (pairing.asked_at, pairing.code.clone()));
        waiting
    }

    /// Record `decision` on the undecided pairing `code` names, durably,
    /// dropping the request at a decline, and wake every wait.
    ///
    /// # Errors
    /// `pair_unknown` when no undecided pairing holds the code; a write that
    /// did not reach the disk leaves the memory as the file is.
    pub fn decide(&mut self, code: &str, decision: Decision) -> Result<(), ServerError> {
        let Some(pairing) = self
            .stored
            .pairings
            .get_mut(code)
            .filter(|pairing| pairing.decision.is_none())
        else {
            return Err(refused(
                "pair_unknown",
                "no agent is waiting with these words",
            ));
        };
        let before = pairing.clone();
        if decision == Decision::Declined {
            pairing.request = String::new();
        }
        pairing.decision = Some(decision);
        if let Err(error) = self.save() {
            self.stored.pairings.insert(code.to_owned(), before);
            return Err(error);
        }
        self.bump();
        Ok(())
    }

    /// The decision on the pairing `wait` names, read: `None` while the
    /// person has not decided. Reading consumes nothing, so a hold a proxy
    /// cut after the decision reads the same answer when it holds again;
    /// the pairing leaves only when the agent says it has its answer
    /// ([`PairStore::taken`]).
    ///
    /// # Errors
    /// `pair_unknown` when no pairing holds the wait id (never asked, or
    /// already taken).
    pub fn decided(&self, wait: &str) -> Result<Option<Decision>, ServerError> {
        self.stored
            .pairings
            .values()
            .find(|pairing| pairing.wait == wait)
            .map(|pairing| pairing.decision.clone())
            .ok_or_else(|| refused("pair_unknown", "no pairing waits under this id"))
    }

    /// The agent has its answer: the decided pairing `wait` names is
    /// removed, durably. Given once: a second call is `pair_unknown`.
    ///
    /// # Errors
    /// `pair_unknown` when no decided pairing holds the wait id; a write
    /// that did not reach the disk leaves the memory as the file is.
    pub fn taken(&mut self, wait: &str) -> Result<(), ServerError> {
        let Some(code) = self
            .stored
            .pairings
            .values()
            .find(|pairing| pairing.wait == wait && pairing.decision.is_some())
            .map(|pairing| pairing.code.clone())
        else {
            return Err(refused(
                "pair_unknown",
                "no decided pairing waits under this id",
            ));
        };
        let Some(pairing) = self.stored.pairings.remove(&code) else {
            return Err(refused("pair_unknown", "no pairing waits under this id"));
        };
        if let Err(error) = self.save() {
            self.stored.pairings.insert(code, pairing);
            return Err(error);
        }
        Ok(())
    }

    /// How many waits are subscribed to the next change.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.changed.receiver_count()
    }

    /// A subscription woken by the next change.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }

    fn bump(&self) {
        self.changed.send_modify(|revision| *revision += 1);
    }

    fn save(&self) -> Result<(), ServerError> {
        let parent = self
            .file
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| unusable(&self.file, &"the pairings file needs a parent directory"))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|error| unusable(&self.file, &error))?;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| unusable(&self.file, &error))?;
        serde_json::to_writer(&mut temporary, &self.stored)
            .map_err(|error| unusable(&self.file, &error))?;
        temporary
            .flush()
            .map_err(|error| unusable(&self.file, &error))?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| unusable(&self.file, &error))?;
        temporary
            .persist(&self.file)
            .map_err(|error| unusable(&self.file, &error))?;
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| unusable(&self.file, &error))
    }
}
