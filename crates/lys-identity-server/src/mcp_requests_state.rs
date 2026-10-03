//! Signed approval intents retain the exact version until both writes settle.

use std::collections::BTreeMap;
use std::str::FromStr;

use lys_core::Ed25519Identity;
use lys_core::attestation::{sign_attestation, verify_attestation_bytes_by_signer};
use lys_identity::{AgentId, OperationId, PersonId};
use lys_log_store::Tail;
use serde::{Deserialize, Serialize};

use crate::error::ServerError;
use crate::mcp_requests_store::{McpRequest, McpRequestState, server_name};
use crate::provisioning_store::Version;

/// The snapshot signature domain shared by both readable state formats.
pub(crate) const DOMAIN: &str = "lys/identity/mcp-requests-state/v1";
const FORMAT: &str = "lys/identity/mcp-requests-state/v2";
const REQUEST_DOMAIN: &str = "lys/identity/mcp-request/v1";
const DECISION_DOMAIN: &str = "lys/identity/mcp-decision/v1";

/// An authorization retained before its profile is written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Intended {
    /// The approval's operation id.
    pub operation: String,
    /// The request being approved.
    pub request: String,
    /// The approving identity.
    pub by: String,
    /// The approver's words.
    pub note: String,
    /// The latest recorded profile over which the approval is written.
    pub from_version: u32,
    /// The complete newly reviewed version authorized by the approval.
    pub version: Version,
    /// When the authorization was made.
    pub at: u64,
}

/// Only a confirmed profile write becomes a decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "line",
    content = "record",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum Event {
    /// Authorization before writing its profile.
    Intended(Intended),
    /// Authorization whose exact profile reads back.
    Decided(Intended),
    /// Authorization whose profile did not commit.
    Withdrawn { request: String, operation: String },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedRequest<T = McpRequest> {
    request: T,
    attestation: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedEvent<T = Event> {
    event: T,
    attestation: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Signed {
    Request(SignedRequest),
    Event(Box<SignedEvent>),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sealed {
    format: String,
    requests: Vec<McpRequest>,
    #[serde(default)]
    events: Vec<Event>,
}

/// Requests retain their original signed shape; later records carry decisions.
#[derive(Default)]
pub(crate) struct Held {
    /// The unchanged signed requests in recording order.
    pub requests: Vec<McpRequest>,
    /// Approval events in their original order, including withdrawals.
    pub events: Vec<Event>,
    request_ids: BTreeMap<String, usize>,
    agent_requests: BTreeMap<String, Vec<usize>>,
    operations: BTreeMap<String, usize>,
    decisions: BTreeMap<String, usize>,
    intents: BTreeMap<String, usize>,
    pending: BTreeMap<String, BTreeMap<usize, usize>>,
    #[cfg(test)]
    visits: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl Clone for Held {
    fn clone(&self) -> Self {
        self.visits.fetch_add(
            self.requests.len() + self.events.len(),
            std::sync::atomic::Ordering::Relaxed,
        );
        Self {
            requests: self.requests.clone(),
            events: self.events.clone(),
            request_ids: self.request_ids.clone(),
            agent_requests: self.agent_requests.clone(),
            operations: self.operations.clone(),
            decisions: self.decisions.clone(),
            intents: self.intents.clone(),
            pending: self.pending.clone(),
            visits: std::sync::Arc::clone(&self.visits),
        }
    }
}

pub(crate) fn unavailable(reason: impl std::fmt::Display) -> ServerError {
    ServerError::McpRequestsUnavailable {
        reason: reason.to_string(),
    }
}

fn identity(id: &str) -> Result<(), ServerError> {
    if PersonId::from_str(id).is_err() {
        AgentId::from_str(id).map_err(unavailable)?;
    }
    Ok(())
}

pub(crate) fn validate(request: &McpRequest) -> Result<(), ServerError> {
    OperationId::from_str(&request.id).map_err(unavailable)?;
    AgentId::from_str(&request.agent).map_err(unavailable)?;
    identity(&request.asked_by)?;
    if request.profile_version == 0
        || server_name(&request.server)? != request.server
        || request.state != McpRequestState::Pending
    {
        return Err(unavailable(
            "the request has no reviewed version or canonical pending state",
        ));
    }
    Ok(())
}

fn payload<T: Serialize>(domain: &str, record: &T) -> Result<Vec<u8>, ServerError> {
    let mut bytes = domain.as_bytes().to_vec();
    bytes.push(0);
    bytes.extend(serde_json::to_vec(record).map_err(unavailable)?);
    Ok(bytes)
}

pub(crate) fn signed_request(
    request: &McpRequest,
    key: &Ed25519Identity,
) -> Result<Vec<u8>, ServerError> {
    let attestation = sign_attestation(&payload(REQUEST_DOMAIN, request)?, key).to_cose_bytes();
    serde_json::to_vec(&SignedRequest {
        request,
        attestation,
    })
    .map_err(unavailable)
}

pub(crate) fn signed_event(event: &Event, key: &Ed25519Identity) -> Result<Vec<u8>, ServerError> {
    let attestation = sign_attestation(&payload(DECISION_DOMAIN, event)?, key).to_cose_bytes();
    serde_json::to_vec(&SignedEvent { event, attestation }).map_err(unavailable)
}

impl Held {
    #[cfg(test)]
    pub(crate) fn visit(&self) {
        self.visits
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    #[cfg(test)]
    pub(crate) fn work(&self) -> usize {
        self.visits.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub(crate) fn request(&self, id: &str) -> Result<Option<&McpRequest>, ServerError> {
        self.request_ids
            .get(id)
            .map(|index| {
                #[cfg(test)]
                self.visit();
                self.requests
                    .get(*index)
                    .ok_or_else(|| unavailable("request index is outside the projection"))
            })
            .transpose()
    }

    pub(crate) fn decision(&self, id: &str) -> Result<Option<&Intended>, ServerError> {
        self.decisions
            .get(id)
            .map(|index| {
                #[cfg(test)]
                self.visit();
                match self.events.get(*index) {
                    Some(Event::Decided(record)) => Ok(record),
                    _ => Err(unavailable("decision index names no approval decision")),
                }
            })
            .transpose()
    }

    pub(crate) fn intent(&self, id: &str) -> Result<Option<&Intended>, ServerError> {
        self.intents
            .get(id)
            .map(|index| {
                #[cfg(test)]
                self.visit();
                match self.events.get(*index) {
                    Some(Event::Intended(record)) => Ok(record),
                    _ => Err(unavailable("intent index names no approval intent")),
                }
            })
            .transpose()
    }

    pub(crate) fn push_request(&mut self, request: McpRequest) -> Result<(), ServerError> {
        self.check_request(&request)?;
        self.apply_request(request);
        Ok(())
    }

    pub(crate) fn check_request(&self, request: &McpRequest) -> Result<(), ServerError> {
        validate(request).map_err(unavailable)?;
        if self.request_ids.contains_key(&request.id) || self.operations.contains_key(&request.id) {
            return Err(unavailable("the log repeats an MCP request operation"));
        }
        Ok(())
    }

    pub(crate) fn apply_request(&mut self, request: McpRequest) {
        let index = self.requests.len();
        self.request_ids.insert(request.id.clone(), index);
        self.agent_requests
            .entry(request.agent.clone())
            .or_default()
            .push(index);
        self.requests.push(request);
    }

    pub(crate) fn push_event(&mut self, event: Event) -> Result<(), ServerError> {
        self.check_event(&event)?;
        self.apply_event(event)
    }

    pub(crate) fn check_event(&self, event: &Event) -> Result<(), ServerError> {
        match event {
            Event::Intended(record) => {
                OperationId::from_str(&record.operation).map_err(unavailable)?;
                identity(&record.by)?;
                let asked = self
                    .request(&record.request)?
                    .ok_or_else(|| unavailable("approval names no request"))?;
                if self.intent(&record.request)?.is_some()
                    || self.decision(&record.request)?.is_some()
                    || self.request(&record.operation)?.is_some()
                    || record.from_version < asked.profile_version
                    || record.from_version.checked_add(1) != Some(record.version.number)
                    || record.version.operation != record.operation
                    || record.version.set_by != record.by
                    || record.version.set_at != record.at
                    || record.note.trim() != record.note
                    || !record
                        .version
                        .settings
                        .mcp_servers
                        .iter()
                        .any(|server| server.name == asked.server)
                    || !record.version.reviewed.as_ref().is_some_and(|review| {
                        review.by == record.by
                            && review.operation == record.operation
                            && review.at == record.at
                    })
                {
                    return Err(unavailable(
                        "approval intent differs from its request or reviewed version",
                    ));
                }
                self.check_operation(&record.operation, &record.request, &record.by, &record.note)?;
            }
            Event::Decided(record) => {
                if self.intent(&record.request)? != Some(record) {
                    return Err(unavailable("decision differs from its retained intent"));
                }
            }
            Event::Withdrawn { request, operation } => {
                if self
                    .intent(request)?
                    .is_none_or(|record| &record.operation != operation)
                {
                    return Err(unavailable("withdrawal names no retained intent"));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn apply_event(&mut self, event: Event) -> Result<(), ServerError> {
        let (request, operation) = match &event {
            Event::Intended(record) | Event::Decided(record) => {
                (&record.request, &record.operation)
            }
            Event::Withdrawn { request, operation } => (request, operation),
        };
        let request_index = *self
            .request_ids
            .get(request)
            .ok_or_else(|| unavailable("event request index is absent"))?;
        #[cfg(test)]
        self.visit();
        let agent = &self
            .requests
            .get(request_index)
            .ok_or_else(|| unavailable("event request index is outside the projection"))?
            .agent;
        let index = self.events.len();
        match &event {
            Event::Intended(_) => {
                self.operations.insert(operation.clone(), index);
                self.intents.insert(request.clone(), index);
                self.pending
                    .entry(agent.clone())
                    .or_default()
                    .insert(request_index, index);
            }
            Event::Decided(_) | Event::Withdrawn { .. } => {
                self.intents.remove(request);
                if let Some(pending) = self.pending.get_mut(agent) {
                    pending.remove(&request_index);
                    if pending.is_empty() {
                        self.pending.remove(agent);
                    }
                }
                if matches!(&event, Event::Decided(_)) {
                    self.decisions.insert(request.clone(), index);
                }
            }
        }
        self.events.push(event);
        Ok(())
    }

    pub(crate) fn listed(&self, agent: &str) -> Result<Vec<McpRequest>, ServerError> {
        self.agent_requests
            .get(agent)
            .into_iter()
            .flatten()
            .map(|index| {
                #[cfg(test)]
                self.visit();
                self.requests
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| unavailable("listed request index is outside the projection"))
            })
            .collect()
    }

    pub(crate) fn pending(&self, agent: &str) -> Result<Vec<Intended>, ServerError> {
        self.pending
            .get(agent)
            .into_iter()
            .flat_map(BTreeMap::values)
            .map(|index| {
                #[cfg(test)]
                self.visit();
                match self.events.get(*index) {
                    Some(Event::Intended(record)) => Ok(record.clone()),
                    _ => Err(unavailable("pending index names no approval intent")),
                }
            })
            .collect()
    }

    pub(crate) fn has_pending(&self, agent: &str) -> bool {
        self.pending.contains_key(agent)
    }

    pub(crate) fn check_operation(
        &self,
        operation: &str,
        request: &str,
        by: &str,
        note: &str,
    ) -> Result<(), ServerError> {
        let reused = match self.operations.get(operation) {
            Some(index) => {
                #[cfg(test)]
                self.visit();
                let Some(Event::Intended(record)) = self.events.get(*index) else {
                    return Err(unavailable("operation index names no approval intent"));
                };
                record.request != request || record.by != by || record.note != note
            }
            None => false,
        };
        if self.request_ids.contains_key(operation) || reused {
            return Err(ServerError::RequestReused {
                request: operation.to_owned(),
            });
        }
        Ok(())
    }

    pub(crate) fn encode(&self) -> Result<Vec<u8>, ServerError> {
        #[derive(Serialize)]
        struct Snapshot<'a> {
            format: &'a str,
            requests: &'a [McpRequest],
            events: &'a [Event],
        }
        serde_json::to_vec(&Snapshot {
            format: FORMAT,
            requests: &self.requests,
            events: &self.events,
        })
        .map_err(unavailable)
    }
}

pub(crate) fn decoded(bytes: &[u8], count: u64) -> Result<Held, ServerError> {
    let sealed: Sealed = serde_json::from_slice(bytes).map_err(unavailable)?;
    if !matches!(sealed.format.as_str(), DOMAIN | FORMAT)
        || (sealed.format == DOMAIN && !sealed.events.is_empty())
        || u64::try_from(
            sealed
                .requests
                .len()
                .checked_add(sealed.events.len())
                .ok_or_else(|| unavailable("snapshot count overflow"))?,
        )
        .map_err(unavailable)?
            != count
    {
        return Err(unavailable(
            "the snapshot's MCP request format or count differs from its log",
        ));
    }
    let mut held = Held::default();
    for request in sealed.requests {
        held.push_request(request)?;
    }
    for event in sealed.events {
        held.push_event(event).map_err(unavailable)?;
    }
    Ok(held)
}

pub(crate) fn fold(held: &mut Held, tail: &Tail, key: &Ed25519Identity) -> Result<(), ServerError> {
    for (index, bytes) in (tail.from..).zip(&tail.leaves) {
        let envelope: Signed = serde_json::from_slice(bytes)
            .map_err(|error| unavailable(format!("leaf {index} is not an MCP record: {error}")))?;
        let signer = key.public_key_bytes();
        match envelope {
            Signed::Request(record) => {
                verify_attestation_bytes_by_signer(
                    &record.attestation,
                    &payload(REQUEST_DOMAIN, &record.request)?,
                    &signer,
                )
                .map_err(|error| unavailable(format!("leaf {index} signature: {error}")))?;
                held.push_request(record.request)?;
            }
            Signed::Event(record) => {
                verify_attestation_bytes_by_signer(
                    &record.attestation,
                    &payload(DECISION_DOMAIN, &record.event)?,
                    &signer,
                )
                .map_err(|error| unavailable(format!("leaf {index} signature: {error}")))?;
                held.push_event(record.event).map_err(unavailable)?;
            }
        }
    }
    Ok(())
}
