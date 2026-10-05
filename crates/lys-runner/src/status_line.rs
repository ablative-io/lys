//! The status line of a run whose model calls go through the proxy: the
//! command the run's harness runs hands what the harness reported (its cost
//! in dollars and its running time among it) to the runner on its socket.
//!
//! The runner proves the asking process from the socket, never from the
//! session the input names, as it does for a judge's hook.

use std::path::Path;

use serde_json::Value;

use crate::error::RunnerError;
use crate::peer::{Collected, PeerAct, PeerRequest};
use crate::protocol::{Answer, PROTOCOL_VERSION, read_reply};

/// The peer request that hands the status line's `stdin` to a runner.
///
/// # Errors
/// Refuses input that is not JSON, without echoing it.
pub fn request(stdin: &str) -> Result<PeerRequest, RunnerError> {
    let input: Value = serde_json::from_str(stdin).map_err(|error| RunnerError::Malformed {
        reason: format!(
            "the status line's input is not JSON ({:?})",
            error.classify()
        ),
    })?;
    Ok(PeerRequest {
        version: PROTOCOL_VERSION,
        peer: PeerAct::Collect(Collected::StatusLine { input }),
    })
}

/// Hand the status line's `stdin` to the runner on `socket`, answering the
/// runner's words on what it kept.
///
/// # Errors
/// Returns malformed input, a runner that cannot be reached, an unproved
/// caller or a refusal by name.
pub fn hand(socket: &Path, stdin: &str) -> Result<String, RunnerError> {
    let line = serde_json::to_string(&request(stdin)?).map_err(|error| RunnerError::Malformed {
        reason: format!("the status line's request could not be encoded: {error}"),
    })?;
    let mut connection = crate::connect(socket)?;
    connection.greeting()?;
    match read_reply(&connection.exchange(&line)?)? {
        Answer::Collected { words } => Ok(words),
        _ => Err(RunnerError::ReplyMalformed {
            reason: "the status line received an answer for another act".to_owned(),
        }),
    }
}

/// The line the harness shows: that Lys counts this run, or, when the
/// figures could not be handed over, that they were not and why. A failure
/// is said on the run's own screen, never passed over.
#[must_use]
pub fn shown(handed: &Result<String, RunnerError>) -> String {
    match handed {
        Ok(_) => "Lys: counted".to_owned(),
        Err(error) => format!("Lys: dollars and running time not counted ({error})"),
    }
}

#[cfg(test)]
mod tests {
    use super::{request, shown};
    use crate::error::RunnerError;
    use crate::peer::{Collected, PeerAct};

    #[test]
    fn the_input_is_handed_whole_as_a_status_line() {
        let asked = request(r#"{"session_id":"s","cost":{"total_cost_usd":0.25}}"#)
            .expect("JSON input is a request");
        let PeerAct::Collect(Collected::StatusLine { input }) = asked.peer else {
            panic!("a status line is collected as one");
        };
        assert_eq!(input["cost"]["total_cost_usd"], 0.25);
    }

    #[test]
    fn input_that_is_not_json_is_refused_without_echoing_it() {
        let error = request("secret words").expect_err("text is not a status line's input");
        assert!(!error.to_string().contains("secret"));
    }

    #[test]
    fn a_failure_is_said_on_the_line_and_never_shown_as_counted() {
        let failed = Err(RunnerError::Malformed {
            reason: "no runner".to_owned(),
        });
        let line = shown(&failed);
        assert!(line.contains("not counted") && line.contains("no runner"));
        assert_eq!(shown(&Ok("held".to_owned())), "Lys: counted");
    }
}
