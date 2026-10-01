use std::sync::{Arc, Mutex};

use super::{Callers, LaunchRecords, StartOwners, StartService};

impl StartService {
    /// Construct a service sharing the pass table its withdrawals end.
    #[must_use]
    pub fn new_with_passes(
        owners: StartOwners,
        callers: Box<dyn Callers>,
        launches: LaunchRecords,
        clock: fn() -> u64,
        passes: Arc<Mutex<crate::agent_pass_store::Passes>>,
    ) -> Self {
        Self {
            pass_state: Some(passes),
            ..Self::new(owners, callers, launches, clock)
        }
    }
}
