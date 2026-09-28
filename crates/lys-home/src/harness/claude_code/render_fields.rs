//! The Claude Code render's checked readers (ADR-055).
//!
//! Every value the render copies from a message entry or one of its parts is
//! read here. A reader returns the field unchanged when it is present and of
//! the type the target takes; an absent field refuses as
//! [`HomeError::RenderField`] with `missing` true, and a present field of
//! another type, a JSON null included, refuses with `missing` false. Either
//! refusal names the session, the entry id, the field and the expected type,
//! never the value read. No reader substitutes a default, with one exception
//! Pi's grammar sets: a thinking part's `redacted` is optional, and its
//! absence reads as not redacted. The stopReason reader maps `toolUse`,
//! `length` and `stop` and refuses every other value by name as
//! [`HomeError::RenderStopReason`]: Claude Code's own files carry no value
//! for an errored or an aborted turn, so none is invented for them.

use serde_json::{Map, Value};

use crate::error::HomeError;

/// A [`HomeError::RenderField`] refusal of `field` in `entry`.
pub(crate) fn field_refusal(
    session: &str,
    entry: &str,
    field: &'static str,
    expected: &'static str,
    missing: bool,
) -> HomeError {
    HomeError::RenderField {
        session: session.to_owned(),
        entry: entry.to_owned(),
        field,
        expected,
        missing,
    }
}

/// `field` of `source` when `pick` takes it, refused by name otherwise.
fn read<'v, T>(
    session: &str,
    entry: &str,
    source: &'v Value,
    field: &'static str,
    expected: &'static str,
    pick: impl FnOnce(&'v Value) -> Option<T>,
) -> Result<T, HomeError> {
    let Some(value) = source.get(field) else {
        return Err(field_refusal(session, entry, field, expected, true));
    };
    match pick(value) {
        Some(read) => Ok(read),
        None => Err(field_refusal(session, entry, field, expected, false)),
    }
}

/// `field` of `source` as a string.
pub(crate) fn string<'v>(
    session: &str,
    entry: &str,
    source: &'v Value,
    field: &'static str,
) -> Result<&'v str, HomeError> {
    read(session, entry, source, field, "string", Value::as_str)
}

/// `field` of `source` as an array.
pub(crate) fn array<'v>(
    session: &str,
    entry: &str,
    source: &'v Value,
    field: &'static str,
) -> Result<&'v Vec<Value>, HomeError> {
    read(session, entry, source, field, "array", Value::as_array)
}

/// `field` of `source` as a boolean.
pub(crate) fn boolean(
    session: &str,
    entry: &str,
    source: &Value,
    field: &'static str,
) -> Result<bool, HomeError> {
    read(session, entry, source, field, "boolean", Value::as_bool)
}

/// `field` of `source` as an object.
pub(crate) fn object<'v>(
    session: &str,
    entry: &str,
    source: &'v Value,
    field: &'static str,
) -> Result<&'v Map<String, Value>, HomeError> {
    read(session, entry, source, field, "object", Value::as_object)
}

/// `field` of `source` when it is an array or a string, as it stands.
pub(crate) fn array_or_string<'v>(
    session: &str,
    entry: &str,
    source: &'v Value,
    field: &'static str,
) -> Result<&'v Value, HomeError> {
    read(session, entry, source, field, "array or string", |value| {
        (value.is_array() || value.is_string()).then_some(value)
    })
}

/// A thinking part's `redacted`: absent reads as not redacted, the one
/// default the render takes; present, it must be a boolean.
pub(crate) fn redacted(session: &str, entry: &str, part: &Value) -> Result<bool, HomeError> {
    match part.get("redacted") {
        None => Ok(false),
        Some(Value::Bool(redacted)) => Ok(*redacted),
        Some(_) => Err(field_refusal(session, entry, "redacted", "boolean", false)),
    }
}

/// An assistant message's `stopReason` as Claude Code's `stop_reason`.
pub(crate) fn stop_reason(
    session: &str,
    entry: &str,
    message: &Value,
) -> Result<&'static str, HomeError> {
    let refused = |value: &str| HomeError::RenderStopReason {
        session: session.to_owned(),
        entry: entry.to_owned(),
        value: value.to_owned(),
    };
    let value = string(session, entry, message, "stopReason")?;
    match value {
        "toolUse" => Ok("tool_use"),
        "length" => Ok("max_tokens"),
        "stop" => Ok("end_turn"),
        "error" => Err(refused("error")),
        "aborted" => Err(refused("aborted")),
        _ => Err(refused(value)),
    }
}
