//! The Chat Completions chunk grammar, assembled into response parts.
//!
//! A Chat Completions stream is a series of `chat.completion.chunk` objects,
//! one per event, each with `choices[]` whose `delta` carries a piece of one
//! choice's message (`role`, `content`, `refusal`, and `tool_calls[]` pieces
//! keyed by their own `index`), ended by the literal event `[DONE]`. Each
//! choice is assembled into the message a complete JSON response holds under
//! `choices[].message`, in choice order: only the members the stream sent are
//! written, and a tool call's `arguments` are its fragments joined.
//!
//! Invariants: the parts are given only for a stream that reached `[DONE]`
//! with at least one choice and no chunk that failed to parse or carried an
//! `error`. Anything else is not a complete stream and gives no parts.

use serde_json::{Map, Value};

use crate::proxy::stream_sse::SseEvent;

/// A Chat Completions stream being assembled.
#[derive(Debug, Default)]
pub struct ChatAssembler {
    choices: Vec<Option<Choice>>,
    done: bool,
    malformed: bool,
}

#[derive(Debug, Default)]
struct Choice {
    role: Option<String>,
    content: Option<String>,
    refusal: Option<String>,
    tool_calls: Vec<Option<ToolCall>>,
}

#[derive(Debug, Default)]
struct ToolCall {
    id: Option<String>,
    kind: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
}

impl ChatAssembler {
    /// An assembler at the start of a stream.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read one event.
    pub fn event(&mut self, event: &SseEvent) {
        if event.data == "[DONE]" {
            self.done = true;
            return;
        }
        let Ok(chunk) = serde_json::from_str::<Value>(&event.data) else {
            self.malformed = true;
            return;
        };
        if chunk.get("error").is_some() {
            self.malformed = true;
            return;
        }
        let Some(choices) = chunk.get("choices").and_then(Value::as_array) else {
            self.malformed = true;
            return;
        };
        for choice in choices {
            self.choice(choice);
        }
    }

    fn choice(&mut self, choice: &Value) {
        let Some(index) = index_of(choice) else {
            self.malformed = true;
            return;
        };
        if self.choices.len() <= index {
            self.choices.resize_with(index + 1, || None);
        }
        let Some(delta) = choice.get("delta") else {
            return;
        };
        let assembled = self.choices[index].get_or_insert_with(Choice::default);
        if let Some(role) = delta.get("role").and_then(Value::as_str) {
            assembled.role = Some(role.to_owned());
        }
        join(&mut assembled.content, delta.get("content"));
        join(&mut assembled.refusal, delta.get("refusal"));
        let calls = delta
            .get("tool_calls")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        for call in calls {
            let Some(at) = index_of(call) else {
                self.malformed = true;
                return;
            };
            if assembled.tool_calls.len() <= at {
                assembled.tool_calls.resize_with(at + 1, || None);
            }
            let piece = assembled.tool_calls[at].get_or_insert_with(ToolCall::default);
            if let Some(id) = call.get("id").and_then(Value::as_str) {
                piece.id = Some(id.to_owned());
            }
            if let Some(kind) = call.get("type").and_then(Value::as_str) {
                piece.kind = Some(kind.to_owned());
            }
            let function = call.get("function");
            join(&mut piece.name, function.and_then(|f| f.get("name")));
            join(&mut piece.arguments, function.and_then(|f| f.get("arguments")));
        }
    }

    /// The assembled messages, one per choice in choice order, only when the
    /// stream ended whole.
    #[must_use]
    pub fn finish(self) -> Option<Vec<Value>> {
        if !self.done || self.malformed || self.choices.is_empty() {
            return None;
        }
        self.choices
            .into_iter()
            .map(|choice| choice.map(message))
            .collect()
    }
}

fn message(choice: Choice) -> Value {
    let mut out = Map::new();
    if let Some(role) = choice.role {
        out.insert("role".to_owned(), Value::String(role));
    }
    if let Some(content) = choice.content {
        out.insert("content".to_owned(), Value::String(content));
    }
    if let Some(refusal) = choice.refusal {
        out.insert("refusal".to_owned(), Value::String(refusal));
    }
    let calls: Vec<Value> = choice
        .tool_calls
        .into_iter()
        .flatten()
        .map(|call| {
            let mut out = Map::new();
            if let Some(id) = call.id {
                out.insert("id".to_owned(), Value::String(id));
            }
            if let Some(kind) = call.kind {
                out.insert("type".to_owned(), Value::String(kind));
            }
            let mut function = Map::new();
            if let Some(name) = call.name {
                function.insert("name".to_owned(), Value::String(name));
            }
            if let Some(arguments) = call.arguments {
                function.insert("arguments".to_owned(), Value::String(arguments));
            }
            out.insert("function".to_owned(), Value::Object(function));
            Value::Object(out)
        })
        .collect();
    if !calls.is_empty() {
        out.insert("tool_calls".to_owned(), Value::Array(calls));
    }
    Value::Object(out)
}

fn index_of(value: &Value) -> Option<usize> {
    value
        .get("index")
        .and_then(Value::as_u64)
        .and_then(|i| usize::try_from(i).ok())
}

fn join(target: &mut Option<String>, piece: Option<&Value>) {
    if let Some(text) = piece.and_then(Value::as_str) {
        target.get_or_insert_with(String::new).push_str(text);
    }
}
