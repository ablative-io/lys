//! Prepared call references survive a crash before the session append.

use super::coding::Coding;
use super::parts::{complete_response_parts, read_json, request_model, request_parts_of};
use super::{Api, CallMeta, CallRecord, CallStatus, IngestReport, OutcomeMeta, already, find_call};
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
    /// How long the call was held while its journal record was put in place; absent if admission was interrupted before its measurement persisted. The sync that makes the record hold through a loss of power is made beside the call and is not in this.
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
    /// The token figures the response's event stream reported, read as it
    /// passed; absent when the stream reported none.
    pub tokens: Option<Tokens>,
    /// The run key the call's path opened with, when it carried one.
    pub run: Option<String>,
    /// The status and the kept headers.
    pub head: Head,
    /// Why the proxy marked the call unrecorded, when it did.
    pub unrecorded: Option<String>,
}

/// The token figures a provider reported for one call, each as the response
/// named it and none computed here. A figure the response did not carry is
/// absent, never zero.
///
/// The two providers count input differently and the figures are kept as
/// reported: a Messages response's `input_tokens` leaves out what was read
/// from or written to the cache, and a Responses response's `input_tokens`
/// includes its cached tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    /// Input tokens, as the provider counts them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    /// Output tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    /// Tokens written to the cache (Messages: `cache_creation_input_tokens`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<u64>,
    /// Tokens read from the cache (Messages: `cache_read_input_tokens`;
    /// Responses: `input_tokens_details.cached_tokens`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u64>,
    /// Output tokens spent reasoning (Responses:
    /// `output_tokens_details.reasoning_tokens`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
}

impl Tokens {
    /// Take each figure `usage` names under `names`, leaving a figure it
    /// does not name as it was: a later report of the same call replaces
    /// only what it carries.
    pub fn read(&mut self, usage: &Value, names: &[(&str, TokenFigure)]) {
        for (name, figure) in names {
            let Some(count) = usage.get(*name).and_then(Value::as_u64) else {
                continue;
            };
            match figure {
                TokenFigure::Input => self.input = Some(count),
                TokenFigure::Output => self.output = Some(count),
                TokenFigure::CacheCreation => self.cache_creation = Some(count),
                TokenFigure::CacheRead => self.cache_read = Some(count),
                TokenFigure::Reasoning => self.reasoning = Some(count),
            }
        }
    }

    /// These figures, or none when the response carried no figure at all.
    #[must_use]
    pub fn reported(&self) -> Option<Self> {
        (*self != Self::default()).then(|| self.clone())
    }
}

/// Which figure of [`Tokens`] a provider's member names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenFigure {
    /// [`Tokens::input`].
    Input,
    /// [`Tokens::output`].
    Output,
    /// [`Tokens::cache_creation`].
    CacheCreation,
    /// [`Tokens::cache_read`].
    CacheRead,
    /// [`Tokens::reasoning`].
    Reasoning,
}

/// The response headers that carry the provider's id for the request, in the
/// order they are read: Anthropic's, then `OpenAI`'s.
const REQUEST_ID: [&str; 2] = ["request-id", "x-request-id"];

/// A call's status and what its record keeps of each side's headers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    /// The upstream's HTTP status; absent when no response head arrived.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// The request's headers.
    pub request: Side,
    /// The response's headers.
    pub response: Side,
}

/// One side's headers as the record keeps them: every name, and the values
/// of a named few. Which names keep their values is the proxy's to say
/// (`proxy::headers`); a header that carries a credential never does.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Side {
    /// Every header's name in the order received, one per header line, so a
    /// header whose value is not kept is still on the record as having been
    /// there.
    pub names: Vec<String>,
    /// The kept values: each name to its values in the order received.
    pub values: BTreeMap<String, Vec<String>>,
}

impl Head {
    /// The provider's id for the request, from the response's own header.
    #[must_use]
    pub fn request_id(&self) -> Option<String> {
        REQUEST_ID
            .iter()
            .find_map(|name| self.response.values.get(*name)?.first().cloned())
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

const NOT_JSON: &str = "the response is not JSON";

/// What reading a whole-body response leaves beside its JSON.
#[derive(Default)]
struct WholeBody {
    /// Why its stored bytes did not decode as the coding it named.
    undecoded: Option<String>,
    /// The token figures the `usage` object at its top names: what an event
    /// stream's reader reads as the stream passes.
    tokens: Option<Tokens>,
}

/// The response stored at `path` as JSON, decoded first when its head names
/// one coding that is decoded. Bytes that do not decode leave why in `whole`
/// and are no complete response; a response that reads leaves the token
/// figures it reported there, whatever its shape goes on to be.
fn read_response(
    path: &Path,
    seen: &Seen,
    api: Api,
    whole: &mut WholeBody,
) -> Result<Value, HomeError> {
    let json: Value = match Coding::of(&seen.head) {
        None => read_json(path, NOT_JSON)?,
        Some(coding) => {
            let stored =
                std::fs::read(path).map_err(|e| HomeError::io("reading a body file", path, e))?;
            let bytes = coding.decode(&stored).map_err(|error| {
                whole.undecoded = Some(format!(
                    "the response's content-encoding is {}, and its stored bytes do not decode as that: {error}",
                    coding.name()
                ));
                HomeError::BodyShape {
                    api: api.as_str(),
                    reason: "the response's bytes do not decode as the coding it names",
                }
            })?;
            serde_json::from_slice(&bytes).map_err(|source| HomeError::Json {
                context: NOT_JSON,
                source,
            })?
        }
    };
    whole.tokens = Tokens::of_whole_body(api, &json);
    Ok(json)
}

/// Why a call that ended whole could not be read into parts. Only the
/// response is parsed here as JSON, so a JSON failure is the response's: the
/// reason then says what the response's own head named as its encoding and
/// whether that was decoded, the one thing that says why stored bytes that
/// reached the client whole do not parse. Nothing is guessed from the bytes.
fn unread(error: &HomeError, head: &Head) -> String {
    match (error, head.response.values.get("content-encoding")) {
        (HomeError::Json { .. }, Some(codings)) => match Coding::of(head) {
            Some(coding) => format!("{error}, after its {} coding was decoded", coding.name()),
            None => format!(
                "{error}; the response's content-encoding is {}, which is not decoded: \
                 one coding of gzip, deflate or br is",
                codings.join(", ")
            ),
        },
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
        let mut whole = WholeBody::default();
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
                            read_response(path, input.seen, meta.api, &mut whole)
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
                    reason = Some(
                        whole
                            .undecoded
                            .take()
                            .unwrap_or_else(|| unread(&error, &input.seen.head)),
                    );
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
                // An event stream's figures were read as it passed; a
                // whole-body response's are the ones its own JSON named.
                usage: input.seen.tokens.clone().or(whole.tokens),
                run: input.seen.run.clone(),
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
