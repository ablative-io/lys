//! Codex hooks ask the existing runner judge over its peer-authenticated socket.
//! Transport failure produces a native deny, never an empty successful reply;
//! this is not a claim that Codex enforces an absent or untrusted hook.

use std::path::Path;

use serde_json::Value;

use crate::codex_judge::{request, response};
use crate::error::RunnerError;
use crate::peer::PeerRequest;
use crate::protocol::{Answer, read_reply};
use crate::refusals::Verdict;

/// Ask the runner to judge native hook stdin. No claimed session is used to
/// choose a server-side identity: the connection itself is authenticated by
/// the runner's existing peer ancestry checks. No second collector is started.
pub fn ask(socket: &Path, stdin: &str) -> Result<Verdict, RunnerError> {
    exchange(socket, &request(stdin)?)
}

/// Send one judge request over the peer socket and read its verdict. Any
/// harness's adapter shares this one exchange.
pub fn exchange(socket: &Path, request: &PeerRequest) -> Result<Verdict, RunnerError> {
    let line = serde_json::to_string(request).map_err(|error| RunnerError::Malformed {
        reason: format!("the judge request could not be encoded: {error}"),
    })?;
    let mut connection = crate::connect(socket)?;
    connection.greeting()?;
    match read_reply(&connection.exchange(&line)?)? {
        Answer::Judged { verdict } => Ok(verdict),
        _ => Err(RunnerError::ReplyMalformed {
            reason: "the judge received an answer for another act".to_owned(),
        }),
    }
}

/// The hook's native output, including a deny on malformed input, missing
/// runner, unproved caller or failed audit. A failure here has no durable
/// refusal receipt and must not be shown as a successfully recorded refusal.
pub fn output(socket: &Path, stdin: &str) -> Value {
    match ask(socket, stdin) {
        Ok(verdict) => response(&verdict),
        Err(error) => response(&Verdict::denied(
            "codex_judge_unavailable",
            format!("Lys could not complete this policy check: {error}"),
            "not_attributed",
        )),
    }
}
