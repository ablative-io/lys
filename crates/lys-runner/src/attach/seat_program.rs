//! ACP attachments render public text, tool state and adapter answers.
use super::{
    ASSISTANT, AttachLine, STATUS, SYSTEM, Sessions, TOOL_CALL, TOOL_RESULT, line, now_ms,
};
use crate::RunnerError;
use serde_json::Value;

fn render(frame: &Value, at: u64) -> Vec<AttachLine> {
    if frame["method"] == "session/update" {
        let update = &frame["params"]["update"];
        match update["sessionUpdate"].as_str() {
            Some("agent_message_chunk") => {
                return update["content"]["text"]
                    .as_str()
                    .map(|text| vec![line(ASSISTANT, text, at)])
                    .unwrap_or_default();
            }
            Some("agent_thought_chunk") => return Vec::new(),
            Some("tool_call") => {
                return vec![line(
                    TOOL_CALL,
                    update["title"].as_str().unwrap_or("tool call"),
                    at,
                )];
            }
            Some("tool_call_update") => {
                return vec![line(
                    TOOL_RESULT,
                    update["status"]
                        .as_str()
                        .unwrap_or("tool state unavailable"),
                    at,
                )];
            }
            _ => return vec![line(STATUS, "ACP session update", at)],
        }
    }
    if frame.get("error").is_some() {
        return vec![line(SYSTEM, "acp_request_refused", at)];
    }
    if frame["id"] == "lys-initialize" {
        return vec![line(STATUS, "ACP initialize answered", at)];
    }
    if frame["id"] == "lys-seat-session" {
        return vec![line(STATUS, "ACP session ready", at)];
    }
    if frame.get("result").is_some() {
        return vec![line(STATUS, "ACP request answered", at)];
    }
    Vec::new()
}

impl Sessions {
    pub(crate) fn seat_attach_frame(
        &self,
        id: &str,
        generation: u64,
        frame: &Value,
    ) -> Result<(), RunnerError> {
        let at = now_ms();
        let lines = render(frame, at);
        let mut table = self.lock()?;
        if let Some(session) = table
            .sessions
            .get_mut(id)
            .filter(|session| session.generation == generation)
        {
            session.last_signal = Some(at);
            for line in lines {
                session.attach.push(line);
            }
        }
        drop(table);
        self.wake();
        Ok(())
    }
}
