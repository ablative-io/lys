//! The sealed store. Every entry is sealed to the store key with
//! lys/sealed-envelope/v1, and its binding (entry id, name, class, owner and
//! sequence) is carried as the authenticated prefix of the sealed plaintext,
//! so a ciphertext moved between entries or rolled back in place is refused.
//!
//! Each sealing is its own file, named by entry id and sequence, and the
//! index names the one in force. A replacement or a rotation writes its new
//! sealings beside the old, and the index write is the one moment it takes
//! effect; a sealing the index does not name is swept when the store opens.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use lys_core::seal::{SealedEnvelope, open, seal};
use serde::{Deserialize, Serialize};

use crate::encoding::{Canonical, Reader, hex, random_bytes};
use crate::error::SecretsError;
use crate::fsutil::{io, write_atomic};
use crate::keys::StoreKey;
use crate::secret::Secret;

mod accounts;
mod lock;
mod policy;
mod scope;

pub use accounts::AccountView;
pub use policy::Recipients;
pub use scope::Scope;

const INDEX: &str = "index.json";
const ENTRIES: &str = "entries";
const ENVELOPE_DOMAIN: &str = "lys-secrets/entry-envelope/v1";
const PLAIN_DOMAIN: &str = "lys-secrets/entry/v1";
const NONCE_LEN: usize = 12;
const MAX_NAME: usize = 128;

/// What an entry holds, which decides how it may be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryClass {
    /// A credential, used only through the proxy and never read.
    Credential,
    /// A key, used only through the proxy and never read.
    Key,
    /// A memory, read by those it is shared with.
    Memory,
    /// An OAuth service grant, used only through the proxy, which refreshes
    /// its access token itself.
    #[serde(rename = "oauth")]
    OAuth,
}

impl EntryClass {
    fn label(self) -> &'static str {
        match self {
            Self::Credential => "credential",
            Self::Key => "key",
            Self::Memory => "memory",
            Self::OAuth => "oauth",
        }
    }
}

/// An entry as the index records it. It holds no secret byte.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryView {
    /// The entry's random id.
    pub id: String,
    /// The entry's name.
    pub name: String,
    /// What the entry holds.
    pub class: EntryClass,
    /// The identity that owns it.
    pub owner: String,
    /// The latest sequence written for it.
    pub sequence: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Index {
    key_id: String,
    entries: BTreeMap<String, EntryView>,
    #[serde(default)]
    accounts: BTreeMap<String, accounts::Ring>,
    #[serde(default)]
    recipients: BTreeMap<String, policy::Recipients>,
    #[serde(default)]
    scopes: BTreeMap<String, scope::Scope>,
}

/// The sealed store, held open by one broker at a time.
#[derive(Debug)]
pub struct SecretStore {
    dir: PathBuf,
    index: Index,
    _lock: lock::StoreLock,
}

impl SecretStore {
    /// Makes a new store in `dir` sealed to `key`.
    ///
    /// # Errors
    ///
    /// `StoreAlreadyExists` when `dir` already holds one, `StoreLocked`, and
    /// `Io` when the directory cannot be written.
    pub fn create(dir: &Path, key: &StoreKey) -> Result<Self, SecretsError> {
        fs::create_dir_all(dir.join(ENTRIES)).map_err(io(format!("creating {}", dir.display())))?;
        if dir.join(INDEX).exists() {
            return Err(SecretsError::StoreAlreadyExists {
                path: dir.to_path_buf(),
            });
        }
        let lock = lock::StoreLock::take(dir)?;
        let store = Self {
            dir: dir.to_path_buf(),
            index: Index {
                key_id: key.id().as_str().to_owned(),
                entries: BTreeMap::new(),
                accounts: BTreeMap::new(),
                recipients: BTreeMap::new(),
                scopes: BTreeMap::new(),
            },
            _lock: lock,
        };
        store.write_index()?;
        Ok(store)
    }

    /// Opens the store in `dir` under `key`.
    ///
    /// # Errors
    ///
    /// `StoreNotFound`, `StoreLocked`, `StoreCorrupt` when the index does not
    /// read, and `StoreKeyMismatch` when `key` is not the store's key.
    pub fn open(dir: &Path, key: &StoreKey) -> Result<Self, SecretsError> {
        let index_path = dir.join(INDEX);
        if !index_path.exists() {
            return Err(SecretsError::StoreNotFound {
                path: dir.to_path_buf(),
            });
        }
        let lock = lock::StoreLock::take(dir)?;
        let bytes =
            fs::read(&index_path).map_err(io(format!("reading {}", index_path.display())))?;
        let index: Index =
            serde_json::from_slice(&bytes).map_err(|error| SecretsError::StoreCorrupt {
                reason: format!("{INDEX} does not read: {error}"),
            })?;
        if index.key_id != key.id().as_str() {
            return Err(SecretsError::StoreKeyMismatch {
                expected: index.key_id,
                presented: key.id().as_str().to_owned(),
            });
        }
        let store = Self {
            dir: dir.to_path_buf(),
            index,
            _lock: lock,
        };
        store.sweep()?;
        Ok(store)
    }

    /// Removes every sealing the index does not name: the old sealings of a
    /// replacement or a rotation, and the new ones of a rotation that never
    /// took effect.
    fn sweep(&self) -> Result<(), SecretsError> {
        let entries = self.dir.join(ENTRIES);
        let listing =
            fs::read_dir(&entries).map_err(io(format!("listing {}", entries.display())))?;
        for found in listing {
            let found = found.map_err(io(format!("listing {}", entries.display())))?;
            let path = found.path();
            let in_force = self
                .index
                .entries
                .values()
                .any(|view| self.entry_path(view) == path);
            if !in_force {
                fs::remove_file(&path).map_err(io(format!(
                    "removing the sealing {} no entry names",
                    path.display()
                )))?;
            }
        }
        Ok(())
    }

    /// The store's directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every entry, by name. No secret byte.
    pub fn entries(&self) -> impl Iterator<Item = &EntryView> {
        self.index.entries.values()
    }

    /// The entry named `name`.
    pub fn entry(&self, name: &str) -> Option<&EntryView> {
        self.index.entries.get(name)
    }

    /// Seals `value` as a new entry named `name`, owned by `owner`.
    ///
    /// # Errors
    ///
    /// `InvalidName`, `SecretExists`, `Random`, `Trust` when sealing fails,
    /// and `Io`.
    pub fn add(
        &mut self,
        key: &StoreKey,
        name: &str,
        class: EntryClass,
        owner: &str,
        value: &Secret,
    ) -> Result<EntryView, SecretsError> {
        check_name("secret name", name)?;
        check_name("owner", owner)?;
        if self.index.entries.contains_key(name) {
            return Err(SecretsError::SecretExists {
                name: name.to_owned(),
            });
        }
        let view = EntryView {
            id: hex(&random_bytes::<16>()?),
            name: name.to_owned(),
            class,
            owner: owner.to_owned(),
            sequence: 1,
        };
        self.write_entry(key, &view, value)?;
        self.index.entries.insert(name.to_owned(), view.clone());
        self.write_index()?;
        Ok(view)
    }

    /// Replaces the value of `name`, advancing its sequence.
    ///
    /// # Errors
    ///
    /// `SecretUnknown`, `Trust` and `Io`.
    pub fn replace(
        &mut self,
        key: &StoreKey,
        name: &str,
        value: &Secret,
    ) -> Result<EntryView, SecretsError> {
        let mut view =
            self.index
                .entries
                .get(name)
                .cloned()
                .ok_or_else(|| SecretsError::SecretUnknown {
                    name: name.to_owned(),
                })?;
        view.sequence = view.sequence.saturating_add(1);
        self.write_entry(key, &view, value)?;
        self.index.entries.insert(name.to_owned(), view.clone());
        self.write_index()?;
        self.sweep()?;
        Ok(view)
    }

    /// Reseals every entry to `new`, advancing each sequence, beside the
    /// current sealing. Nothing is read under `new` until
    /// [`SecretStore::commit_rotation`] writes the index.
    pub(crate) fn prepare_rotation(
        &self,
        old: &StoreKey,
        new: &StoreKey,
    ) -> Result<Vec<EntryView>, SecretsError> {
        let mut staged = Vec::with_capacity(self.index.entries.len());
        for view in self.index.entries.values() {
            let value = self.open_for_use(old, &view.name, view.class)?;
            let mut next = view.clone();
            next.sequence = next.sequence.saturating_add(1);
            Self::write_sealing(new, &next, &value, &self.entry_path(&next))?;
            staged.push(next);
        }
        Ok(staged)
    }

    /// Makes the staged sealings current and records `new` as the store's
    /// key, in one index write, then sweeps the old sealings.
    pub(crate) fn commit_rotation(
        &mut self,
        new: &StoreKey,
        staged: Vec<EntryView>,
    ) -> Result<(), SecretsError> {
        for view in staged {
            self.index.entries.insert(view.name.clone(), view);
        }
        new.id().as_str().clone_into(&mut self.index.key_id);
        self.write_index()?;
        self.sweep()
    }

    /// Opens `name` for use under `key`, refusing any binding other than the
    /// entry asked for and any sequence older than the index records.
    pub(crate) fn open_for_use(
        &self,
        key: &StoreKey,
        name: &str,
        class: EntryClass,
    ) -> Result<Secret, SecretsError> {
        let view = self
            .index
            .entries
            .get(name)
            .ok_or_else(|| SecretsError::SecretUnknown {
                name: name.to_owned(),
            })?;
        let path = self.entry_path(view);
        let bytes = fs::read(&path).map_err(io(format!("reading {}", path.display())))?;
        let envelope = decode_envelope(&bytes)?;
        let plain =
            open(&envelope, &key.identity().x25519_static_secret()).map_err(|_unsealed| {
                SecretsError::EntryUnsealFailed {
                    entry: name.to_owned(),
                }
            })?;
        let plain = Secret::new(plain);
        let mut reader = Reader::new(plain.expose(), "sealed entry");
        let domain = reader.field()?;
        let id = reader.field()?;
        let found_name = reader.field()?;
        let found_class = reader.field()?;
        let found_owner = reader.field()?;
        let sequence = reader.number()?;
        let bound = domain == PLAIN_DOMAIN.as_bytes()
            && id == view.id.as_bytes()
            && found_name == name.as_bytes()
            && found_class == class.label().as_bytes()
            && class == view.class
            && found_owner == view.owner.as_bytes();
        if !bound {
            return Err(SecretsError::EntryBindingMismatch {
                entry: name.to_owned(),
            });
        }
        if sequence < view.sequence {
            return Err(SecretsError::EntryRolledBack {
                entry: name.to_owned(),
                recorded: view.sequence,
                found: sequence,
            });
        }
        let value = reader.field()?;
        Ok(Secret::from_slice(value))
    }

    fn write_entry(
        &self,
        key: &StoreKey,
        view: &EntryView,
        value: &Secret,
    ) -> Result<(), SecretsError> {
        Self::write_sealing(key, view, value, &self.entry_path(view))
    }

    fn write_sealing(
        key: &StoreKey,
        view: &EntryView,
        value: &Secret,
        path: &Path,
    ) -> Result<(), SecretsError> {
        let mut plain = Canonical::new(PLAIN_DOMAIN)?;
        plain
            .field(view.id.as_bytes())?
            .field(view.name.as_bytes())?
            .field(view.class.label().as_bytes())?
            .field(view.owner.as_bytes())?
            .number(view.sequence)?
            .field(value.expose())?;
        let plain = Secret::new(plain.into_bytes());
        let envelope = seal(plain.expose(), &key.sealing_public())?;
        let mut encoded = Canonical::new(ENVELOPE_DOMAIN)?;
        encoded
            .field(&envelope.ephemeral_public_key)?
            .field(&envelope.ciphertext)?
            .field(&envelope.nonce)?;
        write_atomic(path, &encoded.into_bytes())
    }

    fn entry_path(&self, view: &EntryView) -> PathBuf {
        self.dir
            .join(ENTRIES)
            .join(format!("{}.{}.sealed", view.id, view.sequence))
    }

    fn write_index(&self) -> Result<(), SecretsError> {
        let bytes =
            serde_json::to_vec_pretty(&self.index).map_err(|error| SecretsError::Encoding {
                context: "store index",
                reason: error.to_string(),
            })?;
        write_atomic(&self.dir.join(INDEX), &bytes)
    }
}

fn decode_envelope(bytes: &[u8]) -> Result<SealedEnvelope, SecretsError> {
    let mut reader = Reader::new(bytes, "sealed entry envelope");
    if reader.field()? != ENVELOPE_DOMAIN.as_bytes() {
        return Err(SecretsError::StoreCorrupt {
            reason: "an entry file is not a sealed entry".to_owned(),
        });
    }
    let ephemeral: [u8; 32] =
        reader
            .field()?
            .try_into()
            .map_err(|_length| SecretsError::StoreCorrupt {
                reason: "an entry's ephemeral key is not 32 bytes".to_owned(),
            })?;
    let ciphertext = reader.field()?.to_vec();
    let nonce: [u8; NONCE_LEN] =
        reader
            .field()?
            .try_into()
            .map_err(|_length| SecretsError::StoreCorrupt {
                reason: "an entry's nonce is not 12 bytes".to_owned(),
            })?;
    Ok(SealedEnvelope {
        ephemeral_public_key: ephemeral,
        ciphertext,
        nonce,
    })
}

pub(crate) fn check_name(what: &'static str, name: &str) -> Result<(), SecretsError> {
    let reason = if name.is_empty() {
        Some("is empty")
    } else if name.len() > MAX_NAME {
        Some("is longer than 128 bytes")
    } else if name.chars().any(char::is_control) {
        Some("holds a control character")
    } else {
        None
    };
    match reason {
        Some(reason) => Err(SecretsError::InvalidName {
            what,
            name: name.to_owned(),
            reason,
        }),
        None => Ok(()),
    }
}
