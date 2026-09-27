//! One message entry as Codex 0.156.0 response items, with its account rows
//! (HOME-009 R3).
//!
//! Invariants:
//!
//! - Every text part, tool call and tool result is carried whole: a text
//!   byte for byte, a tool call's arguments as `serde_json::to_string` of
//!   the part's arguments, a tool result's text byte for byte. Nothing is
//!   clipped or summarised.
//! - Every thinking part is treated as another provider's: readable thinking
//!   becomes an `output_text` part and is counted changed; redacted and empty
//!   thinking are lost by hash. No signature, redacted data or reasoning item
//!   is written.
//! - A base64 image becomes an `input_image` item with a data URL and no
//!   `detail` key; an image of any other source is lost by hash with its
//!   source type and index, and nothing is fetched or read to carry it.
//! - A part is kept only when its item carries every field it holds; a field
//!   beyond those is named `<field> not carried` in a changed row.
//! - A missing role, id, name or `toolCallId` is never given a default: the
//!   part or message is left out and is a lost row with its reason.
//! - No item carries an `id` key.

use serde_json::{Value, json};

use crate::error::HomeError;
use crate::harness::codex::account::{Rows, part_hash};

/// Why redacted thinking is not carried.
pub const REDACTED: &str = "redacted thinking dropped: another provider";
/// Why thinking with no readable text is not carried.
pub const EMPTY_THINKING: &str = "empty thinking dropped";
/// Why a tool call with no id is not carried.
pub const NO_CALL_ID: &str = "toolCall with no id: Codex's function_call needs a call_id";
/// Why a tool call with no name is not carried.
pub const NO_CALL_NAME: &str = "toolCall with no name: Codex's function_call needs a name";
/// Why a tool call with no arguments is not carried.
pub const NO_CALL_ARGUMENTS: &str =
    "toolCall with no arguments: Codex's function_call needs arguments";
/// How a tool result's one text part is reshaped.
const ONE_ITEM: &str = "content: one-item text array written as a string";
/// Why a base64 image missing its media type or data is not carried.
const NO_IMAGE_DATA: &str = "image source base64 with no media_type or data not carried";
/// Why a tool result with no `toolCallId` is not carried.
pub const NO_RESULT_ID: &str =
    "toolResult with no toolCallId: Codex's function_call_output needs a call_id";
/// Why a message of a role other than user, assistant and tool result is
/// not carried.
pub const OTHER_ROLE: &str = "Codex has no item for a message of this role";
/// The kind a message with no string role is listed under.
pub const NO_ROLE: &str = "message with no role";
/// The fields a thinking part may hold that the translation reads.
const THINKING_FIELDS: [&str; 4] = ["type", "thinking", "thinkingSignature", "redacted"];
/// The fields of a tool call its `function_call` carries.
const CALL_FIELDS: [&str; 4] = ["type", "id", "name", "arguments"];

/// A thinking part as the translation reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thinking<'a> {
    /// Readable text.
    Readable(&'a str),
    /// Redacted: opaque data only.
    Redacted,
    /// No readable text.
    Empty,
}

/// Read a thinking part.
#[must_use]
pub fn thinking_of(part: &Value) -> Thinking<'_> {
    if part.get("redacted").and_then(Value::as_bool) == Some(true) {
        return Thinking::Redacted;
    }
    match part.get("thinking").and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Thinking::Readable(text),
        _ => Thinking::Empty,
    }
}

/// A string field that is present, a string and not empty.
#[must_use]
pub fn named<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

/// A tool call's id, name and arguments, or the reason it cannot be carried.
pub fn check_call(part: &Value) -> Result<(&str, &str, &Value), &'static str> {
    let id = named(part, "id").ok_or(NO_CALL_ID)?;
    let name = named(part, "name").ok_or(NO_CALL_NAME)?;
    let arguments = part.get("arguments").ok_or(NO_CALL_ARGUMENTS)?;
    Ok((id, name, arguments))
}

/// A tool call's arguments as compact JSON, whole.
pub fn arguments_json(arguments: &Value) -> Result<String, HomeError> {
    serde_json::to_string(arguments).map_err(|source| HomeError::Json {
        context: "a tool call's arguments could not be serialised",
        source,
    })
}

/// `<key> not carried` for each key of an object beyond `carried`, sorted.
#[must_use]
pub fn not_carried(value: &Value, carried: &[&str]) -> Vec<String> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    let mut keys: Vec<&String> = object
        .keys()
        .filter(|key| !carried.contains(&key.as_str()))
        .collect();
    keys.sort();
    keys.into_iter()
        .map(|key| format!("{key} not carried"))
        .collect()
}

/// The kind a part is listed under, and why it is not carried when its kind
/// has no item here.
fn lose_kind(entry: &str, part: &Value, rows: &mut Rows) -> Result<(), HomeError> {
    let hash = part_hash(part)?;
    match part.get("type").and_then(Value::as_str) {
        Some(kind) => rows.lose(
            entry,
            Some(hash),
            kind,
            &format!("Codex has no item for kind {kind} in this render"),
        ),
        None => rows.lose(
            entry,
            Some(hash),
            "part with no type",
            "Codex has no item for a part with no type in this render",
        ),
    }
    Ok(())
}

/// A text part as an `input_text` or `output_text` part.
fn text_part(
    entry: &str,
    part: &Value,
    after: &str,
    rows: &mut Rows,
) -> Result<Option<Value>, HomeError> {
    let hash = part_hash(part)?;
    let Some(text) = part.get("text").and_then(Value::as_str) else {
        rows.lose(
            entry,
            Some(hash),
            "text",
            "text part with no text: nothing to carry",
        );
        return Ok(None);
    };
    let how = not_carried(part, &["type", "text"]);
    if how.is_empty() {
        rows.keep(entry, Some(hash), "text", after);
    } else {
        rows.change(entry, Some(hash), "text", after, &how.join("; "));
    }
    Ok(Some(json!({"type": after, "text": text})))
}

/// A string field of an image part's source.
fn source_str<'a>(source: Option<&'a Value>, key: &str) -> Option<&'a str> {
    source.and_then(|s| s.get(key)).and_then(Value::as_str)
}

/// An image part: a base64 source as an `input_image` item, any other
/// source lost with its type and index, never fetched.
fn image_part(
    entry: &str,
    part: &Value,
    index: usize,
    rows: &mut Rows,
) -> Result<Option<Value>, HomeError> {
    let hash = part_hash(part)?;
    let source = part.get("source");
    match source_str(source, "type") {
        Some("base64") => {
            let media = source_str(source, "media_type");
            let data = source_str(source, "data");
            let (Some(media), Some(data)) = (media, data) else {
                rows.lose(
                    entry,
                    Some(hash),
                    "image",
                    &format!("{NO_IMAGE_DATA}: part {index}"),
                );
                return Ok(None);
            };
            let mut how = vec!["source written as a data URL image_url".to_owned()];
            how.extend(not_carried(part, &["type", "source"]));
            rows.change(entry, Some(hash), "image", "input_image", &how.join("; "));
            Ok(Some(json!({
                "type": "input_image",
                "image_url": format!("data:{media};base64,{data}"),
            })))
        }
        Some(other) => {
            rows.lose(
                entry,
                Some(hash),
                "image",
                &format!("image source {other} not carried: part {index}, never fetched"),
            );
            Ok(None)
        }
        None => {
            rows.lose(
                entry,
                Some(hash),
                "image",
                &format!("image source with no type not carried: part {index}, never fetched"),
            );
            Ok(None)
        }
    }
}

/// A thinking part as an `output_text` part, or lost.
fn thinking_part(entry: &str, part: &Value, rows: &mut Rows) -> Result<Option<Value>, HomeError> {
    let hash = part_hash(part)?;
    match thinking_of(part) {
        Thinking::Redacted => {
            rows.lose(entry, Some(hash), "thinking", REDACTED);
            Ok(None)
        }
        Thinking::Empty => {
            rows.lose(entry, Some(hash), "thinking", EMPTY_THINKING);
            Ok(None)
        }
        Thinking::Readable(text) => {
            let first = if part.get("thinkingSignature").is_some() {
                "thinkingSignature not carried"
            } else {
                "thinking written as output_text"
            };
            let mut how = vec![first.to_owned()];
            how.extend(not_carried(part, &THINKING_FIELDS));
            let how = how.join("; ");
            rows.change(entry, Some(hash), "thinking", "output_text", &how);
            Ok(Some(json!({"type": "output_text", "text": text})))
        }
    }
}

/// A tool call as a `function_call` item, or lost.
fn tool_call(entry: &str, part: &Value, rows: &mut Rows) -> Result<Option<Value>, HomeError> {
    let hash = part_hash(part)?;
    let (call_id, name, arguments) = match check_call(part) {
        Ok(call) => call,
        Err(reason) => {
            rows.lose(entry, Some(hash), "toolCall", reason);
            return Ok(None);
        }
    };
    let arguments = arguments_json(arguments)?;
    let how = not_carried(part, &CALL_FIELDS).join("; ");
    if how.is_empty() {
        rows.keep(entry, Some(hash), "toolCall", "function_call");
    } else {
        rows.change(entry, Some(hash), "toolCall", "function_call", &how);
    }
    Ok(Some(json!({
        "type": "function_call",
        "name": name,
        "arguments": arguments,
        "call_id": call_id,
    })))
}

/// The entry-level row of a user or assistant message: every key beyond
/// role and content, named.
fn entry_row(entry: &str, role: &str, message: &Value, rows: &mut Rows) {
    let how = not_carried(message, &["role", "content"]);
    if !how.is_empty() {
        rows.change(
            entry,
            None,
            &format!("{role} message"),
            "message item",
            &how.join("; "),
        );
    }
}

/// A message whose content is neither a string nor a list of parts.
fn unshaped(entry: &str, message: &Value, kind: &str, rows: &mut Rows) -> Result<(), HomeError> {
    rows.lose(
        entry,
        Some(part_hash(message)?),
        kind,
        "message content is neither a string nor a list of parts",
    );
    Ok(())
}

fn user_items(entry: &str, message: &Value, rows: &mut Rows) -> Result<Vec<Value>, HomeError> {
    entry_row(entry, "user", message, rows);
    let mut content = Vec::new();
    match message.get("content") {
        Some(text @ Value::String(s)) => {
            rows.keep(entry, Some(part_hash(text)?), "text", "input_text");
            content.push(json!({"type": "input_text", "text": s}));
        }
        Some(Value::Array(parts)) => {
            for (index, part) in parts.iter().enumerate() {
                let item = match part.get("type").and_then(Value::as_str) {
                    Some("text") => text_part(entry, part, "input_text", rows)?,
                    Some("image") => image_part(entry, part, index, rows)?,
                    _ => {
                        lose_kind(entry, part, rows)?;
                        None
                    }
                };
                content.extend(item);
            }
        }
        _ => unshaped(entry, message, "user message", rows)?,
    }
    if content.is_empty() {
        return Ok(Vec::new());
    }
    let item = json!({"type": "message", "role": "user", "content": content});
    Ok(vec![item])
}

/// Close the run of text parts gathered so far as one assistant message.
fn flush(group: &mut Vec<Value>, items: &mut Vec<Value>) {
    if !group.is_empty() {
        let content = std::mem::take(group);
        items.push(json!({"type": "message", "role": "assistant", "content": content}));
    }
}

fn assistant_items(entry: &str, message: &Value, rows: &mut Rows) -> Result<Vec<Value>, HomeError> {
    entry_row(entry, "assistant", message, rows);
    let mut items = Vec::new();
    let mut group = Vec::new();
    match message.get("content") {
        Some(text @ Value::String(s)) => {
            rows.keep(entry, Some(part_hash(text)?), "text", "output_text");
            group.push(json!({"type": "output_text", "text": s}));
        }
        Some(Value::Array(parts)) => {
            for part in parts {
                match part.get("type").and_then(Value::as_str) {
                    Some("text") => group.extend(text_part(entry, part, "output_text", rows)?),
                    Some("thinking") => group.extend(thinking_part(entry, part, rows)?),
                    Some("toolCall") => {
                        if let Some(call) = tool_call(entry, part, rows)? {
                            flush(&mut group, &mut items);
                            items.push(call);
                        }
                    }
                    _ => lose_kind(entry, part, rows)?,
                }
            }
        }
        _ => unshaped(entry, message, "assistant message", rows)?,
    }
    flush(&mut group, &mut items);
    Ok(items)
}

/// A tool result's content as `function_call_output`'s `output`: one text
/// part as a string, otherwise a list of `input_text` and `input_image`
/// items in content order. The notes name every reshape and every part
/// field not carried.
fn result_output(
    entry: &str,
    parts: &[Value],
    notes: &mut Vec<String>,
    rows: &mut Rows,
) -> Result<Value, HomeError> {
    if let [only] = parts
        && only.get("type").and_then(Value::as_str) == Some("text")
        && let Some(text) = only.get("text").and_then(Value::as_str)
    {
        notes.push(ONE_ITEM.to_owned());
        for field in not_carried(only, &["type", "text"]) {
            notes.push(format!("part 0 {field}"));
        }
        return Ok(Value::String(text.to_owned()));
    }
    let mut output = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        match part.get("type").and_then(Value::as_str) {
            Some("text") => match part.get("text").and_then(Value::as_str) {
                Some(text) => {
                    for field in not_carried(part, &["type", "text"]) {
                        notes.push(format!("part {index} {field}"));
                    }
                    output.push(json!({"type": "input_text", "text": text}));
                }
                None => rows.lose(
                    entry,
                    Some(part_hash(part)?),
                    "text",
                    "text part with no text: nothing to carry",
                ),
            },
            Some("image") => output.extend(image_part(entry, part, index, rows)?),
            _ => lose_kind(entry, part, rows)?,
        }
    }
    Ok(Value::Array(output))
}

fn tool_result_items(
    entry: &str,
    message: &Value,
    rows: &mut Rows,
) -> Result<Vec<Value>, HomeError> {
    let hash = part_hash(message)?;
    let Some(call_id) = named(message, "toolCallId") else {
        rows.lose(entry, Some(hash), "toolResult", NO_RESULT_ID);
        return Ok(Vec::new());
    };
    let mut notes = Vec::new();
    let mut part_rows = Rows::default();
    let output = match message.get("content") {
        Some(Value::Array(parts)) => result_output(entry, parts, &mut notes, &mut part_rows)?,
        Some(Value::String(text)) => Value::String(text.clone()),
        _ => {
            unshaped(entry, message, "toolResult", rows)?;
            return Ok(Vec::new());
        }
    };
    notes.extend(not_carried(message, &["role", "toolCallId", "content"]));
    rows.change(
        entry,
        Some(hash),
        "toolResult",
        "function_call_output",
        &notes.join("; "),
    );
    rows.extend(part_rows);
    Ok(vec![json!({
        "type": "function_call_output",
        "call_id": call_id,
        "output": output,
    })])
}

/// One message entry's response items, in part order, with its rows.
pub fn message_items(
    entry: &str,
    message: &Value,
    rows: &mut Rows,
) -> Result<Vec<Value>, HomeError> {
    match message.get("role").and_then(Value::as_str) {
        Some("user") => user_items(entry, message, rows),
        Some("assistant") => assistant_items(entry, message, rows),
        Some("toolResult") => tool_result_items(entry, message, rows),
        Some(role) => {
            rows.lose(
                entry,
                Some(part_hash(message)?),
                &format!("{role} message"),
                OTHER_ROLE,
            );
            Ok(Vec::new())
        }
        None => {
            rows.lose(entry, Some(part_hash(message)?), NO_ROLE, OTHER_ROLE);
            Ok(Vec::new())
        }
    }
}
