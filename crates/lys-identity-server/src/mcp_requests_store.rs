//! Requests grant no access. Each operation keeps one signed leaf, pinned before
//! its answer, with signed snapshots for restart and read-back after uncertain writes.

use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_core::attestation::{sign_attestation, verify_attestation_bytes_by_signer};
use lys_identity::{AgentId, OperationId, PersonId, SNAPSHOT_EVERY};
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, Tail,
    open_with_snapshot,
};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::ServerError;
use crate::routes::Say;

const ORIGIN: &str = "lys/identity/mcp-requests";
const DOMAIN: &str = "lys/identity/mcp-requests-state/v1";
const RECORD_DOMAIN: &str = "lys/identity/mcp-request/v1";

/// A request has no approval or grant attached to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpRequestState {
    /// The request has been recorded and awaits a decision.
    Pending,
}

/// One request against an already reviewed provisioning version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct McpRequest {
    /// The operation id that names the request.
    pub id: String,
    /// The agent requesting the server.
    pub agent: String,
    /// The declared server's name.
    pub server: String,
    /// The most recent reviewed version when the request was recorded.
    pub profile_version: u32,
    /// Recording a request leaves it pending.
    pub state: McpRequestState,
    /// The authenticated identity that asked.
    pub asked_by: String,
    /// When the request was recorded, in seconds since the Unix epoch.
    pub asked_at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed {
    request: McpRequest,
    attestation: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    requests: Vec<McpRequest>,
}

/// Reopen a leaf store after an append whose outcome is uncertain.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;
type Opened<S> = (FrontierLog<S>, Vec<McpRequest>, Start);

/// The signed, append-only requests, kept separately from provisioning profiles.
pub struct McpRequestStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    requests: Vec<McpRequest>,
    start: Start,
    since_snapshot: u64,
    uncertain: bool,
}

fn unavailable(reason: impl std::fmt::Display) -> ServerError {
    ServerError::McpRequestsUnavailable {
        reason: reason.to_string(),
    }
}

/// Validate the same bounded names provisioning declarations carry.
pub(crate) fn server_name(name: &str) -> Result<String, ServerError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(ServerError::RequestMalformed {
            reason: "server must carry between 1 and 100 characters".to_owned(),
        });
    }
    Ok(name.to_owned())
}

fn validate(request: &McpRequest) -> Result<(), ServerError> {
    OperationId::from_str(&request.id).map_err(unavailable)?;
    AgentId::from_str(&request.agent).map_err(unavailable)?;
    if PersonId::from_str(&request.asked_by).is_err() {
        AgentId::from_str(&request.asked_by).map_err(unavailable)?;
    }
    if request.profile_version == 0 || server_name(&request.server)? != request.server {
        return Err(unavailable(
            "the request has no reviewed version or canonical server name",
        ));
    }
    Ok(())
}

fn payload(request: &McpRequest) -> Result<Vec<u8>, ServerError> {
    let mut bytes = RECORD_DOMAIN.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend(serde_json::to_vec(request).map_err(unavailable)?);
    Ok(bytes)
}

impl McpRequestStore<FileLeafStore> {
    /// Open a sibling of the configured access-request log, without changing configuration.
    pub fn configured(
        config: &Config,
        key: Arc<Ed25519Identity>,
        say: &Say,
    ) -> Result<Option<Self>, ServerError> {
        let Some(dir) = config.requests_dir.as_deref() else {
            return Ok(None);
        };
        let store = Self::open(&dir.with_file_name("mcp-requests"), key)?;
        say(&format!(
            "mcp-requests log {}, holding {} requests",
            store.start,
            store.requests.len()
        ));
        Ok(Some(store))
    }

    /// Open the log, creating it when it is first used by an existing install.
    pub fn open(dir: &Path, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        if !dir.exists() {
            FileLeafStore::create(dir, ORIGIN).map_err(unavailable)?;
        }
        let dir = dir.to_owned();
        Self::over(Box::new(move || FileLeafStore::open(&dir)), key)
    }
}

impl<S: LeafStore> McpRequestStore<S> {
    /// How the log was opened, including the reason for rebuilding from signed leaves.
    pub fn start(&self) -> &Start {
        &self.start
    }

    /// Open pinned leaves and their signed snapshot, verifying every replayed record.
    pub fn over(reopen: Reopen<S>, key: Arc<Ed25519Identity>) -> Result<Self, ServerError> {
        let (log, requests, start) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            log,
            requests,
            start,
            since_snapshot: 0,
            uncertain: false,
        };
        store.after_start()?;
        Ok(store)
    }

    fn after_start(&mut self) -> Result<(), ServerError> {
        match self.start {
            Start::Resumed { replayed, .. } => self.since_snapshot = replayed,
            Start::Rebuilt { .. } => self.write_snapshot()?,
        }
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot()?;
        }
        Ok(())
    }

    fn write_snapshot(&mut self) -> Result<(), ServerError> {
        let bytes = serde_json::to_vec(&Sealed {
            format: DOMAIN.to_owned(),
            requests: self.requests.clone(),
        })
        .map_err(unavailable)?;
        self.log
            .write_snapshot(DOMAIN, &bytes, &self.key)
            .map_err(unavailable)?;
        self.since_snapshot = 0;
        Ok(())
    }

    /// Resolve an uncertain append before answering any read or write from memory.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, requests, start) = opened(&self.reopen, &self.key)?;
            self.log = log;
            self.requests = requests;
            self.start = start;
            self.after_start()?;
            self.uncertain = false;
        }
        Ok(())
    }

    /// Requests for an agent, in the order recorded, after settling any uncertain write.
    pub fn listed(&mut self, agent: &str) -> Result<Vec<McpRequest>, ServerError> {
        self.settle()?;
        Ok(self
            .requests
            .iter()
            .filter(|request| request.agent == agent)
            .cloned()
            .collect())
    }

    /// Replay an operation only for the same authenticated caller, agent and server.
    pub fn replay(
        &mut self,
        id: &str,
        agent: &str,
        server: &str,
        by: &str,
    ) -> Result<Option<McpRequest>, ServerError> {
        self.settle()?;
        match self.requests.iter().find(|request| request.id == id) {
            Some(request)
                if request.agent == agent && request.server == server && request.asked_by == by =>
            {
                Ok(Some(request.clone()))
            }
            Some(_) => Err(ServerError::RequestReused {
                request: id.to_owned(),
            }),
            None => Ok(None),
        }
    }

    /// Append one signed request; a failed append is accepted only after exact read-back.
    pub fn ask(&mut self, request: McpRequest) -> Result<McpRequest, ServerError> {
        validate(&request)?;
        if let Some(kept) = self.replay(
            &request.id,
            &request.agent,
            &request.server,
            &request.asked_by,
        )? {
            return Ok(kept);
        }
        let attestation = sign_attestation(&payload(&request)?, &self.key).to_cose_bytes();
        let bytes = serde_json::to_vec(&Signed {
            request: request.clone(),
            attestation,
        })
        .map_err(unavailable)?;
        let index = self.log.len();
        if let Err(failure) = self.log.append(&bytes) {
            self.uncertain = true;
            self.settle()?;
            match self.log.leaf_bytes(index).map_err(unavailable)? {
                Some(held) if held == bytes => return Ok(request),
                Some(_) => {
                    return Err(unavailable(format!(
                        "leaf {index} belongs to another writer: {failure}"
                    )));
                }
                None => return Err(unavailable(failure)),
            }
        }
        self.requests.push(request.clone());
        self.since_snapshot += 1;
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot()?;
        }
        Ok(request)
    }
}

fn fold(
    requests: &mut Vec<McpRequest>,
    tail: &Tail,
    key: &Ed25519Identity,
) -> Result<(), ServerError> {
    for (index, bytes) in (tail.from..).zip(&tail.leaves) {
        let signed: Signed = serde_json::from_slice(bytes)
            .map_err(|error| unavailable(format!("leaf {index} is not an MCP request: {error}")))?;
        verify_attestation_bytes_by_signer(
            &signed.attestation,
            &payload(&signed.request)?,
            &key.public_key_bytes(),
        )
        .map_err(|error| unavailable(format!("leaf {index} signature: {error}")))?;
        validate(&signed.request).map_err(unavailable)?;
        if requests
            .iter()
            .any(|request| request.id == signed.request.id)
        {
            return Err(unavailable(format!(
                "leaf {index} repeats an MCP request operation"
            )));
        }
        requests.push(signed.request);
    }
    Ok(())
}

fn decoded(bytes: &[u8], count: u64) -> Result<Vec<McpRequest>, ServerError> {
    let sealed: Sealed = serde_json::from_slice(bytes).map_err(unavailable)?;
    if sealed.format != DOMAIN
        || u64::try_from(sealed.requests.len()).map_err(unavailable)? != count
    {
        return Err(unavailable(
            "the snapshot's MCP request format or count differs from its log",
        ));
    }
    let mut ids = BTreeSet::new();
    for request in &sealed.requests {
        validate(request).map_err(unavailable)?;
        if !ids.insert(&request.id) {
            return Err(unavailable("the snapshot repeats an MCP request operation"));
        }
    }
    Ok(sealed.requests)
}

fn opened<S: LeafStore>(
    reopen: &Reopen<S>,
    key: &Ed25519Identity,
) -> Result<Opened<S>, ServerError> {
    let started = open_with_snapshot(
        reopen().map_err(unavailable)?,
        DOMAIN,
        &key.public_key_bytes(),
    )
    .map_err(unavailable)?;
    let mut requests = match started
        .state
        .as_deref()
        .map(|bytes| decoded(bytes, started.tail.from))
    {
        None => Vec::new(),
        Some(Ok(requests)) => requests,
        Some(Err(reason)) => {
            let (log, tail) =
                FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
            let mut requests = Vec::new();
            fold(&mut requests, &tail, key)?;
            let replayed = log.len();
            return Ok((
                log,
                requests,
                Start::Rebuilt {
                    refusal: SnapshotRefusal::StateUnreadable {
                        reason: reason.to_string(),
                    },
                    replayed,
                },
            ));
        }
    };
    fold(&mut requests, &started.tail, key)?;
    Ok((started.log, requests, started.start))
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use identity_contract::harness::{Fault, Harness};

    use super::*;

    type TestResult = Result<(), Box<dyn Error>>;

    fn request() -> Result<McpRequest, Box<dyn Error>> {
        Ok(McpRequest {
            id: OperationId::generate()?.to_string(),
            agent: AgentId::generate()?.to_string(),
            server: "declared-server".to_owned(),
            profile_version: 1,
            state: McpRequestState::Pending,
            asked_by: PersonId::generate()?.to_string(),
            asked_at: 1,
        })
    }

    #[test]
    fn a_failed_append_is_read_back_before_memory_answers() -> TestResult {
        for fault in [
            Fault::BeforeLeaf,
            Fault::LeafStoredWriteFailed,
            Fault::AfterLeaf,
            Fault::AfterLeafUnreadable,
        ] {
            let harness = Harness::new(19)?;
            let key = Arc::new(Ed25519Identity::load(
                &harness.dir.path().join("service.key"),
            )?);
            let mut store = McpRequestStore::over(harness.leaves(), Arc::clone(&key))?;
            let asked = request()?;
            harness.fail(fault);
            let answer = store.ask(asked.clone());
            match fault {
                Fault::BeforeLeaf => {
                    assert!(matches!(
                        answer,
                        Err(ServerError::McpRequestsUnavailable { .. })
                    ));
                    assert!(store.listed(&asked.agent)?.is_empty());
                }
                Fault::AfterLeafUnreadable => {
                    assert!(matches!(
                        answer,
                        Err(ServerError::McpRequestsUnavailable { .. })
                    ));
                    assert!(matches!(
                        store.listed(&asked.agent),
                        Err(ServerError::McpRequestsUnavailable { .. })
                    ));
                    harness.fail(Fault::None);
                }
                Fault::LeafStoredWriteFailed | Fault::AfterLeaf => assert_eq!(answer?, asked),
                Fault::None => return Err("the case must inject an append fault".into()),
            }
            assert_eq!(store.ask(asked.clone())?, asked);
            assert_eq!(store.listed(&asked.agent)?, vec![asked.clone()]);
            assert_eq!(store.log.len(), 1);
            drop(store);
            let mut reopened = McpRequestStore::over(harness.leaves(), key)?;
            assert_eq!(reopened.listed(&asked.agent)?, vec![asked]);
        }
        Ok(())
    }

    #[test]
    fn an_unreadable_snapshot_rebuilds_signed_leaves_and_names_the_reason() -> TestResult {
        let harness = Harness::new(20)?;
        let key = Arc::new(Ed25519Identity::load(
            &harness.dir.path().join("service.key"),
        )?);
        let mut store = McpRequestStore::over(harness.leaves(), Arc::clone(&key))?;
        let asked = request()?;
        store.ask(asked.clone())?;
        store.log.write_snapshot(DOMAIN, b"not a state", &key)?;
        drop(store);
        let mut reopened = McpRequestStore::over(harness.leaves(), key)?;
        assert!(matches!(
            reopened.start,
            Start::Rebuilt {
                refusal: SnapshotRefusal::StateUnreadable { .. },
                replayed: 1
            }
        ));
        assert_eq!(reopened.listed(&asked.agent)?, vec![asked]);
        Ok(())
    }

    #[test]
    fn a_changed_record_or_another_signer_is_refused_before_replay() -> TestResult {
        let harness = Harness::new(21)?;
        let key = Ed25519Identity::load(&harness.dir.path().join("service.key"))?;
        let other = Harness::new(22)?;
        let other_key = Ed25519Identity::load(&other.dir.path().join("service.key"))?;
        let asked = request()?;
        let mut changed = asked.clone();
        changed.server = "another-server".to_owned();
        let original_signature = sign_attestation(&payload(&asked)?, &key).to_cose_bytes();
        let other_signature = sign_attestation(&payload(&asked)?, &other_key).to_cose_bytes();
        for signed in [
            Signed {
                request: changed,
                attestation: original_signature,
            },
            Signed {
                request: asked,
                attestation: other_signature,
            },
        ] {
            let tail = Tail {
                from: 0,
                leaves: vec![serde_json::to_vec(&signed)?],
            };
            let mut kept = Vec::new();
            assert!(matches!(
                fold(&mut kept, &tail, &key),
                Err(ServerError::McpRequestsUnavailable { .. })
            ));
            assert!(kept.is_empty());
        }
        Ok(())
    }
}
