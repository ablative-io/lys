//! The Responses event grammar, assembled into response parts.
//!
//! A Responses stream is typed events (`response.created`,
//! `response.output_item.added`, the text and argument deltas,
//! `response.output_item.done`, ...) ending in `response.completed`, whose
//! `response` is the whole response object a JSON call would have returned.
//! Its `output` array is the parts, as a complete JSON response's `output`
//! is; each finished item also arrives on its own in
//! `response.output_item.done`, and those items, by `output_index`, are the
//! parts when the completed event's `output` is absent.
//!
//! Invariants: the parts are given only for a stream that reached
//! `response.completed` with status `completed`, with no `error`,
//! `response.failed` or `response.incomplete` event and no event that failed
//! to parse. Anything else is not a complete stream and gives no parts.

use serde_json::Value;

use crate::proxy::stream_sse::SseEvent;
use crate::record::call::Api;
use crate::record::call::captured::Tokens;

/// A Responses stream being assembled.
#[derive(Debug, Default)]
pub struct ResponsesAssembler {
    items: Vec<Option<Value>>,
    completed: Option<Value>,
    malformed: bool,
}

impl ResponsesAssembler {
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
        let kind = data
            .get("type")
            .and_then(Value::as_str)
            .or(event.event.as_deref());
        match kind {
            Some("response.output_item.done") => self.item(&data),
            Some("response.completed") => {
                self.completed = data.get("response").cloned();
                if self.completed.is_none() {
                    self.malformed = true;
                }
            }
            Some("error" | "response.failed" | "response.incomplete") | None => {
                self.malformed = true;
            }
            Some(_) => {}
        }
    }

    fn item(&mut self, data: &Value) {
        let index = data
            .get("output_index")
            .and_then(Value::as_u64)
            .and_then(|i| usize::try_from(i).ok());
        let (Some(index), Some(item)) = (index, data.get("item")) else {
            self.malformed = true;
            return;
        };
        if self.items.len() <= index {
            self.items.resize_with(index + 1, || None);
        }
        self.items[index] = Some(item.clone());
    }

    /// The token figures of the completed response's `usage`: `input_tokens`
    /// and `output_tokens`, the cached part of the input under
    /// `input_tokens_details.cached_tokens` and the reasoning part of the
    /// output under `output_tokens_details.reasoning_tokens`; each of the
    /// two is also read where a usage names it beside the others, as
    /// `cached_input_tokens` and `reasoning_output_tokens`. None until
    /// `response.completed` was read, and when it carried no figure.
    #[must_use]
    pub fn tokens(&self) -> Option<Tokens> {
        let usage = self.completed.as_ref()?.get("usage")?;
        let mut tokens = Tokens::default();
        tokens.read_usage(Api::Responses, usage);
        tokens.reported()
    }

    /// The response's output items, only when the stream ended whole.
    #[must_use]
    pub fn finish(self) -> Option<Vec<Value>> {
        if self.malformed {
            return None;
        }
        let response = self.completed?;
        if response.get("status").and_then(Value::as_str) != Some("completed") {
            return None;
        }
        match response.get("output") {
            Some(Value::Array(output)) => Some(output.clone()),
            Some(_) => None,
            None => self.items.into_iter().collect(),
        }
    }
}
