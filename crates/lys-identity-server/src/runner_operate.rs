//! The one seam a durable operation reaches a session's runner by: a goal's
//! reminder, and a budget's compaction, notice or stop. Its owner keeps the
//! operation's id before asking, so after a lost answer the same operation
//! is asked again under the same id and the runner never does it twice.
//!
//! A refusal the runner names itself is final. Anything else (the runner
//! unreachable, a request it refused before reading, a reply it could not
//! give) leaves the operation unknown: it is asked again, never replaced.
//! A stop the runner confirms is recorded as the session's end.

use std::sync::Arc;

use lys_runner::operations::{Operation, OperationOutcome};
use lys_runner::{Act, Answer};

use crate::error::ServerError;
use crate::routes::AppState;
use crate::runner_sessions::{driven, kind, record_end};

/// Why an operation was not answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undelivered {
    /// Refused, by name: it will not be done.
    Refused(String),
    /// Its answer was lost: it is asked again under the same operation id.
    Unknown(String),
}

/// Ask `operation` of its session's runner, answering how it stands.
pub async fn operate(
    state: &Arc<AppState>,
    operation: Operation,
) -> Result<OperationOutcome, Undelivered> {
    let driven = driven(state, &operation.session)
        .map_err(|error| Undelivered::Refused(error.to_string()))?;
    // Lys itself asks a durable operation, so it is attributed to Lys by
    // the operation's name, never to a person who did not ask it.
    let act = Act::AsCaller {
        caller: format!("lys:{}", operation.request.name()),
        done: Box::new(Act::Operate { operation }),
    };
    let answered =
        crate::runner_client::ask(state, &driven.machine, driven.runner.clone(), act).await;
    let outcome = match answered {
        Ok(Answer::Operation { outcome }) => outcome,
        Ok(other) => {
            return Err(Undelivered::Unknown(format!(
                "the runner answered {} to an operation",
                kind(&other)
            )));
        }
        Err(ServerError::Runner { refusal, words }) if !refusal.starts_with("runner_") => {
            return Err(Undelivered::Refused(format!("{refusal}: {words}")));
        }
        Err(other) => return Err(Undelivered::Unknown(other.to_string())),
    };
    if let Some(ended) = &outcome.ended {
        record_end(state, &driven, ended)
            .map_err(|error| Undelivered::Unknown(error.to_string()))?;
    }
    Ok(outcome)
}

/// Withdraw an accepted `operation` of `session` before its boundary,
/// answering how it then stands; a runner refusal (the operation already
/// being typed, or unknown) is final.
pub async fn withdraw(
    state: &Arc<AppState>,
    session: &str,
    operation: &str,
    why: &str,
) -> Result<OperationOutcome, Undelivered> {
    let driven = driven(state, session).map_err(|error| Undelivered::Refused(error.to_string()))?;
    let act = Act::Withdraw {
        operation: operation.to_owned(),
        why: why.to_owned(),
    };
    match crate::runner_client::ask(state, &driven.machine, driven.runner.clone(), act).await {
        Ok(Answer::Operation { outcome }) => Ok(outcome),
        Ok(other) => Err(Undelivered::Unknown(format!(
            "the runner answered {} to a withdrawal",
            kind(&other)
        ))),
        Err(ServerError::Runner { refusal, words }) if !refusal.starts_with("runner_") => {
            Err(Undelivered::Refused(format!("{refusal}: {words}")))
        }
        Err(other) => Err(Undelivered::Unknown(other.to_string())),
    }
}
