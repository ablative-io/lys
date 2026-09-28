#![cfg(test)]
//! A directory write runs off the async workers: while a write is held
//! inside its leaf store, a directory read route answers the state before
//! it, and once the write is let go the read answers it.

use std::error::Error;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, header};
use identity_contract::fake_issuer::{CLIENT_ID, CLIENT_SECRET, FakeIssuer};
use lys_core::Ed25519Identity;
use lys_identity::{Actor, AuthMethod, Directory, LoginBinding, Provenance};
use lys_log_store::{Frontier, LeafStore, PinnedRoot, StoreError, StoreResult};

use super::{Named, app_state, list, register_person};
use crate::config::{Config, ConfiguredLogin};
use crate::directory_access::DirectoryStore;

type TestResult = Result<(), Box<dyn Error>>;

const ORIGIN: &str = "example.test/lys/directory";
const ADMINISTRATOR: &str = "administrator-subject";

/// Where the first leaf write stops: it says it has begun, and waits to be
/// let go.
struct Gate {
    begun: Sender<()>,
    go: Receiver<()>,
}

/// What a store held in memory keeps, shared by every handle opened on it: its
/// leaves, pin and snapshot, the leaf writes made, and the gate the first
/// one stops at.
#[derive(Default)]
struct Disk {
    leaves: Vec<Vec<u8>>,
    pinned: Option<PinnedRoot>,
    snapshot: Option<Vec<u8>>,
    gate: Option<Gate>,
}

struct Held {
    disk: Arc<Mutex<Disk>>,
    writes: Arc<AtomicU64>,
}

impl Held {
    fn disk(&self) -> MutexGuard<'_, Disk> {
        self.disk.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl LeafStore for Held {
    fn origin(&self) -> &str {
        ORIGIN
    }

    fn extent(&self) -> u64 {
        self.disk().leaves.len() as u64
    }

    fn leaf(&self, index: u64) -> StoreResult<Option<Vec<u8>>> {
        Ok(usize::try_from(index)
            .ok()
            .and_then(|slot| self.disk().leaves.get(slot).cloned()))
    }

    fn put_leaf(&mut self, index: u64, bytes: &[u8]) -> StoreResult<()> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        let gate = self.disk().gate.take();
        if let Some(gate) = gate {
            gate.begun.send(()).map_err(|closed| StoreError::Io {
                context: "saying the leaf write began".to_owned(),
                source: std::io::Error::other(closed),
            })?;
            gate.go.recv().map_err(|closed| StoreError::Io {
                context: "waiting to be let go".to_owned(),
                source: std::io::Error::other(closed),
            })?;
        }
        let next = self.extent();
        if index != next {
            return Err(StoreError::LeafWouldLeaveGap { index, next });
        }
        self.disk().leaves.push(bytes.to_vec());
        Ok(())
    }

    fn pinned(&self) -> PinnedRoot {
        self.disk().pinned.unwrap_or(PinnedRoot {
            tree_size: 0,
            root: Frontier::new().root(),
        })
    }

    fn pin(&mut self, pin: PinnedRoot) -> StoreResult<()> {
        self.disk().pinned = Some(pin);
        Ok(())
    }

    fn snapshot(&self) -> StoreResult<Option<Vec<u8>>> {
        Ok(self.disk().snapshot.clone())
    }

    fn put_snapshot(&mut self, bytes: &[u8]) -> StoreResult<()> {
        self.disk().snapshot = Some(bytes.to_vec());
        Ok(())
    }
}

fn secret_file(path: &Path, bytes: &[u8]) -> TestResult {
    std::fs::write(path, bytes)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

fn config(dir: &Path, issuer: &FakeIssuer) -> Config {
    let configured = |subject: &str| ConfiguredLogin {
        issuer: issuer.issuer().to_owned(),
        subject: subject.to_owned(),
    };
    Config {
        listen: std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
        log_dir: dir.join("log"),
        log_origin: ORIGIN.to_owned(),
        event_key_file: dir.join("service.key"),
        issuer: issuer.issuer().to_owned(),
        client_id: CLIENT_ID.to_owned(),
        client_secret_file: dir.join("client.secret"),
        redirect_url: "http://127.0.0.1/callback".to_owned(),
        administrator: configured(ADMINISTRATOR),
        link_audit_source: configured("link-audit-source-subject"),
        session_seconds: 600,
        secure_cookie: false,
        grant_log_dir: dir.join("grant-log"),
        grant_log_origin: "example.test/lys/grants".to_owned(),
        grant_model_file: dir.join("grant-model.json"),
        spicedb: None,
        secrets: None,
        requests_dir: None,
        certificates_dir: None,
        network_file: None,
        roles_file: None,
        provisioning_file: None,
        homes_dir: None,
        runtime_dir: None,
        service_accounts_dir: None,
        teams_dir: None,
        stops_dir: None,
        reviews_dir: None,
        sign_in_providers: None,
        surface_dir: None,
    }
}

#[tokio::test]
async fn a_read_route_answers_while_a_write_is_held_in_its_leaf_store() -> TestResult {
    let dir = tempfile::TempDir::new()?;
    secret_file(&dir.path().join("issuer.key"), &[3; 32])?;
    secret_file(&dir.path().join("service.key"), &[9; 32])?;
    secret_file(&dir.path().join("client.secret"), CLIENT_SECRET.as_bytes())?;
    std::fs::write(
        dir.path().join("grant-model.json"),
        r#"{"version":1,"relations":{"alpha":["read","write"],"beta":["read"]}}"#,
    )?;
    let issuer = FakeIssuer::start(&dir.path().join("issuer.key")).await?;
    let config = config(dir.path(), &issuer);

    let (begun, heard) = channel();
    let (let_go, go) = channel();
    let disk = Arc::new(Mutex::new(Disk {
        gate: Some(Gate { begun, go }),
        ..Disk::default()
    }));
    let writes = Arc::new(AtomicU64::new(0));
    let (shared, counted) = (Arc::clone(&disk), Arc::clone(&writes));
    let directory = Directory::open(
        Box::new(move || {
            Ok(DirectoryStore::new(Held {
                disk: Arc::clone(&shared),
                writes: Arc::clone(&counted),
            }))
        }),
        Ed25519Identity::load(&dir.path().join("service.key"))?,
    )?;
    let state = app_state(&config, Arc::new(|_| {}), directory).await?;

    let administrator = Actor::new(
        LoginBinding::new(issuer.issuer(), ADMINISTRATOR)?,
        Provenance::new(AuthMethod::Oidc, 1_790_000_000),
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        HeaderValue::from_str(&state.sessions.begin(administrator)?)?,
    );

    let write = tokio::spawn(register_person(
        State(Arc::clone(&state)),
        headers.clone(),
        Json(Named {
            operation: format!("op-{:032x}", 1),
            display_name: "Bea".to_owned(),
        }),
    ));
    tokio::task::spawn_blocking(move || heard.recv()).await??;
    assert_eq!(
        writes.load(Ordering::SeqCst),
        1,
        "the write is held inside its one leaf write"
    );

    let Json(during) = list(State(Arc::clone(&state)), headers.clone()).await?;
    assert_eq!(
        during["identities"].as_array().map(Vec::len),
        Some(0),
        "the read answers the state before the write it did not wait on: {during}"
    );
    assert_eq!(
        writes.load(Ordering::SeqCst),
        1,
        "the read answered while the write was still held"
    );

    let_go.send(())?;
    let Json(written) = write.await??;
    assert!(written["person"].is_string(), "{written}");
    let Json(after) = list(State(Arc::clone(&state)), headers).await?;
    assert_eq!(
        after["identities"][0]["id"], written["person"],
        "a read after the write was answered sees it: {after}"
    );
    Ok(())
}
