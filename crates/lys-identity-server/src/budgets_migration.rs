//! Earlier snapshots remain readable during rollback; persist the current fold
//! after the common upgrade-intent check permits it, at start or admission.

use crate::budgets_api::with_budgets;
use crate::error::ServerError;
use crate::routes::AppState;

/// Whether budget migration must remain memory-only, with an unreadable intent refused.
pub fn pending(state: &AppState) -> Result<bool, ServerError> {
    crate::operator::upgrade_pending(state).map_err(|error| ServerError::BudgetsUnavailable {
        reason: format!(
            "budget migration cannot read upgrade intent {}: {error}",
            state.operator_upgrade_file.as_deref().map_or_else(
                || "(unmanaged)".to_owned(),
                |path| path.display().to_string()
            )
        ),
    })
}

/// New-format confirmations are refused while the previous build may be restored.
pub fn require_committed(state: &AppState) -> Result<(), ServerError> {
    if pending(state)? {
        return Err(ServerError::BudgetsUnavailable {
            reason: "upgrade_pending: budget changes are refused while the upgrade is reversible"
                .to_owned(),
        });
    }
    Ok(())
}

/// Finish migration at startup or the next mutating admission, never from a read.
pub fn advance(state: &AppState) -> Result<(), ServerError> {
    if state.budgets.is_some() && !pending(state)? {
        with_budgets(state, crate::budgets_store::BudgetStore::finish_migration)?;
    }
    Ok(())
}
