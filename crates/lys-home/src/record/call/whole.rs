//! One recorded call read back whole: its record and both bodies as JSON.
//!
//! A reader names the session and the entry, so the call is one seek into
//! the session file and one block read for each body; nothing is walked. A
//! body is given decoded from the content coding its side of the head names;
//! an event stream is given as its events in order, each `data` as JSON when
//! it is JSON. A body that cannot be shown says why and is never guessed at.

use serde::Serialize;
use serde_json::Value;

use super::captured::Side;
use super::coding::Coding;
use super::{CallRecord, call_record, named_hash};
use crate::error::HomeError;
use crate::proxy::stream_sse::{SseEvent, SseFramer};
use crate::record::Home;
use crate::record::blocks::BlockStore;
use crate::record::entries::{CUSTOM_CALL, EntryBody};

/// One body of a call as a reader is shown it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Body {
    /// The body as JSON; an event stream as its events in order.
    pub json: Option<Value>,
    /// Why there is no JSON to show, when there is none.
    pub unreadable: Option<String>,
}

/// A recorded call and both its bodies.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallWhole {
    /// The call's record as it is kept.
    pub call: CallRecord,
    /// What was sent.
    pub request: Body,
    /// What came back.
    pub response: Body,
}

/// The call kept as `entry` of `session` in `home`.
pub fn call_whole(home: &Home, session: &str, entry: &str) -> Result<CallWhole, HomeError> {
    let kept = home.read_session(session)?.entry(entry)?;
    let call = match &kept.body {
        EntryBody::Custom {
            custom_type,
            data: Some(data),
        } if custom_type == CUSTOM_CALL => call_record(data)?,
        // Any other entry is no call record, and is refused as one.
        _ => call_record(&Value::Null)?,
    };
    let blocks = home.blocks()?;
    let (sent, came) = call.head.as_ref().map_or((None, None), |head| {
        (Some(&head.request), Some(&head.response))
    });
    let request = body(&blocks, call.raw_request.as_deref(), sent, false)?;
    let response = body(&blocks, call.raw_response.as_deref(), came, call.stream)?;
    Ok(CallWhole {
        call,
        request,
        response,
    })
}

fn unreadable(reason: String) -> Body {
    Body {
        json: None,
        unreadable: Some(reason),
    }
}

fn body(
    blocks: &BlockStore,
    hash: Option<&str>,
    side: Option<&Side>,
    stream: bool,
) -> Result<Body, HomeError> {
    let Some(hash) = hash else {
        return Ok(unreadable("no body was recorded".to_owned()));
    };
    let stored = blocks.get(&named_hash(hash)?)?;
    let named = side.and_then(|side| side.values.get("content-encoding"));
    let bytes = match (side.and_then(Coding::of_side), named) {
        (Some(coding), _) => match coding.decode(&stored) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Ok(unreadable(format!(
                    "the body's content-encoding is {}, and its stored bytes do not decode as that: {error}",
                    coding.name()
                )));
            }
        },
        (None, Some(named)) => {
            return Ok(unreadable(format!(
                "the body's content-encoding is {}, which is not decoded",
                named.join(", ")
            )));
        }
        (None, None) => stored,
    };
    if stream {
        return Ok(events(&bytes));
    }
    Ok(match serde_json::from_slice(&bytes) {
        Ok(json) => Body {
            json: Some(json),
            unreadable: None,
        },
        Err(error) => unreadable(format!("the body is not JSON: {error}")),
    })
}

/// An event stream as its events in order; what the bytes stop inside is
/// said, and the events before it are still shown.
fn events(bytes: &[u8]) -> Body {
    let mut framer = SseFramer::new();
    let mut read = Vec::new();
    framer.feed(bytes, &mut read);
    let json = read
        .into_iter()
        .map(|SseEvent { event, data }| {
            let data = serde_json::from_str(&data).unwrap_or(Value::String(data));
            serde_json::json!({ "event": event, "data": data })
        })
        .collect();
    Body {
        json: Some(Value::Array(json)),
        unreadable: framer
            .pending()
            .then(|| "the stream stops inside an event; the events before it are shown".to_owned()),
    }
}

#[cfg(test)]
#[path = "whole_tests.rs"]
mod tests;
