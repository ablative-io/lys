//! Prepared call references survive a crash before the session append.

use super::parts::{complete_response_parts, read_json, request_model, request_parts_of};
use super::{CallMeta, CallRecord, CallStatus, IngestReport, OutcomeMeta, already, find_call};
use crate::error::HomeError;
use crate::record::{
    Session,
    blocks::BlockStore,
    entries::{CUSTOM_CALL, EntryBody},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::Path;

#[cfg(test)]
#[path = "captured_tests.rs"]
mod tests;

/// The duration is absent only when capture was interrupted before it was persisted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurableTime {
    /// The process ended before the duration was durably recorded.
    Interrupted,
    /// A body the store never took, by rename or by copy; the refusal is in
    /// `refusals` and the call is unrecorded. No duration is claimed.
    NotPlaced,
    /// Nanoseconds from the last response frame to synced, installed body blocks.
    Measured(u64),
}

/// The path a captured body took into the block store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Placement {
    Rename,
    Copy,
}

/// Measurements of admission and the asynchronous capture worker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureTiming {
    /// Journal append and sync hold; absent if admission was interrupted before its measurement persisted.
    pub admission_ns: Option<u64>,
    /// Body durability measurement, with interrupted measurement distinguished explicitly.
    pub durable: DurableTime,
    /// Named placement refusals; empty when the one-pass handoff succeeded.
    pub refusals: Vec<String>,
    /// Response bytes awaiting completion by the worker at the final arrival, including its current frame.
    pub pending_bytes: u64,
    /// Time from the final arrival until the worker drained the response bytes.
    pub drain_ns: u64,
    /// Spool write calls and successfully written bytes.
    pub spool_writes: u64,
    pub spool_bytes: u64,
    /// Successful spool file syncs.
    pub spool_syncs: u64,
    /// Wall time in spool writes and streaming hashes, separately.
    pub write_ns: u64,
    pub hash_ns: u64,
    /// Syncs counted by the block store during preparation and placement.
    pub block_syncs: u64,
    /// Placement and retained-spool state when the call was appended.
    pub request_placement: Option<Placement>,
    pub response_placement: Option<Placement>,
    pub request_spool_kept: bool,
    pub response_spool_kept: bool,
}

impl CaptureTiming {
    pub(crate) fn interrupted(admission_ns: Option<u64>) -> Self {
        Self {
            admission_ns,
            durable: DurableTime::Interrupted,
            refusals: Vec::new(),
            pending_bytes: 0,
            drain_ns: 0,
            spool_writes: 0,
            spool_bytes: 0,
            spool_syncs: 0,
            write_ns: 0,
            hash_ns: 0,
            block_syncs: 0,
            request_placement: None,
            response_placement: None,
            request_spool_kept: false,
            response_spool_kept: false,
        }
    }
}

/// What the proxy read of a call beside its bodies, as the call passed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seen {
    /// The provider's id for the response message, from a Messages stream's
    /// `message_start`.
    pub message_id: Option<String>,
    /// The status and the kept headers.
    pub head: Head,
    /// Why the proxy marked the call unrecorded, when it did.
    pub unrecorded: Option<String>,
}

/// The response headers that carry the provider's id for the request, in the
/// order they are read: Anthropic's, then `OpenAI`'s.
const REQUEST_ID: [&str; 2] = ["request-id", "x-request-id"];

/// A call's status and the few headers its record keeps of each side: each
/// name to its values in the order sent. Which names are kept is the
/// proxy's to say (`proxy::headers`); no credential is among them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    /// The upstream's HTTP status; absent when no response head arrived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// The kept request headers.
    pub request: BTreeMap<String, Vec<String>>,
    /// The kept response headers.
    pub response: BTreeMap<String, Vec<String>>,
}

impl Head {
    /// The provider's id for the request, from the response's own header.
    #[must_use]
    pub fn request_id(&self) -> Option<String> {
        REQUEST_ID
            .iter()
            .find_map(|name| self.response.get(*name)?.first().cloned())
    }
}

/// All references needed to append a call, without reading either body again.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PreparedCall {
    pub record: CallRecord,
    new_parts: u64,
    reused_parts: u64,
    pub raw_blocks: u64,
}

pub(crate) struct Captured<'a> {
    pub meta: &'a OutcomeMeta,
    pub request: Option<&'a Path>,
    pub response: Option<&'a Path>,
    pub response_parts: Option<&'a [Value]>,
    pub raw_request: Option<String>,
    pub raw_response: Option<String>,
    pub timing: CaptureTiming,
    pub seen: &'a Seen,
}

/// Why a call that ended whole could not be read into parts. Only the
/// response is parsed here as JSON, so a JSON failure is the response's: the
/// reason then says what the response's own head named as its encoding, the
/// one thing that says why stored bytes that reached the client whole do not
/// parse. Nothing is guessed from the bytes.
fn unread(error: &HomeError, head: &Head) -> String {
    match (error, head.response.get("content-encoding")) {
        (HomeError::Json { .. }, Some(codings)) => format!(
            "{error}; the response's content-encoding is {}, and a response that is not an \
             event stream is read as stored, not decoded",
            codings.join(", ")
        ),
        (HomeError::Json { .. }, None) if head.status.is_some() => {
            format!("{error}; the response names no content-encoding")
        }
        _ => error.to_string(),
    }
}

impl PreparedCall {
    pub(crate) fn prepare(blocks: &BlockStore, input: Captured<'_>) -> Result<Self, HomeError> {
        let meta = input.meta;
        let (request, model, request_whole) = match input.request {
            Some(path) => match read_json(path, "the request is not JSON") {
                Ok(json) => {
                    let model = request_model(&json);
                    match request_parts_of(meta.api, &json) {
                        Ok(parts) => (parts, model, true),
                        Err(HomeError::BodyShape { .. }) => (Vec::new(), model, false),
                        Err(error) => return Err(error),
                    }
                }
                Err(HomeError::Json { .. }) => (Vec::new(), None, false),
                Err(error) => return Err(error),
            },
            None => (Vec::new(), None, false),
        };
        let mut status = meta.status;
        let mut reason = input.seen.unrecorded.clone();
        let response = if status == CallStatus::Complete {
            let result = match (&model, input.response) {
                (Some(model), Some(path)) if request_whole => {
                    let complete = CallMeta {
                        call_id: meta.call_id.clone(),
                        provider: meta.provider.clone(),
                        api: meta.api,
                        model: model.clone(),
                        started_at: meta.started_at.clone(),
                        duration_ms: meta.duration_ms.unwrap_or_default(),
                        stream: meta.stream,
                    };
                    match input.response_parts {
                        Some(parts) => Ok(Cow::Borrowed(parts)),
                        None => complete_response_parts(&complete, None, || {
                            read_json(path, "the response is not JSON")
                        })
                        .map(Cow::Owned),
                    }
                }
                _ => Err(HomeError::BodyShape {
                    api: meta.api.as_str(),
                    reason: "a complete capture needs both bodies and a model",
                }),
            };
            match result {
                Ok(parts) => parts,
                Err(error @ (HomeError::BodyShape { .. } | HomeError::Json { .. })) => {
                    status = CallStatus::Unrecorded;
                    reason = Some(unread(&error, &input.seen.head));
                    Cow::Owned(Vec::new())
                }
                Err(error) => return Err(error),
            }
        } else {
            Cow::Owned(Vec::new())
        };
        let mut prepared = Self {
            record: CallRecord {
                call_id: meta.call_id.clone(),
                provider: meta.provider.clone(),
                api: meta.api,
                model,
                message_id: input.seen.message_id.clone(),
                request_id: input.seen.head.request_id(),
                head: (input.seen.head != Head::default()).then(|| input.seen.head.clone()),
                unrecorded_reason: reason.filter(|_| status == CallStatus::Unrecorded),
                request: Vec::new(),
                response: Vec::new(),
                raw_request: input.raw_request,
                raw_response: input.raw_response,
                status,
                started_at: meta.started_at.clone(),
                duration_ms: meta.duration_ms,
                stream: meta.stream,
                capture: Some(input.timing),
            },
            new_parts: 0,
            reused_parts: 0,
            raw_blocks: 0,
        };
        prepared.record.request = prepared.parts(blocks, &request)?;
        prepared.record.response = prepared.parts(blocks, &response)?;
        Ok(prepared)
    }

    fn parts(&mut self, blocks: &BlockStore, parts: &[Value]) -> Result<Vec<String>, HomeError> {
        parts
            .iter()
            .map(|part| {
                let bytes = serde_json::to_vec(part).map_err(|source| HomeError::Json {
                    context: "a part could not be serialised",
                    source,
                })?;
                let put = blocks.put(&bytes)?;
                if put.new {
                    self.new_parts += 1;
                } else {
                    self.reused_parts += 1;
                }
                Ok(put.hash.to_string())
            })
            .collect()
    }

    pub(crate) fn append(&self, session: &mut Session) -> Result<IngestReport, HomeError> {
        if let Some(entry) = find_call(session, &self.record.call_id)? {
            return Ok(already(entry));
        }
        let data = serde_json::to_value(&self.record).map_err(|source| HomeError::Json {
            context: "the captured call could not be serialised",
            source,
        })?;
        let entry_id = session.append(EntryBody::Custom {
            custom_type: CUSTOM_CALL.to_owned(),
            data: Some(data),
        })?;
        Ok(IngestReport {
            entry_id,
            already_recorded: false,
            part_blocks_new: self.new_parts,
            part_blocks_reused: self.reused_parts,
            raw_blocks: self.raw_blocks,
            request_parts: self.record.request.len() as u64,
            response_parts: self.record.response.len() as u64,
        })
    }
}
