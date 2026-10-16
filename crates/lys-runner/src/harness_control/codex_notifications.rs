//! Notifications are admitted by vocabulary before their bound conversation is read.
use super::{Observation, RunnerError, Value, required};
enum NotificationKind {
    Started,
    Completed,
    Item,
    Other,
    Ended,
}

pub(super) fn notification(
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
    if global_notification(method, params)? {
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
    let notification = kind(method)?;
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

fn global_notification(method: &str, params: &Value) -> Result<bool, RunnerError> {
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
        return Ok(true);
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
        return Ok(true);
    }
    if matches!(
        method,
        "warning" | "mcpServer/oauthLogin/completed" | "mcpServer/startupStatus/updated"
    ) && params.get("threadId").is_none_or(Value::is_null)
    {
        return Ok(true);
    }
    Ok(false)
}

fn kind(method: &str) -> Result<NotificationKind, RunnerError> {
    Ok(match method {
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
    })
}
