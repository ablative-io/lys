//! Another computer joins Lys from the browser, with a connection code that
//! works once.
//!
//! - `POST /network/machines/{id}/join-code` `{"operation"}`: the
//!   administrator, under the action that names a computer's runner
//!   (`machine.runner.set`), asks for a connection code for the computer
//!   `id`. The answer is given once: `{"machine", "server", "command",
//!   "code"}`, the address the computer's runner dials, the one line to run
//!   on that computer, and the code that line asks for. A code is never
//!   shown again, so the same operation sent again is refused
//!   `RunnerJoinOperationReused`; a new code needs a new operation, and
//!   replaces the computer's earlier code, which stops working. While this
//!   Lys is served only on an address of its own computer, or over plain
//!   http, another computer cannot reach it, and no code is given:
//!   `RunnerJoinUnreachable`, in plain words.
//! - `POST /runner/join` `{"machine", "code", "key"}`: public, admitted by
//!   the code alone. The computer made its own key pair and sends only the
//!   public half. A code that is right, and not used or replaced, records
//!   the computer's runner as dialled in with that key, is spent, and the
//!   answer is `{"machine", "server_key"}`, the public key every request to
//!   a runner is signed with. A code that is wrong, used or replaced is
//!   refused `RunnerJoinRefused`, which never says which.
//!
//! Only the SHA-256 digest of a code is kept, while it waits, beside the
//! operation that asked for it, who asked and when, and later when it was
//! used and with which key, or which operation replaced it: one record per
//! code, so every change names the person answerable for it. The records
//! are one file beside the machines' file, written whole and flushed before
//! anything is answered; when a write fails, the file is read again, so
//! memory never runs ahead of or behind it. A code has no expiry. It is
//! never logged, never kept whole and never placed in an address.

use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path as RoutePath, State};
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use lys_identity::OperationId;
use rand::{TryRngCore, rngs::OsRng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::config::Config;
use crate::error::ServerError;
use crate::error_machine::MachineError;
use crate::network_api::with_network;
use crate::routes::{AppState, signed_in, with_directory};
use crate::runner_client::RunnerRecord;
use crate::session::now;

/// The format the connection codes' file is written in.
const FORMAT: &str = "lys-runner-joins/v1";

/// How a connection code stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum JoinStanding {
    /// Given, and not yet used: only its digest is kept.
    Waiting {
        /// The SHA-256 digest of the code, as 64 lowercase hexadecimal
        /// characters.
        digest: String,
    },
    /// Used once, by the computer whose public key is named.
    Used {
        /// When, in seconds since the Unix epoch.
        at: u64,
        /// The computer's Ed25519 public key, as 64 hexadecimal characters.
        key: String,
    },
    /// Replaced by a newer code for the same computer, before it was used.
    Replaced {
        /// When, in seconds since the Unix epoch.
        at: u64,
        /// The operation that asked for the newer code.
        by: String,
    },
}

/// One connection code, as it is kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinRecord {
    /// The operation that asked for it.
    pub operation: String,
    /// The computer it connects.
    pub machine: String,
    /// The person who asked for it, answerable for the runner it connects.
    pub issued_by: String,
    /// When it was given, in seconds since the Unix epoch.
    pub issued_at: u64,
    /// How it stands now.
    pub standing: JoinStanding,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Kept {
    format: String,
    codes: Vec<JoinRecord>,
}

/// The connection codes, kept in one file.
pub struct JoinStore {
    path: PathBuf,
    kept: Kept,
}

fn unavailable(reason: impl Into<String>) -> ServerError {
    ServerError::NetworkUnavailable {
        reason: reason.into(),
    }
}

/// The SHA-256 digest of `code`, as 64 lowercase hexadecimal characters.
fn digest(code: &str) -> String {
    lys_runner::protocol::hex(&Sha256::digest(code.as_bytes()))
}

/// Whether `held` and `given` are the same bytes, compared in time that
/// does not depend on where they differ.
fn same(held: &[u8], given: &[u8]) -> bool {
    held.len() == given.len()
        && held
            .iter()
            .zip(given)
            .fold(0_u8, |differ, (x, y)| differ | (x ^ y))
            == 0
}

fn lowercase_hex(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Write `bytes` as the whole of `path`: written beside it, flushed, moved
/// over it, and the folder flushed, so the file holds the old bytes or the
/// new, never part of either.
fn replace(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let beside = path.with_extension("writing");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&beside)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&beside, path)?;
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => fs::File::open(parent)?.sync_all(),
        _ => Ok(()),
    }
}

/// The file the connection codes are kept in, beside the machines' file
/// `network`.
pub fn beside(network: &Path) -> PathBuf {
    network.with_extension("joins.json")
}

impl JoinStore {
    /// The connection codes kept beside the machines' file `config` names;
    /// none when it names none, since then no computer is kept either.
    pub fn configured(config: &Config) -> Result<Option<Self>, ServerError> {
        config
            .network_file
            .as_deref()
            .map(|network| Self::open(&beside(network)))
            .transpose()
    }

    /// The connection codes kept in the file `path`, none when it does not
    /// exist.
    pub fn open(path: &Path) -> Result<Self, ServerError> {
        let kept = match fs::read(path) {
            Ok(bytes) => {
                let kept: Kept = serde_json::from_slice(&bytes).map_err(|error| {
                    unavailable(format!(
                        "the connection codes in {} do not read, at line {}, column {}",
                        path.display(),
                        error.line(),
                        error.column()
                    ))
                })?;
                checked(path, kept)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Kept {
                format: FORMAT.to_owned(),
                codes: Vec::new(),
            },
            Err(error) => {
                return Err(unavailable(format!("reading {}: {error}", path.display())));
            }
        };
        Ok(Self {
            path: path.to_owned(),
            kept,
        })
    }

    /// Every connection code ever given, in the order given.
    pub fn records(&self) -> &[JoinRecord] {
        &self.kept.codes
    }

    /// Whether a code for `machine` waits to be used.
    pub fn waiting(&self, machine: &str) -> bool {
        self.kept.codes.iter().any(|record| {
            record.machine == machine && matches!(record.standing, JoinStanding::Waiting { .. })
        })
    }

    /// A new connection code for `machine`, asked for under `operation` by
    /// `by` at `at`, replacing any code for it still waiting. Only its
    /// digest is kept; the code itself is answered here once.
    pub fn issue(
        &mut self,
        operation: &str,
        machine: &str,
        by: &str,
        at: u64,
    ) -> Result<String, ServerError> {
        if self
            .kept
            .codes
            .iter()
            .any(|record| record.operation == operation)
        {
            return Err(MachineError::JoinOperationReused {
                operation: operation.to_owned(),
            }
            .into());
        }
        let mut random = [0_u8; 32];
        OsRng.try_fill_bytes(&mut random).map_err(|error| {
            unavailable(format!(
                "no random bytes could be read for a connection code: {error}"
            ))
        })?;
        let code = lys_runner::protocol::hex(&random);
        random.zeroize();
        let mut next = self.kept.clone();
        for record in &mut next.codes {
            if record.machine == machine && matches!(record.standing, JoinStanding::Waiting { .. })
            {
                record.standing = JoinStanding::Replaced {
                    at,
                    by: operation.to_owned(),
                };
            }
        }
        next.codes.push(JoinRecord {
            operation: operation.to_owned(),
            machine: machine.to_owned(),
            issued_by: by.to_owned(),
            issued_at: at,
            standing: JoinStanding::Waiting {
                digest: digest(&code),
            },
        });
        self.write(next)?;
        Ok(code)
    }

    /// Spend the code `code` for `machine` on the computer whose public key
    /// is `key`, at `at`, answering who asked for the code. A code that is
    /// wrong, used or replaced is refused `RunnerJoinRefused`, and which of
    /// these is never said.
    pub fn redeem(
        &mut self,
        machine: &str,
        code: &str,
        key: &str,
        at: u64,
    ) -> Result<String, ServerError> {
        let given = digest(code);
        let found = self
            .kept
            .codes
            .iter()
            .enumerate()
            .find_map(|(index, record)| match &record.standing {
                JoinStanding::Waiting { digest } if record.machine == machine => {
                    Some((index, digest.as_bytes()))
                }
                _ => None,
            });
        // A computer with no code waiting is compared all the same, so the
        // answer takes the same time whichever way the code is refused.
        let none = [0_u8; 64];
        let (index, held) = found.map_or((None, &none[..]), |(index, held)| (Some(index), held));
        let matched = same(held, given.as_bytes());
        let Some(index) = index.filter(|_| matched) else {
            return Err(MachineError::JoinRefused.into());
        };
        let mut next = self.kept.clone();
        let Some(record) = next.codes.get_mut(index) else {
            return Err(MachineError::JoinRefused.into());
        };
        record.standing = JoinStanding::Used {
            at,
            key: key.to_owned(),
        };
        let by = record.issued_by.clone();
        self.write(next)?;
        Ok(by)
    }

    /// Keep `next`: written and flushed first, then held. When the write
    /// fails, what the file holds is read again and held, and the failure
    /// is answered by name.
    fn write(&mut self, next: Kept) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec_pretty(&next).map_err(|error| {
            unavailable(format!(
                "the connection codes do not write as JSON: {error}"
            ))
        })?;
        match replace(&self.path, &bytes) {
            Ok(()) => {
                self.kept = next;
                Ok(())
            }
            Err(failure) => {
                self.kept = Self::open(&self.path)?.kept;
                Err(unavailable(format!(
                    "writing {}: {failure}",
                    self.path.display()
                )))
            }
        }
    }
}

/// `kept`, read from `path`, refused by name when it is not this format or
/// holds what no write of this store makes.
fn checked(path: &Path, kept: Kept) -> Result<Kept, ServerError> {
    let refused = |what: &str| unavailable(format!("{} {what}", path.display()));
    if kept.format != FORMAT {
        return Err(refused(&format!("is not in the format {FORMAT}")));
    }
    let mut operations = std::collections::BTreeSet::new();
    let mut waiting = std::collections::BTreeSet::new();
    for record in &kept.codes {
        if !operations.insert(record.operation.as_str()) {
            return Err(refused("names one operation for two connection codes"));
        }
        if let JoinStanding::Waiting { digest } = &record.standing {
            if !lowercase_hex(digest) {
                return Err(refused("holds a code digest that is not a digest"));
            }
            if !waiting.insert(record.machine.as_str()) {
                return Err(refused("holds two waiting codes for one computer"));
            }
        }
    }
    Ok(kept)
}

/// Act on the connection codes.
fn with_joins<T>(
    state: &AppState,
    act: impl FnOnce(&mut JoinStore) -> Result<T, ServerError>,
) -> Result<T, ServerError> {
    let store = state.joins.as_ref().ok_or_else(|| {
        unavailable(
            "the configuration names no network_file, beside which connection codes are kept",
        )
    })?;
    let mut store = store
        .lock()
        .map_err(|error| unavailable(format!("the connection codes' lock is poisoned: {error}")))?;
    act(&mut store)
}

/// Whether a connection code for `machine` waits to be used.
pub(crate) fn joining(state: &AppState, machine: &str) -> Result<bool, ServerError> {
    with_joins(state, |joins| Ok(joins.waiting(machine)))
}

/// A request for a connection code.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct JoinCodeBody {
    /// The operation id naming this request; a code is never answered twice.
    operation: String,
}

/// A connection code, answered once.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct JoinCodeGiven {
    /// The computer it connects.
    machine: String,
    /// The address the computer's runner dials.
    server: String,
    /// The one line to run on that computer; it asks for the code.
    command: String,
    /// The code: it works once, and Lys does not show it again.
    code: String,
}

/// A computer joining with its code.
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunnerJoinBody {
    /// The computer joining, as Lys names it.
    machine: String,
    /// The connection code Lys gave for it.
    code: String,
    /// The computer's own Ed25519 public key, as 64 hexadecimal characters.
    key: String,
}

/// A computer joined.
#[derive(Serialize, utoipa::ToSchema)]
pub(crate) struct RunnerJoined {
    /// The computer that joined.
    machine: String,
    /// The Ed25519 public key every request to its runner is signed with,
    /// as 64 hexadecimal characters.
    server_key: String,
}

/// The connection code routes.
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/network/machines/{id}/join-code", post(issue))
        .route("/runner/join", post(join))
}

fn malformed(rejection: &JsonRejection) -> ServerError {
    ServerError::RequestMalformed {
        reason: rejection.body_text(),
    }
}

/// Where a runner on another computer dials this Lys: this service's public
/// address as the configuration names it, the callback's origin and folder.
/// Refused by name where another computer cannot reach it: an address on
/// this computer alone, or plain http, which a dialling runner speaks only
/// to its own computer.
fn dial_address(redirect: &str) -> Result<String, ServerError> {
    let refused = |reason: String| ServerError::from(MachineError::JoinUnreachable { reason });
    let url = reqwest::Url::parse(redirect).map_err(|error| {
        refused(format!(
            "this Lys's own address, {redirect}, does not read as an address: {error}"
        ))
    })?;
    let origin = url.origin().ascii_serialization();
    let host = url.host_str().unwrap_or_default();
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    if bare.is_empty()
        || bare.eq_ignore_ascii_case("localhost")
        || bare
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    {
        return Err(refused(format!(
            "this Lys is served only at {origin}, an address on the computer it runs on, so another computer cannot reach it. Serve Lys at an address the other computer can reach, over https, and ask again"
        )));
    }
    if url.scheme() != "https" {
        return Err(refused(format!(
            "this Lys is served at {origin} over plain http, and another computer connects its runner to Lys only over https. Serve Lys over https and ask again"
        )));
    }
    let folder = url
        .path()
        .rsplit_once('/')
        .map_or("", |(folder, _callback)| folder);
    Ok(format!("{origin}{folder}"))
}

/// `text` as one word of a shell command: as it is when every character is
/// plain, otherwise in single quotes.
fn word(text: &str) -> String {
    let plain =
        |character: char| character.is_ascii_alphanumeric() || "-_./:@%+=,".contains(character);
    if !text.is_empty() && text.chars().all(plain) {
        text.to_owned()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

async fn issue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    RoutePath(id): RoutePath<String>,
    given: Result<Json<JoinCodeBody>, JsonRejection>,
) -> Result<Json<JoinCodeGiven>, ServerError> {
    let actor = signed_in(&state, &headers)?;
    crate::routes::administrator(&state, &actor)?;
    let Json(body) = given.map_err(|rejection| malformed(&rejection))?;
    let operation = OperationId::from_str(&body.operation)?.to_string();
    let by = with_directory(&state, |directory| {
        Ok(crate::read_api::own_person(directory.projection()?, &actor)?.to_string())
    })?;
    with_network(&state, |store| {
        let machine = store.machine(&id).ok_or(ServerError::MachineUnknown)?;
        if machine.retired.is_some() {
            return Err(ServerError::MachineRetired);
        }
        Ok(())
    })?;
    let server = dial_address(state.oidc.redirect_url())?;
    let code = with_joins(&state, |joins| joins.issue(&operation, &id, &by, now()))?;
    let command = format!(
        "lys runner join --server {} --machine {}",
        word(&server),
        word(&id)
    );
    Ok(Json(JoinCodeGiven {
        machine: id,
        server,
        command,
        code,
    }))
}

async fn join(
    State(state): State<Arc<AppState>>,
    given: Result<Json<RunnerJoinBody>, JsonRejection>,
) -> Result<Json<RunnerJoined>, ServerError> {
    let Json(mut body) = given.map_err(|rejection| malformed(&rejection))?;
    let record = RunnerRecord::Dialled {
        key: body.key.clone(),
        runner: None,
    }
    .checked();
    let redeemed = record.and_then(|record| {
        with_joins(&state, |joins| {
            joins.redeem(&body.machine, &body.code, &body.key, now())
        })
        .map(|by| (record, by))
    });
    body.code.zeroize();
    let (record, by) = redeemed?;
    with_network(&state, |store| {
        store.name_runner(&body.machine, Some(record))
    })?;
    (state.say)(&format!(
        "computer {} joined with key {}, under a connection code given by {by}",
        body.machine, body.key
    ));
    Ok(Json(RunnerJoined {
        machine: body.machine,
        server_key: lys_runner::protocol::hex(&state.runners.public_key()),
    }))
}
