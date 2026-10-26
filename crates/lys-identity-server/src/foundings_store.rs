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
use std::collections::BTreeMap;
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    format: String,
    /// By the token's digest.
    foundings: BTreeMap<String, Founding>,
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
        let taken = replaced.and_then(|old| self.stored.foundings.remove(old));
        let before = self.stored.foundings.insert(digest.clone(), founding);
        if let Err(error) = self.save() {
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
mod tests {
    use super::*;

    fn founding(email: &str, operation: &str) -> Founding {
        Founding {
            organisation: "organisation-1".to_owned(),
            founder: "person-1".to_owned(),
            email: email.to_owned(),
            operation: operation.to_owned(),
            spent: false,
            withdrawn: false,
            withdrawal_event: None,
            handed: false,
            kind: Kind::Founding,
            continue_to: None,
        }
    }

    #[test]
    fn a_token_is_thirty_two_bytes_and_only_its_digest_is_stored() -> Result<(), ServerError> {
        let token = new_token()?;
        assert_eq!(URL_SAFE_NO_PAD.decode(&token).map(|b| b.len()), Ok(32));
        assert_ne!(new_token()?, token);
        let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
        let file = dir.path().join("foundings.json");
        let mut foundings = Foundings::open(file.clone())?;
        foundings.record(digest(&token), founding("ada@example.test", "op-1"), None)?;
        let written = std::fs::read_to_string(&file).map_err(|e| unusable(&file, &e))?;
        assert!(!written.contains(&token), "the token is in the file");
        assert!(written.contains(&digest(&token)));
        let reopened = Foundings::open(file)?;
        assert_eq!(
            reopened.by_digest(&digest(&token)),
            Some(&founding("ada@example.test", "op-1"))
        );
        assert_eq!(
            reopened
                .unspent_for_email("ada@example.test")
                .map(|(d, _)| d),
            Some(digest(&token).as_str())
        );
        Ok(())
    }

    #[test]
    fn a_second_founding_for_the_address_replaces_the_first_in_one_write() -> Result<(), ServerError>
    {
        let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
        let file = dir.path().join("foundings.json");
        let mut foundings = Foundings::open(file.clone())?;
        foundings.record("d1".to_owned(), founding("ada@example.test", "op-1"), None)?;
        let (old, _) = foundings
            .unspent_for_email("ada@example.test")
            .map(|(d, f)| (d.to_owned(), f.clone()))
            .ok_or(ServerError::ProofRefused)?;
        foundings.record(
            "d2".to_owned(),
            founding("ada@example.test", "op-1"),
            Some(&old),
        )?;
        assert!(foundings.by_digest("d1").is_none());
        assert_eq!(
            foundings
                .unspent_for_email("ada@example.test")
                .map(|(d, _)| d),
            Some("d2")
        );
        foundings.spend("d2")?;
        assert!(foundings.unspent_for_email("ada@example.test").is_none());
        assert!(foundings.by_digest("d2").is_some_and(|f| f.spent));
        assert!(matches!(
            foundings.spend("d9"),
            Err(ServerError::ProofRefused)
        ));
        let reopened = Foundings::open(file)?;
        assert!(reopened.by_digest("d2").is_some_and(|f| f.spent));
        Ok(())
    }

    #[test]
    fn a_file_of_another_format_is_refused() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let file = dir.path().join("foundings.json");
        std::fs::write(&file, br#"{"format":"other/1","foundings":{}}"#).expect("written");
        assert!(matches!(
            Foundings::open(file),
            Err(ServerError::DirectoryUnavailable { .. })
        ));
    }

    #[test]
    fn withdrawing_an_admission_keeps_its_record_and_a_later_link_works() -> Result<(), ServerError> {
        let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
        let file = dir.path().join("foundings.json");
        let mut held = Foundings::open(file.clone())?;
        let mut admission = founding("ada@example.test", "op-1");
        admission.kind = Kind::Admission;
        held.record("d1".to_owned(), admission.clone(), None)?;
        held.withdraw("d1", vec![1, 2, 3])?;
        assert!(held.unspent_admission_for("person-1").is_none());
        assert!(held.by_digest("d1").is_some_and(|f| f.withdrawn));
        assert_eq!(
            held.by_digest("d1").and_then(|f| f.withdrawal_event.as_deref()),
            Some([1, 2, 3].as_slice())
        );
        let mut held = Foundings::open(file)?;
        assert!(held.by_digest("d1").is_some_and(|f| f.withdrawn));
        held.record("d2".to_owned(), admission, None)?;
        assert_eq!(held.unspent_admission_for("person-1"), Some("d2"));
        assert!(held.by_digest("d1").is_some_and(|f| f.withdrawn));
        assert!(matches!(
            held.withdraw("d1", vec![1, 2, 3]),
            Err(ServerError::NoUnspentAdmission)
        ));
        Ok(())
    }

    #[test]
    fn a_refused_withdrawal_save_leaves_memory_and_disk_whole() -> Result<(), ServerError> {
        let dir = tempfile::tempdir().map_err(|e| unusable(Path::new("tmp"), &e))?;
        let file = dir.path().join("foundings.json");
        let mut held = Foundings::open(file.clone())?;
        let mut admission = founding("ada@example.test", "op-1");
        admission.kind = Kind::Admission;
        held.record("d1".to_owned(), admission, None)?;
        let before = std::fs::read(&file).map_err(|e| unusable(&file, &e))?;
        let writable = held.file.clone();
        held.file = file.join("refused");
        assert!(matches!(
            held.withdraw("d1", vec![1, 2, 3]),
            Err(ServerError::DirectoryUnavailable { .. })
        ));
        assert_eq!(held.unspent_admission_for("person-1"), Some("d1"));
        assert!(held
            .by_digest("d1")
            .is_some_and(|f| !f.withdrawn && f.withdrawal_event.is_none()));
        assert_eq!(
            std::fs::read(&writable).map_err(|e| unusable(&writable, &e))?,
            before
        );
        assert_eq!(
            Foundings::open(writable)?.unspent_admission_for("person-1"),
            Some("d1")
        );
        Ok(())
    }
}
