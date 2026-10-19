//! Operation checkpoints retain one encrypted connection outcome across retries and restart.
//!
//! Beside each outcome, `<code digest>.code` names the operation a code
//! opens, and `<app digest>.app` the app's latest connect, so a newer connect
//! marks the one still waiting for its exchange replaced in one read.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use lys_core::Ed25519Identity;
use lys_core::attestation::Attestation;
use lys_core::seal::{SealedEnvelope, open_and_verify, sign_and_seal};
use lys_identity::OperationId;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::{CODE_LIFETIME_MS, Connected, Exchanged, unavailable};
use crate::apps_state::By;
use crate::error::ServerError;
use crate::routes::AppState;

const FORMAT: &str = "lys/app-connection/v1";
type Gates = Mutex<BTreeMap<PathBuf, Weak<tokio::sync::Mutex<()>>>>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
    format: String,
    pub(super) request: String,
    pub(super) operation: String,
    pub(super) app: String,
    pub(super) issuer: String,
    pub(super) at: u64,
    pub(super) prior: Vec<String>,
    pub(super) client: Option<(String, Option<String>)>,
    pub(super) bearer: Option<(String, Option<String>)>,
    pub(super) code: Option<String>,
    pub(super) connected: Option<Connected>,
    spent: bool,
    /// The administrator who approved, and their person, whom the exchange
    /// acts as; absent from an outcome kept before DIRECTORY-102.
    #[serde(default)]
    pub(super) acting: Option<(By, String)>,
    /// Whether the values were handed over by an exchange.
    #[serde(default)]
    pub(super) handed: bool,
    /// Whether a newer connect of the app replaced this one before its exchange.
    #[serde(default)]
    replaced: bool,
    /// Whether every prior credential still live at the hand-over is revoked.
    #[serde(default)]
    pub(super) prior_revoked: bool,
}

/// What opening a code finds: the outcome to hand over, or the refusal's
/// name with the outcome when it was already handed over.
pub(super) enum Opened {
    Ready(Record),
    Refused(&'static str, Option<Record>),
}

impl Record {
    pub(super) fn new(
        request: String,
        operation: String,
        app: String,
        issuer: String,
        prior: Vec<String>,
        at: u64,
    ) -> Self {
        Self {
            format: FORMAT.to_owned(),
            request,
            operation,
            app,
            issuer,
            prior,
            at,
            client: None,
            bearer: None,
            code: None,
            connected: None,
            spent: false,
            acting: None,
            handed: false,
            replaced: false,
            prior_revoked: false,
        }
    }

    /// The record acting as administrator `by`, the person `person`.
    pub(super) fn acting_as(mut self, by: By, person: String) -> Self {
        self.acting = Some((by, person));
        self
    }

    fn answer(&self) -> Result<Exchanged, ServerError> {
        let client_secret = self
            .client
            .as_ref()
            .and_then(|(_, value)| value.clone())
            .ok_or_else(|| {
                unavailable("the connection checkpoint has no client credential value")
            })?;
        let workspace_credential = self
            .bearer
            .as_ref()
            .and_then(|(_, value)| value.clone())
            .ok_or_else(|| {
                unavailable("the connection checkpoint has no workspace credential value")
            })?;
        Ok(Exchanged {
            app: self.app.clone(),
            issuer: self.issuer.clone(),
            client_id: self.app.clone(),
            client_secret,
            workspace_credential,
        })
    }

    fn consume(&mut self) {
        self.spent = true;
        if let Some((_, value)) = &mut self.client {
            *value = None;
        }
        if let Some((_, value)) = &mut self.bearer {
            *value = None;
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    envelope: SealedEnvelope,
    signature: Vec<u8>,
}

pub(super) struct Checkpoint {
    root: PathBuf,
    key: Ed25519Identity,
}

fn failed(context: &str, error: impl std::fmt::Display) -> ServerError {
    unavailable(&format!(
        "the connect checkpoint could not {context}: {error}"
    ))
}

pub(super) fn json_failed(context: &str, error: &serde_json::Error) -> ServerError {
    // Parser messages can quote credential values; retain category and position instead.
    unavailable(&format!(
        "the connect checkpoint could not {context}: {:?} at line {}, column {}",
        error.classify(),
        error.line(),
        error.column(),
    ))
}

fn read(path: &Path) -> Result<Option<Vec<u8>>, ServerError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failed("inspect its file", error)),
    };
    if !metadata.is_file() || metadata.len() > 65_536 || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(unavailable(
            "the connect checkpoint is not a bounded owner-only file",
        ));
    }
    fs::read(path)
        .map(Some)
        .map_err(|error| failed("read its file", error))
}

impl Checkpoint {
    pub(super) fn open(state: &AppState) -> Result<Self, ServerError> {
        let root = state.grant_setup.log_dir.with_file_name("app-connections");
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&root)
            .map_err(|error| failed("open its directory", error))?;
        let metadata =
            fs::symlink_metadata(&root).map_err(|error| failed("inspect its directory", error))?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(unavailable(
                "the connect checkpoint is not an owner-only directory",
            ));
        }
        let key = Ed25519Identity::load(&state.grant_setup.key_file)
            .map_err(|error| failed("load its encryption identity", error))?;
        Ok(Self { root, key })
    }

    pub(super) async fn gate(&self) -> Result<tokio::sync::OwnedMutexGuard<()>, ServerError> {
        static GATES: OnceLock<Gates> = OnceLock::new();
        let gate = {
            let mut gates = GATES
                .get_or_init(Mutex::default)
                .lock()
                .map_err(|error| failed("lock its operation", error))?;
            gates.retain(|_, gate| gate.strong_count() > 0);
            if let Some(gate) = gates.get(&self.root).and_then(Weak::upgrade) {
                gate
            } else {
                let gate = Arc::new(tokio::sync::Mutex::new(()));
                gates.insert(self.root.clone(), Arc::downgrade(&gate));
                gate
            }
        };
        Ok(gate.lock_owned().await)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), ServerError> {
        let mut file = tempfile::NamedTempFile::new_in(&self.root)
            .map_err(|error| failed("create its replacement", error))?;
        file.write_all(bytes)
            .map_err(|error| failed("write its replacement", error))?;
        file.as_file()
            .sync_all()
            .map_err(|error| failed("flush its replacement", error))?;
        file.persist(path)
            .map_err(|error| failed("publish its replacement", error.error))?;
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| failed("flush its directory", error))
    }

    pub(super) fn load(&self, operation: &str) -> Result<Option<Record>, ServerError> {
        let Some(bytes) = read(&self.root.join(format!("{operation}.json")))? else {
            return Ok(None);
        };
        let sealed: Sealed = serde_json::from_slice(&bytes)
            .map_err(|error| json_failed("read its envelope", &error))?;
        let signature = Attestation::from_cose_bytes(&sealed.signature)
            .map_err(|error| failed("read its signature", error))?;
        let payload = Zeroizing::new(
            open_and_verify(
                &sealed.envelope,
                &signature,
                &self.key.public_key_bytes(),
                &self.key.x25519_static_secret(),
            )
            .map_err(|error| failed("authenticate and decrypt its outcome", error))?,
        );
        let record: Record = serde_json::from_slice(&payload)
            .map_err(|error| json_failed("read its outcome", &error))?;
        if record.format != FORMAT || record.operation != operation {
            return Err(unavailable(
                "the connect checkpoint's format or operation does not match",
            ));
        }
        Ok(Some(record))
    }

    pub(super) fn save(&self, record: &Record) -> Result<(), ServerError> {
        let bytes = Zeroizing::new(
            serde_json::to_vec(record)
                .map_err(|error| json_failed("encode its outcome", &error))?,
        );
        let (envelope, signature) = sign_and_seal(&bytes, &self.key, &self.key.x25519_public_key())
            .map_err(|error| failed("encrypt and sign its outcome", error))?;
        let sealed = serde_json::to_vec(&Sealed {
            envelope,
            signature: signature.to_cose_bytes(),
        })
        .map_err(|error| json_failed("encode its envelope", &error))?;
        self.write(
            &self.root.join(format!("{}.json", record.operation)),
            &sealed,
        )
    }

    pub(super) fn index(&self, record: &Record) -> Result<(), ServerError> {
        let code = record
            .code
            .as_ref()
            .ok_or_else(|| unavailable("the completed connection has no exchange code"))?;
        let path = self
            .root
            .join(format!("{}.code", crate::apps_binding::sha256_hex(code)));
        match read(&path)? {
            Some(held) if held == record.operation.as_bytes() => Ok(()),
            Some(_) => Err(unavailable(
                "the connect code already indexes another operation",
            )),
            None => self.write(&path, record.operation.as_bytes()),
        }
    }

    /// Mark the app's connect still waiting for its exchange, when another
    /// than `record`'s, replaced, and name `record` the app's latest.
    pub(super) fn replace_waiting(&self, record: &Record) -> Result<(), ServerError> {
        let path = self.root.join(format!(
            "{}.app",
            crate::apps_binding::sha256_hex(&record.app)
        ));
        if let Some(held) = read(&path)? {
            let earlier = std::str::from_utf8(&held)
                .ok()
                .and_then(|operation| OperationId::from_str(operation).ok())
                .ok_or_else(|| unavailable("the app's connect index is malformed"))?
                .to_string();
            if earlier != record.operation
                && let Some(mut waiting) = self.load(&earlier)?
                && !waiting.spent
            {
                waiting.replaced = true;
                waiting.consume();
                self.save(&waiting)?;
            }
        }
        self.write(&path, record.operation.as_bytes())
    }

    /// Record `record`'s values handed over, answering them.
    pub(super) fn hand_over(&self, record: &mut Record) -> Result<Exchanged, ServerError> {
        let answer = record.answer()?;
        record.consume();
        record.handed = true;
        self.save(record)?;
        Ok(answer)
    }

    pub(super) fn open_code(&self, code: &str, at: u64) -> Result<Opened, ServerError> {
        let digest = crate::apps_binding::sha256_hex(code);
        let Some(operation) = read(&self.root.join(format!("{digest}.code")))? else {
            return Ok(Opened::Refused("connect_code_unknown", None));
        };
        let operation = std::str::from_utf8(&operation)
            .ok()
            .and_then(|operation| OperationId::from_str(operation).ok())
            .ok_or_else(|| unavailable("the connect code's operation index is malformed"))?
            .to_string();
        let mut record = self
            .load(&operation)?
            .ok_or_else(|| unavailable("the connect code's operation is absent"))?;
        if record.connected.is_none()
            || record
                .code
                .as_ref()
                .is_none_or(|code| crate::apps_binding::sha256_hex(code) != digest)
        {
            return Err(unavailable(
                "the connect code does not match its completed operation",
            ));
        }
        if record.replaced {
            return Ok(Opened::Refused("connect_code_replaced", None));
        }
        if at.saturating_sub(record.at) > CODE_LIFETIME_MS {
            if !record.spent {
                record.consume();
                self.save(&record)?;
            }
            return Ok(Opened::Refused("connect_code_expired", None));
        }
        if record.spent {
            return Ok(Opened::Refused("connect_code_used", Some(record)));
        }
        Ok(Opened::Ready(record))
    }
}
