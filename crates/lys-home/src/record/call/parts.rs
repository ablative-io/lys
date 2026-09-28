//! The parts of a call's bodies: a request or a response body split, in
//! order, by the api it follows.

use std::path::Path;

use serde_json::Value;

use crate::error::HomeError;
use crate::record::call::{Api, CallMeta};

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
pub(super) fn read_json(path: &Path, reason: &'static str) -> Result<Value, HomeError> {
    let bytes = std::fs::read(path).map_err(|e| HomeError::io("reading a body file", path, e))?;
    serde_json::from_slice(&bytes).map_err(|source| HomeError::Json {
        context: reason,
        source,
    })
}

pub(super) fn request_parts_of(api: Api, json: &Value) -> Result<Vec<Value>, HomeError> {
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
///
/// [`ingest_call`]: crate::record::call::ingest_call
/// [`ingest_call_files`]: crate::record::call::ingest_call_files
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
pub(super) fn complete_response_parts(
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
