#![cfg(test)]
//! One fixture per refusal of the Claude Code render (ADR-055): each builds
//! a session `thin` holding a well-formed user entry `u1` and one entry `x1`
//! under it that carries every field but the one under test, renders it into
//! an empty directory, and asserts the refusal names `x1`, the field and the
//! expected type, and that the directory is still empty. Beside them, the
//! passing cases: a role the render does not know is skipped, a thinking
//! part with no `redacted` renders as not redacted, and the three mapped
//! stopReasons.

use std::error::Error;
use std::path::PathBuf;

use serde_json::{Map, Value, json};
use tempfile::TempDir;

use crate::error::HomeError;
use crate::harness::claude_code::render::{RenderReport, RenderTarget, render_claude_code};
use crate::harness::claude_code::{API, PROVIDER};
use crate::record::Home;
use crate::record::entries::{Entry, EntryBase, EntryBody};

type Gate = Result<(), Box<dyn Error>>;

const MODEL: &str = "claude-opus-5-5";

/// How a refusal of a missing provider or api ends.
const NEEDS: &str = "the render needs a record whose assistant messages carry provider and api";

/// What a render of a fixture session left: its result, the directory it
/// rendered into, the home and the session file's bytes before the render.
struct Run {
    result: Result<RenderReport, HomeError>,
    out: TempDir,
    home: TempDir,
    session_file: PathBuf,
    session_bytes: Vec<u8>,
}

impl Run {
    fn rendered(&self) -> PathBuf {
        self.out.path().join("r.jsonl")
    }
}

fn entry(id: &str, parent: &str, message: Value) -> Entry {
    Entry {
        base: EntryBase {
            id: id.to_owned(),
            parent_id: (!parent.is_empty()).then(|| parent.to_owned()),
            timestamp: "2026-01-01T00:00:00.000Z".to_owned(),
        },
        body: EntryBody::Message { message },
    }
}

/// Render session `thin`: the user entry `u1`, then each of `messages` under
/// the one before, into `r.jsonl` in an empty temporary directory.
fn render(messages: &[(&str, Value)]) -> Result<Run, Box<dyn Error>> {
    let home_dir = tempfile::tempdir()?;
    let out = tempfile::tempdir()?;
    let home = Home::open(home_dir.path().join("home"))?;
    let mut session = home.create_session("thin", "/w", None)?;
    let question = json!({"role": "user", "content": [text("fixture question")], "timestamp": 0});
    session.append_entry(&entry("u1", "", question))?;
    let mut parent = "u1";
    for &(id, ref message) in messages {
        session.append_entry(&entry(id, parent, message.clone()))?;
        parent = id;
    }
    let target = RenderTarget {
        session_id: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned(),
        cwd: "/w".to_owned(),
        model: MODEL.to_owned(),
        version: "2.1.281".to_owned(),
        out: Some(out.path().join("r.jsonl")),
        canon: None,
    };
    let session_file = session.file().to_path_buf();
    let session_bytes = std::fs::read(&session_file)?;
    let result = render_claude_code(&session, &target, None);
    Ok(Run {
        result,
        out,
        home: home_dir,
        session_file,
        session_bytes,
    })
}

/// Render `thin` with `x1` holding `message`, and return the refusal after
/// checking the output directory holds 0 entries.
fn refusal(message: Value) -> Result<HomeError, Box<dyn Error>> {
    let run = render(&[("x1", message)])?;
    assert_eq!(std::fs::read_dir(run.out.path())?.count(), 0);
    assert!(run.session_file.starts_with(run.home.path()));
    assert_eq!(std::fs::read(&run.session_file)?, run.session_bytes);
    match run.result {
        Err(error) => Ok(error),
        Ok(report) => Err(format!("rendered {} records", report.records).into()),
    }
}

/// Assert `error` is a field refusal of `x1` in `thin` naming `field`,
/// `expected` and `missing`.
fn assert_field(error: &HomeError, field: &str, expected: &str, missing: bool) {
    let named = match error {
        HomeError::RenderField {
            session,
            entry,
            field: named_field,
            expected: named_type,
            missing: named_missing,
        } => Some((
            session.as_str(),
            entry.as_str(),
            *named_field,
            *named_type,
            *named_missing,
        )),
        _ => None,
    };
    assert_eq!(
        named,
        Some(("thin", "x1", field, expected, missing)),
        "{error}"
    );
}

/// Assert `error` is a stopReason refusal of `x1` in `thin` naming `value`.
fn assert_stop(error: &HomeError, value: &str) {
    let named = match error {
        HomeError::RenderStopReason {
            session,
            entry,
            value: named_value,
        } => Some((session.as_str(), entry.as_str(), named_value.as_str())),
        _ => None,
    };
    assert_eq!(named, Some(("thin", "x1", value)), "{error}");
    assert!(error.to_string().contains("stopReason"), "{error}");
}

fn text(words: &str) -> Value {
    json!({"type": "text", "text": words})
}

fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// `message` without `field`.
fn without(message: Value, field: &str) -> Value {
    let mut map = object(message);
    map.remove(field);
    Value::Object(map)
}

/// `message` with `field` set to `value`.
fn with(message: Value, field: &str, value: Value) -> Value {
    let mut map = object(message);
    map.insert(field.to_owned(), value);
    Value::Object(map)
}

fn tool_result() -> Value {
    json!({"role": "toolResult", "toolCallId": "toolu_1", "toolName": "Bash",
        "content": [text("fixture output")], "isError": false, "timestamp": 0})
}

fn user() -> Value {
    json!({"role": "user", "content": [text("fixture follow-up")], "timestamp": 0})
}

/// An assistant message of the target's provider, api and model holding
/// `parts` and stopping with `stop`.
fn assistant(parts: &Value, stop: &str) -> Value {
    json!({"role": "assistant", "content": parts, "api": API, "provider": PROVIDER,
        "model": MODEL, "stopReason": stop, "timestamp": 0})
}

fn answer() -> Value {
    assistant(&json!([text("fixture answer")]), "stop")
}

fn tool_call() -> Value {
    json!({"type": "toolCall", "id": "toolu_1", "name": "Bash", "arguments": {"command": "ls"}})
}

fn thinking() -> Value {
    json!({"type": "thinking", "thinking": "fixture reasoning",
        "thinkingSignature": "fixture-signature"})
}

/// An assistant message holding the one part `part`.
fn holding(part: &Value) -> Value {
    assistant(&json!([part]), "stop")
}

#[test]
fn a_message_with_no_role_refuses_by_role() -> Gate {
    let error = refusal(without(user(), "role"))?;
    assert_field(&error, "role", "string", true);
    Ok(())
}

#[test]
fn a_message_whose_role_is_a_number_refuses_by_role() -> Gate {
    let error = refusal(with(user(), "role", json!(12345)))?;
    assert_field(&error, "role", "string", false);
    assert!(!error.to_string().contains("12345"), "{error}");
    Ok(())
}

#[test]
fn a_tool_result_with_no_tool_call_id_refuses() -> Gate {
    let error = refusal(without(tool_result(), "toolCallId"))?;
    assert_field(&error, "toolCallId", "string", true);
    Ok(())
}

#[test]
fn a_tool_result_with_no_content_refuses() -> Gate {
    let error = refusal(without(tool_result(), "content"))?;
    assert_field(&error, "content", "array or string", true);
    Ok(())
}

#[test]
fn a_tool_result_with_no_is_error_refuses() -> Gate {
    let error = refusal(without(tool_result(), "isError"))?;
    assert_field(&error, "isError", "boolean", true);
    Ok(())
}

#[test]
fn a_tool_result_whose_is_error_is_a_string_refuses() -> Gate {
    let error = refusal(with(tool_result(), "isError", json!("maybe")))?;
    assert_field(&error, "isError", "boolean", false);
    assert!(!error.to_string().contains("maybe"), "{error}");
    Ok(())
}

#[test]
fn a_user_message_with_no_content_refuses() -> Gate {
    let error = refusal(without(user(), "content"))?;
    assert_field(&error, "content", "array or string", true);
    Ok(())
}

#[test]
fn a_user_message_of_one_text_part_with_no_text_refuses() -> Gate {
    let error = refusal(with(user(), "content", json!([{"type": "text"}])))?;
    assert_field(&error, "text", "string", true);
    Ok(())
}

#[test]
fn an_assistant_message_with_no_content_refuses() -> Gate {
    let error = refusal(without(answer(), "content"))?;
    assert_field(&error, "content", "array", true);
    Ok(())
}

#[test]
fn an_assistant_message_whose_content_is_a_string_refuses() -> Gate {
    let error = refusal(with(answer(), "content", json!("fixture words")))?;
    assert_field(&error, "content", "array", false);
    assert!(!error.to_string().contains("fixture words"), "{error}");
    Ok(())
}

#[test]
fn an_assistant_message_with_no_model_refuses() -> Gate {
    let error = refusal(without(answer(), "model"))?;
    assert_field(&error, "model", "string", true);
    Ok(())
}

#[test]
fn an_assistant_message_with_no_provider_refuses_naming_the_record_it_needs() -> Gate {
    let error = refusal(without(answer(), "provider"))?;
    assert_field(&error, "provider", "string", true);
    let words = error.to_string();
    assert!(words.ends_with(NEEDS), "{words}");
    Ok(())
}

#[test]
fn an_assistant_message_with_no_api_refuses_naming_the_record_it_needs() -> Gate {
    let error = refusal(without(answer(), "api"))?;
    assert_field(&error, "api", "string", true);
    let words = error.to_string();
    assert!(words.ends_with(NEEDS), "{words}");
    Ok(())
}

#[test]
fn an_assistant_message_with_no_stop_reason_refuses() -> Gate {
    let error = refusal(without(answer(), "stopReason"))?;
    assert_field(&error, "stopReason", "string", true);
    Ok(())
}

#[test]
fn a_stop_reason_of_error_refuses_by_the_value() -> Gate {
    let error = refusal(with(answer(), "stopReason", json!("error")))?;
    assert_stop(&error, "error");
    Ok(())
}

#[test]
fn a_stop_reason_of_aborted_refuses_by_the_value() -> Gate {
    let error = refusal(with(answer(), "stopReason", json!("aborted")))?;
    assert_stop(&error, "aborted");
    Ok(())
}

#[test]
fn an_unmapped_stop_reason_refuses_by_the_value() -> Gate {
    let error = refusal(with(answer(), "stopReason", json!("halted")))?;
    assert_stop(&error, "halted");
    Ok(())
}

#[test]
fn an_assistant_text_part_with_no_text_refuses() -> Gate {
    let error = refusal(holding(&json!({"type": "text"})))?;
    assert_field(&error, "text", "string", true);
    Ok(())
}

#[test]
fn a_tool_call_part_with_no_id_refuses() -> Gate {
    let error = refusal(holding(&without(tool_call(), "id")))?;
    assert_field(&error, "id", "string", true);
    Ok(())
}

#[test]
fn a_tool_call_part_with_no_name_refuses() -> Gate {
    let error = refusal(holding(&without(tool_call(), "name")))?;
    assert_field(&error, "name", "string", true);
    Ok(())
}

#[test]
fn a_tool_call_part_with_no_arguments_refuses() -> Gate {
    let error = refusal(holding(&without(tool_call(), "arguments")))?;
    assert_field(&error, "arguments", "object", true);
    Ok(())
}

#[test]
fn a_thinking_part_with_no_thinking_refuses() -> Gate {
    let error = refusal(holding(&without(thinking(), "thinking")))?;
    assert_field(&error, "thinking", "string", true);
    Ok(())
}

#[test]
fn a_redacted_thinking_part_with_no_signature_refuses() -> Gate {
    let unsigned = without(thinking(), "thinkingSignature");
    let part = with(unsigned, "redacted", json!(true));
    let error = refusal(holding(&part))?;
    assert_field(&error, "thinkingSignature", "string", true);
    Ok(())
}

#[test]
fn a_thinking_part_whose_signature_is_null_refuses() -> Gate {
    let part = with(thinking(), "thinkingSignature", Value::Null);
    let error = refusal(holding(&part))?;
    assert_field(&error, "thinkingSignature", "string", false);
    Ok(())
}

#[test]
fn a_thinking_part_whose_redacted_is_a_string_refuses() -> Gate {
    let error = refusal(holding(&with(thinking(), "redacted", json!("yes"))))?;
    assert_field(&error, "redacted", "boolean", false);
    Ok(())
}

/// The records of a render that succeeded.
fn records(run: &Run) -> Result<Vec<Value>, Box<dyn Error>> {
    if let Err(error) = &run.result {
        return Err(format!("the render was refused: {error}").into());
    }
    let text = std::fs::read_to_string(run.rendered())?;
    let mut records = Vec::new();
    for line in text.lines() {
        records.push(serde_json::from_str(line)?);
    }
    Ok(records)
}

#[test]
fn a_role_the_render_does_not_know_is_skipped_and_the_render_succeeds() -> Gate {
    let run = render(&[("x1", with(user(), "role", json!("narrator")))])?;
    let report = run.result.as_ref().map_err(ToString::to_string)?;
    assert_eq!(report.records, 1);
    assert_eq!(records(&run)?.len(), 1);
    Ok(())
}

#[test]
fn a_thinking_part_with_no_redacted_field_renders_whole_and_not_redacted() -> Gate {
    let run = render(&[("x1", holding(&thinking()))])?;
    let report = run.result.as_ref().map_err(ToString::to_string)?;
    assert_eq!(report.thinking_kept, 1);
    let records = records(&run)?;
    assert_eq!(records.len(), 2);
    let part = &records[1]["message"]["content"][0];
    assert_eq!(part["type"], "thinking");
    assert_eq!(part["signature"], "fixture-signature");
    let file = std::fs::read_to_string(run.rendered())?;
    assert!(!file.contains("redacted_thinking"));
    Ok(())
}

#[test]
fn the_stop_reasons_stop_tool_use_and_length_map_to_their_claude_code_values() -> Gate {
    let run = render(&[
        ("x1", answer()),
        ("x2", assistant(&json!([tool_call()]), "toolUse")),
        ("x3", assistant(&json!([text("fixture cut")]), "length")),
    ])?;
    let stops: Vec<Value> = records(&run)?
        .iter()
        .skip(1)
        .map(|record| record["message"]["stop_reason"].clone())
        .collect();
    let expected = [json!("end_turn"), json!("tool_use"), json!("max_tokens")];
    assert_eq!(stops, expected);
    Ok(())
}
