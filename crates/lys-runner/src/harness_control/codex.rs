//! Codex admission and completed items are separate protocol observations.

use serde_json::{Value, json};

use super::{Boundary, Controller, Dispatch, Kind, Observation, Pending, Update};
use crate::error::RunnerError;

/// Preserve explicit model and sandbox settings in app-server config overrides.
///
/// # Errors
/// Refuses unknown flags and directory overrides whose base roots are unproved.
pub fn arguments(given: &[String]) -> Result<Vec<String>, RunnerError> {
    let invalid = || RunnerError::refused("control_launch_invalid", "launch flag has no value");
    let mut arguments = Vec::new();
    let mut roots: Option<Vec<String>> = None;
    let mut additional = Vec::new();
    let mut at = 0;
    while at < given.len() {
        let value = given.get(at + 1).ok_or_else(invalid)?;
        match given[at].as_str() {
            "-c" | "--config" => {
                if let Some(encoded) = value.strip_prefix("sandbox_workspace_write.writable_roots=")
                {
                    roots = Some(serde_json::from_str(encoded).map_err(|error| {
                        RunnerError::refused(
                            "control_launch_unsupported",
                            format!("writable roots must be an explicit string array: {error}"),
                        )
                    })?);
                } else {
                    arguments.extend(["-c".to_owned(), value.clone()]);
                }
            }
            "--model" | "-m" | "--sandbox" | "-s" => {
                let name = if matches!(given[at].as_str(), "--model" | "-m") {
                    "model"
                } else {
                    "sandbox_mode"
                };
                let encoded = serde_json::to_string(value).map_err(|error| {
                    RunnerError::refused("control_launch_invalid", error.to_string())
                })?;
                arguments.extend(["-c".to_owned(), format!("{name}={encoded}")]);
            }
            "--add-dir" => additional.push(value.clone()),
            _ => {
                return Err(RunnerError::refused(
                    "control_launch_unsupported",
                    "launch flag is not supported by the app-server adapter",
                ));
            }
        }
        at += 2;
    }
    if !additional.is_empty() && roots.is_none() {
        return Err(RunnerError::refused(
            "control_launch_unsupported",
            "additional directories require explicit base writable roots",
        ));
    }
    if let Some(mut roots) = roots {
        roots.extend(additional);
        let encoded = serde_json::to_string(&roots)
            .map_err(|error| RunnerError::refused("control_launch_invalid", error.to_string()))?;
        arguments.extend([
            "-c".to_owned(),
            format!("sandbox_workspace_write.writable_roots={encoded}"),
        ]);
    }
    arguments.push("app-server".to_owned());
    arguments.extend(["--listen".to_owned(), "stdio://".to_owned()]);
    Ok(arguments)
}

pub(super) fn request(conversation: &str, pending: &Pending) -> Value {
    if pending.kind == Kind::Compact {
        json!({"id":pending.id,"method":"thread/compact/start","params":{"threadId":conversation}})
    } else {
        json!({"id":pending.id,"method":"turn/start","params":{
            "threadId":conversation, "clientUserMessageId":pending.uuid,
            "input":[{"type":"text","text":pending.text,"text_elements":[]}]}})
    }
}

pub(super) fn observe(
    conversation: &str,
    value: &Value,
    pending: Option<&Pending>,
) -> Result<Observation, RunnerError> {
    if value.get("method").is_some() && value.get("id").is_some() {
        return Err(RunnerError::refused(
            "control_approval_unsupported",
            "server request requires a policy authority response; no approval was fabricated",
        ));
    }
    if let Some(id) = value.get("id").and_then(Value::as_str) {
        let Some(pending) = pending.filter(|pending| pending.id == id) else {
            return Ok(Observation::Other);
        };
        if value.get("error").is_some() {
            return Ok(Observation::Refused);
        }
        let result = value.get("result").ok_or_else(|| {
            RunnerError::refused(
                "control_protocol_unsupported",
                "RPC reply contains neither result nor error",
            )
        })?;
        if pending.kind == Kind::Compact {
            return Ok(Observation::Accepted);
        }
        let turn = result
            .get("turn")
            .and_then(|turn| turn.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RunnerError::refused(
                    "control_correlation_unsupported",
                    "turn/start reply has no admitted turn",
                )
            })?;
        return Ok(Observation::Admitted {
            uuid: pending.uuid.clone(),
            turn: Some(turn.to_owned()),
        });
    }
    let method = value.get("method").and_then(Value::as_str).ok_or_else(|| {
        RunnerError::refused(
            "control_protocol_unsupported",
            "frame is neither a correlated reply nor a notification",
        )
    })?;
    let params = value.get("params").ok_or_else(|| {
        notification_error(
            method,
            &Value::Null,
            RunnerError::refused(
                "control_protocol_unsupported",
                "notification has no params object",
            ),
        )
    })?;
    notification(conversation, method, params)
        .map_err(|error| notification_error(method, params, error))
}

enum NotificationKind {
    Started,
    Completed,
    Item,
    Other,
    Ended,
}

fn notification(
    conversation: &str,
    method: &str,
    params: &Value,
) -> Result<Observation, RunnerError> {
    if !params.is_object() {
        return Err(RunnerError::refused(
            "control_protocol_unsupported",
            "notification has no params object",
        ));
    }
    if method == "thread/started" {
        if params
            .pointer("/thread/id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            return Err(RunnerError::refused(
                "control_correlation_unsupported",
                "params.thread.id must be a nonempty string",
            ));
        }
        return Ok(Observation::Other);
    }
    if matches!(
        method,
        "skills/changed"
            | "project/changed"
            | "command/exec/outputDelta"
            | "process/outputDelta"
            | "process/exited"
            | "mcpServer/event/stream/notification"
            | "account/updated"
            | "account/gatewayOAuth/changed"
            | "account/rateLimits/updated"
            | "app/list/updated"
            | "remoteControl/status/changed"
            | "externalAgentConfig/import/progress"
            | "externalAgentConfig/import/completed"
            | "fs/changed"
            | "deprecationNotice"
            | "configWarning"
            | "fuzzyFileSearch/sessionUpdated"
            | "fuzzyFileSearch/sessionCompleted"
            | "windows/worldWritableWarning"
            | "windowsSandbox/setupCompleted"
            | "account/login/completed"
    ) {
        return Ok(Observation::Other);
    }
    if matches!(
        method,
        "warning" | "mcpServer/oauthLogin/completed" | "mcpServer/startupStatus/updated"
    ) && params.get("threadId").is_none_or(Value::is_null)
    {
        return Ok(Observation::Other);
    }
    let thread = params
        .get("threadId")
        .and_then(Value::as_str)
        .filter(|thread| !thread.is_empty())
        .ok_or_else(|| {
            RunnerError::refused(
                "control_correlation_unsupported",
                "params.threadId must be a nonempty string",
            )
        })?;
    let notification = match method {
        "turn/started" => NotificationKind::Started,
        "turn/completed" => NotificationKind::Completed,
        "item/started" | "item/completed" => NotificationKind::Item,
        "thread/closed" | "thread/archived" | "thread/deleted" | "thread/reverted" => {
            NotificationKind::Ended
        }
        "error"
        | "thread/status/changed"
        | "thread/unarchived"
        | "thread/name/updated"
        | "thread/attachment/updated"
        | "thread/goal/updated"
        | "thread/goal/cleared"
        | "thread/queue/changed"
        | "thread/project/updated"
        | "thread/environment/connected"
        | "thread/environment/disconnected"
        | "thread/settings/updated"
        | "thread/tokenUsage/updated"
        | "hook/started"
        | "hook/completed"
        | "turn/diff/updated"
        | "turn/plan/updated"
        | "item/autoApprovalReview/started"
        | "item/autoApprovalReview/completed"
        | "autoApprovalReview/strictReviewRequired"
        | "rawResponseItem/completed"
        | "rawResponse/completed"
        | "item/agentMessage/delta"
        | "item/plan/delta"
        | "item/commandExecution/outputDelta"
        | "item/commandExecution/terminalInteraction"
        | "item/fileChange/outputDelta"
        | "item/fileChange/patchUpdated"
        | "serverRequest/resolved"
        | "item/mcpToolCall/progress"
        | "mcpServer/oauthLogin/completed"
        | "mcpServer/startupStatus/updated"
        | "item/reasoning/summaryTextDelta"
        | "item/reasoning/summaryPartAdded"
        | "item/reasoning/textDelta"
        | "thread/compacted"
        | "model/rerouted"
        | "model/verification"
        | "modelProvider/authRecoveryStarted"
        | "modelProvider/authRecoveryCompleted"
        | "turn/moderationMetadata"
        | "model/safetyBuffering/updated"
        | "warning"
        | "guardianWarning"
        | "thread/realtime/started"
        | "thread/realtime/itemAdded"
        | "thread/realtime/item/started"
        | "thread/realtime/item/transcript/delta"
        | "thread/realtime/item/completed"
        | "thread/realtime/transcript/delta"
        | "thread/realtime/transcript/done"
        | "thread/realtime/outputAudio/delta"
        | "thread/realtime/sdp"
        | "thread/realtime/error"
        | "thread/realtime/closed" => NotificationKind::Other,
        _ => {
            return Err(RunnerError::refused(
                "control_protocol_unsupported",
                "notification method is unsupported",
            ));
        }
    };
    if thread != conversation {
        return Ok(Observation::Other);
    }
    match notification {
        NotificationKind::Started => Ok(Observation::Started {
            turn: required(params, "/turn/id")?,
        }),
        NotificationKind::Completed => {
            let status = required(params, "/turn/status")?;
            if !matches!(status.as_str(), "completed" | "failed" | "interrupted") {
                return Err(RunnerError::refused(
                    "control_protocol_unsupported",
                    "terminal turn has an unsupported status",
                ));
            }
            Ok(Observation::Completed {
                turn: Some(required(params, "/turn/id")?),
                failed: status != "completed",
            })
        }
        NotificationKind::Item => {
            let kind = required(params, "/item/type")?;
            if kind == "contextCompaction" && method == "item/completed" {
                Ok(Observation::Compacted {
                    turn: Some(required(params, "/turnId")?),
                })
            } else if matches!(
                kind.as_str(),
                "userMessage"
                    | "hookPrompt"
                    | "agentMessage"
                    | "functionCallOutput"
                    | "plan"
                    | "reasoning"
                    | "commandExecution"
                    | "fileChange"
                    | "mcpToolCall"
                    | "dynamicToolCall"
                    | "collabAgentToolCall"
                    | "subAgentActivity"
                    | "webSearch"
                    | "imageView"
                    | "sleep"
                    | "imageGeneration"
                    | "enteredReviewMode"
                    | "exitedReviewMode"
                    | "contextCompaction"
            ) {
                Ok(Observation::Other)
            } else {
                Err(RunnerError::refused(
                    "control_protocol_unsupported",
                    "notification has an unsupported item variant",
                ))
            }
        }
        NotificationKind::Other => Ok(Observation::Other),
        NotificationKind::Ended => Err(RunnerError::refused(
            "control_thread_ended",
            format!("the bound conversation ended or was rewritten by {method}"),
        )),
    }
}

fn notification_error(method: &str, params: &Value, mut error: RunnerError) -> RunnerError {
    if let RunnerError::Refused { words, .. } = &mut error {
        let keys: Vec<&str> = params
            .as_object()
            .into_iter()
            .flat_map(|params| params.keys().map(String::as_str))
            .collect();
        *words = format!(
            "{words}; method={}; params_keys={}",
            json!(method),
            json!(keys)
        );
    }
    error
}

fn required(value: &Value, path: &str) -> Result<String, RunnerError> {
    value
        .pointer(path)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            RunnerError::refused(
                "control_protocol_unsupported",
                "notification has no turn identity",
            )
        })
}

impl Controller {
    pub(super) fn bind_codex(&mut self, value: &Value) -> Result<Option<Update>, RunnerError> {
        if value.get("id").and_then(Value::as_str) == Some("lys-initialize") {
            if value.get("error").is_some() {
                return Err(RunnerError::refused(
                    "control_protocol_unsupported",
                    "initialize was refused",
                ));
            }
            if self.initialized {
                return Ok(Some(Update::default()));
            }
            let reported = value
                .pointer("/result/userAgent")
                .and_then(Value::as_str)
                .and_then(|agent| agent.split_whitespace().next())
                .and_then(|product| product.split_once('/'))
                .map(|(_, version)| version);
            if reported != Some(self.binding.harness_version.as_str()) {
                return Err(RunnerError::refused(
                    "control_adapter_unqualified",
                    "Codex initialize version differs from its probed version or is absent",
                ));
            }
            self.initialized = true;
            let (method, params) = if self.binding.conversation.is_empty() {
                ("thread/start", json!({}))
            } else {
                (
                    "thread/resume",
                    json!({"threadId": self.binding.conversation}),
                )
            };
            return Ok(Some(Update {
                dispatches: vec![
                    Dispatch {
                        operation: String::new(),
                        frame: json!({"method":"initialized"}),
                    },
                    Dispatch {
                        operation: String::new(),
                        frame: json!({"id":"lys-thread","method":method,"params":params}),
                    },
                ],
                ..Update::default()
            }));
        }
        if value.get("id").and_then(Value::as_str) == Some("lys-thread") {
            if !self.initialized {
                return Err(RunnerError::refused(
                    "control_source_unbound",
                    "thread reply arrived before initialization",
                ));
            }
            if self.ready {
                return Ok(Some(Update::default()));
            }
            let thread = value.pointer("/result/thread").ok_or_else(|| {
                RunnerError::refused("control_source_unbound", "thread reply has no thread")
            })?;
            let id = thread.get("id").and_then(Value::as_str).ok_or_else(|| {
                RunnerError::refused("control_source_unbound", "thread reply has no identity")
            })?;
            if !self.binding.conversation.is_empty() && id != self.binding.conversation {
                return Err(RunnerError::refused(
                    "control_source_mismatch",
                    "thread reply names another conversation",
                ));
            }
            if thread.pointer("/status/type").and_then(Value::as_str) != Some("idle") {
                return Err(RunnerError::refused(
                    "control_source_unbound",
                    "resumed thread has no proved idle boundary",
                ));
            }
            id.clone_into(&mut self.binding.conversation);
            self.ready = true;
            self.compact = true;
            self.boundary = Boundary::Idle;
            return Ok(Some(Update {
                events: vec![self.event("control_bound", None, None)],
                ..Update::default()
            }));
        }
        Ok(None)
    }
}

pub(super) fn passive_frame(value: &Value) -> bool {
    matches!(
        value.get("method").and_then(Value::as_str),
        Some(
            "item/agentMessage/delta"
                | "item/reasoning/textDelta"
                | "item/reasoning/summaryTextDelta"
                | "item/commandExecution/outputDelta"
                | "thread/tokenUsage/updated"
        )
    )
}
