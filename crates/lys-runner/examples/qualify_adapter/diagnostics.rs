//! Retain only closed protocol facts for a current compaction failure.

use lys_runner::harness_control::Update;
use lys_runner::operations::OperationState;
use serde::Serialize;
use serde_json::Value;

use super::Result;

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Compacting,
    Requesting,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum CompactResult {
    Success,
    Failed,
}

#[derive(Serialize)]
struct StatusFrame {
    status: Option<Status>,
    compact_result: Option<CompactResult>,
}

#[derive(Serialize)]
pub(super) struct Compaction {
    #[serde(skip)]
    conversation: String,
    #[serde(skip)]
    uuid: String,
    compact_boundary_seen: bool,
    matching_user_replay_seen: bool,
    replay_before_boundary: bool,
    replay_after_boundary: bool,
    result_seen: bool,
    result_is_error: Option<bool>,
    statuses: Vec<StatusFrame>,
}

impl Compaction {
    pub(super) fn new(conversation: &str, uuid: &str) -> Self {
        Self {
            conversation: conversation.to_owned(),
            uuid: uuid.to_owned(),
            compact_boundary_seen: false,
            matching_user_replay_seen: false,
            replay_before_boundary: false,
            replay_after_boundary: false,
            result_seen: false,
            result_is_error: None,
            statuses: Vec::new(),
        }
    }

    pub(super) fn observe(&mut self, frame: &Value) -> Result<()> {
        if frame.get("session_id").and_then(Value::as_str) != Some(self.conversation.as_str()) {
            return Ok(());
        }
        match (
            frame.get("type").and_then(Value::as_str),
            frame.get("subtype").and_then(Value::as_str),
        ) {
            (Some("system"), Some("compact_boundary")) => self.compact_boundary_seen = true,
            (Some("user"), _)
                if frame.get("uuid").and_then(Value::as_str) == Some(self.uuid.as_str())
                    && frame.pointer("/message/role").and_then(Value::as_str) == Some("user")
                    && frame.get("parent_tool_use_id") == Some(&Value::Null) =>
            {
                self.matching_user_replay_seen = true;
                if self.compact_boundary_seen {
                    self.replay_after_boundary = true;
                } else {
                    self.replay_before_boundary = true;
                }
            }
            (Some("result"), _) => {
                self.result_seen = true;
                let failed = frame
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .ok_or("qualification_compaction_result_invalid: is_error must be a boolean")?;
                self.result_is_error = Some(failed);
            }
            (Some("system"), Some("status")) => {
                let status = match frame.get("status") {
                    Some(Value::Null) => None,
                    Some(Value::String(status)) if status == "compacting" => {
                        Some(Status::Compacting)
                    }
                    Some(Value::String(status)) if status == "requesting" => {
                        Some(Status::Requesting)
                    }
                    _ => return Err(
                        "qualification_compaction_status_invalid: status is not a closed SDK value"
                            .to_owned(),
                    ),
                };
                let compact_result = match frame.get("compact_result") {
                    None => None,
                    Some(Value::String(result)) if result == "success" => Some(CompactResult::Success),
                    Some(Value::String(result)) if result == "failed" => Some(CompactResult::Failed),
                    _ => return Err("qualification_compaction_status_invalid: compact_result is not a closed SDK value".to_owned()),
                };
                self.statuses.push(StatusFrame {
                    status,
                    compact_result,
                });
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) fn check_compaction(update: &Update) -> Result<bool> {
    let Some(receipt) = update
        .receipts
        .iter()
        .find(|receipt| receipt.operation == "qualification-compaction")
    else {
        return Ok(false);
    };
    if receipt.state != OperationState::Confirmed || receipt.reason != "harness_compacted" {
        return Err(format!(
            "qualification_compaction_unconfirmed: state={:?}; reason={}",
            receipt.state, receipt.reason
        ));
    }
    Ok(true)
}
