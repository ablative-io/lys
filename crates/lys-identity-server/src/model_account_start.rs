//! Admission of the exact profile's account handles before its first spawn.

use axum::http::StatusCode;
use lys_runner::Rotation;
use lys_runner::rotation::RotationState;

use crate::ServerError;
use crate::launch_template::HandleName;
use crate::provisioning_store::Version;

/// Check every account against the agent's live handle inventory.
///
/// The caller supplies the inventory read for this agent from the broker.
/// Secret names cannot stand in for handle ids, including later accounts
/// that will only be used after a usage limit.
///
/// # Errors
/// Returns `account_handle_unheld` for an account outside that inventory,
/// or the runner's named refusal for a malformed rotation declaration.
pub fn accounts(version: &Version, held: &[HandleName]) -> Result<Option<Rotation>, ServerError> {
    let Some(rotation) = version
        .settings
        .session
        .as_ref()
        .and_then(|session| session.accounts.as_ref())
    else {
        return Ok(None);
    };
    RotationState::new(rotation.clone())?;
    for account in &rotation.accounts {
        if !held.iter().any(|handle| handle.id == *account) {
            return Err(ServerError::SecretsRefused {
                status: StatusCode::FORBIDDEN,
                refusal: "account_handle_unheld".to_owned(),
                reason: format!("the agent does not hold account handle `{account}`"),
            });
        }
    }
    Ok(Some(rotation.clone()))
}
