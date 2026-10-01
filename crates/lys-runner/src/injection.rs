//! Explicit injection admission and attribution, independent of the grant engine.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::RunnerError;
use crate::session::{Sessions, now_ms, unknown};
use crate::tracking_store::{Body, Commit};

/// The verified caller and request context supplied by the admitting caller.
pub struct Injection<'a> {
    /// The bare session id.
    pub session: &'a str,
    /// The verified person or agent identity, retained verbatim.
    pub sender: &'a str,
    /// The exact bytes to deliver, never stored in the feed.
    pub bytes: &'a [u8],
    /// Whether the caller verified a request signature.
    pub signed: bool,
    /// The accompanying cookie, used only to refuse mixed credentials.
    pub cookie: Option<&'a str>,
}

/// The exact grant question asked before an injection is recorded or delivered.
pub struct InputGrant<'a> {
    /// Always `session`.
    pub kind: &'static str,
    /// The bare session id.
    pub id: &'a str,
    /// Always `input`.
    pub action: &'static str,
    /// The caller's verified identity.
    pub sender: &'a str,
}

/// A caller-supplied live grant judge; there is no default authority.
pub trait InputJudge {
    /// Allow the question, or refuse it by a stable name.
    ///
    /// The callback holds no session table lock. It must not inject into
    /// this same input path while judging it.
    ///
    /// # Errors
    /// Returns the grant source's named refusal.
    fn judge(&self, grant: &InputGrant<'_>) -> Result<(), RunnerError>;
}

impl<F> InputJudge for F
where
    F: Fn(&InputGrant<'_>) -> Result<(), RunnerError>,
{
    fn judge(&self, grant: &InputGrant<'_>) -> Result<(), RunnerError> {
        self(grant)
    }
}

/// The injection's sender and payload digest, carrying no input text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InjectionRecord {
    /// The verified sender, or `lys` for an in-process notice.
    pub sender: String,
    /// The input's byte length.
    pub length: usize,
    /// The lowercase hex SHA-256 of the exact input bytes.
    pub sha256: String,
}

/// Admit an explicitly attributed injection through the supplied grant judge.
///
/// Its leaf and commit are durable before terminal delivery. The returned
/// number names the injection entry, not its commit. A delivery failure is
/// returned by name; its durable admission record remains.
///
/// # Errors
/// Returns mixed credentials, invalid sender, grant, session, durability or
/// terminal failures. A grant refusal writes neither feed nor terminal.
pub fn inject(
    sessions: &Sessions,
    injection: &Injection<'_>,
    judge: &dyn InputJudge,
) -> Result<u64, RunnerError> {
    if injection.signed && injection.cookie.is_some() {
        return Err(RunnerError::refused(
            "AgentCookieRefused",
            "a signed injection cannot carry a cookie",
        ));
    }
    if injection.sender.is_empty()
        || injection.sender == "lys"
        || injection.sender.trim() != injection.sender
        || injection.sender.chars().any(char::is_control)
    {
        return Err(RunnerError::refused(
            "InjectionSenderInvalid",
            "a verified person or agent sender is required",
        ));
    }
    deliver(
        sessions,
        injection.session,
        injection.sender,
        injection.bytes,
        Some(judge),
    )
}

/// Inject an in-process service notice, attributed to `lys`.
///
/// This entry has no protocol act or network route. It is separate from
/// the caller entry so a supplied sender cannot claim the service's authority.
///
/// # Errors
/// Returns session, durability or terminal failures.
pub fn lys_notice(sessions: &Sessions, session: &str, bytes: &[u8]) -> Result<u64, RunnerError> {
    deliver(sessions, session, "lys", bytes, None)
}

pub(crate) fn deliver(
    sessions: &Sessions,
    id: &str,
    sender: &str,
    bytes: &[u8],
    judge: Option<&dyn InputJudge>,
) -> Result<u64, RunnerError> {
    record(sessions, id, sender, bytes, judge, true)
}

pub(crate) fn record(
    sessions: &Sessions,
    id: &str,
    sender: &str,
    bytes: &[u8],
    judge: Option<&dyn InputJudge>,
    send: bool,
) -> Result<u64, RunnerError> {
    let (writer, generation) = {
        let mut table = sessions.lock()?;
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        (session.live(id)?.writer.clone(), session.generation)
    };
    // Only this input path is held, keeping attribution and delivery in order.
    let input_guard = writer.injection()?;
    if let Some(judge) = judge {
        judge.judge(&InputGrant {
            kind: "session",
            id,
            action: "input",
            sender,
        })?;
    }
    let record = InjectionRecord {
        sender: sender.to_owned(),
        length: bytes.len(),
        sha256: crate::protocol::hex(&Sha256::digest(bytes)),
    };
    let index = {
        let mut table = sessions.lock()?;
        let session = table.sessions.get_mut(id).ok_or_else(|| unknown(id))?;
        session.live(id)?;
        if session.generation != generation {
            return Err(RunnerError::refused(
                "InjectionSessionChanged",
                "the terminal changed before admission",
            ));
        }
        table.feed.append(
            id,
            now_ms(),
            vec![Body::Injection(record)],
            Commit::default(),
        )?
    };
    sessions.writer.barrier()?;
    if send {
        writer.write(bytes.to_vec())?;
    }
    drop(input_guard);
    Ok(index)
}
