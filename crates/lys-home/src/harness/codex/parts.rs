//! One message entry into Codex 0.156.0 response items (HOME-009 R3).
//!
//! A user message becomes one `message` item of `input_text` parts, a base64
//! image among them as `input_image`; an assistant message becomes, in part
//! order, `message` items of `output_text` parts (text, and readable thinking
//! as another provider's) broken by one `function_call` per tool call; a
//! tool result becomes one `function_call_output`. Every text, argument and
//! output is copied whole. Redacted and empty thinking, an image of any
//! source but base64, a part of a kind Codex has no item for, a tool call with
//! no id or name, a tool result with no call id and a message of any other
//! role are left out and are lost rows with their reason; nothing missing is
//! given an empty default. A part holding a field its item does not carry is
//! changed rather than kept, naming the field; so is a message holding a key
//! beyond role and content. No item carries an `id`, no `input_image` a
//! `detail`, and no thinking signature or redacted data is written anywhere.
//!
//! An item borrows every text, argument, output and image it carries from
//! the entry and is serialised straight into the rollout; its keys are
//! declared in the sorted order the rollout's lines hold.

use std::fmt;

use serde::Serialize;
use serde::ser::Serializer;
use serde_json::{Map, Value};

use crate::harness::codex::account::{Rows, json_hash, part_hash};

/// Why a redacted thinking part is lost.
pub const REDACTED: &str = "redacted thinking dropped: another provider";
/// Why a thinking part with no readable text is lost.
pub const EMPTY_THINKING: &str = "empty thinking dropped";
/// Why a tool call with no id is lost.
pub const NO_CALL_ID: &str = "toolCall with no id: Codex's function_call needs a call_id";
/// Why a tool call with no name is lost.
pub const NO_CALL_NAME: &str = "toolCall with no name: Codex's function_call needs a name";
/// Why a tool call with no arguments is lost.
pub const NO_ARGUMENTS: &str = "toolCall with no arguments: Codex's function_call needs arguments";
/// Why a tool result with no call id is lost.
pub const NO_RESULT_ID: &str =
    "toolResult with no toolCallId: Codex's function_call_output needs a call_id";
/// Why a tool result whose content is neither a string nor a list is lost.
pub const NO_OUTPUT: &str =
    "toolResult with no content: Codex's function_call_output needs an output";
/// Why a message of a role Codex has no item for is lost.
pub const NO_ROLE_ITEM: &str = "Codex has no item for a message of this role";
/// Why a user or assistant message with no content is lost.
pub const NO_CONTENT: &str = "message with no content: Codex's message item needs content";
/// Why a text part with no text is lost.
pub const NO_TEXT: &str = "text part with no text: Codex's text parts need a text";

/// A part's `type`, or the words naming its absence.
pub(crate) fn kind_of(part: &Value) -> &str {
    part.get("type")
        .and_then(Value::as_str)
        .unwrap_or("part with no type")
}

/// Why a part of a kind Codex has no item for is lost.
fn no_item(kind: &str) -> String {
    format!("Codex has no item for kind {kind} in this render")
}

/// A key's value when it is a string and not empty.
pub(crate) fn named<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
}

/// Each key of an object outside `carried`, sorted, as `<key> not carried`.
pub(crate) fn not_carried(value: &Value, carried: &[&str]) -> Vec<String> {
    let mut keys: Vec<&str> = value
        .as_object()
        .into_iter()
        .flat_map(Map::keys)
        .map(String::as_str)
        .filter(|key| !carried.contains(key))
        .collect();
    keys.sort_unstable();
    keys.into_iter()
        .map(|key| format!("{key} not carried"))
        .collect()
}

/// One response item, borrowing what it carries.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Item<'a> {
    /// A `message` of content parts.
    Message {
        /// The parts.
        content: Vec<Part<'a>>,
        /// The role.
        role: &'a str,
        /// `message`.
        #[serde(rename = "type")]
        kind: &'static str,
    },
    /// A `function_call`.
    FunctionCall {
        /// The arguments, written as their compact JSON in a string.
        arguments: Compact<'a>,
        /// The call's id.
        call_id: &'a str,
        /// The tool's name.
        name: &'a str,
        /// `function_call`.
        #[serde(rename = "type")]
        kind: &'static str,
    },
    /// A `function_call_output`.
    FunctionCallOutput {
        /// The call's id.
        call_id: &'a str,
        /// The output.
        output: Output<'a>,
        /// `function_call_output`.
        #[serde(rename = "type")]
        kind: &'static str,
    },
}

impl<'a> Item<'a> {
    /// A `message` item.
    pub(crate) fn message(role: &'a str, content: Vec<Part<'a>>) -> Self {
        Self::Message {
            content,
            role,
            kind: "message",
        }
    }
}

/// One content part of a message or a tool output.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Part<'a> {
    /// An `input_text` or `output_text` part.
    Text {
        /// The text.
        text: &'a str,
        /// `input_text` or `output_text`.
        #[serde(rename = "type")]
        kind: &'static str,
    },
    /// An `input_image` part.
    Image {
        /// The image as a data URL.
        image_url: DataUrl<'a>,
        /// `input_image`.
        #[serde(rename = "type")]
        kind: &'static str,
    },
}

/// A tool output: one string, or a list of parts.
#[derive(Serialize)]
#[serde(untagged)]
pub(crate) enum Output<'a> {
    /// A string.
    Text(&'a str),
    /// A list of parts.
    List(Vec<Part<'a>>),
}

/// A base64 image as a data URL, written without building the URL first.
pub(crate) struct DataUrl<'a> {
    media: &'a str,
    data: &'a str,
}

impl fmt::Display for DataUrl<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "data:{};base64,{}", self.media, self.data)
    }
}

impl Serialize for DataUrl<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A JSON value written as a string of its compact JSON, without building
/// the string first.
pub(crate) struct Compact<'a>(&'a Value);

impl Serialize for Compact<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self.0)
    }
}

/// What a thinking part holds.
pub(crate) enum Thinking<'a> {
    /// Readable text.
    Readable(&'a str),
    /// A redacted block.
    Redacted,
    /// No readable text.
    Empty,
}

/// Read a thinking part.
pub(crate) fn thinking_of(part: &Value) -> Thinking<'_> {
    if part.get("redacted").and_then(Value::as_bool) == Some(true) {
        return Thinking::Redacted;
    }
    match part.get("thinking").and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Thinking::Readable(text),
        _ => Thinking::Empty,
    }
}

/// A tool call's id and name, or why it cannot be carried.
pub(crate) fn call_of(part: &Value) -> Result<(&str, &str), &'static str> {
    let id = named(part, "id").ok_or(NO_CALL_ID)?;
    let name = named(part, "name").ok_or(NO_CALL_NAME)?;
    Ok((id, name))
}

/// A message's role when Codex has an item for it, or the lost row's kind.
pub(crate) fn role_of(message: &Value) -> Result<&str, String> {
    match message.get("role").and_then(Value::as_str) {
        Some(role @ ("user" | "assistant" | "toolResult")) => Ok(role),
        Some(role) => Err(format!("{role} message")),
        None => Err("message with no role".to_owned()),
    }
}

/// Add a part's row: kept when every field is carried, otherwise changed,
/// naming `lead` first when given and then each field not carried.
fn part_row(
    rows: &mut Rows,
    entry: &str,
    part: &Value,
    (before, after): (&str, &str),
    lead: Option<&str>,
    carried: &[&str],
) {
    let mut how: Vec<String> = lead.map(str::to_owned).into_iter().collect();
    how.extend(not_carried(part, carried));
    if how.is_empty() {
        rows.kept(entry, Some(part_hash(part)), before, after);
    } else {
        rows.changed(entry, Some(part_hash(part)), before, after, &how.join("; "));
    }
}

/// The items of one message entry, adding its rows to the account, as JSON
/// values. A borrowed item always converts; were one not to, it would stand
/// as null.
pub fn message_items(entry: &str, message: &Value, rows: &mut Rows) -> Vec<Value> {
    items_of(entry, message, rows)
        .iter()
        .map(|item| serde_json::to_value(item).unwrap_or(Value::Null))
        .collect()
}

/// The items of one message entry, borrowing from it, adding its rows to
/// the account.
pub(crate) fn items_of<'a>(entry: &str, message: &'a Value, rows: &mut Rows) -> Vec<Item<'a>> {
    let role = match role_of(message) {
        Ok(role) => role,
        Err(kind) => {
            rows.lost(entry, Some(part_hash(message)), &kind, NO_ROLE_ITEM);
            return Vec::new();
        }
    };
    if role == "toolResult" {
        return tool_result(entry, message, rows).into_iter().collect();
    }
    let how = not_carried(message, &["role", "content"]);
    if !how.is_empty() {
        rows.changed(
            entry,
            None,
            &format!("{role} message"),
            "message item",
            &how.join("; "),
        );
    }
    match message.get("content") {
        Some(Value::String(text)) => {
            let kind = if role == "user" {
                "input_text"
            } else {
                "output_text"
            };
            rows.kept(entry, Some(json_hash(text)), "text", kind);
            vec![Item::message(role, vec![Part::Text { text, kind }])]
        }
        Some(Value::Array(parts)) if role == "user" => user_items(entry, parts, rows),
        Some(Value::Array(parts)) => assistant_items(entry, parts, rows),
        _ => {
            rows.lost(entry, None, &format!("{role} message"), NO_CONTENT);
            Vec::new()
        }
    }
}

/// A text part as a `<kind>` part, or a lost row when it holds no text.
fn text_part<'a>(
    entry: &str,
    part: &'a Value,
    kind: &'static str,
    rows: &mut Rows,
) -> Option<Part<'a>> {
    let Some(text) = part.get("text").and_then(Value::as_str) else {
        rows.lost(entry, Some(part_hash(part)), "text", NO_TEXT);
        return None;
    };
    part_row(rows, entry, part, ("text", kind), None, &["type", "text"]);
    Some(Part::Text { text, kind })
}

/// An image part as `input_image` when its source is base64, with no
/// `detail` key; otherwise a lost row naming its source type and index, and
/// no byte of the source written anywhere.
fn image_part<'a>(entry: &str, index: usize, part: &'a Value, rows: &mut Rows) -> Option<Part<'a>> {
    let source = part.get("source");
    let source_type = source
        .and_then(|s| s.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("none");
    let media = source.and_then(|s| named(s, "media_type"));
    let data = source.and_then(|s| s.get("data")).and_then(Value::as_str);
    if let ("base64", Some(media), Some(data)) = (source_type, media, data) {
        part_row(
            rows,
            entry,
            part,
            ("image", "input_image"),
            Some("source written as a data URL image_url"),
            &["type", "source"],
        );
        return Some(Part::Image {
            image_url: DataUrl { media, data },
            kind: "input_image",
        });
    }
    let reason = format!("image source {source_type} not carried: part {index}, never fetched");
    rows.lost(entry, Some(part_hash(part)), "image", &reason);
    None
}

/// The content parts of a user message or a tool output list.
fn input_parts<'a>(entry: &str, parts: &'a [Value], rows: &mut Rows) -> Vec<Part<'a>> {
    let mut content = Vec::with_capacity(parts.len());
    for (index, part) in parts.iter().enumerate() {
        let item = match kind_of(part) {
            "text" => text_part(entry, part, "input_text", rows),
            "image" => image_part(entry, index, part, rows),
            kind => {
                rows.lost(entry, Some(part_hash(part)), kind, &no_item(kind));
                None
            }
        };
        content.extend(item);
    }
    content
}

fn user_items<'a>(entry: &str, parts: &'a [Value], rows: &mut Rows) -> Vec<Item<'a>> {
    let content = input_parts(entry, parts, rows);
    if content.is_empty() {
        return Vec::new();
    }
    vec![Item::message("user", content)]
}

fn assistant_items<'a>(entry: &str, parts: &'a [Value], rows: &mut Rows) -> Vec<Item<'a>> {
    let mut items = Vec::new();
    let mut said: Vec<Part<'a>> = Vec::new();
    let flush = |said: &mut Vec<Part<'a>>, items: &mut Vec<Item<'a>>| {
        if !said.is_empty() {
            items.push(Item::message("assistant", std::mem::take(said)));
        }
    };
    for part in parts {
        match kind_of(part) {
            "text" => said.extend(text_part(entry, part, "output_text", rows)),
            "thinking" => match thinking_of(part) {
                Thinking::Readable(text) => {
                    let lead = if part.get("thinkingSignature").is_some() {
                        "thinkingSignature not carried"
                    } else {
                        "thinking written as output_text"
                    };
                    let carried = ["type", "thinking", "thinkingSignature", "redacted"];
                    part_row(
                        rows,
                        entry,
                        part,
                        ("thinking", "output_text"),
                        Some(lead),
                        &carried,
                    );
                    said.push(Part::Text {
                        text,
                        kind: "output_text",
                    });
                }
                Thinking::Redacted => {
                    rows.lost(entry, Some(part_hash(part)), "thinking", REDACTED);
                }
                Thinking::Empty => {
                    rows.lost(entry, Some(part_hash(part)), "thinking", EMPTY_THINKING);
                }
            },
            "toolCall" => {
                let call = call_of(part).and_then(|(id, name)| {
                    part.get("arguments")
                        .map(|arguments| (id, name, arguments))
                        .ok_or(NO_ARGUMENTS)
                });
                match call {
                    Ok((call_id, name, arguments)) => {
                        let carried = ["type", "id", "name", "arguments"];
                        part_row(
                            rows,
                            entry,
                            part,
                            ("toolCall", "function_call"),
                            None,
                            &carried,
                        );
                        flush(&mut said, &mut items);
                        items.push(Item::FunctionCall {
                            arguments: Compact(arguments),
                            call_id,
                            name,
                            kind: "function_call",
                        });
                    }
                    Err(reason) => rows.lost(entry, Some(part_hash(part)), "toolCall", reason),
                }
            }
            kind => rows.lost(entry, Some(part_hash(part)), kind, &no_item(kind)),
        }
    }
    flush(&mut said, &mut items);
    items
}

/// A tool result as one `function_call_output`: its one text part's text as
/// a string, otherwise a list of `input_text` and `input_image` items.
fn tool_result<'a>(entry: &str, message: &'a Value, rows: &mut Rows) -> Option<Item<'a>> {
    let Some(call_id) = named(message, "toolCallId") else {
        rows.lost(entry, Some(part_hash(message)), "toolResult", NO_RESULT_ID);
        return None;
    };
    let mut how = Vec::new();
    let output = match message.get("content") {
        Some(Value::String(text)) => Output::Text(text),
        Some(Value::Array(parts)) => {
            let one_text = match parts.as_slice() {
                [part] if kind_of(part) == "text" => part
                    .get("text")
                    .and_then(Value::as_str)
                    .map(|text| (part, text)),
                _ => None,
            };
            if let Some((part, text)) = one_text {
                how.push("content: one-item text array written as a string".to_owned());
                how.extend(not_carried(part, &["type", "text"]));
                Output::Text(text)
            } else {
                how.push("content: written as a list of input items".to_owned());
                Output::List(input_parts(entry, parts, rows))
            }
        }
        _ => {
            rows.lost(entry, Some(part_hash(message)), "toolResult", NO_OUTPUT);
            return None;
        }
    };
    how.extend(not_carried(message, &["role", "toolCallId", "content"]));
    let (before, after) = ("toolResult", "function_call_output");
    if how.is_empty() {
        rows.kept(entry, Some(part_hash(message)), before, after);
    } else {
        rows.changed(
            entry,
            Some(part_hash(message)),
            before,
            after,
            &how.join("; "),
        );
    }
    Some(Item::FunctionCallOutput {
        call_id,
        output,
        kind: "function_call_output",
    })
}
