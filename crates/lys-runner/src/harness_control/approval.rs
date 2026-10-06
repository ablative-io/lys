//! Native approval requests retain the existing policy authority.

use super::events::{apply, runtime};
use super::{Binding, Update};
use crate::error::RunnerError;
use crate::session::Sessions;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

impl Sessions {
    pub(super) fn managed_approval(
        self: &Arc<Self>,
        id: &str,
        generation: u64,
        source: &Binding,
        value: serde_json::Value,
    ) -> Result<(), RunnerError> {
        let mut table = self.lock()?;
        let held = runtime(&mut table, id, generation)?;
        if held.approval_active {
            return Err(RunnerError::refused(
                "control_approval_unsupported",
                "another policy authority request is active",
            ));
        }
        let conversation = &held.controller.binding.conversation;
        let turn = held.controller.active_turn().ok_or_else(|| {
            RunnerError::refused(
                "control_approval_unproved",
                "owned thread has no proved active turn",
            )
        })?;
        if value
            .pointer("/params/threadId")
            .and_then(serde_json::Value::as_str)
            != Some(conversation.as_str())
            || value
                .pointer("/params/turnId")
                .and_then(serde_json::Value::as_str)
                != Some(turn)
        {
            return Err(RunnerError::refused(
                "control_approval_unproved",
                "approval does not name the owned active thread and turn",
            ));
        }
        held.approval_active = true;
        let left = Arc::clone(&held.left);
        let sessions = Arc::clone(self);
        let source = source.clone();
        let owned = id.to_owned();
        drop(table);
        std::thread::Builder::new()
            .name("runner-control-authority".to_owned())
            .spawn(move || {
                if let Err(error) =
                    sessions.answer_managed_approval(&owned, generation, &source, &value, &left)
                {
                    if let Err(lost) = sessions.managed_lost(&owned, generation, &error.to_string())
                    {
                        crate::error::said(&format!("control_approval_loss_unrecorded: {lost}"));
                    }
                    crate::error::said(&format!("control_approval_refused: {error}"));
                }
            })
            .map(drop)
            .map_err(|error| {
                RunnerError::refused("control_approval_worker_failed", error.to_string())
            })
    }

    fn answer_managed_approval(
        &self,
        id: &str,
        generation: u64,
        source: &Binding,
        value: &serde_json::Value,
        left: &AtomicBool,
    ) -> Result<(), RunnerError> {
        let method = value.get("method").and_then(serde_json::Value::as_str);
        let tool_name = match method {
            Some("item/commandExecution/requestApproval") => "Bash",
            Some("item/fileChange/requestApproval") => "apply_patch",
            _ => {
                return Err(RunnerError::refused(
                    "control_approval_unsupported",
                    "server request has no existing policy mapping",
                ));
            }
        };
        let request = value.get("id").ok_or_else(|| {
            RunnerError::refused("control_approval_unproved", "server request has no id")
        })?;
        let asked = crate::refusals::JudgeAsk {
            attempt: Some(serde_json::json!([source.session, generation, request]).to_string()),
            tool_name: tool_name.to_owned(),
            tool_input: serde_json::json!({"command":value.pointer("/params/command")}),
            claimed_session: Some(id.to_owned()),
            subagent: false,
        };
        let verdict =
            crate::refusal_log::answer_proved(self, id, generation, &source.leader, &asked, left);
        let (name, frame) = if verdict.deny {
            (
                "control_approval_declined",
                serde_json::json!({"id":request,"result":{"decision":"decline"}}),
            )
        } else {
            // The tool judge can lift a denial; it cannot grant native approval.
            (
                "control_approval_unsupported",
                serde_json::json!({"id":request,"error":{"code":-32000,
                "message":"control_approval_unsupported: native approval has no explicit authority"}}),
            )
        };
        let mut table = self.lock()?;
        let held = runtime(&mut table, id, generation)?;
        held.approval_active = false;
        let event = super::ManagedEvent {
            binding: held.controller.binding.clone(),
            event: name.to_owned(),
            turn: held.controller.active_turn().map(str::to_owned),
            operation: None,
        };
        apply(
            &mut table,
            id,
            generation,
            Update {
                events: vec![event],
                dispatches: vec![super::Dispatch {
                    operation: String::new(),
                    frame,
                }],
                ..Update::default()
            },
        )?;
        drop(table);
        self.writer.barrier()?;
        self.wake();
        Ok(())
    }
}
