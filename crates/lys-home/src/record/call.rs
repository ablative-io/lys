//! The proxy call record: one `lys.call` custom entry per model call, naming
//! the request's and the response's parts by block hash, with the raw bodies
//! kept as blocks too. A conversation resent whole adds no new part blocks.
//!
//! Every call has a stable `call_id` the proxy chooses before forwarding, and
//! ingest is idempotent on it: a second ingest of the same id in the same
//! session records nothing and answers with the entry already there, so a
//! crash between the append and the proxy's journal retirement never leaves
//! two records. Only a `complete` call carries response parts; the other
//! outcomes (`cancelled`, `partial`, `unrecorded`, `lost`) are recorded
//! through [`ingest_outcome`], which takes whatever bodies exist and marks
//! what is absent as absent rather than inventing an empty body or a model.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::HomeError;
use crate::record::Session;
use crate::record::blocks::{BlockStore, Hash, Put};
use crate::record::entries::{CUSTOM_CALL, EntryBody};

/// Which provider api a body follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Api {
    /// Anthropic Messages: `system` plus `messages[].content` parts.
    #[serde(rename = "anthropic-messages")]
    Messages,
    /// `OpenAI` Chat Completions: `messages[]`.
    #[serde(rename = "openai-chat-completions")]
    ChatCompletions,
    /// `OpenAI` Responses: `instructions` plus `input[]` items.
    #[serde(rename = "openai-responses")]
    Responses,
}

impl Api {
    /// The api's wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Messages => "anthropic-messages",
            Self::ChatCompletions => "openai-chat-completions",
            Self::Responses => "openai-responses",
        }
    }
}

/// How a call ended. Only `Complete` carries a response part list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallStatus {
    /// The response ended as the provider meant it to.
    Complete,
    /// The client closed before the response ended.
    Cancelled,
    /// The upstream stream ended early or malformed.
    Partial,
    /// The call was forwarded but its capture could not be written.
    Unrecorded,
    /// The process died mid-call; known from the open-call journal.
    Lost,
}

/// The data of a `lys.call` entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallRecord {
    /// The proxy's stable id for this call.
    pub call_id: String,
    /// The provider called.
    pub provider: String,
    /// The api the bodies follow.
    pub api: Api,
    /// The model asked for; absent when the request never became readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The request's parts, by block hash, in order; empty when the request
    /// was not readable.
    pub request: Vec<String>,
    /// The response's parts, by block hash, in order; non-empty only when
    /// the status is `complete`.
    pub response: Vec<String>,
    /// The raw request body, by block hash; absent when no body exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_request: Option<String>,
    /// The raw response body, by block hash; absent when no body exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<String>,
    /// How the call ended.
    pub status: CallStatus,
    /// When the call started, RFC 3339.
    pub started_at: String,
    /// How long it took, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// Whether the response was streamed.
    pub stream: bool,
}

/// What an ingest reported: counts only.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestReport {
    /// The entry appended, or the one already holding this call id.
    pub entry_id: String,
    /// `true` when the call id was already recorded and nothing was written.
    pub already_recorded: bool,
    /// Request and response parts whose bytes were not yet held.
    pub part_blocks_new: u64,
    /// Request and response parts already held.
    pub part_blocks_reused: u64,
    /// Raw bodies stored (2 when both were new).
    pub raw_blocks: u64,
    /// Request parts found.
    pub request_parts: u64,
    /// Response parts found.
    pub response_parts: u64,
}

/// What a complete call carries besides its bodies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallMeta {
    /// The proxy's stable id for this call.
    pub call_id: String,
    /// The provider.
    pub provider: String,
    /// The api.
    pub api: Api,
    /// The model.
    pub model: String,
    /// When it started, RFC 3339.
    pub started_at: String,
    /// How long it took.
    pub duration_ms: u64,
    /// Whether streamed.
    pub stream: bool,
}

/// What a call that did not complete carries: only what is known.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomeMeta {
    /// The proxy's stable id for this call.
    pub call_id: String,
    /// The provider.
    pub provider: String,
    /// The api.
    pub api: Api,
    /// How it ended; never `Complete` here.
    pub status: CallStatus,
    /// When it started, RFC 3339.
    pub started_at: String,
    /// How long it took, when known.
    pub duration_ms: Option<u64>,
    /// Whether streamed.
    pub stream: bool,
}

/// Split a request body into its parts, in order, by api.
pub fn request_parts(api: Api, body: &[u8]) -> Result<Vec<Value>, HomeError> {
    let json: Value = serde_json::from_slice(body).map_err(|source| HomeError::Json {
        context: "the request is not JSON",
        source,
    })?;
    request_parts_of(api, &json)
}

/// Read a request body from a file and split it, holding only the parsed
/// value, never a second copy of the bytes.
pub fn request_parts_file(api: Api, path: &Path) -> Result<Vec<Value>, HomeError> {
    let json = read_json(path, "the request is not JSON")?;
    request_parts_of(api, &json)
}

/// The model a request body names, when it does.
#[must_use]
pub fn request_model(body: &Value) -> Option<String> {
    body.get("model").and_then(Value::as_str).map(str::to_owned)
}

/// Read a body file as JSON. A file that cannot be read (missing, a
/// directory, a failing disk) is an I/O error; one that reads but does not
/// parse is a JSON error; callers tell the two apart.
fn read_json(path: &Path, reason: &'static str) -> Result<Value, HomeError> {
    let bytes = std::fs::read(path).map_err(|e| HomeError::io("reading a body file", path, e))?;
    serde_json::from_slice(&bytes).map_err(|source| HomeError::Json {
        context: reason,
        source,
    })
}

fn request_parts_of(api: Api, json: &Value) -> Result<Vec<Value>, HomeError> {
    let mut parts = Vec::new();
    match api {
        Api::Messages => {
            if let Some(system) = json.get("system") {
                push_content(system, &mut parts);
            }
            let messages =
                json.get("messages")
                    .and_then(Value::as_array)
                    .ok_or(HomeError::BodyShape {
                        api: api.as_str(),
                        reason: "no messages array",
                    })?;
            for message in messages {
                match message.get("content") {
                    Some(content) => push_content(content, &mut parts),
                    None => parts.push(message.clone()),
                }
            }
        }
        Api::ChatCompletions => {
            let messages =
                json.get("messages")
                    .and_then(Value::as_array)
                    .ok_or(HomeError::BodyShape {
                        api: api.as_str(),
                        reason: "no messages array",
                    })?;
            parts.extend(messages.iter().cloned());
        }
        Api::Responses => {
            if let Some(instructions) = json.get("instructions") {
                parts.push(instructions.clone());
            }
            match json.get("input") {
                Some(Value::Array(items)) => parts.extend(items.iter().cloned()),
                Some(other) => parts.push(other.clone()),
                None => {
                    return Err(HomeError::BodyShape {
                        api: api.as_str(),
                        reason: "no input",
                    });
                }
            }
        }
    }
    Ok(parts)
}

/// Split a complete JSON response body into its parts, in order, by api. A
/// body that is not JSON, or not the shape the api answers with (an error
/// body, an empty object), is refused by name: a complete call's response is
/// never recorded from a body that holds no parts. A raw event stream never
/// comes here: the proxy, which reads the stream as it forwards it, supplies
/// the parts it assembled as `response_parts` to [`ingest_call`] or
/// [`ingest_call_files`].
pub fn response_parts(api: Api, body: &[u8]) -> Result<Vec<Value>, HomeError> {
    let json = serde_json::from_slice::<Value>(body).map_err(|source| HomeError::Json {
        context: "the response is not JSON",
        source,
    })?;
    response_parts_of(api, &json)
}

fn response_parts_of(api: Api, json: &Value) -> Result<Vec<Value>, HomeError> {
    let mut parts = Vec::new();
    let shape = |reason: &'static str| HomeError::BodyShape {
        api: api.as_str(),
        reason,
    };
    match api {
        // Each api's successful shape is required, not merely its member found:
        // an error body, a null content, a choice without a message or a
        // response whose status is not completed is no complete response.
        Api::Messages => {
            if json.get("type").and_then(Value::as_str) == Some("error") {
                return Err(shape("an error body is not a complete response"));
            }
            let content = json
                .get("content")
                .and_then(Value::as_array)
                .ok_or_else(|| shape("content is not an array"))?;
            parts.extend(content.iter().cloned());
        }
        Api::ChatCompletions => {
            let choices = json
                .get("choices")
                .and_then(Value::as_array)
                .filter(|c| !c.is_empty())
                .ok_or_else(|| shape("no choices"))?;
            for choice in choices {
                let message = choice
                    .get("message")
                    .filter(|m| m.is_object())
                    .ok_or_else(|| shape("a choice without a message"))?;
                parts.push(message.clone());
            }
        }
        Api::Responses => {
            if json.get("status").and_then(Value::as_str) != Some("completed") {
                return Err(shape("status is not completed"));
            }
            let output = json
                .get("output")
                .and_then(Value::as_array)
                .ok_or_else(|| shape("no output array"))?;
            parts.extend(output.iter().cloned());
        }
    }
    Ok(parts)
}

/// The response parts of a complete call: the proxy's assembled parts when it
/// supplied them, else the parts of the JSON body; a streamed call without
/// assembled parts is refused, as is a body that holds no parts.
fn complete_response_parts(
    meta: &CallMeta,
    supplied: Option<Vec<Value>>,
    body: impl FnOnce() -> Result<Value, HomeError>,
) -> Result<Vec<Value>, HomeError> {
    match supplied {
        Some(parts) => Ok(parts),
        None if meta.stream => Err(HomeError::BodyShape {
            api: meta.api.as_str(),
            reason: "a streamed response needs the proxy's assembled parts",
        }),
        None => response_parts_of(meta.api, &body()?),
    }
}

fn push_content(content: &Value, parts: &mut Vec<Value>) {
    match content {
        Value::Array(items) => parts.extend(items.iter().cloned()),
        other => parts.push(other.clone()),
    }
}

/// The `lys.call` entry anywhere in the session's file that holds this call
/// id, if any: every durable entry is searched, whatever the head, so a crash
/// after the append and before the head advanced still finds the call.
pub fn find_call(session: &Session, call_id: &str) -> Result<Option<String>, HomeError> {
    for entry in session.customs_everywhere(CUSTOM_CALL)? {
        if let EntryBody::Custom {
            data: Some(data), ..
        } = &entry.body
            && data.get("call_id").and_then(Value::as_str) == Some(call_id)
        {
            return Ok(Some(entry.id().to_owned()));
        }
    }
    Ok(None)
}

/// Store a complete call's parts and raw bodies as blocks and append one
/// `lys.call` entry under the session's head. Idempotent on the call id.
pub fn ingest_call(
    session: &mut Session,
    blocks: &BlockStore,
    meta: &CallMeta,
    request_body: &[u8],
    response_body: &[u8],
    response_parts: Option<Vec<Value>>,
) -> Result<IngestReport, HomeError> {
    if let Some(entry_id) = find_call(session, &meta.call_id)? {
        return Ok(already(entry_id));
    }
    let req = request_parts(meta.api, request_body)?;
    let resp = complete_response_parts(meta, response_parts, || {
        serde_json::from_slice::<Value>(response_body).map_err(|source| HomeError::Json {
            context: "the response is not JSON",
            source,
        })
    })?;
    let raw_req = blocks.put(request_body)?;
    let raw_resp = blocks.put(response_body)?;
    let record = CallRecord {
        call_id: meta.call_id.clone(),
        provider: meta.provider.clone(),
        api: meta.api,
        model: Some(meta.model.clone()),
        request: Vec::new(),
        response: Vec::new(),
        raw_request: Some(raw_req.hash.to_string()),
        raw_response: Some(raw_resp.hash.to_string()),
        status: CallStatus::Complete,
        started_at: meta.started_at.clone(),
        duration_ms: Some(meta.duration_ms),
        stream: meta.stream,
    };
    finish_ingest(
        session,
        blocks,
        record,
        &req,
        &resp,
        raw_req.new,
        raw_resp.new,
    )
}

/// As [`ingest_call`], with the bodies in files as a proxy that spools while
/// forwarding hands them over: the raw bodies are streamed into the store,
/// the request is parsed from its file, and the response's parts are either
/// parsed from a JSON body or supplied by the proxy for an event stream.
pub fn ingest_call_files(
    session: &mut Session,
    blocks: &BlockStore,
    meta: &CallMeta,
    request_file: &Path,
    response_file: &Path,
    response_parts: Option<Vec<Value>>,
) -> Result<IngestReport, HomeError> {
    if let Some(entry_id) = find_call(session, &meta.call_id)? {
        return Ok(already(entry_id));
    }
    let req = request_parts_file(meta.api, request_file)?;
    let resp = complete_response_parts(meta, response_parts, || {
        read_json(response_file, "the response is not JSON")
    })?;
    let raw_req = blocks.put_file(request_file)?;
    let raw_resp = blocks.put_file(response_file)?;
    let record = CallRecord {
        call_id: meta.call_id.clone(),
        provider: meta.provider.clone(),
        api: meta.api,
        model: Some(meta.model.clone()),
        request: Vec::new(),
        response: Vec::new(),
        raw_request: Some(raw_req.hash.to_string()),
        raw_response: Some(raw_resp.hash.to_string()),
        status: CallStatus::Complete,
        started_at: meta.started_at.clone(),
        duration_ms: Some(meta.duration_ms),
        stream: meta.stream,
    };
    finish_ingest(
        session,
        blocks,
        record,
        &req,
        &resp,
        raw_req.new,
        raw_resp.new,
    )
}

/// Record a call that did not complete, from whatever exists: a request file
/// that parses gives its parts and model; one that does not, or none, gives
/// neither; a response file is kept raw, and never yields response parts.
/// Idempotent on the call id. Refuses `Complete`.
pub fn ingest_outcome(
    session: &mut Session,
    blocks: &BlockStore,
    meta: &OutcomeMeta,
    request_file: Option<&Path>,
    response_file: Option<&Path>,
) -> Result<IngestReport, HomeError> {
    if meta.status == CallStatus::Complete {
        return Err(HomeError::BodyShape {
            api: meta.api.as_str(),
            reason: "a complete call goes through ingest_call",
        });
    }
    if let Some(entry_id) = find_call(session, &meta.call_id)? {
        return Ok(already(entry_id));
    }
    // A request that is not JSON, or not the api's shape, is recorded as absence
    // (a half-written file is the usual case); a request that cannot be read
    // is an error, since absence and an unreadable disk are not the same thing.
    let (req, model) = match request_file {
        Some(path) => match read_json(path, "the request is not JSON") {
            Ok(json) => (
                request_parts_of(meta.api, &json).unwrap_or_default(),
                request_model(&json),
            ),
            Err(HomeError::Json { .. }) => (Vec::new(), None),
            Err(e) => return Err(e),
        },
        None => (Vec::new(), None),
    };
    let raw_req = request_file.map(|p| blocks.put_file(p)).transpose()?;
    let raw_resp = response_file.map(|p| blocks.put_file(p)).transpose()?;
    let raw_new = u64::from(raw_req.as_ref().is_some_and(|p| p.new))
        + u64::from(raw_resp.as_ref().is_some_and(|p| p.new));
    let record = CallRecord {
        call_id: meta.call_id.clone(),
        provider: meta.provider.clone(),
        api: meta.api,
        model,
        request: Vec::new(),
        response: Vec::new(),
        raw_request: raw_req.map(|p| p.hash.to_string()),
        raw_response: raw_resp.map(|p| p.hash.to_string()),
        status: meta.status,
        started_at: meta.started_at.clone(),
        duration_ms: meta.duration_ms,
        stream: meta.stream,
    };
    finish_ingest_counted(session, blocks, record, &req, &[], raw_new)
}

fn already(entry_id: String) -> IngestReport {
    IngestReport {
        entry_id,
        already_recorded: true,
        part_blocks_new: 0,
        part_blocks_reused: 0,
        raw_blocks: 0,
        request_parts: 0,
        response_parts: 0,
    }
}

fn finish_ingest(
    session: &mut Session,
    blocks: &BlockStore,
    record: CallRecord,
    req: &[Value],
    resp: &[Value],
    raw_req_new: bool,
    raw_resp_new: bool,
) -> Result<IngestReport, HomeError> {
    finish_ingest_counted(
        session,
        blocks,
        record,
        req,
        resp,
        u64::from(raw_req_new) + u64::from(raw_resp_new),
    )
}

fn finish_ingest_counted(
    session: &mut Session,
    blocks: &BlockStore,
    mut record: CallRecord,
    req: &[Value],
    resp: &[Value],
    raw_blocks: u64,
) -> Result<IngestReport, HomeError> {
    let mut new = 0u64;
    let mut reused = 0u64;
    let mut put_parts = |parts: &[Value]| -> Result<Vec<String>, HomeError> {
        let mut hashes = Vec::with_capacity(parts.len());
        for part in parts {
            let bytes = serde_json::to_vec(part).map_err(|source| HomeError::Json {
                context: "a part could not be serialised",
                source,
            })?;
            let put: Put = blocks.put(&bytes)?;
            if put.new {
                new += 1;
            } else {
                reused += 1;
            }
            hashes.push(put.hash.to_string());
        }
        Ok(hashes)
    };
    record.request = put_parts(req)?;
    // Only a complete call carries response parts.
    record.response = if record.status == CallStatus::Complete {
        put_parts(resp)?
    } else {
        Vec::new()
    };
    let response_parts = record.response.len() as u64;
    let data = serde_json::to_value(&record).map_err(|source| HomeError::Json {
        context: "the call record could not be serialised",
        source,
    })?;
    let entry_id = session.append(EntryBody::Custom {
        custom_type: CUSTOM_CALL.to_owned(),
        data: Some(data),
    })?;
    Ok(IngestReport {
        entry_id,
        already_recorded: false,
        part_blocks_new: new,
        part_blocks_reused: reused,
        raw_blocks,
        request_parts: req.len() as u64,
        response_parts,
    })
}

/// Read a `lys.call` entry's data back.
pub fn call_record(data: &Value) -> Result<CallRecord, HomeError> {
    serde_json::from_value(data.clone()).map_err(|source| HomeError::Json {
        context: "the entry's data is not a call record",
        source,
    })
}

/// A hash named in a record, checked.
pub fn named_hash(text: &str) -> Result<Hash, HomeError> {
    Hash::parse(text)
}
