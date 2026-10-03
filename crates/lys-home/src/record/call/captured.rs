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
use std::path::Path;

#[cfg(test)]
#[path = "captured_tests.rs"]
mod tests;

/// The duration is absent only when capture was interrupted before it was persisted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurableTime {
    /// The process ended before the duration was durably recorded.
    Interrupted,
    /// Nanoseconds from the last response frame to synced, installed body blocks.
    Measured(u64),
}

/// Measurements of admission and the asynchronous capture worker.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureTiming {
    /// Journal append and sync hold; absent if admission was interrupted before its measurement persisted.
    pub admission_ns: Option<u64>,
    /// Body durability measurement, with interrupted measurement distinguished explicitly.
    pub durable: DurableTime,
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
                Err(HomeError::BodyShape { .. } | HomeError::Json { .. }) => {
                    status = CallStatus::Unrecorded;
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
