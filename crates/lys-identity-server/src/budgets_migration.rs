//! Earlier snapshots remain readable during rollback; persist the current fold
//! after the common upgrade-intent check permits it, at start or admission.

use crate::budgets_api::with_budgets;
use crate::error::ServerError;
use crate::error_agents::AgentsError;
use crate::error_budget::BudgetError;
use crate::routes::AppState;

/// Whether budget migration must remain memory-only, with an unreadable intent refused.
///
/// # Errors
/// Refuses an upgrade intent that cannot be inspected.
pub fn pending(state: &AppState) -> Result<bool, ServerError> {
    crate::operator::upgrade_pending(state).map_err(|error| {
        ServerError::Agents(AgentsError::Budget(BudgetError::BudgetsUnavailable {
            reason: format!(
                "budget migration cannot read upgrade intent {}: {error}",
                state.operator_upgrade_file.as_deref().map_or_else(
                    || "(unmanaged)".to_owned(),
                    |path| path.display().to_string()
                )
            ),
        }))
    })
}

/// New-format confirmations are refused while the previous build may be restored.
///
/// # Errors
/// Refuses an unreadable upgrade intent or a reversible upgrade window
/// still open.
pub fn require_committed(state: &AppState) -> Result<(), ServerError> {
    if pending(state)? {
        return Err(ServerError::Agents(AgentsError::Budget(
            BudgetError::BudgetsUnavailable {
                reason:
                    "upgrade_pending: budget changes are refused while the upgrade is reversible"
                        .to_owned(),
            },
        )));
    }
    Ok(())
}

/// Finish migration at startup or the next mutating admission, never from a read.
///
/// # Errors
/// Refuses an unreadable upgrade intent or budgets settlement and durable
/// migration completion failures.
pub fn advance(state: &AppState) -> Result<(), ServerError> {
    if state.budgets.is_some() && !pending(state)? {
        with_budgets(state, crate::budgets_store::BudgetStore::finish_migration)?;
    }
    Ok(())
}
