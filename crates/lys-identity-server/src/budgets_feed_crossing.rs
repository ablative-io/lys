//! A crossing keeps only a later control outcome from its runner.
use super::{
    AppState, Arc, ServerError, later_control_outcome, refused, with_budgets, with_budgets_mut,
};
pub(super) fn keep(
    state: &Arc<AppState>,
    outcome: &lys_runner::operations::OperationOutcome,
) -> Result<(), ServerError> {
    let crossing = with_budgets(state, |store| {
        Ok(store.held().crossings.holds(&outcome.operation))
    })?;
    if crossing {
        let observed = i64::try_from(outcome.at)
            .map_err(|error| refused(format!("runner outcome instant: {error}")))?;
        with_budgets_mut(state, |store| {
            let acted = crate::budgets_crossing::Acted::from_runner(outcome, observed);
            if store
                .held()
                .crossings
                .acted
                .get(&outcome.operation)
                .is_none_or(|prior| later_control_outcome(prior, &acted))
            {
                store.acted(acted)?;
            }
            Ok(())
        })?;
    }
    Ok(())
}
