//! The stream readers' coordinator: picks the grammar by api and hands each
//! event on as the bytes pass.
//!
//! The reader is fed the same bytes the client is sent, after they are sent
//! on; it never holds the stream back and never changes it. What it keeps is
//! the assembly of the response's parts, which the sink records for a
//! complete call instead of re-reading an event stream it cannot parse as
//! JSON (R6).
//!
//! Invariants: [`StreamReader::finish`] gives parts only when the framing
//! ended on an event boundary with every line UTF-8 and the api's grammar
//! says the stream ended whole; anything less is a partial stream.

use serde_json::Value;

use crate::proxy::stream_chat::ChatAssembler;
use crate::proxy::stream_messages::MessagesAssembler;
use crate::proxy::stream_responses::ResponsesAssembler;
use crate::proxy::stream_sse::{SseEvent, SseFramer};
use crate::record::call::Api;
use crate::record::call::captured::Tokens;

/// One api's grammar.
#[derive(Debug)]
enum Grammar {
    Messages(MessagesAssembler),
    Chat(ChatAssembler),
    Responses(ResponsesAssembler),
}

/// An event stream being read as it passes.
#[derive(Debug)]
pub struct StreamReader {
    framer: SseFramer,
    grammar: Grammar,
    events: u64,
    scratch: Vec<SseEvent>,
}

impl StreamReader {
    /// A reader for a stream of this api.
    #[must_use]
    pub fn for_api(api: Api) -> Self {
        let grammar = match api {
            Api::Messages => Grammar::Messages(MessagesAssembler::new()),
            Api::ChatCompletions => Grammar::Chat(ChatAssembler::new()),
            Api::Responses => Grammar::Responses(ResponsesAssembler::new()),
        };
        Self {
            framer: SseFramer::new(),
            grammar,
            events: 0,
            scratch: Vec::new(),
        }
    }

    /// Read the next bytes of the stream.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.framer.feed(bytes, &mut self.scratch);
        for event in self.scratch.drain(..) {
            self.events += 1;
            match &mut self.grammar {
                Grammar::Messages(g) => g.event(&event),
                Grammar::Chat(g) => g.event(&event),
                Grammar::Responses(g) => g.event(&event),
            }
        }
    }

    /// How many events have been read.
    #[must_use]
    pub const fn events(&self) -> u64 {
        self.events
    }

    /// The provider's id for the message, once a Messages stream's
    /// `message_start` was read. The other two grammars name none here.
    #[must_use]
    pub fn message_id(&self) -> Option<&str> {
        match &self.grammar {
            Grammar::Messages(g) => g.message_id(),
            Grammar::Chat(_) | Grammar::Responses(_) => None,
        }
    }

    /// The token figures the stream has reported so far: a Messages stream's
    /// `message_start` and `message_delta`, a Responses stream's
    /// `response.completed`. A Chat Completions stream's are not read here.
    #[must_use]
    pub fn tokens(&self) -> Option<Tokens> {
        match &self.grammar {
            Grammar::Messages(g) => g.tokens(),
            Grammar::Responses(g) => g.tokens(),
            Grammar::Chat(_) => None,
        }
    }

    /// The response's parts, only when the stream ended whole.
    #[must_use]
    pub fn finish(self) -> Option<Vec<Value>> {
        if self.framer.malformed() || self.framer.pending() {
            return None;
        }
        match self.grammar {
            Grammar::Messages(g) => g.finish(),
            Grammar::Chat(g) => g.finish(),
            Grammar::Responses(g) => g.finish(),
        }
    }
}
