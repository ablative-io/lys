//! The members of each api's `usage` object and the figure each names.
//!
//! One table for every reader: an event stream's reader takes a usage object
//! as it passes, and a whole-body response's reader takes the one at the top
//! of its JSON. Both fill the same [`Tokens`].
//!
//! - Messages names `input_tokens`, `output_tokens`,
//!   `cache_creation_input_tokens` and `cache_read_input_tokens`.
//! - Responses names `input_tokens` and `output_tokens`, the cached part of
//!   the input under `input_tokens_details.cached_tokens` and the reasoning
//!   part of the output under `output_tokens_details.reasoning_tokens`; each
//!   of the two is also read where a usage names it beside the others, as
//!   `cached_input_tokens` and `reasoning_output_tokens`.
//! - Chat Completions names `prompt_tokens` and `completion_tokens`, with
//!   `prompt_tokens_details.cached_tokens` and
//!   `completion_tokens_details.reasoning_tokens`.

use serde_json::Value;

use super::Api;
use super::captured::{TokenFigure, Tokens};

const MESSAGES: [(&str, TokenFigure); 4] = [
    ("input_tokens", TokenFigure::Input),
    ("output_tokens", TokenFigure::Output),
    ("cache_creation_input_tokens", TokenFigure::CacheCreation),
    ("cache_read_input_tokens", TokenFigure::CacheRead),
];
const RESPONSES: [(&str, TokenFigure); 4] = [
    ("input_tokens", TokenFigure::Input),
    ("output_tokens", TokenFigure::Output),
    ("cached_input_tokens", TokenFigure::CacheRead),
    ("reasoning_output_tokens", TokenFigure::Reasoning),
];
const CHAT: [(&str, TokenFigure); 2] = [
    ("prompt_tokens", TokenFigure::Input),
    ("completion_tokens", TokenFigure::Output),
];
/// The members of a details object: the cached part of an input, the
/// reasoning part of an output.
const CACHED: [(&str, TokenFigure); 1] = [("cached_tokens", TokenFigure::CacheRead)];
const REASONING: [(&str, TokenFigure); 1] = [("reasoning_tokens", TokenFigure::Reasoning)];

impl Tokens {
    /// Take each figure `usage` names by `api`'s own member names, leaving a
    /// figure it does not name as it was.
    pub fn read_usage(&mut self, api: Api, usage: &Value) {
        // The members beside the others, and where the two details objects
        // are when the api has them.
        let (names, details): (&[(&str, TokenFigure)], Option<(&str, &str)>) = match api {
            Api::Messages => (&MESSAGES, None),
            Api::Responses => (
                &RESPONSES,
                Some(("input_tokens_details", "output_tokens_details")),
            ),
            Api::ChatCompletions => (
                &CHAT,
                Some(("prompt_tokens_details", "completion_tokens_details")),
            ),
        };
        self.read(usage, names);
        let Some((input, output)) = details else {
            return;
        };
        if let Some(details) = usage.get(input) {
            self.read(details, &CACHED);
        }
        if let Some(details) = usage.get(output) {
            self.read(details, &REASONING);
        }
    }

    /// The figures the `usage` object at the top of a whole-body response
    /// names, or none when it has no such object or the object names no
    /// figure.
    #[must_use]
    pub fn of_whole_body(api: Api, response: &Value) -> Option<Self> {
        let mut tokens = Self::default();
        tokens.read_usage(api, response.get("usage")?);
        tokens.reported()
    }
}
