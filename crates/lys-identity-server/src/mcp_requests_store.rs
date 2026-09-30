//! Requests grant no access. Each operation keeps one signed leaf, pinned before
//! its answer, with signed snapshots for restart and read-back after uncertain writes.

use std::path::Path;
use std::sync::Arc;

use lys_core::Ed25519Identity;
use lys_identity::SNAPSHOT_EVERY;
use lys_log_store::{
    FileLeafStore, FrontierLog, LeafStore, SnapshotRefusal, Start, StoreResult, open_with_snapshot,
};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::error::ServerError;
use crate::mcp_requests_state::{
    DOMAIN, Event, Held, Intended, decoded, fold, signed_event, signed_request, unavailable,
    validate,
};
use crate::routes::Say;

const ORIGIN: &str = "lys/identity/mcp-requests";

/// The state presented when a request is read back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpRequestState {
    /// The request has been recorded and awaits a decision.
    Pending,
    /// The profile write and its signed decision are confirmed.
    Approved,
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

/// Reopen a leaf store after an append whose outcome is uncertain.
pub type Reopen<S> = Box<dyn Fn() -> StoreResult<S> + Send>;
type Opened<S> = (FrontierLog<S>, Held, Start);

/// The signed, append-only requests, kept separately from provisioning profiles.
pub struct McpRequestStore<S: LeafStore = FileLeafStore> {
    reopen: Reopen<S>,
    key: Arc<Ed25519Identity>,
    log: FrontierLog<S>,
    held: Held,
    start: Start,
    since_snapshot: u64,
    uncertain: bool,
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
            store.held.requests.len()
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
        let (log, held, start) = opened(&reopen, &key)?;
        let mut store = Self {
            reopen,
            key,
            log,
            held,
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
        let bytes = self.held.encode()?;
        self.log
            .write_snapshot(DOMAIN, &bytes, &self.key)
            .map_err(unavailable)?;
        self.since_snapshot = 0;
        Ok(())
    }

    /// Resolve an uncertain append before answering any read or write from memory.
    pub fn settle(&mut self) -> Result<(), ServerError> {
        if self.uncertain {
            let (log, held, start) = opened(&self.reopen, &self.key)?;
            self.log = log;
            self.held = held;
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
            .held
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
        match self.held.requests.iter().find(|request| request.id == id) {
            Some(request)
                if request.agent == agent && request.server == server && request.asked_by == by =>
            {
                Ok(Some(request.clone()))
            }
            Some(_) => Err(ServerError::RequestReused {
                request: id.to_owned(),
            }),
            None => {
                self.held.check_operation(id, id, by, "")?;
                Ok(None)
            }
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
        let mut next = self.held.clone();
        next.push_request(request.clone())?;
        self.append(&signed_request(request.clone(), &self.key)?, next)?;
        Ok(request)
    }

    fn append(&mut self, bytes: &[u8], next: Held) -> Result<(), ServerError> {
        let index = self.log.len();
        if let Err(failure) = self.log.append(bytes) {
            self.uncertain = true;
            self.settle()?;
            match self.log.leaf_bytes(index).map_err(unavailable)? {
                Some(held) if held.as_slice() == bytes => return Ok(()),
                Some(_) => {
                    return Err(unavailable(format!(
                        "leaf {index} belongs to another writer: {failure}"
                    )));
                }
                None => return Err(unavailable(failure)),
            }
        }
        self.held = next;
        self.since_snapshot += 1;
        if self.since_snapshot >= SNAPSHOT_EVERY.get() {
            self.write_snapshot()?;
        }
        Ok(())
    }
    pub(crate) fn held(&self) -> &Held {
        &self.held
    }

    pub(crate) fn event(&mut self, event: Event) -> Result<(), ServerError> {
        self.settle()?;
        let mut next = self.held.clone();
        next.push_event(event.clone())?;
        self.append(&signed_event(event, &self.key)?, next)
    }

    pub(crate) fn pending(&self, agent: &str) -> Vec<Intended> {
        self.held
            .requests
            .iter()
            .filter(|request| request.agent == agent)
            .filter_map(|request| self.held.intent(&request.id).cloned())
            .collect()
    }
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
        None => Held::default(),
        Some(Ok(requests)) => requests,
        Some(Err(reason)) => {
            let (log, tail) =
                FrontierLog::open(reopen().map_err(unavailable)?).map_err(unavailable)?;
            let mut requests = Held::default();
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
    use lys_core::attestation::sign_attestation;
    use lys_identity::{AgentId, OperationId, PersonId};
    use lys_log_store::Tail;

    #[derive(Serialize)]
    struct Signed {
        request: McpRequest,
        attestation: Vec<u8>,
    }

    fn payload(request: &McpRequest) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut bytes = b"lys/identity/mcp-request/v1\0".to_vec();
        bytes.extend(serde_json::to_vec(request)?);
        Ok(bytes)
    }

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
            let mut kept = Held::default();
            assert!(matches!(
                fold(&mut kept, &tail, &key),
                Err(ServerError::McpRequestsUnavailable { .. })
            ));
            assert!(kept.requests.is_empty());
        }
        Ok(())
    }

    #[test]
    fn an_old_install_reads_its_original_signed_leaf_and_snapshot() -> TestResult {
        let harness = Harness::new(23)?;
        let key = Arc::new(Ed25519Identity::load(
            &harness.dir.path().join("service.key"),
        )?);
        let mut store = McpRequestStore::over(harness.leaves(), Arc::clone(&key))?;
        let asked = request()?;
        let attestation = sign_attestation(&payload(&asked)?, &key).to_cose_bytes();
        store.log.append(&serde_json::to_vec(&Signed {
            request: asked.clone(),
            attestation,
        })?)?;
        let snapshot = serde_json::to_vec(
            &serde_json::json!({"format": "lys/identity/mcp-requests-state/v1", "requests": [asked.clone()]}),
        )?;
        store.log.write_snapshot(DOMAIN, &snapshot, &key)?;
        drop(store);
        let mut reopened = McpRequestStore::over(harness.leaves(), Arc::clone(&key))?;
        assert!(matches!(reopened.start(), Start::Resumed { .. }));
        assert_eq!(reopened.listed(&asked.agent)?, vec![asked.clone()]);
        let operation = OperationId::generate()?.to_string();
        let intended = Intended {
            operation: operation.clone(),
            request: asked.id.clone(),
            by: asked.asked_by.clone(),
            note: String::new(),
            from_version: 1,
            at: 2,
            version: serde_json::from_value(serde_json::json!({
                "number": 2, "operation": operation, "set_by": asked.asked_by, "set_at": 2,
                "reviewed": {"operation": operation, "by": asked.asked_by, "at": 2},
                "settings": {"model_access": [], "tools": [], "skills": [],
                    "mcp_servers": [{"name": asked.server, "url": "https://tools.example.test/mcp"}],
                    "instructions": "", "note": ""}
            }))?,
        };
        reopened.event(Event::Intended(intended))?;
        reopened.write_snapshot()?;
        drop(reopened);
        let mut reopened = McpRequestStore::over(harness.leaves(), Arc::clone(&key))?;
        assert!(matches!(reopened.start(), Start::Resumed { .. }));
        assert_eq!(reopened.pending(&asked.agent).len(), 1);
        reopened.event(Event::Withdrawn {
            request: asked.id.clone(),
            operation,
        })?;
        reopened.log.write_snapshot(DOMAIN, b"not a state", &key)?;
        drop(reopened);
        let mut rebuilt = McpRequestStore::over(harness.leaves(), key)?;
        assert!(matches!(
            rebuilt.start(),
            Start::Rebuilt { replayed: 3, .. }
        ));
        assert_eq!(rebuilt.listed(&asked.agent)?, vec![asked]);
        Ok(())
    }
}
