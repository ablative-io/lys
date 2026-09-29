//! Which of a profile's models its declared harness carries. The first
//! model is the session's own; every further one goes where the harness
//! takes further models, or the profile is refused by name, so no model is
//! dropped.

use lys_home::harness::launch_fields::{DeclaredHarness, HarnessKind};

use crate::error::ServerError;

/// Refuse the first of `models` that `harness` cannot carry.
pub fn models(harness: &DeclaredHarness, models: &[String]) -> Result<(), ServerError> {
    let refused = |model: &String, reason: &str| ServerError::ModelUnrepresentable {
        harness: match harness.kind {
            HarnessKind::ClaudeCode => "claude_code",
            HarnessKind::Codex => "codex",
        }
        .to_owned(),
        model: model.clone(),
        reason: reason.to_owned(),
    };
    match harness.kind {
        HarnessKind::ClaudeCode => {
            models
                .iter()
                .find(|model| model.contains(','))
                .map_or(Ok(()), |model| {
                    Err(refused(
                        model,
                        "Claude Code takes further models as one comma-separated list",
                    ))
                })
        }
        HarnessKind::Codex => match models {
            [_] => Ok(()),
            [] => Err(refused(
                &String::new(),
                "the Codex build takes exactly one model and the profile names none",
            )),
            [_, further, ..] => Err(refused(
                further,
                "the Codex build takes one model for a session and no further ones",
            )),
        },
    }
}
