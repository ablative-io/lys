//! The foundings waiting on their proof (DIRECTORY-111 R1, R2): the lowercase
//! hex SHA-256 of each proving token beside the organisation and founder it
//! proves, the address the letter went to and the founding operation, and
//! whether it is spent. The token itself is never held: the letter alone
//! carries it. Every change is one durable write, the whole file written to
//! a sibling, fsynced and renamed over the old, as the grant token table
//! writes. No expiry and no timer (Tom, 15 October): a founding is spent by
//! its proof or replaced by a later founding from the same address, never by
//! the clock.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use crate::error::ServerError;

/// The file's format name.
pub const FORMAT: &str = "lys-foundings/1";

/// One founding, as its proof finds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Founding {
    /// The organisation founded, Registered until proven.
    pub organisation: String,
    /// The founder, Registered until proven.
    pub founder: String,
    /// The address the founded letter went to.
    pub email: String,
    /// The founding act's operation; every proving step derives its own
    /// from it.
    pub operation: String,
    /// Whether the proof was used.
    pub spent: bool,
    /// An administrator withdrew this admission; the digest remains held.
    #[serde(default)]
    pub withdrawn: bool,
    /// The canonical withdrawal event, kept in the same durable write so a
    /// refused directory append can be finished by the next read or act.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawal_event: Option<Vec<u8>>,
    /// Whether the link was handed on the screen because the install has no
    /// mail settings, not sent in a letter (DIRECTORY-112 R1).
    #[serde(default)]
    pub handed: bool,
    /// A founding, or a person's admission whose handed link signs them in
    /// once (DIRECTORY-112 R3).
    #[serde(default)]
    pub kind: Kind,
    /// Where an admission asked by a product goes on to: the origin of that
    /// product's first return address in Lys's own record of it, never a
    /// request's word (DIRECTORY-113 R2). Absent for every other link.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continue_to: Option<String>,
}

/// What a proving link proves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// An organisation and its founder, both Registered until proven.
    #[default]
    Founding,
    /// A person admitted into an organisation, already Active and bound:
    /// `founder` names the person, `organisation` where they were admitted.
    Admission,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    /// By the token's digest.
    foundings: BTreeMap<String, Founding>,
    /// The held address survives link replacement; only confirmed changes replace it.
    #[serde(default)]
    admitted_email: BTreeMap<String, String>,
    /// Original provenance remains visible after an authorized email change.
    #[serde(default)]
    original_email: BTreeMap<String, String>,
    #[serde(default)]
    sign_in_recorded: BTreeSet<String>,
}

/// The foundings, held in memory and written whole at every change.
pub struct Foundings {
    file: PathBuf,
    stored: Stored,
}

fn unusable(file: &Path, error: &dyn std::fmt::Display) -> ServerError {
    ServerError::DirectoryUnavailable {
        reason: format!(
            "the foundings at {} cannot be used: {error}",
            file.display()
        ),
    }
}

/// The lowercase hex SHA-256 of `token`.
#[must_use]
pub fn digest(token: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    Sha256::digest(token.as_bytes())
        .into_iter()
        .flat_map(|byte| {
            [
                char::from(HEX[usize::from(byte >> 4)]),
                char::from(HEX[usize::from(byte & 15)]),
            ]
        })
        .collect()
}

/// A fresh proving token: thirty-two bytes from the operating system,
/// URL-safe base64 without padding.
///
/// # Errors
/// Names the operating system's refusal to give random bytes.
pub fn new_token() -> Result<String, ServerError> {
    let mut bytes = [0_u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .map_err(|error| ServerError::DirectoryUnavailable {
            reason: format!(
                "the operating system gave no random bytes for a proving token: {error}"
            ),
        })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

impl Foundings {
    /// Open `file`; absent, there are no foundings yet and the file is made
    /// at the first.
    ///
    /// # Errors
    /// Names a file that cannot be read or is not a foundings file.
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
                foundings: BTreeMap::new(),
                admitted_email: BTreeMap::new(),
                original_email: BTreeMap::new(),
                sign_in_recorded: BTreeSet::new(),
            },
            Err(error) => return Err(unusable(&file, &error)),
        };
        Ok(Self { file, stored })
    }

    /// The founding `digest` names, spent or not.
    #[must_use]
    pub fn by_digest(&self, digest: &str) -> Option<&Founding> {
        self.stored.foundings.get(digest)
    }

    /// The held address; older stores fall back to their retained founding
    /// before a later admission. This read never rewrites a store.
    pub(crate) fn email_for(&self, person: &str) -> Option<&str> {
        self.stored
            .admitted_email
            .get(person)
            .map(String::as_str)
            .or_else(|| {
                self.stored
                    .foundings
                    .values()
                    .filter(|held| held.founder == person)
                    .min_by_key(|held| held.kind != Kind::Founding)
                    .map(|held| held.email.as_str())
            })
    }

    /// Preserve the original row provenance independently of authorized changes.
    pub(crate) fn original_email_for(&self, person: &str) -> Option<&str> {
        self.stored
            .original_email
            .get(person)
            .map(String::as_str)
            .or_else(|| self.email_for(person))
    }

    pub(crate) fn recorded_at_sign_in(&self, person: &str) -> bool {
        self.stored.sign_in_recorded.contains(person)
    }

    /// Replace the held address and retain its provenance in one durable write.
    /// A failed write restores every in-memory field to the previous record.
    pub(crate) fn hold_email(
        &mut self,
        person: &str,
        email: &str,
        at_sign_in: bool,
    ) -> Result<(), ServerError> {
        let before = self.stored.clone();
        let missing = self.email_for(person).is_none();
        let original = self.original_email_for(person).unwrap_or(email).to_owned();
        self.stored
            .original_email
            .entry(person.to_owned())
            .or_insert(original);
        self.stored
            .admitted_email
            .insert(person.to_owned(), email.to_owned());
        if missing && at_sign_in {
            self.stored.sign_in_recorded.insert(person.to_owned());
        }
        if let Err(error) = self.save() {
            self.stored = before;
            return Err(error);
        }
        Ok(())
    }

    /// The unspent founding for `email`, with its digest, if one waits; an
    /// admission's link is never a founding.
    #[must_use]
    pub fn unspent_for_email(&self, email: &str) -> Option<(&str, &Founding)> {
        self.stored
            .foundings
            .iter()
            .find(|(_, founding)| {
                !founding.spent
                    && !founding.withdrawn
                    && founding.kind == Kind::Founding
                    && founding.email == email
            })
            .map(|(digest, founding)| (digest.as_str(), founding))
    }

    /// The digest of `person`'s unspent admission link, if one waits
    /// (DIRECTORY-112 R3).
    #[must_use]
    pub fn unspent_admission_for(&self, person: &str) -> Option<&str> {
        self.stored
            .foundings
            .iter()
            .find(|(_, founding)| {
                !founding.spent
                    && !founding.withdrawn
                    && founding.kind == Kind::Admission
                    && founding.founder == person
            })
            .map(|(digest, _)| digest.as_str())
    }

    /// Record `founding` under `digest`, dropping `replaced` (the digest of
    /// an earlier unspent founding for the same address) in the same write.
    ///
    /// # Errors
    /// Names a write that did not reach the disk; the memory is then as the
    /// file is.
    pub fn record(
        &mut self,
        digest: String,
        founding: Founding,
        replaced: Option<&str>,
    ) -> Result<(), ServerError> {
        let person = founding.founder.clone();
        let remembered = !self.stored.admitted_email.contains_key(&person);
        if remembered {
            let email = self.email_for(&person).unwrap_or(&founding.email).to_owned();
            self.stored.admitted_email.insert(person.clone(), email);
        }
        let taken = replaced.and_then(|old| self.stored.foundings.remove(old));
        let before = self.stored.foundings.insert(digest.clone(), founding);
        if let Err(error) = self.save() {
            if remembered {
                self.stored.admitted_email.remove(&person);
            }
            self.stored.foundings.remove(&digest);
            if let Some(before) = before {
                self.stored.foundings.insert(digest, before);
            }
            if let (Some(old), Some(taken)) = (replaced, taken) {
                self.stored.foundings.insert(old.to_owned(), taken);
            }
            return Err(error);
        }
        Ok(())
    }

    /// Keep the withdrawal and its event in one write, retaining the digest.
    ///
    /// # Errors
    /// Refuses an absent, spent or withdrawn admission, or a failed save;
    /// on a failed save memory and disk keep the whole previous record.
    pub fn withdraw(&mut self, digest: &str, event: Vec<u8>) -> Result<(), ServerError> {
        let mut admission = self
            .by_digest(digest)
            .filter(|held| held.kind == Kind::Admission && !held.spent && !held.withdrawn)
            .cloned()
            .ok_or(ServerError::NoUnspentAdmission)?;
        admission.withdrawn = true;
        admission.withdrawal_event = Some(event);
        self.record(digest.to_owned(), admission, None)
    }

    /// Withdrawal events kept for this person, including ones whose
    /// directory append must be retried. Replaying an event appends nothing.
    pub(crate) fn withdrawals_for<'a>(&'a self, person: &'a str) -> impl Iterator<Item = &'a [u8]> {
        self.stored
            .foundings
            .values()
            .filter(move |held| held.founder == person && held.withdrawn)
            .filter_map(|held| held.withdrawal_event.as_deref())
    }

    /// Mark `digest` spent, durably, as the last step of its proof.
    ///
    /// # Errors
    /// Names an unknown digest or a write that did not reach the disk; the
    /// memory is then as the file is.
    pub fn spend(&mut self, digest: &str) -> Result<(), ServerError> {
        let Some(founding) = self.stored.foundings.get_mut(digest) else {
            return Err(ServerError::ProofRefused);
        };
        if founding.spent || founding.withdrawn {
            return Err(ServerError::ProofRefused);
        }
        founding.spent = true;
        if let Err(error) = self.save() {
            if let Some(founding) = self.stored.foundings.get_mut(digest) {
                founding.spent = false;
            }
            return Err(error);
        }
        Ok(())
    }

    fn save(&self) -> Result<(), ServerError> {
        let parent = self
            .file
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| unusable(&self.file, &"the foundings file needs a parent directory"))?;
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

#[cfg(test)]
#[path = "foundings_store_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "foundings_store_email_tests.rs"]
mod email_tests;
