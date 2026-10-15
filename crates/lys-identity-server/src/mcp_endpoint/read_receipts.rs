//! Own-view reads have signed durable leaves without posing as directory changes.

use lys_core::Ed25519Identity;
use lys_core::attestation::{sign_attestation, verify_attestation_bytes_by_signer};
use lys_log_store::{FileLeafStore, Log};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::ServerError;
use crate::mcp_oauth::Connection;
use crate::routes::{AppState, hex};

const FORMAT: &str = "lys-mcp-person-read/v1";
const ORIGIN: &str = "lys/identity/mcp-person-reads";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    format: String,
    connection: String,
    client: String,
    person: String,
    method: String,
    route: String,
    request_sha256: String,
    at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed {
    read: Read,
    signature: Vec<u8>,
}

pub(crate) struct Reads {
    key: Ed25519Identity,
    log: Log<FileLeafStore>,
}

fn unavailable(error: impl std::fmt::Display) -> ServerError {
    ServerError::ConfigInvalid {
        reason: format!("MCP person read receipts are unavailable: {error}"),
    }
}

impl Reads {
    pub(crate) fn open(state: &AppState) -> Result<Self, ServerError> {
        let path = state.estate_plan_file.with_file_name("mcp-person-reads");
        match std::fs::metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                FileLeafStore::create(&path, ORIGIN).map_err(unavailable)?;
            }
            Err(error) => return Err(unavailable(error)),
        }
        let key = lys_identity::signer::load_service_key(&state.grant_setup.key_file)?;
        let log =
            Log::open(FileLeafStore::open(&path).map_err(unavailable)?).map_err(unavailable)?;
        if log.origin() != ORIGIN {
            return Err(unavailable("the read log has another origin"));
        }
        for index in 0..log.tree().len() {
            let leaf = log
                .leaf_bytes(index)
                .ok_or_else(|| unavailable("the read log is missing a leaf"))?;
            let signed: Signed = serde_json::from_slice(leaf).map_err(unavailable)?;
            if signed.read.format != FORMAT {
                return Err(unavailable("a read receipt has an unknown format"));
            }
            let payload = serde_json::to_vec(&signed.read).map_err(unavailable)?;
            verify_attestation_bytes_by_signer(
                &signed.signature,
                &payload,
                &key.public_key_bytes(),
            )
            .map_err(unavailable)?;
        }
        if let Some(size) = log.recovered_to() {
            tracing::warn!(size, "MCP person read log recovered its durable tail");
        }
        Ok(Self { key, log })
    }

    pub(crate) fn before(
        &mut self,
        connection: &Connection,
        method: &str,
        uri: &axum::http::Uri,
        message: &[u8],
    ) -> Result<Value, ServerError> {
        let person = connection
            .person_reads
            .as_ref()
            .ok_or_else(|| unavailable("the connection has no person-read consent"))?;
        let read = Read {
            format: FORMAT.to_owned(),
            connection: connection.agent.to_string(),
            client: connection.client_id.clone(),
            person: person.person.clone(),
            method: method.to_owned(),
            route: super::recorded_path(uri),
            request_sha256: super::digest(message),
            at: crate::session::now(),
        };
        let payload = serde_json::to_vec(&read).map_err(unavailable)?;
        let signed = Signed {
            read,
            signature: sign_attestation(&payload, &self.key).to_cose_bytes(),
        };
        let message = serde_json::to_vec(&signed).map_err(unavailable)?;
        let (index, leaf_hash) = self.log.append(&message).map_err(unavailable)?;
        let root = self.log.tree().root();
        let proof = self
            .log
            .tree()
            .prove_inclusion(index)
            .map_err(unavailable)?;
        Ok(json!({
            "read":signed.read, "payload":hex(&payload), "message":hex(&message), "index":index,
            "leaf_hash":hex(&leaf_hash), "checkpoint":{"tree_size":root.num_leaves(), "root":hex(root.as_bytes())},
            "inclusion_proof":hex(proof.as_bytes()),
        }))
    }
}
