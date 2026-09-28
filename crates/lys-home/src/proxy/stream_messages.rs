//! The Messages event grammar, assembled into response parts.
//!
//! A Messages stream is `message_start`, then for each content block a
//! `content_block_start`, its `content_block_delta`s and a
//! `content_block_stop`, then `message_delta` and `message_stop`; `ping`
//! may come anywhere. Each block is assembled from its start and its deltas
//! into the object a complete JSON response holds in its `content` array:
//! `text_delta` appends to `text`, `thinking_delta` to `thinking`,
//! `signature_delta` to `signature`, `citations_delta` pushes onto
//! `citations`, and the `input_json_delta` fragments, joined, are parsed as a
//! tool call's `input`.
//!
//! Invariants: the parts are given only for a stream that reached
//! `message_stop` with every started block stopped, in index order, with no
//! `error` event, no delta of a kind this grammar does not know and no input
//! that fails to parse. Anything else is not a complete stream and gives no
//! parts, so a call is never recorded complete from a stream that was not.

use serde_json::{Map, Value};

use crate::proxy::stream_sse::SseEvent;

/// A Messages stream being assembled.
#[derive(Debug, Default)]
pub struct MessagesAssembler {
    blocks: Vec<Block>,
    stopped: bool,
    malformed: bool,
}

#[derive(Debug)]
struct Block {
    value: Map<String, Value>,
    input_json: String,
    stopped: bool,
}

impl MessagesAssembler {
    /// An assembler at the start of a stream.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Read one event.
    pub fn event(&mut self, event: &SseEvent) {
        let Ok(data) = serde_json::from_str::<Value>(&event.data) else {
            self.malformed = true;
            return;
        };
        match data.get("type").and_then(Value::as_str) {
            Some("message_start" | "message_delta" | "ping") => {}
            Some("message_stop") => self.stopped = true,
            Some("content_block_start") => self.start(&data),
            Some("content_block_delta") => self.delta(&data),
            Some("content_block_stop") => self.stop(&data),
            _ => self.malformed = true,
        }
    }

    fn start(&mut self, data: &Value) {
        let block = data.get("content_block").and_then(Value::as_object);
        match (index_of(data), block) {
            (Some(index), Some(block)) if index == self.blocks.len() => self.blocks.push(Block {
                value: block.clone(),
                input_json: String::new(),
                stopped: false,
            }),
            _ => self.malformed = true,
        }
    }

    fn delta(&mut self, data: &Value) {
        let delta = data.get("delta");
        let block = index_of(data).and_then(|i| self.blocks.get_mut(i));
        let (Some(block), Some(delta)) = (block, delta) else {
            self.malformed = true;
            return;
        };
        let text = |key: &str| delta.get(key).and_then(Value::as_str);
        let applied = match delta.get("type").and_then(Value::as_str) {
            Some("text_delta") => text("text").map(|t| append(&mut block.value, "text", t)),
            Some("thinking_delta") => {
                text("thinking").map(|t| append(&mut block.value, "thinking", t))
            }
            Some("signature_delta") => {
                text("signature").map(|t| append(&mut block.value, "signature", t))
            }
            Some("input_json_delta") => text("partial_json").map(|t| block.input_json.push_str(t)),
            Some("citations_delta") => delta.get("citation").map(|c| {
                if let Some(Value::Array(list)) = block.value.get_mut("citations") {
                    list.push(c.clone());
                } else {
                    block
                        .value
                        .insert("citations".to_owned(), Value::Array(vec![c.clone()]));
                }
            }),
            _ => None,
        };
        if applied.is_none() {
            self.malformed = true;
        }
    }

    fn stop(&mut self, data: &Value) {
        let Some(block) = index_of(data).and_then(|i| self.blocks.get_mut(i)) else {
            self.malformed = true;
            return;
        };
        block.stopped = true;
        if block.input_json.is_empty() {
            return;
        }
        match serde_json::from_str::<Value>(&block.input_json) {
            Ok(input) => {
                block.value.insert("input".to_owned(), input);
            }
            Err(_) => self.malformed = true,
        }
    }

    /// The assembled parts, only when the stream ended whole.
    #[must_use]
    pub fn finish(self) -> Option<Vec<Value>> {
        let whole = self.stopped && !self.malformed && self.blocks.iter().all(|b| b.stopped);
        whole.then(|| {
            self.blocks
                .into_iter()
                .map(|b| Value::Object(b.value))
                .collect()
        })
    }
}

fn index_of(data: &Value) -> Option<usize> {
    data.get("index")
        .and_then(Value::as_u64)
        .and_then(|i| usize::try_from(i).ok())
}

fn append(value: &mut Map<String, Value>, key: &str, text: &str) {
    if let Some(Value::String(existing)) = value.get_mut(key) {
        existing.push_str(text);
    } else {
        value.insert(key.to_owned(), Value::String(text.to_owned()));
    }
}
