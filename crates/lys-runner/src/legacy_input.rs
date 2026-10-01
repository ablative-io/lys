//! Server requests retain explicit caller admission for owned sessions.
//! An unowned legacy session records server-key attribution only.

use crate::error::RunnerError;
use crate::injection::{Injection, InputJudge, inject};
use crate::operations::{Operation, OperationOutcome, OperationRequest};
use crate::protocol::Key;
use crate::session::Sessions;

/// Identity already verified by the admitting caller, with its live grant judge.
pub struct InputContext<'a> {
    /// The verified person or agent, never inferred from the session's owner.
    pub sender: &'a str,
    /// Whether a caller signature was verified.
    pub signed: bool,
    /// A cookie, used only to refuse mixed authority.
    pub cookie: Option<&'a str>,
    /// The current session/input grant judge.
    pub judge: &'a dyn InputJudge,
}

fn sender(
    sessions: &Sessions,
    id: &str,
    server: &[u8; 32],
    context: Option<&InputContext<'_>>,
) -> Result<String, RunnerError> {
    if let Some(context) = context {
        if context.signed && context.cookie.is_some() {
            return Err(RunnerError::refused(
                "AgentCookieRefused",
                "signed input cannot carry a cookie",
            ));
        }
        if context.sender.is_empty()
            || context.sender == "lys"
            || context.sender.trim() != context.sender
            || context.sender.chars().any(char::is_control)
        {
            return Err(RunnerError::refused(
                "InjectionSenderInvalid",
                "a verified caller is required",
            ));
        }
        return Ok(context.sender.to_owned());
    }
    if sessions.lock()?.responsible.contains_key(id) {
        return Err(RunnerError::refused(
            "SessionInputContextMissing",
            "owned input requires a verified caller and live grant judge",
        ));
    }
    Ok(format!("server:{}", crate::protocol::hex(server)))
}

/// Admit one legacy byte input and retain the exact sender before delivery.
///
/// # Errors
/// Returns missing context, grant, session, durability or terminal errors by name.
pub fn write(
    sessions: &Sessions,
    server: &[u8; 32],
    id: &str,
    bytes: &[u8],
    context: Option<&InputContext<'_>>,
) -> Result<(), RunnerError> {
    let sender = sender(sessions, id, server, context)?;
    if let Some(context) = context {
        inject(
            sessions,
            &Injection {
                session: id,
                sender: &sender,
                bytes,
                signed: context.signed,
                cookie: context.cookie,
            },
            context.judge,
        )?;
    } else {
        crate::injection::deliver(sessions, id, &sender, bytes, None)?;
    }
    Ok(())
}

/// Judge compact before its existing durable operation acceptance.
/// Its command's attribution is recorded before any boundary delivery.
///
/// # Errors
/// Returns missing context, grant, session, recording or operation errors by name.
pub fn compact(
    sessions: &Sessions,
    server: &[u8; 32],
    operation: Operation,
    context: Option<&InputContext<'_>>,
) -> Result<OperationOutcome, RunnerError> {
    let OperationRequest::Compact { text } = &operation.request else {
        return Err(RunnerError::refused(
            "SessionInputInvalid",
            "the operation is not a compaction",
        ));
    };
    let sender = sender(sessions, &operation.session, server, context)?;
    let mut bytes = text.as_bytes().to_vec();
    bytes.extend_from_slice(Key::Enter.bytes());
    crate::injection::record(
        sessions,
        &operation.session,
        &sender,
        &bytes,
        context.map(|context| context.judge),
        false,
    )?;
    sessions.operate_admitted(operation)
}
