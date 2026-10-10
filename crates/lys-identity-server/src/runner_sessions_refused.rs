//! A start the runner refused by its own name started nothing: the session
//! kept as starting is kept stopped in the runner's name and words, so a
//! refused start leaves no live session, and a start sent again under that
//! session is answered with the same refusal without asking the runner.

use super::{Driven, report_id, runner_report};
use crate::error::ServerError;
use crate::routes::AppState;
use crate::runtime_api::with_runtime;
use crate::runtime_state::Reported;

/// The report a start the runner refused is kept stopped under.
const REFUSED: &str = "refused";

/// The refusals that name the session as held by another start, whose
/// session that start keeps.
const HELD: [&str; 2] = ["session_exists", "seat_owner_held"];

/// Whether the runner's refusal `refusal` of a start says by its own name
/// that it started nothing for this start: a failure to reach or read the
/// runner (named `runner_`) leaves the start unknown, and a session held by
/// another start is that start's.
pub(super) fn started_nothing(refusal: &str) -> bool {
    !refusal.starts_with("runner_") && !HELD.contains(&refusal)
}

/// Keep that the runner refused `driven`'s start, so the session kept as
/// starting does not stay live: its record ends stopped, in the runner's
/// own name (`what`) and words (`confirmation`). Answers the runner's
/// refusal, or both failures when its stop cannot be kept.
pub(super) fn record_refused(
    state: &AppState,
    driven: &Driven,
    refusal: String,
    words: String,
) -> ServerError {
    let kept = if state.runtime.is_none() {
        Ok(())
    } else {
        with_runtime(state, |store| {
            let latest = store
                .session(&driven.session)
                .and_then(|tracked| tracked.latest().map(|report| report.state));
            if latest != Some(Reported::Starting) {
                return Ok(());
            }
            let mut report =
                runner_report(driven, Reported::Stopped, refusal.clone(), words.clone());
            report.operation = report_id(&driven.session, REFUSED);
            store.report(report).map(drop)
        })
    };
    match kept {
        Ok(()) => ServerError::Runner { refusal, words },
        Err(recording) => ServerError::RuntimeUnavailable {
            reason: format!(
                "the runner refused the start ({refusal}: {words}); keeping that it started nothing failed: {recording}"
            ),
        },
    }
}

/// The runner's refusal a start of `session` was already answered with:
/// a start sent again is answered the same, and its runner is not asked
/// again under a session already kept stopped.
pub(super) fn refused_before(
    state: &AppState,
    session: &str,
) -> Result<Option<ServerError>, ServerError> {
    if state.runtime.is_none() {
        return Ok(None);
    }
    let id = report_id(session, REFUSED);
    with_runtime(state, |store| {
        Ok(store
            .session(session)
            .and_then(|tracked| tracked.reports.iter().find(|report| report.operation == id))
            .map(|kept| ServerError::Runner {
                refusal: kept.what.clone(),
                words: kept.confirmation.clone(),
            }))
    })
}
