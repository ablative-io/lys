//! Upgrade acts are accepted only through the authenticated server socket.

use crate::{Act, Answer, RunnerError, Sessions};
use std::sync::Arc;

pub(super) fn perform(sessions: &Arc<Sessions>, act: Act) -> Result<Answer, RunnerError> {
    match act {
        Act::UpgradeSessions => Ok(Answer::UpgradeSessions {
            sessions: sessions.prepare_upgrade()?,
        }),
        Act::CancelUpgrade => {
            sessions.cancel_upgrade();
            Ok(Answer::UpgradeCancelled)
        }
        Act::ResumeUpgrade { saved } => {
            let session = saved.launch.session.clone();
            let (pid, started_at) = sessions.resume_upgrade(*saved)?;
            Ok(Answer::Started {
                qualification: sessions.qualification(&session)?,
                session,
                pid,
                started_at,
            })
        }
        _ => Err(RunnerError::refused(
            "runner_upgrade_invalid",
            "this act is not an upgrade operation",
        )),
    }
}
